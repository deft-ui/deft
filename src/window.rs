pub mod page;
pub mod popup;

use crate as deft;
use crate::app::{exit_app, AppEvent, InsetType};
use crate::base::MouseEventType::{MouseClick, MouseUp};
use crate::base::{Callback, EventContext, EventHandler, EventListener, EventRegistration, JsValueContext, MouseDetail, MouseEventType, Rect, ResultWaiter, Touch, TouchDetail};
use crate::cursor::search_cursor;
use crate::element::body::Body;
use crate::element::util::get_tree_level;
use crate::element::{Element, ElementBackend, ElementParent};
use crate::error::{DeftError, DeftResult};
use crate::event::{
    build_modifier, named_key_to_str, str_to_named_key, BlurEvent, ClickEvent, ContextMenuEvent,
    DragOverEvent, DragStartEvent, DropEvent, DroppedFileEvent, FocusEvent, FocusShiftEvent,
    HoveredFileEvent, KeyDownEvent, KeyEventDetail, KeyUpEvent, MouseDownEvent, MouseEnterEvent,
    MouseLeaveEvent, MouseMoveEvent, MouseUpEvent, MouseWheelEvent, TextInputEvent,
    TouchCancelEvent, TouchEndEvent, TouchMoveEvent, TouchStartEvent, KEY_MOD_ALT, KEY_MOD_CTRL,
    KEY_MOD_META, KEY_MOD_SHIFT,
};
use crate::event_loop::run_with_event_loop;
use crate::ext::ext_window::{
    WindowAttrs, MODAL_TO_OWNERS, WINDOWS, WINDOW_TYPE_MENU, WINDOW_TYPE_NORMAL, WINIT_TO_WINDOW,
};
use crate::frame_rate::FrameRateController;
use crate::js::{BorrowFromJs, FromJsValue, JsError};
use crate::paint::{PaintContext, Painter, RenderTree};
use crate::platform::support_multiple_windows;
use crate::render::painter::ElementPainter;
use crate::resource_table::ResourceTable;
use crate::state::{State, StateManager, StateMutRef};
use crate::style::length::LengthContext;
use crate::style::style_vars::StyleVars;
use crate::timer::{set_timeout_nanos, TimerHandle};
use crate::window::page::Page;
use crate::window::popup::Popup;
use crate::{
    base, bind_js_event_listener, ok_or_return, send_app_event, show_focus_hint, some_or_return,
    warn_time,
};
use anyhow::Error;
use deft_macros::{js_methods, window_event};
use log::{debug, error};
use quick_js::{JsValue, ValueError};
use skia_safe::{Color, Point};
use skia_window::renderer::Renderer;
use skia_window::skia_window::{RenderBackendType, SkiaWindow};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::string::ToString;
use std::time::SystemTime;
use std::{env, mem};
use winit::dpi::Position::Logical;
use winit::dpi::{LogicalPosition, LogicalSize, Size};
use winit::event::{
    ElementState, Ime, Modifiers, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent,
};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
#[cfg(windows)]
use winit::platform::windows::WindowExtWindows;
#[cfg(x11_platform)]
use winit::platform::x11::WindowAttributesExtX11;
use winit::window::{
    Cursor, CursorIcon, Fullscreen, ResizeDirection, Theme, WindowAttributes, WindowButtons,
    WindowId,
};
use crate::tooltip::Tooltip;

thread_local! {
    static WIN_STATE_MANAGER: RefCell<StateManager> = RefCell::new(StateManager::new());
}

#[derive(Clone)]
struct MouseDownInfo {
    button_enum: MouseButton,
    button: i32,
    window_x: f32,
    window_y: f32,
}

struct TouchingInfo {
    start_time: SystemTime,
    times: u32,
    max_identifiers: usize,
    start_point: (f32, f32),
    scrolled: bool,
    touches: HashMap<u64, Touch>,
}

fn treat_mouse_as_touch() -> bool {
    std::env::var("MOUSE_AS_TOUCH")
        .unwrap_or("0".to_string())
        .as_str()
        != "0"
}

#[derive(PartialEq)]
pub enum WindowType {
    Normal,
    Menu,
}

#[derive(Clone, Debug)]
pub struct LayerRoot {
    body: Element,
    x: f32,
    y: f32,
    focusing: Element,
    focusable: bool,
}

impl LayerRoot {
    pub fn new(element: Element, x: f32, y: f32) -> Self {
        let focusing = element.clone();
        Self { body: element, x, y, focusing, focusable: true }
    }

    pub fn new_not_focusable(element: Element, x: f32, y: f32) -> Self {
        let focusing = element.clone();
        Self { body: element, x, y, focusing, focusable: false }
    }

}

pub struct Window {
    handle: WindowHandle,
    id: i32,
    pub(crate) window: SkiaWindow,
    cursor_position: LogicalPosition<f64>,
    pub(crate) window_type: WindowType,
    cursor_root_position: LogicalPosition<f64>,
    pages: Vec<Page>,
    layer_roots: Vec<LayerRoot>,
    focusing: Option<Element>,
    /// (element, button)
    pressing: Option<(Element, MouseDownInfo)>,
    drag_window_called: bool,
    touching: TouchingInfo,
    dragging: bool,
    last_drag_over: Option<Element>,
    hover: Option<Element>,
    modifiers: Modifiers,
    dirty: bool,
    layout_dirty_list: HashMap<u32, Element>,
    repaint_timer_handle: Option<TimerHandle>,
    event_registration: EventRegistration<WindowHandle>,
    attributes: WindowAttributes,
    init_width: Option<f32>,
    init_height: Option<f32>,
    background_color: Color,
    pub renderer_idle: bool,
    next_frame_callbacks: Vec<Callback>,
    next_paint_callbacks: Vec<Callback>,
    pub render_tree: HashMap<Element, RenderTree>,
    pub style_vars: StyleVars,
    frame_rate_controller: FrameRateController,
    next_frame_timer_handle: Option<TimerHandle>,
    resource_table: ResourceTable,
    render_backend_types: Vec<RenderBackendType>,
    /// (ElementId, Tooltip)
    tooltip_instance: Option<(u32, Tooltip)>,
}

#[derive(Clone, PartialEq)]
pub struct WindowHandle {
    state: State<Window>,
}

impl WindowHandle {
    pub fn upgrade_mut(&self) -> DeftResult<StateMutRef<Window>> {
        Ok(self.state.upgrade_mut()?)
    }
}

pub type WindowEventHandler = EventHandler<WindowHandle>;
pub type WindowEventContext = EventContext<WindowHandle>;

thread_local! {
    pub static NEXT_WINDOW_ID: Cell<i32> = Cell::new(1);
}

#[window_event]
pub struct WindowResizeEvent {
    pub width: u32,
    pub height: u32,
}

#[window_event]
pub struct WindowCloseEvent;

#[window_event]
pub struct WindowFocusEvent;

#[window_event]
pub struct WindowBlurEvent;

impl BorrowFromJs for Window {
    fn borrow_from_js<R, F: FnOnce(&mut Self) -> R>(
        value: JsValue,
        receiver: F,
    ) -> Result<R, ValueError> {
        let window = WindowHandle::from_js_value(value)?;
        let mut wi = window
            .upgrade_mut()
            .map_err(|_| ValueError::UnexpectedType)?;
        Ok(receiver(&mut wi))
    }
}

