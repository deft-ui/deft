use crate as lento;
use crate::app::{exit_app, AppEvent, AppEventPayload};
use crate::base::MouseEventType::{MouseClick, MouseUp};
use crate::base::{ElementEvent, Event, EventContext, EventHandler, EventListener, EventRegistration, MouseDetail, MouseEventType, ResultWaiter, Touch, TouchDetail, UnsafeFnOnce};
use crate::canvas_util::CanvasHelper;
use crate::cursor::search_cursor;
use crate::element::{Element, ElementWeak, PaintInfo};
use crate::event::{build_modifier, named_key_to_str, BlurEvent, CaretChangeEventListener, ClickEvent, ContextMenuEvent, DragOverEvent, DragStartEvent, DropEvent, FocusEvent, FocusShiftEvent, KeyDownEvent, KeyEventDetail, KeyUpEvent, MouseDownEvent, MouseEnterEvent, MouseLeaveEvent, MouseMoveEvent, MouseUpEvent, MouseWheelEvent, TextInputEvent, TouchCancelEvent, TouchEndEvent, TouchMoveEvent, TouchStartEvent, KEY_MOD_ALT, KEY_MOD_CTRL, KEY_MOD_META, KEY_MOD_SHIFT};
use crate::event_loop::{create_event_loop_proxy, run_with_event_loop};
use crate::ext::common::create_event_handler;
use crate::ext::ext_frame::{FrameAttrs, FRAMES, FRAME_TYPE_MENU, FRAME_TYPE_NORMAL, MODAL_TO_OWNERS, WINDOW_TO_FRAME};
use crate::js::JsError;
use crate::mrc::{Mrc, MrcWeak};
use crate::renderer::CpuRenderer;
use crate::timer::{set_timeout, set_timeout_nanos, TimerHandle};
use anyhow::{anyhow, Error};
use lento_macros::{event, frame_event, js_func, js_methods, mrc_object};
use measure_time::print_time;
use quick_js::{JsValue, ResourceValue};
use skia_bindings::{SkCanvas_SrcRectConstraint, SkClipOp, SkPathOp, SkRect};
use skia_safe::{Canvas, Color, ColorType, IRect, Image, ImageInfo, Matrix, Paint, Path, Point, Rect};
use skia_window::skia_window::{RenderBackendType, SkiaWindow};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::num::NonZeroU32;
use std::ops::{Deref, DerefMut};
use std::process::exit;
use std::rc::Rc;
use std::slice;
use std::string::ToString;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use skia_bindings::SkPaint_Style::{Fill, Stroke};
use skia_safe::canvas::SetMatrix;
use skia_safe::wrapper::NativeTransmutableWrapper;
use winit::dpi::Position::Logical;
use winit::dpi::{LogicalPosition, LogicalSize, Position, Size};
use winit::error::ExternalError;
use winit::event::{ElementState, Ime, Modifiers, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::{Key, NamedKey};
#[cfg(feature = "x11")]
use winit::platform::x11::WindowAttributesExtX11;
use winit::window::{Cursor, CursorGrabMode, CursorIcon, Window, WindowAttributes, WindowId};
use crate::{bind_js_event_listener, is_snapshot_usable};
use crate::frame_rate::{get_total_frames, next_frame, FRAME_RATE_CONTROLLER};
use crate::paint::{InvalidArea, PartialInvalidArea, Painter, RenderNode, RenderTree, SkiaPainter, UniqueRect, InvalidRects, RenderOp, Snapshot, SnapshotManager, MatrixCalculator, ClipPath};
use crate::style::ColorHelper;

#[derive(Clone)]
struct MouseDownInfo {
    button: i32,
    frame_x: f32,
    frame_y: f32,
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
    std::env::var("MOUSE_AS_TOUCH").unwrap_or("0".to_string()).as_str() != "0"
}

#[derive(PartialEq)]
pub enum FrameType {
    Normal,
    Menu,
}

pub enum InvalidMode<'a> {
    Full,
    Rect(&'a Rect),
    UniqueRect(&'a UniqueRect),
}

#[mrc_object]
pub struct Frame {
    id: i32,
    pub(crate) window: SkiaWindow,
    cursor_position: LogicalPosition<f64>,
    pub(crate) frame_type: FrameType,
    cursor_root_position: LogicalPosition<f64>,
    body: Option<Element>,
    snapshots: SnapshotManager,
    focusing: Option<Element>,
    /// (element, button)
    pressing: Option<(Element, MouseDownInfo)>,
    touching: TouchingInfo,
    dragging: bool,
    last_drag_over: Option<Element>,
    hover: Option<Element>,
    modifiers: Modifiers,
    layout_dirty: bool,
    invalid_area: InvalidArea,
    repaint_timer_handle: Option<TimerHandle>,
    event_registration: EventRegistration<FrameWeak>,
    attributes: WindowAttributes,
    init_width: Option<f32>,
    init_height: Option<f32>,
    ime_height: f32,
    background_color: Color,
    pub render_tree: Arc<Mutex<RenderTree>>,
}

pub type FrameEventHandler = EventHandler<FrameWeak>;
pub type FrameEventContext = EventContext<FrameWeak>;

thread_local! {
    pub static NEXT_FRAME_ID: Cell<i32> = Cell::new(1);
}

#[frame_event]
pub struct FrameResizeEvent {
    pub width: u32,
    pub height: u32,
}

#[frame_event]
pub struct FrameCloseEvent;

#[frame_event]
pub struct FrameFocusEvent;

#[frame_event]
pub struct FrameBlurEvent;

#[js_methods]
impl Frame {

    #[js_func]
    pub fn create(attrs: FrameAttrs) -> Result<Self, Error> {
        let frame = Frame::create_inner(attrs);
        let window_id = frame.get_window_id();
        FRAMES.with_borrow_mut(|m| m.insert(frame.get_id(), frame.clone()));
        WINDOW_TO_FRAME.with_borrow_mut(|m| m.insert(window_id, frame.as_weak()));
        Ok(frame)
    }

    fn create_inner(attrs: FrameAttrs) -> Self {
        let id = NEXT_FRAME_ID.get();
        NEXT_FRAME_ID.set(id + 1);

        let mut attributes = Window::default_attributes();
        if let Some(t) = &attrs.title {
            attributes.title = t.to_string();
        } else {
            attributes.title = "".to_string();
        }
        attributes.visible = attrs.visible.unwrap_or(true);
        attributes.resizable = attrs.resizable.unwrap_or(true);
        attributes.decorations = attrs.decorations.unwrap_or(true);
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
        #[cfg(feature = "x11")]
        {
            attributes = attributes.with_override_redirect(attrs.override_redirect.unwrap_or(false));
        }
        if let Some(position) = attrs.position {
            attributes.position = Some(Logical(LogicalPosition {
                x: position.0 as f64,
                y: position.1 as f64,
            }));
        }
        let frame_type = match attrs.frame_type.unwrap_or(FRAME_TYPE_NORMAL.to_string()).as_str() {
            FRAME_TYPE_MENU => FrameType::Menu,
            _ => FrameType::Normal,
        };

        let window = Self::create_window(attributes.clone());
        let state = FrameData {
            id,
            window,
            cursor_position: LogicalPosition {
                x: 0.0,
                y: 0.0,
            },
            cursor_root_position: LogicalPosition {
                x: 0.0,
                y: 0.0,
            },
            body: None,
            pressing: None,
            focusing: None,
            hover: None,
            modifiers: Modifiers::default(),
            layout_dirty: false,
            invalid_area: InvalidArea::None,
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
            frame_type,
            init_width: attrs.width,
            init_height: attrs.height,
            ime_height: 0.0,
            background_color: Color::from_rgb(0, 0, 0),
            repaint_timer_handle: None,
            snapshots: SnapshotManager::new(),
            render_tree: Arc::new(Mutex::new(RenderTree::new())),
        };
        let mut handle = Frame {
            inner: Mrc::new(state),
        };
        // handle.body.set_window(Some(win.clone()));
        handle.on_resize();
        handle
    }

    pub fn resume(&mut self) {
        self.window = Self::create_window(self.attributes.clone());
    }

    pub fn remove_unique_invalid_rect(&mut self, rect: &UniqueRect) {
        self.invalid_area.remove_unique_rect(rect);
    }

    pub fn invalid(&mut self, mode: InvalidMode) {
        let is_first_invalid = self.invalid_area == InvalidArea::None;
        let snapshot_usable = is_snapshot_usable();
        match mode {
            InvalidMode::Full => {
                self.invalid_area.add_rect(None);
            }
            InvalidMode::Rect(r) => {
                if snapshot_usable {
                    self.invalid_area.add_rect(Some(r));
                } else {
                    self.invalid_area.add_rect(None);
                }
            }
            InvalidMode::UniqueRect(ur) => {
                if snapshot_usable {
                    self.invalid_area.add_unique_rect(ur);
                } else {
                    self.invalid_area.add_rect(None);
                }
            }
        }
        if is_first_invalid {
            let time_to_wait = next_frame();

            let me = self.as_weak();
            //TODO use another timer
            self.repaint_timer_handle = Some(set_timeout_nanos(move || {
                if let Ok(mut me) = me.upgrade() {
                    me.update();
                }
            }, time_to_wait));
        }
    }

    pub fn invalid_layout(&mut self) {
        // Note: Uncomment to debug layout problems
        // if layout_dirty && !self.layout_dirty { crate::trace::print_trace("layout dirty") }
        self.layout_dirty = true;
        self.invalid(InvalidMode::Full);
    }

    pub fn mark_dirty_and_update_immediate(&mut self, layout_dirty: bool) -> ResultWaiter<bool> {
        if let Some(body) = &mut self.body {
            body.mark_dirty(true);
        }
        self.update()
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_window_id(&self) -> WindowId {
        self.window.id()
    }

    #[js_func]
    pub fn set_modal(&mut self, owner: Frame) -> Result<(), JsError> {
        self.window.set_modal(&owner.window);
        let frame_id = self.get_window_id();
        MODAL_TO_OWNERS.with_borrow_mut(|m| m.insert(frame_id, owner.get_window_id()));
        Ok(())
    }

    #[js_func]
    pub fn close(&mut self) -> Result<(), JsError> {
        let window_id = self.get_window_id();
        if self.allow_close() {
            WINDOW_TO_FRAME.with_borrow_mut(|m| m.remove(&window_id));
            MODAL_TO_OWNERS.with_borrow_mut(|m| m.remove(&window_id));
            FRAMES.with_borrow_mut(|m| {
                m.remove(&self.get_id());
                if m.is_empty() {
                    let _ = exit_app(0);
                }
            });
        }
        Ok(())
    }

    #[js_func]
    pub fn set_visible(&mut self, visible: bool) -> Result<(), JsError> {
        self.window.set_visible(visible);
        Ok(())
    }
    
    pub fn allow_close(&mut self) -> bool {
        let ctx = self.emit(FrameCloseEvent);
        !ctx.prevent_default
    }

    pub fn handle_input(&mut self, content: &str) {
        if let Some(focusing) = &mut self.focusing {
            focusing.emit(TextInputEvent(content.to_string()));
        }
    }

    pub fn handle_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {
                self.invalid(InvalidMode::Full);
                self.paint();
            }
            WindowEvent::Resized(_physical_size) => {
                self.on_resize();
            }
            WindowEvent::ModifiersChanged(new_modifiers) => self.modifiers = new_modifiers,
            WindowEvent::Ime(ime) => {
                match ime {
                    Ime::Enabled => {}
                    Ime::Preedit(_, _) => {}
                    Ime::Commit(str) => {
                        println!("input:{}", str);
                        self.handle_input(&str);
                    }
                    Ime::Disabled => {}
                }
            }
            WindowEvent::KeyboardInput {
                event,
                ..
            } => {
                let key = match &event.logical_key {
                    Key::Named(n) => {Some(named_key_to_str(n).to_string())},
                    Key::Character(c) => Some(c.as_str().to_string()),
                    Key::Unidentified(_) => {None}
                    Key::Dead(_) => {None},
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
                if pressed && (named_key == Some(NamedKey::Super) || named_key == Some(NamedKey::Meta)) {
                    modifiers |= KEY_MOD_META;
                }
                if pressed && named_key == Some(NamedKey::Alt) {
                    modifiers |= KEY_MOD_ALT;
                }
                let detail = KeyEventDetail {
                    modifiers ,
                    ctrl_key: modifiers & KEY_MOD_CTRL != 0 ,
                    alt_key:  modifiers & KEY_MOD_ALT != 0,
                    meta_key: modifiers & KEY_MOD_META != 0,
                    shift_key:modifiers & KEY_MOD_SHIFT != 0,
                    named_key,
                    key_str,
                    key,
                    repeat: event.repeat,
                    pressed: event.state == ElementState::Pressed,
                };

                if let Some(focusing) = &mut self.focusing {
                    if detail.pressed {
                        focusing.emit(KeyDownEvent(detail));
                    } else {
                        focusing.emit(KeyUpEvent(detail));
                    }
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                // println!("mouse:{:?}:{:?}", button, state);
                if treat_mouse_as_touch() {
                    match state {
                        ElementState::Pressed => {
                            self.emit_touch_event(0, TouchPhase::Started, self.cursor_position.x as f32, self.cursor_position.y as f32);
                        }
                        ElementState::Released => {
                            self.emit_touch_event(0, TouchPhase::Ended, self.cursor_position.x as f32, self.cursor_position.y as f32);
                        }
                    }
                } else {
                    self.emit_click(button, state);
                }
            }
            WindowEvent::CursorMoved { position, root_position, .. } => {
                //println!("cursor moved:{:?}", position);
                self.cursor_position = position.to_logical(self.window.scale_factor());
                self.cursor_root_position = root_position.to_logical(self.window.scale_factor());
                if treat_mouse_as_touch() {
                    if !self.touching.touches.is_empty() {
                        self.emit_touch_event(0, TouchPhase::Moved, self.cursor_position.x as f32, self.cursor_position.y as f32);
                    }
                } else {
                    self.handle_cursor_moved();
                }
            }
            WindowEvent::MouseWheel {delta,..} => {
                match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        self.handle_mouse_wheel((x, y));
                    }
                    MouseScrollDelta::PixelDelta(_) => {}
                }
                // println!("delta:{:?}", delta);
            }
            WindowEvent::Touch(touch) => {
                let loc = touch.location.to_logical(self.window.scale_factor());
                self.emit_touch_event(touch.id, touch.phase, loc.x, loc.y);
            }
            WindowEvent::Focused(focus) => {
                let target = self.as_weak();
                if focus {
                    self.emit(FrameFocusEvent);
                } else {
                    self.emit(FrameBlurEvent);
                }
            }
            _ => (),
        }
    }

    pub fn add_event_listener(&mut self, event_type: &str, handler: Box<FrameEventHandler>) -> u32 {
        self.event_registration.add_event_listener(event_type, handler)
    }

    pub fn bind_event_listener<T: 'static, F: FnMut(&mut FrameEventContext, &mut T) + 'static>(&mut self, event_type: &str, handler: F) -> u32 {
        self.event_registration.bind_event_listener(event_type, handler)
    }

    #[js_func]
    pub fn bind_js_event_listener(&mut self, event_type: String, listener: JsValue) -> Result<u32, JsError> {
        let id = bind_js_event_listener!(
            self, event_type.as_str(), listener;
            "resize" => FrameResizeEventListener,
            "close"  => FrameCloseEventListener,
            "focus"  => FrameFocusEventListener,
            "blur"   => FrameBlurEventListener,
        );
        Ok(id)
    }

    #[js_func]
    pub fn unbind_js_event_listener(&mut self, id: u32) {
        self.event_registration.unregister_event_listener(id)
    }

    pub fn register_event_listener<T: 'static, H: EventListener<T, FrameWeak> + 'static>(&mut self, mut listener: H) -> u32 {
        self.event_registration.register_event_listener(listener)
    }

    pub fn unregister_event_listener(&mut self, id: u32) {
        self.event_registration.unregister_event_listener(id)
    }

    #[js_func]
    pub fn remove_event_listener(&mut self, event_type: String, id: u32) {
        self.event_registration.remove_event_listener(&event_type, id)
    }

    fn handle_mouse_wheel(&mut self, delta: (f32, f32)) {
        if let Some(mut target_node) = self.get_node_by_point() {
            target_node.emit(MouseWheelEvent { cols: delta.0, rows: delta.1 });
        }

    }

    fn handle_cursor_moved(&mut self) {
        let frame_x = self.cursor_position.x as f32;
        let frame_y = self.cursor_position.y as f32;
        let screen_x = self.cursor_root_position.x as f32;
        let screen_y = self.cursor_root_position.y as f32;
        let mut target_node = self.get_node_by_point();
        let dragging = self.dragging;
        let render_tree = self.render_tree.clone();
        if let Some((pressing, down_info)) = &mut self.pressing.clone() {
            if dragging {
                if let Some(target) = &mut target_node {
                    if target != pressing {
                        target.emit(DragOverEvent {});
                        self.last_drag_over = Some(target.clone());
                    }
                }
            } else {
                if pressing.is_draggable() && (
                    f32::abs(frame_x - down_info.frame_x) > 3.0
                    || f32::abs(frame_y - down_info.frame_y) > 3.0
                ) {
                    pressing.emit(DragStartEvent);
                    //TODO check preventDefault?
                    self.window.set_cursor(Cursor::Icon(CursorIcon::Grabbing));
                    self.dragging = true;
                } else {
                    self.update_cursor(pressing);
                    emit_mouse_event(&render_tree, pressing, MouseEventType::MouseMove, 0, frame_x, frame_y, screen_x, screen_y);
                }
            }
            //TODO should emit mouseenter|mouseleave?
        } else if let Some(mut node) = target_node {
            self.update_cursor(&node);
            if let Some(hover) = &mut self.hover {
                if hover != &node {
                    emit_mouse_event(&render_tree, hover, MouseEventType::MouseLeave, 0, frame_x, frame_y, screen_x, screen_y);
                    self.mouse_enter_node(node.clone(), frame_x, frame_y, screen_x, screen_y);
                } else {
                    emit_mouse_event(&render_tree, &mut node, MouseEventType::MouseMove, 0, frame_x, frame_y, screen_x, screen_y);
                }
            } else {
                self.mouse_enter_node(node.clone(), frame_x, frame_y, screen_x, screen_y);
            }
        }
    }

    fn update_cursor(&mut self, node: &Element) {
        let cursor = search_cursor(node);
        //TODO cache?
        self.window.set_cursor(Cursor::Icon(cursor))
    }

    fn mouse_enter_node(&mut self, mut node: Element, offset_x: f32, offset_y: f32, screen_x: f32, screen_y: f32) {
        let render_tree = self.render_tree.clone();
        emit_mouse_event(&render_tree, &mut node, MouseEventType::MouseEnter, 0, offset_x, offset_y, screen_x, screen_y);
        self.hover = Some(node);
    }

    fn is_pressing(&self, node: &Element) -> bool {
        match &self.pressing {
            None => false,
            Some((p,_)) => p == node
        }
    }

    pub fn emit_click(&mut self, mouse_button: MouseButton, state: ElementState) {
        //TODO to logical?
        let frame_x = self.cursor_position.x as f32;
        let frame_y = self.cursor_position.y as f32;
        let screen_x = self.cursor_root_position.x as f32;
        let screen_y = self.cursor_root_position.y as f32;
        let render_tree = self.render_tree.clone();
        //TODO impl

        if let Some(mut node) = self.get_node_by_point() {
            let (e_type, event_type) = match state {
                ElementState::Pressed =>("mousedown", MouseEventType::MouseDown),
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
                    self.focus(node.clone());
                    self.pressing = Some((node.clone(), MouseDownInfo {button, frame_x, frame_y}));
                    emit_mouse_event(&render_tree, &mut node, event_type, button, frame_x, frame_y, screen_x, screen_y);
                }
                ElementState::Released => {
                    if let Some(mut pressing) = self.pressing.clone() {
                        emit_mouse_event(&render_tree, &mut pressing.0, MouseUp, button, frame_x, frame_y, screen_x, screen_y);
                        if pressing.0 == node && pressing.1.button == button {
                            let ty = match mouse_button {
                                MouseButton::Left => Some(MouseEventType::MouseClick),
                                MouseButton::Right => Some(MouseEventType::ContextMenu),
                                _ => None
                            };
                            if let Some(ty) = ty {
                                emit_mouse_event(&render_tree, &mut node, ty, button, frame_x, frame_y, screen_x, screen_y);
                            }
                        }
                        self.release_press();
                    } else {
                        emit_mouse_event(&render_tree, &mut node, MouseUp, button, frame_x, frame_y, screen_x, screen_y);
                    }
                }
            }
        }
        if state == ElementState::Released {
            if let Some(pressing) = &mut self.pressing {
                emit_mouse_event(&render_tree, &mut pressing.0, MouseUp, pressing.1.button, frame_x, frame_y, screen_x, screen_y);
                self.release_press();
            }
        }
    }

    pub fn emit_touch_event(&mut self, identifier: u64, phase: TouchPhase, frame_x: f32, frame_y: f32) -> Option<()> {
        if let Some(mut node) = self.get_node_by_pos(frame_x, frame_y) {
            let _e_type = match phase {
                TouchPhase::Started => "touchstart",
                TouchPhase::Ended => "touchend",
                TouchPhase::Moved => "touchmove",
                TouchPhase::Cancelled => "touchcancel",
            };
            let node_bounds = node.get_origin_bounds();
            let (border_top, _, _, border_left) = node.get_border_width();

            let Point {x: relative_x, y: relative_y} = {
                let render_tree = self.render_tree.clone();
                let render_tree = render_tree.lock().unwrap();
                let render_node = render_tree.get_by_element_id(node.get_id())?;
                let inverted_matrix = render_node.total_matrix.invert()?;
                inverted_matrix.map_xy(frame_x, frame_y)
            };
            let offset_x = relative_x - border_left;
            let offset_y = relative_y - border_top;
            match phase {
                TouchPhase::Started => {
                    let touch_info = Touch {
                        identifier,
                        offset_x,
                        offset_y,
                        frame_x,
                        frame_y,
                    };
                    if self.touching.touches.is_empty() {
                        if SystemTime::now().duration_since(self.touching.start_time).unwrap().as_millis() < 300 {
                            self.touching.times += 1;
                        } else {
                            self.touching.start_time = SystemTime::now();
                            self.touching.times = 1;
                        }
                    }
                    self.touching.touches.insert(identifier, touch_info);
                    self.touching.scrolled = false;
                    self.touching.start_point = (frame_x, frame_y);
                }
                TouchPhase::Moved => {
                    if let Some(e) = self.touching.touches.get_mut(&identifier) {
                        e.offset_x = offset_x;
                        e.offset_y = offset_y;
                        e.frame_x = frame_x;
                        e.frame_y = frame_y;
                    }
                    self.touching.scrolled = self.touching.scrolled
                        || (frame_x - self.touching.start_point.0).abs() > 5.0
                        || (frame_y - self.touching.start_point.1).abs() > 5.0;
                }
                TouchPhase::Cancelled => {
                    self.touching.touches.remove(&identifier);
                }
                TouchPhase::Ended => {
                    self.touching.touches.remove(&identifier);
                }
            }
            self.touching.max_identifiers = usize::max(self.touching.max_identifiers, self.touching.touches.len());
            let touches: Vec<Touch> = self.touching.touches.values().cloned().collect();
            let touch_detail = TouchDetail { touches };
            match phase {
                TouchPhase::Started => {
                    println!("touch start:{:?}", touch_detail);
                    node.emit(TouchStartEvent(touch_detail));
                }
                TouchPhase::Moved => {
                    // println!("touch move:{:?}", &touch_detail);
                    node.emit(TouchMoveEvent(touch_detail));
                }
                TouchPhase::Ended => {
                    println!("touch end:{:?}", &touch_detail);
                    node.emit(TouchEndEvent(touch_detail));
                    if self.touching.max_identifiers == 1
                        && self.touching.times == 1
                        && !self.touching.scrolled
                        && SystemTime::now().duration_since(self.touching.start_time).unwrap().as_millis() < 1000
                    {
                        let mut node = node.clone();
                        self.focus(node.clone());
                        println!("clicked");
                        let render_tree = self.render_tree.clone();
                        //TODO fix screen_x, screen_y
                        emit_mouse_event(&render_tree, &mut node, MouseClick, 0, frame_x, frame_y, 0.0, 0.0);
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
        let focusing = Some(node.clone());
        if self.focusing != focusing {
            if let Some(old_focusing) = &mut self.focusing {
                old_focusing.emit(BlurEvent);

                old_focusing.emit(FocusShiftEvent);
            }
            self.focusing = focusing;
            node.emit(FocusEvent);
        }
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

    pub fn update(&mut self) -> ResultWaiter<bool> {
        if self.invalid_area == InvalidArea::None {
            // skip duplicate update
            return ResultWaiter::new_finished(false);
        }
        print_time!("frame update time");
        let auto_size = !self.attributes.resizable;
        if self.layout_dirty {
            let size = self.window.inner_size();
            let scale_factor = self.window.scale_factor() as f32;
            let width = if auto_size {
                self.init_width.unwrap_or(f32::NAN)
            } else {
                size.width as f32 / scale_factor
            };
            let mut height = if auto_size {
                self.init_height.unwrap_or(f32::NAN)
            } else {
                size.height as f32 / scale_factor
            };
            height -= self.ime_height / scale_factor;
            if let Some(body) = &mut self.body {
                // print_time!("calculate layout, {} x {}", width, height);
                body.calculate_layout(width, height);
                if auto_size {
                    let (final_width, final_height) = body.get_size();
                    if size.width != final_width as u32 && size.height != final_height as u32 {
                        self.resize(crate::base::Size {
                            width: final_width,
                            height: final_height,
                        });
                    }
                }
            }
            // print_time!("collect element time");
        }
        let r = self.paint();
        self.layout_dirty = false;
        self.invalid_area = InvalidArea::None;
        r
    }

    #[js_func]
    pub fn set_body(&mut self, mut body: Element) {
        body.set_window(Some(self.as_weak()));
        self.focusing = Some(body.clone());

        //TODO unbind when change body
        let myself = self.as_weak();
        body.register_event_listener(CaretChangeEventListener::new(move |detail, e| {
            myself.upgrade_mut(|myself| {
                if myself.focusing == e.target.upgrade().ok() {
                    let origin_ime_rect = &detail.origin_bounds;
                    myself.window.set_ime_cursor_area(Position::Logical(LogicalPosition {
                        x: origin_ime_rect.x as f64,
                        y: origin_ime_rect.bottom() as f64,
                    }), Size::Logical(LogicalSize {
                        width: origin_ime_rect.width as f64,
                        height: origin_ime_rect.height as f64
                    }));
                }
            });
        }));
        self.body = Some(body);
        self.invalid_layout();
    }

    #[js_func]
    pub fn set_title(&mut self, title: String) {
        self.window.set_title(&title);
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
        self.invalid_layout();
        let scale_factor = self.window.scale_factor();
        self.emit(FrameResizeEvent {
            width: (width as f64 / scale_factor) as u32,
            height: (height as f64 / scale_factor) as u32,
        });
    }

    pub fn emit<T: 'static>(&mut self, mut event: T) -> EventContext<FrameWeak> {
        let mut ctx = EventContext {
            target: self.as_weak(),
            propagation_cancelled: false,
            prevent_default: false,
        };
        self.event_registration.emit(&mut event, &mut ctx);
        ctx
    }

    fn paint(&mut self) -> ResultWaiter<bool> {
        let size = self.window.inner_size();
        let (width, height) = (size.width, size.height);
        print_time!("paint time: {} {}", width, height);
        let waiter = ResultWaiter::new();
        if width <= 0 || height <= 0 {
            waiter.finish(false);
            return waiter;
        }
        let mut body = match self.body.clone() {
            Some(b) => b,
            None => {
                waiter.finish(false);
                return waiter;
            },
        };
        let start = SystemTime::now();
        let scale_factor = self.window.scale_factor() as f32;
        let background_color = self.background_color;
        let mut me = self.clone();
        let invalid_area = me.invalid_area.clone();
        let old_snapshots = me.snapshots.clone();
        let snapshots = SnapshotManager::new();
        me.snapshots = snapshots.clone();
        let viewport = Rect::new(0.0, 0.0, width as f32, height as f32);
        let mut render_tree = if let Some(body) = &mut me.body {
            // print_time!("build render nodes time");
            build_render_nodes(body, invalid_area, scale_factor, viewport.clone())
        } else {
            return ResultWaiter::new_finished(false);
        };
        let render_tree = Arc::new(Mutex::new(render_tree));
        self.render_tree = render_tree.clone();
        let waiter_finisher = waiter.clone();
        self.window.render_with_result(move |canvas| {
            //print_time!("render time");
            canvas.save();
            if scale_factor != 1.0 {
                canvas.scale((scale_factor, scale_factor));
            }
            let mut painter = SkiaPainter::new(canvas);
            let mut render_tree = render_tree.lock().unwrap();
            if !background_color.is_transparent() {
                canvas.save();
                painter.set_invalid_rects(render_tree.invalid_rects_list[0].build(viewport));
                canvas.clear(background_color);
                canvas.restore();
            }
            draw_elements(canvas, &mut render_tree, &mut painter, scale_factor, old_snapshots, snapshots);

            canvas.restore();
        }, move|r| {
            waiter_finisher.finish(r);
        });
        waiter
    }

    #[inline]
    fn get_logical_len(&self, physical_len: f32) -> f32 {
        physical_len * self.window.scale_factor() as f32
    }

    fn get_node_by_point(&self) -> Option<Element> {
        let x = self.cursor_position.x as f32;
        let y = self.cursor_position.y as f32;
        self.get_node_by_pos(x, y)
    }

    fn get_element_by_id(&self, element: &Element, id: u32) -> Option<Element> {
        if element.get_id() == id {
            return Some(element.clone());
        }
        for child in element.get_children() {
            if let Some(element) = self.get_element_by_id(&child, id) {
                return Some(element)
            }
        }
        None
    }

    fn get_node_by_pos(&self, x: f32, y: f32) -> Option<Element> {
        let mut render_tree = self.render_tree.clone();
        let render_tree = render_tree.lock().unwrap();
        let body = self.body.clone()?;
        for n in render_tree.nodes.iter().rev() {
            if n.absolute_transformed_visible_path.contains((x, y)) {
                return self.get_element_by_id(&body, n.element_id);
            }
        }
        None
    }

    fn create_window(attributes: WindowAttributes) -> SkiaWindow {
        run_with_event_loop(|el| {
            //TODO support RenderBackedType parameter#[cfg(not(target_os = "android"))]
            let backend_type = RenderBackendType::SoftBuffer;
            #[cfg(target_os = "android")]
            let backend_type = RenderBackendType::GL;
            SkiaWindow::new(el, attributes, backend_type)
        })
    }

}

pub struct WeakWindowHandle {
    inner: MrcWeak<FrameData>,
}

impl WeakWindowHandle {
    pub fn upgrade(&self) -> Option<Frame> {
        self.inner.upgrade().map(|i| Frame::from_inner(i)).ok()
    }
}

fn draw_elements(canvas: &Canvas,
                 tree: &mut RenderTree,
                 painter: &mut dyn Painter,
                 scale: f32,
                 old_snapshots: SnapshotManager,
                 new_snapshots: SnapshotManager,
) {
    let viewport = tree.viewport.clone();
    let mut last_invalid_rects_idx = None;
    canvas.save();
    // let ops = tree.ops.clone();
    println!("render ops length: {}", tree.ops.len());
    for op in &mut tree.ops {
        match op {
            RenderOp::Render(idx) => {
                let node = &mut tree.nodes[*idx];
                if last_invalid_rects_idx != Some(node.invalid_rects_idx) {
                    canvas.restore();
                    canvas.save();
                    painter.set_invalid_rects(tree.invalid_rects_list[node.invalid_rects_idx].build(viewport));
                    last_invalid_rects_idx = Some(node.invalid_rects_idx);
                }
                canvas.session(|c| {
                    draw_element(canvas, node, painter);
                });
                if let Some(snapshot) = &old_snapshots.remove(node.element_id) {
                    let (img_rect, img) = (&snapshot.rect, &snapshot.image);
                    if let Some((x, y, rect)) = &node.reuse_bounds {
                        let src_rect = Rect::from_xywh(
                            rect.left * scale,
                            rect.top * scale,
                            rect.width() * scale,
                            rect.height() * scale,
                        );
                        let dst_rect = rect.with_offset((img_rect.left - x, img_rect.top - y));
                        let dst_rect = Rect::from_xywh(
                            dst_rect.left * scale,
                            dst_rect.top * scale,
                            dst_rect.width() * scale,
                            dst_rect.height() * scale,
                        );
                        unimplemented!();
                        /*
                        let ob = node.origin_bounds;
                        let vp_path = node.children_viewport.as_ref().map(|r| Path::rect(r, None));
                        canvas.session(|c| {
                            let children_invalid_rects = tree.invalid_rects_list[node.children_invalid_rects_idx].build(viewport);
                            let children_invalid_path = children_invalid_rects.to_path(viewport);
                            c.translate((ob.left, ob.top));
                            if let Some(vp_path) = vp_path {
                                c.clip_path(&vp_path, SkClipOp::Intersect, false);
                            }
                            c.translate((-ob.left, -ob.top));
                            c.clip_path(&children_invalid_path, SkClipOp::Difference, false);
                            c.scale((1.0 / scale, 1.0 / scale));
                            c.draw_image_rect(img, Some((&src_rect, SkCanvas_SrcRectConstraint::Fast)), &dst_rect, &Paint::default());
                        });
                         */
                    }
                }
                // node.snapshot = None;
            }
            RenderOp::Finish(idx) => {
                let node = &mut tree.nodes[*idx];
                if is_snapshot_usable() && node.need_snapshot {
                    unimplemented!()
                    /*
                    unsafe {
                        if let Some(mut surface) = canvas.surface() {
                            //TODO result will be wrong if has transform
                            let mut bounds = node.origin_bounds;
                            if scale != 1.0 {
                                bounds.left *= scale;
                                bounds.top *= scale;
                                bounds.right *= scale;
                                bounds.bottom *= scale;
                            }
                            let i_bounds = IRect {
                                left: bounds.left as i32,
                                top: bounds.top as i32,
                                right: bounds.right() as i32,
                                bottom: bounds.bottom() as i32,
                            };

                            // wrong result when transform applied
                            if let Some(img) = surface.image_snapshot_with_bounds(&i_bounds) {
                                new_snapshots.insert(node.element_id, Snapshot::new(node.origin_bounds, img));
                            }
                            // let snapshot = surface.image_snapshot_with_bounds(&i_bounds)
                            //     .map(|img| (bounds, img));


                        }
                    }
                    */
                }
            }
        }
    }
    canvas.restore();
}

fn draw_element(canvas: &Canvas, node: &mut RenderNode, painter: &mut dyn Painter) {
    let width = node.width;
    let height = node.height;
    canvas.concat(&node.total_matrix);
    node.clip_path.apply(canvas);

    canvas.session(move |canvas| {
        // set clip path
        let clip_path = &node.border_box_path;
        canvas.clip_path(&clip_path, SkClipOp::Intersect, false);

        // draw background and border
        node.draw_background(&canvas);
        node.draw_border(&canvas);

        // draw padding box and content box
        canvas.save();
        if width > 0.0 && height > 0.0 {
            let (border_top_width, _, _, border_left_width) = node.border_width;
            // let (padding_top, _, _, padding_left) = element.get_padding();
            // draw content box
            canvas.translate((border_left_width, border_top_width));
            if let Some(render_fn) = node.render_fn.take() {
                render_fn.run(canvas);
            }
        }
        canvas.restore();
    });
}

fn build_render_nodes(root: &mut Element, mut invalid_area: InvalidArea, scale: f32,  viewport: Rect) -> RenderTree {
    let count = count_elements(root);
    let viewport_rect = UniqueRect::from_rect(viewport);
    invalid_area.add_unique_rect(&viewport_rect);
    let mut render_tree = RenderTree {
        invalid_rects_list: vec![invalid_area],
        nodes: Vec::with_capacity(count),
        ops: Vec::with_capacity(count * 2),
        viewport,
    };
    root.need_snapshot = true;
    root.invalid_unique_rect = Some(viewport_rect);
    let mut mc = MatrixCalculator::new();
    let bounds = root.get_bounds().to_skia_rect();
    collect_render_nodes(root, &mut render_tree, 0, scale, None, &mut mc, bounds);
    render_tree
}

fn collect_render_nodes(
    root: &mut Element,
    result: &mut RenderTree,
    mut invalid_rects_idx: usize,
    scale: f32,
    parent_node_idx: Option<usize>,
    matrix_calculator: &mut MatrixCalculator,
    bounds: Rect,
) {
    let invalid_rects = result.invalid_rects_list[invalid_rects_idx].build(result.viewport);
    let origin_bounds = root.get_origin_bounds().to_skia_rect();
    let border_box_path =  root.get_border_box_path();

    root.apply_transform(matrix_calculator);
    //TODO support overflow:visible
    matrix_calculator.intersect_clip_path(&ClipPath::from_path(&border_box_path));

    let total_matrix = matrix_calculator.get_total_matrix();
    let clip_path = matrix_calculator.get_clip_path().clone();

    let absolute_transformed_visible_path = clip_path.clip(&border_box_path).with_transform(&total_matrix);

    let (transformed_bounds, _) = total_matrix.map_rect(Rect::from_xywh(0.0, 0.0, bounds.width(), bounds.height()));
    if !invalid_rects.has_intersects(&transformed_bounds) {
        return;
    }

    let mut node = RenderNode {
        element_id: root.get_id(),
        invalid_rects_idx,
        absolute_transformed_visible_path,
        children_invalid_rects_idx: invalid_rects_idx,
        absolute_transformed_bounds: transformed_bounds,
        width: origin_bounds.width(),
        height: origin_bounds.height(),
        total_matrix,
        clip_path,
        border_width: root.get_border_width(),
        border_box_path,
        render_fn: Some(root.get_backend_mut().render()),
        background_image: root.style.background_image.clone(),
        background_color: root.style.computed_style.background_color,
        border_paths: root.style.get_border_paths(),
        border_color: root.style.border_color,
        children_viewport: root.get_children_viewport(),
        need_snapshot: root.need_snapshot,
        reuse_bounds: None,
    };
    let mut scroll_delta = (0.0, 0.0);
    if let Some(lpi) = &root.last_paint_info {
        scroll_delta.0 = root.scroll_left - lpi.scroll_left;
        scroll_delta.1 = root.scroll_top - lpi.scroll_top;
    }
    root.last_paint_info = Some(PaintInfo {
        scroll_left: root.scroll_left,
        scroll_top: root.scroll_top,
    });
    if let Some(ir) = &root.invalid_unique_rect {
        let old_origin_bounds = root.get_origin_padding_bounds();
        let reuse_bounds = old_origin_bounds.translate(scroll_delta.0, scroll_delta.1)
            .intersect(&old_origin_bounds)
            .translate(-old_origin_bounds.x, -old_origin_bounds.y).to_skia_rect();
        if !reuse_bounds.is_empty() {
            node.reuse_bounds = Some((
                scroll_delta.0,
                scroll_delta.1,
                reuse_bounds
            ));
        } else {
            node.reuse_bounds = None;
        }
        let mut children_invalid_area = result.invalid_rects_list[invalid_rects_idx].clone();
        children_invalid_area.remove_unique_rect(ir);
        children_invalid_area.offset(-scroll_delta.0, -scroll_delta.1);
        let reuse_bounds = reuse_bounds.with_offset((-scroll_delta.0, -scroll_delta.1));
        //TODO scroll bar?
        // let bounds = node.element.get_origin_bounds();
        if !reuse_bounds.is_empty() {
            let invalid_top_width = reuse_bounds.top;
            let invalid_left_width = reuse_bounds.left;
            let invalid_bottom_width = old_origin_bounds.height - reuse_bounds.bottom;
            let invalid_right_width = old_origin_bounds.width - reuse_bounds.right;

            if invalid_left_width > 0.0 {
                let rect = Rect::new(old_origin_bounds.x, old_origin_bounds.y, old_origin_bounds.x + invalid_left_width, old_origin_bounds.bottom());
                children_invalid_area.add_rect(Some(&rect));
            }
            if invalid_right_width > 0.0 {
                let rect = Rect::new(old_origin_bounds.right() - invalid_right_width, old_origin_bounds.y, old_origin_bounds.right(), old_origin_bounds.bottom());
                children_invalid_area.add_rect(Some(&rect));
            }
            if invalid_top_width > 0.0 {
                let rect = Rect::new(old_origin_bounds.x, old_origin_bounds.y, old_origin_bounds.right(), old_origin_bounds.y + invalid_top_width);
                children_invalid_area.add_rect(Some(&rect));
            }
            if invalid_bottom_width > 0.0 {
                let rect = Rect::new(old_origin_bounds.x, reuse_bounds.bottom() - invalid_bottom_width, old_origin_bounds.right(), old_origin_bounds.bottom());
                children_invalid_area.add_rect(Some(&rect));
            }
        } else {
            children_invalid_area.add_rect(Some(&ir.rect));
        }

        result.invalid_rects_list.push(children_invalid_area);
        invalid_rects_idx = result.invalid_rects_list.len() - 1;
        root.invalid_unique_rect =  None;
    }
    node.children_invalid_rects_idx = invalid_rects_idx;
    result.nodes.push(node);
    let node_idx = result.nodes.len() - 1;
    result.ops.push(RenderOp::Render(node_idx));

    let children = root.get_children();
    for mut child in children {
        let child_bounds = child.get_bounds().translate(-root.scroll_left, -root.scroll_top);
        matrix_calculator.save();
        matrix_calculator.translate((child_bounds.x, child_bounds.y));
        collect_render_nodes(&mut child, result, invalid_rects_idx, scale, Some(node_idx), matrix_calculator, child_bounds.to_skia_rect());
        matrix_calculator.restore();
    }
    result.ops.push(RenderOp::Finish(node_idx));
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
    let name = node.get_backend().get_name();
    let children = node.get_children();
    if children.is_empty() {
        println!("{}{}", padding, name);
    } else {
        println!("{}{}{}", padding, name, " {");
        for c in children {
            let c_padding = padding.to_string() + "  ";
            print_tree(&c, &c_padding);
        }
        println!("{}{}", padding, "}");
    }
}

fn emit_mouse_event(render_tree: &Arc<Mutex<RenderTree>>, node: &mut Element, event_type_enum: MouseEventType, button: i32, frame_x: f32, frame_y: f32, screen_x: f32, screen_y: f32) {
    let render_tree = render_tree.clone();
    let mut render_tree = render_tree.lock().unwrap();
    let render_node = match render_tree.get_by_element_id(node.get_id()) {
        Some(node) => node,
        None => return,
    };
    let (border_top, _, _, border_left) = node.get_border_width();

    //TODO maybe not inverted?
    let inverted_matrix = render_node.total_matrix.invert().unwrap();

    let Point { x: relative_x, y: relative_y } = inverted_matrix.map_xy(frame_x, frame_y);
    let off_x = relative_x - border_left;
    let off_y = relative_y - border_top;

    let detail = MouseDetail {
        event_type: event_type_enum,
        button,
        offset_x: off_x,
        offset_y: off_y,
        frame_x,
        frame_y,
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

pub fn frame_input(frame_id: i32, content: String) {
    FRAMES.with_borrow_mut(|m| {
        if let Some(f) = m.get_mut(&frame_id) {
            f.handle_input(&content);
        }
    });
}

pub fn frame_ime_resize(frame_id: i32, height: f32) {
    FRAMES.with_borrow_mut(|m| {
        if let Some(f) = m.get_mut(&frame_id) {
            f.ime_height = height;
            f.mark_dirty_and_update_immediate(true).wait_finish();
        }
    });
}