#[js_methods]
impl Window {
    #[js_func]
    pub fn create(attrs: WindowAttrs) -> Result<WindowHandle, DeftError> {
        Self::create_with_raw_attrs(attrs, winit::window::Window::default_attributes())
    }

    pub fn create_with_raw_attrs(
        attrs: WindowAttrs,
        raw_attrs: WindowAttributes,
    ) -> Result<WindowHandle, DeftError> {
        let handle = Self::create_inner(attrs, raw_attrs);
        let mut ws = handle.upgrade_mut()?;
        send_app_event(AppEvent::BindWindow(ws.get_id())).unwrap();
        ws.update_inset(InsetType::Ime, Rect::new_empty());
        ws.update_inset(InsetType::Navigation, Rect::new_empty());
        ws.update_inset(InsetType::StatusBar, Rect::new_empty());

        let winit_window_id = ws.get_window_id();
        let wid = ws.get_id();
        WINIT_TO_WINDOW.with_borrow_mut(|m| m.insert(winit_window_id, handle.clone()));
        WINDOWS.with_borrow_mut(|m| m.insert(wid, handle.clone()));
        Ok(handle)
    }

    #[js_func]
    pub fn support_multiple_windows() -> bool {
        support_multiple_windows()
    }

    fn create_inner(attrs: WindowAttrs, mut attributes: WindowAttributes) -> WindowHandle {
        let id = NEXT_WINDOW_ID.get();
        NEXT_WINDOW_ID.set(id + 1);

        if let Some(t) = &attrs.title {
            attributes.title = t.to_string();
        } else {
            attributes.title = "".to_string();
        }
        if let Some(closable) = attrs.closable {
            attributes
                .enabled_buttons
                .set(WindowButtons::CLOSE, closable);
        }
        if let Some(minimizable) = attrs.minimizable {
            attributes
                .enabled_buttons
                .set(WindowButtons::MINIMIZE, minimizable);
        }
        if let Some(maximizable) = attrs.maximizable {
            attributes
                .enabled_buttons
                .set(WindowButtons::MAXIMIZE, maximizable);
        }

        attributes.visible = attrs.visible.unwrap_or(true);
        attributes.resizable = attrs.resizable.unwrap_or(true);
        if !attributes.resizable {
            attributes.enabled_buttons.remove(WindowButtons::MAXIMIZE);
        }
        attributes.decorations = attrs.decorations.unwrap_or(true);
        #[cfg(desktop_platform)]
        if attrs.width.is_some() || attrs.height.is_some() {
            let (default_width, default_height) = if attributes.resizable {
                (800.0, 600.0)
            } else {
                (50.0, 50.0)
            };
            let size = LogicalSize {
                width: attrs.width.unwrap_or(default_width) as f64,
                height: attrs.height.unwrap_or(default_height) as f64,
            };
            attributes.inner_size = Some(Size::Logical(size));
        }
        #[cfg(x11_platform)]
        {
            attributes =
                attributes.with_override_redirect(attrs.override_redirect.unwrap_or(false));
        }
        if let Some(position) = attrs.position {
            attributes.position = Some(Logical(LogicalPosition {
                x: position.0 as f64,
                y: position.1 as f64,
            }));
        }
        let window_type = match attrs
            .window_type
            .unwrap_or(WINDOW_TYPE_NORMAL.to_string())
            .as_str()
        {
            WINDOW_TYPE_MENU => WindowType::Menu,
            _ => WindowType::Normal,
        };

        let user_pf_backends = match attrs.preferred_renderers {
            None => Vec::new(),
            Some(str) => {
                RenderBackendType::from_str_list(&str.iter().map(|it| it.as_str()).collect())
            }
        };
        let env_pf_backends = RenderBackendType::from_split_str(
            &env::var("DEFT_RENDERERS").unwrap_or("".to_string()),
        );
        let render_backend_types = RenderBackendType::merge(&user_pf_backends, &env_pf_backends);
        let render_backend_types =
            RenderBackendType::merge(&render_backend_types, &RenderBackendType::all());
        let window = Self::create_window(attributes.clone(), &render_backend_types);
        // window.set_ime_allowed(true);
        let body = Element::create(Body::create);
        let mut render_tree = HashMap::new();
        render_tree.insert(body.clone(), RenderTree::new(0));
        let handle = WindowHandle {
            state: State::invalid(),
        };
        let state = WIN_STATE_MANAGER.with_borrow_mut(|wsm| {
            let mut win_info = Window {
                handle,
                id,
                window,
                cursor_position: LogicalPosition { x: 0.0, y: 0.0 },
                cursor_root_position: LogicalPosition { x: 0.0, y: 0.0 },
                layer_roots: vec![LayerRoot::new(body, 0.0, 0.0)],
                pressing: None,
                focusing: None,
                hover: None,
                modifiers: Modifiers::default(),
                dirty: false,
                dragging: false,
                last_drag_over: None,
                event_registration: EventRegistration::new(),
                attributes,
                touching: TouchingInfo {
                    start_time: SystemTime::now(),
                    times: 0,
                    max_identifiers: 0,
                    touches: Default::default(),
                    scrolled: false,
                    start_point: (0.0, 0.0),
                },
                window_type,
                init_width: attrs.width,
                init_height: attrs.height,
                background_color: Color::from_rgb(0, 0, 0),
                repaint_timer_handle: None,
                renderer_idle: true,
                next_frame_callbacks: Vec::new(),
                next_paint_callbacks: Vec::new(),
                render_tree,
                style_vars: StyleVars::new(),
                frame_rate_controller: FrameRateController::new(),
                next_frame_timer_handle: None,
                resource_table: ResourceTable::new(),
                drag_window_called: false,
                render_backend_types,
                layout_dirty_list: HashMap::new(),
                pages: Vec::new(),
                tooltip_instance: None,
            };
            win_info.on_resize();
            wsm.new_state(win_info)
        });
        let handle = WindowHandle {
            state: state.clone(),
        };
        let mut ws = state.upgrade_mut().unwrap();
        ws.handle = handle.clone();
        handle
    }

    pub fn inner_position(&self) -> (f32, f32) {
        let pos = ok_or_return!(self.window.inner_position(), (0.0, 0.0));
        let pos = pos.to_logical(self.window.scale_factor());
        (pos.x, pos.y)
    }

    pub fn update_inset(&mut self, ty: InsetType, rect: Rect) {
        let name = match ty {
            InsetType::Ime => "deft-ime-height",
            InsetType::StatusBar => "deft-status-height",
            InsetType::Navigation => "deft-navigation-height",
        };
        let height = rect.height() / self.window.scale_factor() as f32;
        debug!("updating style variable: {} {}", name, height);
        self.style_vars.set(name, &format!("{:.6}", height));
        for mut lr in self.layer_roots.clone() {
            lr.body.mark_style_dirty();
        }
    }

    pub fn resume(&mut self) {
        self.window = Self::create_window(self.attributes.clone(), &self.render_backend_types);
        #[cfg(ohos)]
        crate::platform::resume_ime();
    }

    pub fn notify_update(&mut self) {
        if !self.dirty {
            self.dirty = true;
            send_app_event(AppEvent::Update(self.get_id())).unwrap();
        }
    }

    pub fn invalid_layout(&mut self, element: Element) {
        // Note: Uncomment to debug layout problems
        // if self.layout_dirty_list.is_empty() { crate::trace::print_trace("layout dirty") }
        self.layout_dirty_list.insert(element.get_eid(), element);
        self.notify_update();
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_window_id(&self) -> WindowId {
        self.window.id()
    }

    #[js_func]
    pub fn set_modal(&mut self, owner: WindowHandle) -> Result<(), JsError> {
        let owner_state = owner.state.upgrade_mut()?;
        self.window.set_modal(&owner_state.window);
        #[cfg(windows)]
        owner_state.window.set_enable(false);
        let window_id = self.get_window_id();
        MODAL_TO_OWNERS.with_borrow_mut(|m| m.insert(window_id, owner));
        Ok(())
    }

    #[js_func]
    pub fn close(&mut self) -> Result<(), JsError> {
        let window_id = self.get_window_id();
        if self.allow_close() {
            WINIT_TO_WINDOW.with_borrow_mut(|m| m.remove(&window_id));
            #[allow(unused)]
            if let Some(modal_parent) = MODAL_TO_OWNERS.with_borrow_mut(|m| m.remove(&window_id)) {
                #[cfg(windows_platform)]
                if let Ok(p) = modal_parent.upgrade_mut() {
                    p.window.set_enable(true);
                }
            }
            WINDOWS.with_borrow_mut(|m| {
                m.remove(&self.get_id());
                if m.is_empty() {
                    let _ = exit_app(0);
                }
            });
            WIN_STATE_MANAGER.with_borrow_mut(|m| {
                m.remove_state(&self.handle.state);
            });
            self.window.set_visible(false);
        }
        Ok(())
    }

    #[js_func]
    pub fn set_visible(&mut self, visible: bool) -> Result<(), JsError> {
        self.window.set_visible(visible);
        Ok(())
    }

    #[js_func]
    pub fn is_visible(&self) -> Option<bool> {
        self.window.is_visible()
    }

    #[js_func]
    fn request_fullscreen(&mut self) {
        self.window
            .set_fullscreen(Some(Fullscreen::Borderless(None)));
    }

    #[js_func]
    fn exit_fullscreen(&mut self) {
        self.window.set_fullscreen(None);
    }

    #[js_func]
    fn is_fullscreen(&self) -> bool {
        self.window.fullscreen().is_some()
    }

    #[js_func]
    fn drag(&mut self) {
        self.drag_window_called = true;
        let _ = self.window.drag_window();
    }

    #[js_func]
    fn set_maximized(&mut self, maximized: bool) {
        self.window.set_maximized(maximized);
    }

    #[js_func]
    fn is_maximized(&self) -> bool {
        self.window.is_maximized()
    }

    #[js_func]
    fn set_minimized(&mut self, minimized: bool) {
        self.window.set_minimized(minimized);
    }

    #[js_func]
    fn is_minimized(&self) -> Option<bool> {
        self.window.is_minimized()
    }

    #[js_func]
    pub fn set_js_context(&mut self, context: JsValue) {
        self.resource_table.put(JsValueContext { context });
    }

    #[js_func]
    pub fn get_js_context(&self) -> Result<JsValue, Error> {
        let e = self
            .resource_table
            .get::<JsValueContext>()
            .map(|e| e.context.clone())
            .unwrap_or(JsValue::Undefined);
        Ok(e)
    }

    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.window.set_cursor_visible(visible);
    }

    #[js_func]
    pub fn set_outer_position(&mut self, x: f32, y: f32) {
        self.window.set_outer_position(Logical(LogicalPosition {
            x: x as f64,
            y: y as f64,
        }))
    }

    #[js_func]
    pub fn get_outer_position(&self) -> Result<(f32, f32), JsError> {
        let p: LogicalPosition<f32> = self
            .window
            .outer_position()?
            .to_logical(self.window.scale_factor());
        Ok((p.x, p.y))
    }

    #[js_func]
    pub fn get_monitor_size(&self) -> (f32, f32) {
        if let Some(monitor) = self.window.current_monitor() {
            let size = monitor.size().to_logical(monitor.scale_factor());
            (size.width, size.height)
        } else {
            (0.0, 0.0)
        }
    }

    #[js_func]
    pub fn popup(&self, content: Element, target: base::Rect) -> Popup {
        Popup::new(content, target, &self.handle)
    }

    pub fn popup_ex(&self, content: Element, target: base::Rect, focusable: bool) -> Popup {
        Popup::new_ex(content, target, &self.handle, focusable)
    }

    pub fn allow_close(&mut self) -> bool {
        let ctx = self.emit(WindowCloseEvent);
        !ctx.prevent_default
    }

    pub fn handle_input(&mut self, content: &str) {
        if let Some(focusing) = &self.focusing {
            focusing.emit(TextInputEvent(content.to_string()));
        }
    }

    pub fn handle_key(
        &mut self,
        modifiers: u32,
        scancode: Option<u32>,
        named_key: Option<NamedKey>,
        key: Option<String>,
        key_str: Option<String>,
        repeat: bool,
        pressed: bool,
    ) {
        let detail = KeyEventDetail {
            scancode,
            modifiers,
            ctrl_key: modifiers & KEY_MOD_CTRL != 0,
            alt_key: modifiers & KEY_MOD_ALT != 0,
            meta_key: modifiers & KEY_MOD_META != 0,
            shift_key: modifiers & KEY_MOD_SHIFT != 0,
            named_key,
            key_str,
            key,
            repeat,
            pressed,
        };

        if let Some(focusing) = &self.focusing {
            if detail.pressed {
                focusing.emit(KeyDownEvent(detail));
            } else {
                focusing.emit(KeyUpEvent(detail));
            }
        }
    }

    pub fn handle_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {
                self.dirty = true;
                self.update();
            }
            WindowEvent::Resized(_physical_size) => {
                self.on_resize();
            }
            WindowEvent::ModifiersChanged(new_modifiers) => self.modifiers = new_modifiers,
            WindowEvent::Ime(ime) => match ime {
                Ime::Enabled => {}
                Ime::Preedit(_, _) => {}
                Ime::Commit(str) => {
                    self.handle_input(&str);
                }
                Ime::Disabled => {}
            },
            WindowEvent::KeyboardInput { event, .. } => {
                let scancode = match event.physical_key {
                    PhysicalKey::Code(c) => get_scancode(c),
                    PhysicalKey::Unidentified(_e) => None,
                };
                let key = match &event.logical_key {
                    Key::Named(n) => Some(named_key_to_str(n).to_string()),
                    Key::Character(c) => Some(c.as_str().to_string()),
                    Key::Unidentified(_) => None,
                    Key::Dead(_) => None,
                };
                let key_str = match &event.logical_key {
                    Key::Character(c) => Some(c.as_str().to_string()),
                    _ => None,
                };
                let named_key = match event.logical_key {
                    Key::Named(n) => Some(n),
                    _ => None,
                };
                let mut modifiers = build_modifier(&self.modifiers.state());
                let pressed = event.state == ElementState::Pressed;
                if pressed && named_key == Some(NamedKey::Control) {
                    modifiers |= KEY_MOD_CTRL;
                }
                if pressed && named_key == Some(NamedKey::Shift) {
                    modifiers |= KEY_MOD_SHIFT;
                }
                if pressed
                    && (named_key == Some(NamedKey::Super) || named_key == Some(NamedKey::Meta))
                {
                    modifiers |= KEY_MOD_META;
                }
                if pressed && named_key == Some(NamedKey::Alt) {
                    modifiers |= KEY_MOD_ALT;
                }
                let repeat = event.repeat;
                let pressed = event.state == ElementState::Pressed;
                self.handle_key(
                    modifiers, scancode, named_key, key, key_str, repeat, pressed,
                );
            }
            WindowEvent::MouseInput { button, state, .. } => {
                // debug!("mouse:{:?}:{:?}", button, state);
                if let Some((dir, _)) = self.get_resize_direction() {
                    if let Err(e) = self.window.drag_resize_window(dir) {
                        error!("Failed to drag resize window: {:?}", e);
                    }
                    return;
                }
                if treat_mouse_as_touch() {
                    match state {
                        ElementState::Pressed => {
                            self.emit_touch_event(
                                0,
                                TouchPhase::Started,
                                self.cursor_position.x as f32,
                                self.cursor_position.y as f32,
                            );
                        }
                        ElementState::Released => {
                            self.emit_touch_event(
                                0,
                                TouchPhase::Ended,
                                self.cursor_position.x as f32,
                                self.cursor_position.y as f32,
                            );
                        }
                    }
                } else {
                    self.emit_click(button, state);
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if self.drag_window_called {
                    self.drag_window_called = false;
                    if let Some((_, m)) = &self.pressing {
                        self.emit_click(m.button_enum, ElementState::Released);
                    }
                }
            }
            WindowEvent::CursorMoved {
                position,
                root_position,
                ..
            } => {
                //debug!("cursor moved:{:?}", position);
                self.cursor_position = position.to_logical(self.window.scale_factor());
                self.cursor_root_position = root_position.to_logical(self.window.scale_factor());
                if treat_mouse_as_touch() {
                    if !self.touching.touches.is_empty() {
                        self.emit_touch_event(
                            0,
                            TouchPhase::Moved,
                            self.cursor_position.x as f32,
                            self.cursor_position.y as f32,
                        );
                    }
                } else {
                    self.handle_cursor_moved();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        self.handle_mouse_wheel((x, y));
                    }
                    MouseScrollDelta::PixelDelta(_) => {}
                }
                // debug!("delta:{:?}", delta);
            }
            WindowEvent::Touch(touch) => {
                let loc = touch.location.to_logical(self.window.scale_factor());
                self.emit_touch_event(touch.id, touch.phase, loc.x, loc.y);
            }
            WindowEvent::Focused(focus) => {
                if focus {
                    self.emit(WindowFocusEvent);
                } else {
                    self.emit(WindowBlurEvent);
                }
            }
            WindowEvent::DroppedFile(path) => {
                // println!("dropped file: {:?}", path);
                self.emit_dropped_file_event(
                    self.cursor_position.x,
                    self.cursor_position.y,
                    path.to_string_lossy().to_string(),
                );
            }
            WindowEvent::HoveredFile(path) => {
                self.emit_hovered_file_event(
                    self.cursor_position.x,
                    self.cursor_position.y,
                    path.to_string_lossy().to_string(),
                );
            }
            _ => (),
        }
    }

    pub fn add_event_listener(
        &mut self,
        event_type: &str,
        handler: Box<WindowEventHandler>,
    ) -> u32 {
        self.event_registration
            .add_event_listener(event_type, handler)
    }

    pub fn bind_event_listener<T: 'static, F: FnMut(&mut WindowEventContext, &mut T) + 'static>(
        &mut self,
        event_type: &str,
        handler: F,
    ) -> u32 {
        self.event_registration
            .bind_event_listener(event_type, handler)
    }

    #[js_func]
    pub fn bind_js_event_listener(
        &mut self,
        event_type: String,
        listener: JsValue,
    ) -> Result<u32, JsError> {
        let id = bind_js_event_listener!(
            self, event_type.as_str(), listener;
            "resize" => WindowResizeEventListener,
            "close"  => WindowCloseEventListener,
            "focus"  => WindowFocusEventListener,
            "blur"   => WindowBlurEventListener,
        );
        let id = id.ok_or_else(|| JsError::new(format!("unknown event_type:{}", event_type)))?;
        Ok(id)
    }

    #[js_func]
    pub fn unbind_js_event_listener(&mut self, id: u32) {
        self.event_registration.unregister_event_listener(id)
    }

    pub fn register_event_listener<T: 'static, H: EventListener<T, WindowHandle> + 'static>(
        &mut self,
        listener: H,
    ) -> u32 {
        self.event_registration.register_event_listener(listener)
    }

    pub fn unregister_event_listener(&mut self, id: u32) {
        self.event_registration.unregister_event_listener(id)
    }

    #[js_func]
    pub fn remove_event_listener(&mut self, event_type: String, id: u32) {
        self.event_registration
            .remove_event_listener(&event_type, id)
    }

    pub fn on_element_removed(&mut self, _element: &Element) {
        if let Some(f) = &self.focusing {
            if f.get_window().is_none() {
                let lr = self.get_focused_layer();
                self.focus(lr.body.clone());
            }
        }
    }

    fn get_focused_layer(&self) -> &LayerRoot {
        let layer_count = self.layer_roots.len();
        for i in 0..layer_count {
            let lr = &self.layer_roots[layer_count - i - 1];
            if lr.focusable {
                return lr;
            }
        }
        panic!("no focused layer found");
    }

    fn get_focused_layer_mut(&mut self) -> &mut LayerRoot {
        let layer_count = self.layer_roots.len();
        for i in 0..layer_count {
            let idx = layer_count - i - 1;
            if self.layer_roots[idx].focusable {
                return &mut self.layer_roots[idx];
            }
        }
        panic!("no focused layer found");
    }

    fn handle_mouse_wheel(&mut self, delta: (f32, f32)) {
        let (target_node, _, _) = self.get_node_by_point();
        target_node.emit(MouseWheelEvent {
            cols: delta.0,
            rows: delta.1,
        });
    }

    fn handle_cursor_moved(&mut self) {
        let window_x = self.cursor_position.x as f32;
        let window_y = self.cursor_position.y as f32;
        let screen_x = self.cursor_root_position.x as f32;
        let screen_y = self.cursor_root_position.y as f32;

        if self.attributes.resizable {
            if let Some((_, icon)) = self.get_resize_direction() {
                self.window.set_cursor(Cursor::Icon(icon));
                return;
            }
        }

        let mut target_node = self.get_node_by_point();
        let dragging = self.dragging;
        if let Some((pressing, down_info)) = &mut self.pressing.clone() {
            if dragging {
                let (target, _, _) = &mut target_node;
                if target != pressing {
                    target.emit(DragOverEvent {});
                    self.last_drag_over = Some(target.clone());
                }
            } else {
                if pressing.is_draggable()
                    && (f32::abs(window_x - down_info.window_x) > 3.0
                        || f32::abs(window_y - down_info.window_y) > 3.0)
                {
                    pressing.emit(DragStartEvent);
                    //TODO check preventDefault?
                    self.window.set_cursor(Cursor::Icon(CursorIcon::Grabbing));
                    self.dragging = true;
                } else {
                    self.update_cursor(pressing);
                    self.emit_mouse_event(
                        pressing,
                        MouseEventType::MouseMove,
                        0,
                        window_x,
                        window_y,
                        screen_x,
                        screen_y,
                    );
                }
            }
            //TODO should emit mouseenter|mouseleave?
        } else {
            let (mut node, _, _) = target_node;
            self.update_cursor(&node);
            if let Some(hover) = &mut self.hover.clone() {
                if hover != &node {
                    //Leave node
                    self.emit_mouse_event(
                        hover,
                        MouseEventType::MouseLeave,
                        0,
                        window_x,
                        window_y,
                        screen_x,
                        screen_y,
                    );
                    self.mouse_enter_node(node.clone(), window_x, window_y, screen_x, screen_y);
                } else {
                    self.emit_mouse_event(
                        &mut node,
                        MouseEventType::MouseMove,
                        0,
                        window_x,
                        window_y,
                        screen_x,
                        screen_y,
                    );
                }
            } else {
                self.mouse_enter_node(node.clone(), window_x, window_y, screen_x, screen_y);
            }
        }
    }

    fn get_resize_direction(&self) -> Option<(ResizeDirection, CursorIcon)> {
        if self.pressing.is_some() || self.window.is_decorated() {
            return None;
        }
        let window_x = self.cursor_position.x as f32;
        let window_y = self.cursor_position.y as f32;
        let size = self.window.outer_size();
        let scale_factor = self.window.scale_factor() as f32;
        let win_width = size.width as f32 / scale_factor;
        let win_height = size.height as f32 / scale_factor;
        let delta = 3.0;
        let is_left = window_x < delta;
        let is_right = window_x > win_width - delta;
        let is_top = window_y < delta;
        let is_bottom = window_y > win_height - delta;
        let (dir, icon) = if is_left && is_top {
            (ResizeDirection::NorthWest, CursorIcon::NwResize)
        } else if is_left && is_bottom {
            (ResizeDirection::SouthWest, CursorIcon::SwResize)
        } else if is_right && is_top {
            (ResizeDirection::NorthEast, CursorIcon::NeResize)
        } else if is_right && is_bottom {
            (ResizeDirection::SouthEast, CursorIcon::SeResize)
        } else if is_left {
            (ResizeDirection::West, CursorIcon::WResize)
        } else if is_top {
            (ResizeDirection::North, CursorIcon::NResize)
        } else if is_right {
            (ResizeDirection::East, CursorIcon::EResize)
        } else if is_bottom {
            (ResizeDirection::South, CursorIcon::SResize)
        } else {
            return None;
        };
        Some((dir, icon))
    }

    fn update_cursor(&mut self, node: &Element) {
        let cursor = search_cursor(node);
        //TODO cache?
        self.window.set_cursor(cursor)
    }

    fn mouse_enter_node(
        &mut self,
        mut node: Element,
        offset_x: f32,
        offset_y: f32,
        screen_x: f32,
        screen_y: f32,
    ) {
        self.emit_mouse_event(
            &mut node,
            MouseEventType::MouseEnter,
            0,
            offset_x,
            offset_y,
            screen_x,
            screen_y,
        );
        if let Some((eid, tooltip, target)) = Self::find_tooltip(&node, offset_x) {
            let is_same = self.tooltip_instance.as_ref().map(|ti| ti.0 == eid).unwrap_or(false);
            if !is_same {
                self.tooltip_instance = Some((eid, Tooltip::new(self.handle.clone(), tooltip, target)));
            }
        } else {
            self.tooltip_instance = None;
        }
        self.hover = Some(node);
    }

    fn find_tooltip(node: &Element, x: f32) -> Option<(u32, String, Rect)> {
        if !node.tooltip.is_empty() {
            let mut bounds = node.get_origin_bounds();
            bounds.x = x;
            bounds.y -= 4.0;
            bounds.width = 1.0;
            bounds.height += 8.0;
            Some((node.get_eid(), node.tooltip.clone(), bounds))
        } else if let Some(p) = node.get_parent() {
            Self::find_tooltip(&p, x)
        } else {
            None
        }
    }

    fn is_pressing(&self, node: &Element) -> bool {
        match &self.pressing {
            None => false,
            Some((p, _)) => p == node,
        }
    }

    pub fn emit_click(&mut self, mouse_button: MouseButton, state: ElementState) {
        //TODO to logical?
        let window_x = self.cursor_position.x as f32;
        let window_y = self.cursor_position.y as f32;
        let screen_x = self.cursor_root_position.x as f32;
        let screen_y = self.cursor_root_position.y as f32;
        //TODO impl

        let (mut node, _, _) = self.get_node_by_point();
        let (_e_type, event_type) = match state {
            ElementState::Pressed => ("mousedown", MouseEventType::MouseDown),
            ElementState::Released => ("mouseup", MouseEventType::MouseUp),
        };
        let button = match mouse_button {
            MouseButton::Left => 1,
            MouseButton::Right => 2,
            MouseButton::Middle => 3,
            MouseButton::Back => 4,
            MouseButton::Forward => 5,
            MouseButton::Other(_) => 6,
        };
        match state {
            ElementState::Pressed => {
                self.pressing = Some((
                    node.clone(),
                    MouseDownInfo {
                        button,
                        button_enum: mouse_button,
                        window_x,
                        window_y,
                    },
                ));
                self.emit_mouse_event(
                    &mut node, event_type, button, window_x, window_y, screen_x, screen_y,
                );
            }
            ElementState::Released => {
                if let Some(mut pressing) = self.pressing.clone() {
                    self.emit_mouse_event(
                        &mut pressing.0,
                        MouseUp,
                        button,
                        window_x,
                        window_y,
                        screen_x,
                        screen_y,
                    );
                    if pressing.0 == node && pressing.1.button == button {
                        let ty = match mouse_button {
                            MouseButton::Left => Some(MouseEventType::MouseClick),
                            MouseButton::Right => Some(MouseEventType::ContextMenu),
                            _ => None,
                        };
                        if let Some(ty) = ty {
                            self.emit_mouse_event(
                                &mut node, ty, button, window_x, window_y, screen_x, screen_y,
                            );
                        }
                    }
                    self.release_press();
                } else {
                    self.emit_mouse_event(
                        &mut node, MouseUp, button, window_x, window_y, screen_x, screen_y,
                    );
                }
            }
        }
        if state == ElementState::Released {
            if let Some(pressing) = &mut self.pressing.clone() {
                self.emit_mouse_event(
                    &mut pressing.0,
                    MouseUp,
                    pressing.1.button,
                    window_x,
                    window_y,
                    screen_x,
                    screen_y,
                );
                self.release_press();
            }
        }
    }

    pub fn emit_dropped_file_event(
        &mut self,
        window_x: f64,
        window_y: f64,
        path: String,
    ) -> Option<()> {
        let (node, _relative_x, _relative_y) =
            self.get_node_by_pos(window_x as f32, window_y as f32)?;
        node.emit(DroppedFileEvent(path));
        Some(())
    }

    pub fn emit_hovered_file_event(
        &mut self,
        window_x: f64,
        window_y: f64,
        path: String,
    ) -> Option<()> {
        let (node, _relative_x, _relative_y) =
            self.get_node_by_pos(window_x as f32, window_y as f32)?;
        node.emit(HoveredFileEvent(path));
        Some(())
    }

    pub fn emit_touch_event(
        &mut self,
        identifier: u64,
        phase: TouchPhase,
        window_x: f32,
        window_y: f32,
    ) -> Option<()> {
        if let Some((node, relative_x, relative_y)) = self.get_node_by_pos(window_x, window_y) {
            let _e_type = match phase {
                TouchPhase::Started => "touchstart",
                TouchPhase::Ended => "touchend",
                TouchPhase::Moved => "touchmove",
                TouchPhase::Cancelled => "touchcancel",
            };
            let (border_top, _, _, border_left) = node.get_border_width();

            let offset_x = relative_x - border_left;
            let offset_y = relative_y - border_top;
            match phase {
                TouchPhase::Started => {
                    let touch_info = Touch {
                        identifier,
                        offset_x,
                        offset_y,
                        window_x,
                        window_y,
                    };
                    if self.touching.touches.is_empty() {
                        if SystemTime::now()
                            .duration_since(self.touching.start_time)
                            .unwrap()
                            .as_millis()
                            < 300
                        {
                            self.touching.times += 1;
                        } else {
                            self.touching.start_time = SystemTime::now();
                            self.touching.times = 1;
                        }
                    }
                    self.touching.touches.insert(identifier, touch_info);
                    self.touching.scrolled = false;
                    self.touching.start_point = (window_x, window_y);
                }
                TouchPhase::Moved => {
                    if let Some(e) = self.touching.touches.get_mut(&identifier) {
                        e.offset_x = offset_x;
                        e.offset_y = offset_y;
                        e.window_x = window_x;
                        e.window_y = window_y;
                    }
                    self.touching.scrolled = self.touching.scrolled
                        || (window_x - self.touching.start_point.0).abs() > 5.0
                        || (window_y - self.touching.start_point.1).abs() > 5.0;
                }
                TouchPhase::Cancelled => {
                    self.touching.touches.remove(&identifier);
                }
                TouchPhase::Ended => {
                    self.touching.touches.remove(&identifier);
                }
            }
            self.touching.max_identifiers =
                usize::max(self.touching.max_identifiers, self.touching.touches.len());
            let touches: Vec<Touch> = self.touching.touches.values().cloned().collect();
            let touch_detail = TouchDetail { touches };
            match phase {
                TouchPhase::Started => {
                    debug!("touch start:{:?}", touch_detail);
                    node.emit(TouchStartEvent(touch_detail));
                }
                TouchPhase::Moved => {
                    // debug!("touch move:{:?}", &touch_detail);
                    node.emit(TouchMoveEvent(touch_detail));
                }
                TouchPhase::Ended => {
                    // debug!("touch end:{:?}", &touch_detail);
                    node.emit(TouchEndEvent(touch_detail));
                    if self.touching.max_identifiers == 1
                        && self.touching.times == 1
                        && !self.touching.scrolled
                        && SystemTime::now()
                            .duration_since(self.touching.start_time)
                            .unwrap()
                            .as_millis()
                            < 1000
                    {
                        let mut node = node.clone();
                        debug!("clicked");
                        //TODO fix screen_x, screen_y
                        self.emit_mouse_event(
                            &mut node, MouseClick, 0, window_x, window_y, 0.0, 0.0,
                        );
                    }
                }
                TouchPhase::Cancelled => {
                    node.emit(TouchCancelEvent(touch_detail));
                }
            }
        }
        None
    }

    pub fn focus(&mut self, mut node: Element) {
        if !node.is_focusable() {
            if let Some(p) = node.get_parent() {
                self.focus(p);
            }
            return;
        }
        let last_layer = self.get_focused_layer();
        if &node.get_root_element() != &last_layer.body {
            return;
        }

        let focusing = Some(node.clone());
        if self.focusing != focusing {
            // debug!("focusing {:?}", node.get_id());
            let mut old_focusing = self.focusing.clone();
            let layer = self.get_focused_layer_mut();
            layer.focusing = node.clone();
            self.focusing = focusing;
            if let Some(old_focusing) = &mut old_focusing {
                old_focusing.emit(BlurEvent);

                old_focusing.emit(FocusShiftEvent);
                if show_focus_hint() {
                    old_focusing.mark_dirty(false);
                }
                old_focusing.update_select_style_recurse();
            }
            if show_focus_hint() {
                node.mark_dirty(false);
            }
            node.update_select_style_recurse();
            node.emit(FocusEvent);
            self.window.set_ime_allowed(node.allow_ime);
        }
    }

    pub fn is_focusing(&self, element: &Element) -> bool {
        self.focusing.as_ref() == Some(element)
    }

    fn release_press(&mut self) {
        let dragging = self.dragging;
        if let Some(_) = &mut self.pressing {
            if dragging {
                self.dragging = false;
                self.window.set_cursor(Cursor::Icon(CursorIcon::Default));
                if let Some(last_drag_over) = &mut self.last_drag_over {
                    last_drag_over.emit(DropEvent);
                }
            }
            self.pressing = None;
        }
    }

    #[js_func]
    pub fn get_inner_size(&self) -> (f32, f32) {
        let scale_factor = self.window.scale_factor();
        let size = self.window.inner_size().to_logical(scale_factor);
        (size.width, size.height)
    }

    fn update_layout(&mut self, mut roots: Vec<Element>) {
        let auto_size = !self.attributes.resizable;
        let (win_width, win_height) = self.get_inner_size();
        let width = if auto_size {
            self.init_width.unwrap_or(f32::NAN)
        } else {
            win_width
        };
        let height = if auto_size {
            self.init_height.unwrap_or(f32::NAN)
        } else {
            win_height
        };
        debug!("calculate layout, {} x {}", width, height);
        roots.sort_by(|a, b| {
            let level_a = get_tree_level(a);
            let level_b = get_tree_level(b);
            level_a.cmp(&level_b)
        });
        for mut root in roots {
            let (w, h) = match &root.parent {
                ElementParent::None => (f32::NAN, f32::NAN),
                ElementParent::Element(p) => {
                    let p_bounds = p.upgrade().unwrap().get_content_bounds();
                    (p_bounds.width, p_bounds.height)
                }
                ElementParent::Window(_) => (width, height),
                ElementParent::Page(_) => (f32::NAN, f32::NAN),
            };
            //TODO skip if calculated
            root.calculate_layout(w, h);
        }
        if auto_size {
            let lr = some_or_return!(self.layer_roots.first());
            let (final_width, final_height) = lr.body.get_size();
            if win_width as u32 != final_width as u32 || win_height as u32 != final_height as u32 {
                self.resize(crate::base::Size {
                    width: final_width,
                    height: final_height,
                });
            }
        }
    }

    pub fn update(&mut self) -> ResultWaiter<bool> {
        if !self.renderer_idle {
            return ResultWaiter::new_finished(false);
        }
        if self.next_frame_timer_handle.is_some() {
            return ResultWaiter::new_finished(false);
        }
        let sleep_time = self.frame_rate_controller.next_frame();
        if sleep_time > 0 {
            let me = self.handle.clone();
            let next_frame_timer_handle = set_timeout_nanos(
                move || {
                    if let Ok(mut me) = me.upgrade_mut() {
                        me.next_frame_timer_handle = None;
                        me.update_force();
                    }
                },
                sleep_time,
            );
            self.next_frame_timer_handle = Some(next_frame_timer_handle);
            return ResultWaiter::new_finished(false);
        }
        self.update_force()
    }

    fn update_force(&mut self) -> ResultWaiter<bool> {
        // print_time!("window update time");
        let mut frame_callbacks = Vec::new();
        frame_callbacks.append(&mut self.next_frame_callbacks);
        for cb in frame_callbacks {
            cb.call();
        }
        if !self.dirty {
            // skip duplicate update
            return ResultWaiter::new_finished(false);
        }
        let (viewport_width, viewport_height) = self.get_inner_size();
        warn_time!(16, "update window");
        for lr in &mut self.layer_roots.clone() {
            let body = &mut lr.body;
            let length_ctx = LengthContext {
                root: body.style.font_size,
                font_size: body.style.font_size,
                viewport_width,
                viewport_height,
            };
            //TODO compute font size only when any font size changed
            body.resolve_style_vars_recurse(&self.style_vars);
            body.compute_font_size_recurse(&length_ctx);
            body.apply_style_update(false, &length_ctx);
        }
        let dirty_roots = mem::take(&mut self.layout_dirty_list);
        let layout_dirty = !dirty_roots.is_empty();
        if layout_dirty {
            self.update_layout(dirty_roots.values().cloned().collect());
            //TODO should move to Popup?
            let win_size = self
                .window
                .inner_size()
                .to_logical(self.window.scale_factor());
            for i in 1..self.layer_roots.len() {
                let lr = &mut self.layer_roots[i];
                let (root, x, y) = (&mut lr.body, &mut lr.x, &mut lr.y);
                let bounds = root.get_bounds();
                if x.is_nan() {
                    *x = (win_size.width - bounds.width) / 2.0;
                    *y = (win_size.height - bounds.height) / 2.0;
                } else {
                    if bounds.width + *x > win_size.width {
                        *x = win_size.width - bounds.width;
                    }
                    if bounds.height + *y > win_size.height {
                        *y = win_size.height - bounds.height;
                    }
                }
            }
        }
        //TODO optimize performance
        // if layout_dirty {
        self.render_tree.clear();
        for lr in self.layer_roots.clone() {
            //TODO call before_renderer when layout is not dirty
            let mut body = lr.body;
            body.before_render_recurse();
            let rt = build_render_nodes(&mut body);
            let body = body.clone();
            self.render_tree.insert(body, rt);
        }
        // }
        let r = self.paint();
        self.layout_dirty_list.clear();
        self.dirty = false;
        r
    }

    #[js_func]
    pub fn set_body(&mut self, body: Element) -> DeftResult<()> {
        self.layer_roots[0] = LayerRoot::new(body.clone(), 0.0, 0.0);
        self.init_element_root(body, ElementParent::Window(self.handle.clone()));
        Ok(())
    }

    #[js_func]
    pub fn create_page(&mut self, element: Element, x: f32, y: f32) -> Page {
        self.create_page_ex(element, x, y, true)
    }

    pub fn create_page_ex(&mut self, element: Element, x: f32, y: f32, focusable: bool) -> Page {
        let page = Page::new(self.handle.clone(), element.clone());
        let body = page.get_body().clone();
        self.pages.push(page.clone());
        let root = if focusable {
            LayerRoot::new(body.clone(), x, y)
        } else {
            LayerRoot::new_not_focusable(body.clone(), x, y)
        };
        self.layer_roots.push(root);
        self.init_element_root(body, ElementParent::Page(self.handle.clone()));
        page
    }

    #[js_func]
    pub fn close_page(&mut self, page: Page) {
        self.pages.retain(|p| p != &page);
        self.layer_roots.retain(|e| &e.body != page.get_body());
        let new_layer = self.get_focused_layer();
        self.focus(new_layer.focusing.clone());
        self.notify_update();
        //TODO emit close event?
    }

    fn init_element_root(&mut self, mut body: Element, parent: ElementParent) {
        body.set_parent(parent);
        body.set_focusable(true);
        let theme = self.window.theme().unwrap_or(Theme::Light);
        let theme = match theme {
            Theme::Light => "light",
            Theme::Dark => "dark",
        };
        body.set_attribute("theme".to_string(), theme.to_string());
        // if self.focusing.is_none() {
        // TODO move focusing to page?
        self.focus(body.clone());
        // }
        self.invalid_layout(body);
    }

    #[js_func]
    pub fn get_body(&self) -> Option<Element> {
        Some(self.layer_roots[0].body.clone())
    }

    #[js_func]
    pub fn set_title(&mut self, title: String) {
        self.window.set_title(&title);
    }

    #[js_func]
    pub fn get_title(&self) -> String {
        self.window.title()
    }

    #[js_func]
    pub fn resize(&mut self, size: crate::base::Size) {
        let _ = self.window.request_inner_size(LogicalSize {
            width: size.width,
            height: size.height,
        });
    }

    fn on_resize(&mut self) {
        let size = self.window.inner_size();
        let (width, height) = (size.width, size.height);
        if width <= 0 || height <= 0 {
            return;
        }
        self.window.resize_surface(width, height);
        if let Some(body) = self.get_body() {
            self.invalid_layout(body.clone());
        }
        let scale_factor = self.window.scale_factor();
        self.emit(WindowResizeEvent {
            width: (width as f64 / scale_factor) as u32,
            height: (height as f64 / scale_factor) as u32,
        });
    }

    pub fn emit<T: 'static>(&mut self, event: T) -> EventContext<WindowHandle> {
        let mut ctx = EventContext::new(self.handle.clone());
        self.event_registration.emit(event, &mut ctx);
        ctx
    }

    pub fn request_next_frame_callback(&mut self, callback: Callback) {
        self.next_frame_callbacks.push(callback);
        if self.next_frame_callbacks.len() == 1 {
            send_app_event(AppEvent::Update(self.get_id())).unwrap();
        }
    }

    pub fn request_next_paint_callback(&mut self, callback: Callback) {
        self.next_paint_callbacks.push(callback);
        if self.next_paint_callbacks.len() == 1 {
            send_app_event(AppEvent::Update(self.get_id())).unwrap();
        }
    }

    fn paint(&mut self) -> ResultWaiter<bool> {
        let size = self.window.inner_size();
        let (width, height) = (size.width, size.height);
        // print_time!("paint time: {} {}", width, height);
        let waiter = ResultWaiter::new();
        if width <= 0 || height <= 0 {
            waiter.finish(false);
            return waiter;
        }
        let mut paint_callbacks = Vec::new();
        paint_callbacks.append(&mut self.next_paint_callbacks);
        for cb in paint_callbacks {
            cb.call();
        }
        let scale_factor = self.window.scale_factor() as f32;
        let background_color = self.background_color;
        let viewport = Rect::new(
            0.0,
            0.0,
            width as f32 / scale_factor,
            height as f32 / scale_factor,
        );
        //TODO support config
        let layer_cache_enabled = false;
        let mut paint_tree = Vec::new();
        for lr in &mut self.layer_roots.clone() {
            let (root, x, y) = (&mut lr.body, lr.x, lr.y);
            self.render_tree
                .get_mut(root)
                .unwrap()
                .rebuild_render_tree(root, layer_cache_enabled);
            let pt = self
                .render_tree
                .get_mut(root)
                .unwrap()
                .build_paint_tree(&viewport);
            //TODO notify absolute position change
            paint_tree.push((pt, x, y));
        }
        let waiter_finisher = waiter.clone();
        let window_id = self.get_id();
        self.renderer_idle = false;
        self.window.render_with_result(
            Renderer::new(move |canvas, ctx| {
                // print_time!("drawing time");
                canvas.save();
                if scale_factor != 1.0 {
                    canvas.scale((scale_factor, scale_factor));
                }
                canvas.clear(background_color);
                let mut element_painter = ElementPainter::take(ctx);
                element_painter.update_viewport(scale_factor, viewport);
                element_painter.set_layer_cache(layer_cache_enabled);
                let paint_context = PaintContext { scale_factor };
                let painter = Painter::new(canvas, paint_context);
                for (tree, x, y) in &mut paint_tree {
                    painter.canvas.save();
                    painter.canvas.translate((*x, *y));
                    element_painter.draw_root(&painter, tree, ctx);
                    painter.canvas.restore();
                }
                element_painter.put(ctx);
                canvas.restore();
            }),
            move |r| {
                waiter_finisher.finish(r);
                send_app_event(AppEvent::RenderIdle(window_id)).unwrap();
            },
        );
        waiter
    }

    #[inline]
    fn get_logical_len(&self, physical_len: f32) -> f32 {
        physical_len * self.window.scale_factor() as f32
    }

    fn get_node_by_point(&self) -> (Element, f32, f32) {
        let x = self.cursor_position.x as f32;
        let y = self.cursor_position.y as f32;
        self.get_node_by_pos(x, y).unwrap_or_else(|| {
            let element = self.get_focused_layer().body.clone();
            (element, x, y)
        })
    }

    fn get_element_by_id(&self, element: &Element, id: u32) -> Option<Element> {
        if element.get_eid() == id {
            return Some(element.clone());
        }
        for child in element.get_children() {
            if let Some(element) = self.get_element_by_id(&child, id) {
                return Some(element);
            }
        }
        None
    }

    fn get_node_by_pos(&self, window_x: f32, window_y: f32) -> Option<(Element, f32, f32)> {
        // debug!("search node time in layers");
        let lr = self.get_focused_layer();
        let (body, x, y) = (&lr.body, lr.x, lr.y);
        let (eo, x, y) = self
            .render_tree
            .get(body)?
            .get_element_object_by_pos(window_x - x, window_y - y)?;
        let element_id = eo.element_id;
        // debug!("found element id: {}", element_id);
        let element = self.get_element_by_id(&body, element_id)?;
        Some((element, x, y))
    }

    fn emit_mouse_event(
        &mut self,
        node: &mut Element,
        event_type_enum: MouseEventType,
        button: i32,
        window_x: f32,
        window_y: f32,
        screen_x: f32,
        screen_y: f32,
    ) {
        let root = node.get_root_element();
        let render_tree = some_or_return!(self.render_tree.get(&root));
        let node_matrix = some_or_return!(render_tree.get_element_total_matrix(node));
        let (border_top, _, _, border_left) = node.get_border_width();

        //TODO maybe not inverted?
        let inverted_matrix = node_matrix.invert().unwrap();

        let Point {
            x: relative_x,
            y: relative_y,
        } = inverted_matrix.map_xy(window_x, window_y);
        let off_x = relative_x - border_left;
        let off_y = relative_y - border_top;

        let detail = MouseDetail {
            event_type: event_type_enum,
            button,
            offset_x: off_x,
            offset_y: off_y,
            window_x,
            window_y,
            screen_x,
            screen_y,
        };
        match event_type_enum {
            MouseEventType::MouseDown => node.emit(MouseDownEvent(detail)),
            MouseEventType::MouseUp => node.emit(MouseUpEvent(detail)),
            MouseEventType::MouseClick => node.emit(ClickEvent(detail)),
            MouseEventType::ContextMenu => node.emit(ContextMenuEvent(detail)),
            MouseEventType::MouseMove => node.emit(MouseMoveEvent(detail)),
            MouseEventType::MouseEnter => node.emit(MouseEnterEvent(detail)),
            MouseEventType::MouseLeave => node.emit(MouseLeaveEvent(detail)),
        }
    }

    fn create_window(
        attributes: WindowAttributes,
        backend_types: &Vec<RenderBackendType>,
    ) -> SkiaWindow {
        run_with_event_loop(|el| {
            debug!("render backends: {:?}", backend_types);
            for bt in backend_types {
                let init_attributes = attributes.clone().with_visible(false);
                if let Some(sw) = SkiaWindow::new(el, init_attributes, *bt) {
                    if attributes.visible {
                        sw.set_visible(true);
                    }
                    debug!("created window with backend {:?}", bt);
                    return sw;
                } else {
                    debug!("failed to create window with backend: {:?}", bt);
                }
            }
            panic!("Failed to create window with backends: {:?}", backend_types);
        })
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        WIN_STATE_MANAGER.with_borrow_mut(|wsm| {
            wsm.remove_state(&self.handle.state);
        })
    }
}

fn get_scancode(code: KeyCode) -> Option<u32> {
    #[cfg(any(windows_platform, macos_platform, x11_platform, wayland_platform))]
    {
        use winit::platform::scancode::PhysicalKeyExtScancode;
        return code.to_scancode();
    }
    #[cfg(not(any(windows_platform, macos_platform, x11_platform, wayland_platform)))]
    {
        let _ = code;
        None
    }
}

pub fn build_render_nodes(root: &mut Element) -> RenderTree {
    let count = count_elements(root);
    let mut render_tree = RenderTree::new(count);
    root.need_snapshot = true;
    collect_render_nodes(root, &mut render_tree);
    render_tree
}

fn collect_render_nodes(root: &mut Element, tree: &mut RenderTree) {
    // build_render_paint_info(root, &mut result.invalid_rects_list, &mut invalid_rects_idx, &mut node);
    tree.create_node(root);
    let children = root.get_children();
    for mut child in children {
        collect_render_nodes(&mut child, tree);
    }
}

fn count_elements(root: &Element) -> usize {
    let mut elements_count = 1;
    let children = root.get_children();
    for child in children {
        elements_count += count_elements(&child);
    }
    elements_count
}

fn print_tree(node: &Element, padding: &str) {
    let name = &node.tag;
    let children = node.get_children();
    if children.is_empty() {
        debug!("{}{}", padding, name);
    } else {
        debug!("{}{}{}", padding, name, " {");
        for c in children {
            let c_padding = padding.to_string() + "  ";
            print_tree(&c, &c_padding);
        }
        debug!("{}{}", padding, "}");
    }
}

pub fn window_input(window_id: i32, content: String) {
    WINDOWS.with_borrow_mut(|m| {
        if let Some(f) = m.get_mut(&window_id) {
            if let Ok(mut f) = f.upgrade_mut() {
                f.handle_input(&content);
            }
        }
    });
}

pub fn window_send_key(window_id: i32, key: &str, pressed: bool) {
    if let Some(k) = str_to_named_key(&key) {
        WINDOWS.with_borrow_mut(|m| {
            if let Some(f) = m.get_mut(&window_id) {
                if let Ok(mut f) = f.upgrade_mut() {
                    //FIXME scancode
                    f.handle_key(
                        0,
                        None,
                        Some(k),
                        Some(key.to_string()),
                        None,
                        false,
                        pressed,
                    );
                }
            }
        });
    }
}

pub fn window_update_inset(window_id: i32, ty: InsetType, rect: Rect) {
    WINDOWS.with_borrow_mut(|m| {
        if let Some(f) = m.get_mut(&window_id) {
            if let Ok(mut f) = f.upgrade_mut() {
                f.update_inset(ty, rect);
                // f.mark_dirty_and_update_immediate(true).wait_finish();
            }
        }
    });
}

pub fn window_on_render_idle(window_id: i32) {
    WINDOWS.with_borrow_mut(|m| {
        if let Some(f) = m.get_mut(&window_id) {
            if let Ok(mut f) = f.upgrade_mut() {
                f.renderer_idle = true;
                f.update();
            }
        }
    });
}

pub fn window_check_update(window_id: i32) {
    WINDOWS.with_borrow_mut(|m| {
        if let Some(f) = m.get_mut(&window_id) {
            if let Ok(mut f) = f.upgrade_mut() {
                f.update();
            }
        }
    });
}
