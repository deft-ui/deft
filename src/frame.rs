use crate as lento;
use crate::app::{exit_app, AppEvent, AppEventPayload};
use crate::base::MouseEventType::{MouseClick, MouseUp};
use crate::base::{Callback, ElementEvent, Event, EventContext, EventHandler, EventListener, EventRegistration, MouseDetail, MouseEventType, ResultWaiter, Touch, TouchDetail, UnsafeFnOnce};
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
use skia_safe::{Canvas, ClipOp, Color, ColorType, Contains, IRect, Image, ImageInfo, Matrix, Paint, PaintStyle, Path, Point, Rect};
use skia_window::skia_window::{RenderBackendType, SkiaWindow};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::num::NonZeroU32;
use std::ops::{Deref, DerefMut};
use std::process::exit;
use std::rc::Rc;
use std::{env, mem, slice};
use std::string::ToString;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use skia_bindings::SkPaint_Style::{Fill, Stroke};
use skia_safe::canvas::SetMatrix;
use skia_safe::wrapper::{NativeTransmutableWrapper, PointerWrapper};
use skia_window::context::RenderContext;
use skia_window::layer::Layer;
use skia_window::renderer::Renderer;
use winit::dpi::Position::Logical;
use winit::dpi::{LogicalPosition, LogicalSize, Position, Size};
use winit::error::ExternalError;
use winit::event::{ElementState, Ime, Modifiers, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::{Key, NamedKey};
#[cfg(feature = "x11")]
use winit::platform::x11::WindowAttributesExtX11;
use winit::window::{Cursor, CursorGrabMode, CursorIcon, Window, WindowAttributes, WindowId};
use crate::{bind_js_event_listener, is_snapshot_usable, send_app_event, show_repaint_area, some_or_continue, some_or_return};
use crate::frame_rate::{get_total_frames, next_frame, FRAME_RATE_CONTROLLER};
use crate::paint::{InvalidArea, PartialInvalidArea, Painter, RenderNode, RenderTree, SkiaPainter, UniqueRect, InvalidRects, RenderOp, MatrixCalculator, ClipPath, RenderPaintInfo, ClipChain, RenderLayer, RenderLayerNode, RenderState, RenderLayerKey, LayerState};
use crate::render::paint_layer::PaintLayer;
use crate::render::paint_node::PaintNode;
use crate::render::paint_tree::PaintTree;
use crate::style::border_path::BorderPath;
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
    focusing: Option<Element>,
    /// (element, button)
    pressing: Option<(Element, MouseDownInfo)>,
    touching: TouchingInfo,
    dragging: bool,
    last_drag_over: Option<Element>,
    hover: Option<Element>,
    modifiers: Modifiers,
    dirty: bool,
    layout_dirty: bool,
    repaint_timer_handle: Option<TimerHandle>,
    event_registration: EventRegistration<FrameWeak>,
    attributes: WindowAttributes,
    init_width: Option<f32>,
    init_height: Option<f32>,
    ime_height: f32,
    background_color: Color,
    pub render_tree: RenderTree,
    renderer_idle: bool,
    next_frame_callbacks: Vec<Callback>,
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
            frame_type,
            init_width: attrs.width,
            init_height: attrs.height,
            ime_height: 0.0,
            background_color: Color::from_rgb(0, 0, 0),
            repaint_timer_handle: None,
            render_tree: RenderTree::new(0),
            renderer_idle: true,
            next_frame_callbacks: Vec::new(),
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

    pub fn invalid_element(&mut self, element_id: u32) {
        self.render_tree.invalid_element(element_id);
        self.notify_update();
    }

    pub fn update_layer_scroll_left(&mut self, element_id: u32, scroll_left: f32) {
        self.render_tree.update_scroll_left(element_id, scroll_left);
        self.notify_update();
    }

    pub fn update_layer_scroll_top(&mut self, element_id: u32, scroll_top: f32) {
        self.render_tree.update_scroll_top(element_id, scroll_top);
        self.notify_update();
    }

    fn notify_update(&mut self) {
        if !self.dirty {
            self.dirty = true;
            send_app_event(AppEvent::Update(self.get_id()));
        }
    }

    pub fn invalid_layout(&mut self) {
        // Note: Uncomment to debug layout problems
        // if layout_dirty && !self.layout_dirty { crate::trace::print_trace("layout dirty") }
        self.layout_dirty = true;
        self.notify_update();
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
                self.dirty = true;
                self.update();
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
        if let Some((mut target_node, _, _)) = self.get_node_by_point() {
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
        if let Some((pressing, down_info)) = &mut self.pressing.clone() {
            if dragging {
                if let Some((target, _, _)) = &mut target_node {
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
                    emit_mouse_event(&self.render_tree, pressing, MouseEventType::MouseMove, 0, frame_x, frame_y, screen_x, screen_y);
                }
            }
            //TODO should emit mouseenter|mouseleave?
        } else if let Some((mut node, _, _)) = target_node {
            self.update_cursor(&node);
            if let Some(hover) = &mut self.hover.clone() {
                if hover != &node {
                    emit_mouse_event(&self.render_tree, hover, MouseEventType::MouseLeave, 0, frame_x, frame_y, screen_x, screen_y);
                    self.mouse_enter_node(node.clone(), frame_x, frame_y, screen_x, screen_y);
                } else {
                    emit_mouse_event(&self.render_tree, &mut node, MouseEventType::MouseMove, 0, frame_x, frame_y, screen_x, screen_y);
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
        emit_mouse_event(&self.render_tree, &mut node, MouseEventType::MouseEnter, 0, offset_x, offset_y, screen_x, screen_y);
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
        //TODO impl

        if let Some((mut node, _, _)) = self.get_node_by_point() {
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
                    emit_mouse_event(&self.render_tree, &mut node, event_type, button, frame_x, frame_y, screen_x, screen_y);
                }
                ElementState::Released => {
                    if let Some(mut pressing) = self.pressing.clone() {
                        emit_mouse_event(&self.render_tree, &mut pressing.0, MouseUp, button, frame_x, frame_y, screen_x, screen_y);
                        if pressing.0 == node && pressing.1.button == button {
                            let ty = match mouse_button {
                                MouseButton::Left => Some(MouseEventType::MouseClick),
                                MouseButton::Right => Some(MouseEventType::ContextMenu),
                                _ => None
                            };
                            if let Some(ty) = ty {
                                emit_mouse_event(&self.render_tree, &mut node, ty, button, frame_x, frame_y, screen_x, screen_y);
                            }
                        }
                        self.release_press();
                    } else {
                        emit_mouse_event(&self.render_tree, &mut node, MouseUp, button, frame_x, frame_y, screen_x, screen_y);
                    }
                }
            }
        }
        if state == ElementState::Released {
            if let Some(pressing) = &mut self.pressing.clone() {
                emit_mouse_event(&self.render_tree, &mut pressing.0, MouseUp, pressing.1.button, frame_x, frame_y, screen_x, screen_y);
                self.release_press();
            }
        }
    }

    pub fn emit_touch_event(&mut self, identifier: u64, phase: TouchPhase, frame_x: f32, frame_y: f32) -> Option<()> {
        if let Some((mut node, relative_x, relative_y)) = self.get_node_by_pos(frame_x, frame_y) {
            let _e_type = match phase {
                TouchPhase::Started => "touchstart",
                TouchPhase::Ended => "touchend",
                TouchPhase::Moved => "touchmove",
                TouchPhase::Cancelled => "touchcancel",
            };
            let node_bounds = node.get_origin_bounds();
            let (border_top, _, _, border_left) = node.get_border_width();

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
                        //TODO fix screen_x, screen_y
                        emit_mouse_event(&self.render_tree, &mut node, MouseClick, 0, frame_x, frame_y, 0.0, 0.0);
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
        if !self.renderer_idle {
            return ResultWaiter::new_finished(false);
        }
        print_time!("frame update time");
        let mut frame_callbacks = Vec::new();
        frame_callbacks.append(&mut self.next_frame_callbacks);
        for cb in frame_callbacks {
            cb.call();
        }
        if !self.dirty {
            // skip duplicate update
            return ResultWaiter::new_finished(false);
        }
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
            self.render_tree = if let Some(body) = &mut self.body {
                // print_time!("calculate layout, {} x {}", width, height);
                body.calculate_layout(width, height);
                let mut render_tree = build_render_nodes(body);
                render_tree.update_layout_info_recurse(body, body.get_bounds().to_skia_rect());
                if auto_size {
                    let (final_width, final_height) = body.get_size();
                    if size.width != final_width as u32 && size.height != final_height as u32 {
                        self.resize(crate::base::Size {
                            width: final_width,
                            height: final_height,
                        });
                    }
                }
                render_tree
            } else {
                RenderTree::new(0)
            };
            // print_time!("collect element time");
        }
        let r = self.paint();
        self.layout_dirty = false;
        self.dirty = false;
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

    pub fn request_next_frame_callback(&mut self, callback: Callback) {
        self.next_frame_callbacks.push(callback);
        if self.next_frame_callbacks.len() == 1 {
            send_app_event(AppEvent::Update(self.get_id()));
        }
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
        let viewport = Rect::new(0.0, 0.0, width as f32 / scale_factor, height as f32 / scale_factor);
        if let Some(body) = &mut me.body {
            // print_time!("build render nodes time");
            print_time!("update layout time");
            self.render_tree.rebuild_layers(body);
        } else {
            return ResultWaiter::new_finished(false);
        };
        let mut paint_tree = {
            build_repaint_nodes(&mut self.render_tree, &mut body, viewport.clone());
            self.render_tree.build_repaint_tree(&viewport)
        };
        let waiter_finisher = waiter.clone();
        let frame_id = self.get_id();
        self.renderer_idle = false;
        self.window.render_with_result(Renderer::new(move |canvas, ctx| {
            //print_time!("render time");
            canvas.save();
            if scale_factor != 1.0 {
                canvas.scale((scale_factor, scale_factor));
            }
            let mut painter = SkiaPainter::new(canvas);
            canvas.clear(background_color);
            let mut layers = Vec::new();
            layers.append(&mut paint_tree.layers);
            let mut render_data = RenderState::take(ctx);
            let mut old_layers = mem::take(&mut render_data.layers);
            for mut layer in layers {
                draw_layer(canvas, ctx, &mut old_layers, &mut render_data, &mut paint_tree, &mut layer, scale_factor, &viewport);
            }
            //draw_elements(canvas, ctx, &mut render_tree, invalid_rects_list, scale_factor, old_snapshots, snapshots, &viewport);
            render_data.put(ctx);
            canvas.restore();
        }), move|r| {
            waiter_finisher.finish(r);
            send_app_event(AppEvent::RenderIdle(frame_id));
        });
        waiter
    }

    #[inline]
    fn get_logical_len(&self, physical_len: f32) -> f32 {
        physical_len * self.window.scale_factor() as f32
    }

    fn get_node_by_point(&self) -> Option<(Element, f32, f32)> {
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

    fn get_node_by_pos(&self, x: f32, y: f32) -> Option<(Element, f32, f32)> {
        let body = self.body.clone()?;
        // print_time!("search node time in layers");
        let mut layer_idx = self.render_tree.layers.len();
        for layer in self.render_tree.layers.iter().rev() {
            layer_idx -= 1;
            let im = layer.total_matrix.invert().unwrap();
            let point = im.map_xy(x, y);
            if point.x >= 0.0 && point.x <= layer.width && point.y >= 0.0 && point.y <= layer.height {
                let nodes = self.render_tree.nodes();
                for node in nodes.iter().rev() {
                    let x = point.x;
                    let y = point.y;
                    if node.layer_idx == layer_idx && x >= node.layer_x && x <= node.layer_x + node.width && y >= node.layer_y && y <= node.layer_y + node.height {
                        // println!("found node: {}", node.element_id);
                        //TODO check border path
                        return (
                            self.get_element_by_id(&body, node.element_id)
                                .map(|e| (e, x - node.layer_x, y - node.layer_y))
                        );
                    }
                }
            }
        }
        None
    }

    fn create_window(attributes: WindowAttributes) -> SkiaWindow {
        run_with_event_loop(|el| {
            //TODO support RenderBackedType parameter
            #[cfg(not(target_os = "android"))]
            let default_backend_type = "softbuffer";
            #[cfg(target_os = "android")]
            let default_backend_type = "gl";
            let backend_type_str = env::var("renderer").unwrap_or(default_backend_type.to_string());
            let backend_type = match backend_type_str.as_str() {
                "softbuffer" => RenderBackendType::SoftBuffer,
                _ => RenderBackendType::GL,
            };
            println!("render backend: {:?}", backend_type);
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

fn draw_layer(
    root_canvas: &Canvas,
    context: &mut RenderContext,
    old_graphic_layers: &mut HashMap<RenderLayerKey, LayerState>,
    render_data: &mut RenderState,
    render_tree: &mut PaintTree,
    layer: &mut PaintLayer,
    scale: f32,
    viewport: &Rect,
) {
    let max_len = (viewport.width() * viewport.width() + viewport.height() * viewport.height()).sqrt();
    let surface_width = (f32::min(layer.width, max_len) * scale) as usize;
    let surface_height = (f32::min(layer.height, max_len) * scale) as usize;
    if surface_width <= 0 || surface_height <= 0 {
        return;
    }
    let mut graphic_layer = if let Some(mut ogl_state) = old_graphic_layers.remove(&layer.key) {
        if ogl_state.surface_width != surface_width || ogl_state.surface_height != surface_height {
            None
        } else {
            //TODO fix scroll delta
            let scroll_delta_x = layer.scroll_left - ogl_state.last_scroll_left;
            let scroll_delta_y = layer.scroll_top - ogl_state.last_scroll_top;
            if scroll_delta_x != 0.0 || scroll_delta_y != 0.0 {
                //TODO optimize size
                let tp_width = (viewport.width() * scale) as usize;
                let tp_height = (viewport.height() * scale) as usize;
                let mut temp_gl = context.create_layer(tp_width, tp_height).unwrap();
                temp_gl.canvas().session(|canvas| {
                    canvas.clip_rect(&Rect::new(0.0, 0.0, layer.width * scale, layer.height * scale), ClipOp::Intersect, false);
                    canvas.draw_image(&ogl_state.layer.as_image(), (-scroll_delta_x * scale, -scroll_delta_y * scale), None);
                });
                unsafe {
                    let sf = root_canvas.surface();
                    let sf = sf.unwrap();
                    sf.direct_context().unwrap().flush_and_submit();
                }
                ogl_state.layer.canvas().session(|canvas| {
                    canvas.clear(Color::TRANSPARENT);
                    canvas.clip_rect(&Rect::from_xywh(0.0, 0.0, layer.width, layer.height), ClipOp::Intersect, false);
                    canvas.scale((1.0 / scale, 1.0 / scale));
                    canvas.draw_image(&temp_gl.as_image(), (0.0, 0.0), None);
                });
                unsafe {
                    let sf = root_canvas.surface();
                    let sf = sf.unwrap();
                    sf.direct_context().unwrap().flush_and_submit();
                }
                ogl_state.last_scroll_left = layer.scroll_left;
                ogl_state.last_scroll_top = layer.scroll_top;
            }
            Some(ogl_state)
        }
    } else {
        None
    }.unwrap_or_else(|| {
        let mut gl = context.create_layer(surface_width, surface_height).unwrap();
        gl.canvas().scale((scale, scale));
        LayerState {
            layer: gl,
            last_scroll_left: 0.0,
            last_scroll_top: 0.0,
            surface_width,
            surface_height,
        }
    });
    let layer_canvas = graphic_layer.layer.canvas();
    layer_canvas.save();
    layer_canvas.clip_path(&layer.invalid_path, ClipOp::Intersect, false);
    layer_canvas.clip_rect(&Rect::from_xywh(0.0, 0.0, layer.width, layer.height), ClipOp::Intersect, false);
    layer_canvas.clear(Color::TRANSPARENT);
    draw_nodes_recurse(layer_canvas, scale, &mut layer.roots, render_tree);
    layer_canvas.restore();

    unsafe  {
        let sf = root_canvas.surface();
        let sf = sf.unwrap();
        sf.direct_context().unwrap().flush_and_submit();
    }
    root_canvas.save();
    root_canvas.concat(&layer.total_matrix);
    root_canvas.scale((1.0 / scale, 1.0 / scale));
    root_canvas.draw_image(graphic_layer.layer.as_image(), (0.0, 0.0), None);
    if show_repaint_area() {
        root_canvas.scale((scale, scale));
        let path = &layer.invalid_path;
        if !path.is_empty() {
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Stroke);
            paint.set_color(Color::from_rgb(200, 0, 0));
            root_canvas.draw_path(&path, &paint);
        }
    }
    root_canvas.restore();
    unsafe  {
        let sf = root_canvas.surface();
        let sf = sf.unwrap();
        sf.direct_context().unwrap().flush_and_submit();
    }
    render_data.layers.insert(layer.key.clone(), graphic_layer);
}

fn draw_nodes_recurse(
    canvas: &Canvas,
    scale: f32,
    nodes: &mut Vec<PaintNode>,
    render_tree: &mut PaintTree,
) {
    for n in nodes {
        draw_node_recurse(canvas, scale, n, render_tree);
    }
}

fn draw_node_recurse(
    canvas: &Canvas,
    scale: f32,
    node: &mut PaintNode,
    render_tree: &mut PaintTree,
) {
    // let node = render_tree.get_node_mut(layer_node.node_idx).unwrap();
    canvas.save();
    canvas.translate((node.layer_x, node.layer_y));
    //TODO children viewport
    // let vp_path = node.children_viewport.as_ref().map(|r| Path::rect(r, None));
    // if let Some(vp_path) = vp_path {
    //     c.clip_path(&vp_path, SkClipOp::Intersect, false);
    // }
    //TODO support overflow:visible
    // set clip path
    let clip_path = &node.border_box_path;
    canvas.clip_path(clip_path, SkClipOp::Intersect, false);
    // if node.paint_info.as_ref().is_some_and(|p| p.need_paint) {
    //     node.paint_info.as_mut().unwrap().need_paint = false;
        let mut painter = SkiaPainter::new(canvas);
        draw_element(canvas, node, &mut painter);
    // }
    canvas.translate((-node.layer_x, -node.layer_y));
    draw_nodes_recurse(canvas, scale, &mut node.children, render_tree);
    canvas.restore();
}

fn draw_element(canvas: &Canvas, node: &mut PaintNode, painter: &mut dyn Painter) {
    let width = node.width;
    let height = node.height;
    //TODO fix clip
    // node.clip_chain.apply(canvas);
    // canvas.concat(&node.total_matrix);
    // node.clip_path.apply(canvas);

    canvas.session(move |canvas| {

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
            // let paint_info = some_or_return!(&mut node.paint_info);
            if let Some(render_fn) = node.render_fn.take() {
                render_fn.run(canvas);
            }
        }
        canvas.restore();
    });
}

fn build_render_nodes(root: &mut Element) -> RenderTree {
    let count = count_elements(root);
    let mut render_tree = RenderTree::new(count);
    root.need_snapshot = true;
    collect_render_nodes(root, &mut render_tree);
    render_tree
}

fn build_repaint_nodes(
    render_tree: &mut RenderTree,
    root: &mut Element,
    viewport: Rect
) {
    print_time!("build repaint nodes");
    update_node_paint_info_recursive(render_tree, root, &viewport);
}

fn collect_render_nodes(
    root: &mut Element,
    result: &mut RenderTree,
) {
    let mut node = RenderNode {
        element_id: root.get_id(),
        width: 0.0,
        height: 0.0,
        border_width: root.get_border_width(),
        border_path: BorderPath::new(0.0, 0.0, [0.0; 4], [0.0; 4]),
        children_viewport: root.get_children_viewport(),
        need_snapshot: root.need_snapshot,
        paint_info: None,
        layer_idx: 0,
        layer_x: 0.0,
        layer_y: 0.0,
        need_repaint: false,
        // reuse_bounds: None,
    };

    // build_render_paint_info(root, &mut result.invalid_rects_list, &mut invalid_rects_idx, &mut node);

    let node_idx = result.add_node(node);

    let children = root.get_children();
    for mut child in children {
        collect_render_nodes(&mut child, result);
    }
}

fn update_node_paint_info_recursive(
    tree: &mut RenderTree,
    element: &mut Element,
    viewport: &Rect,
) {
    let layer_idx = some_or_return!(tree.get_layer_idx(element.get_id()));
    let invalid_rects = tree.layers[layer_idx].build_invalid_rects(viewport);
    let node = some_or_return!(tree.get_mut_by_element_id(element.get_id()));
    build_render_paint_info(element, &invalid_rects, node, viewport);
    for mut n in element.get_children() {
        update_node_paint_info_recursive(tree, &mut n, viewport);
    }
}

fn build_render_paint_info(
    root: &mut Element,
    invalid_rects: &InvalidRects,
    node: &mut RenderNode,
    viewport: &Rect,
) {
    let scroll_delta = if let Some(lpi) = &root.last_paint_info {
        (root.scroll_left - lpi.scroll_left, root.scroll_top - lpi.scroll_top)
    } else {
        (0.0, 0.0)
    };
    root.last_paint_info = Some(PaintInfo {
        scroll_left: root.scroll_left,
        scroll_top: root.scroll_top,
    });
    let bounds = Rect::from_xywh(node.layer_x, node.layer_y, node.width, node.height);
    node.need_repaint = invalid_rects.has_intersects(&bounds);
    if node.paint_info.is_none() || node.need_repaint {
        let paint_info = RenderPaintInfo {
            render_fn: Some(root.get_backend_mut().render()),
            background_image: root.style.background_image.clone(),
            background_color: root.style.computed_style.background_color,
            border_color: root.style.border_color,
            scroll_delta,
        };

        node.paint_info = Some(paint_info);
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

fn emit_mouse_event(render_tree: &RenderTree, node: &mut Element, event_type_enum: MouseEventType, button: i32, frame_x: f32, frame_y: f32, screen_x: f32, screen_y: f32) {
    let node_matrix = some_or_return!(render_tree.get_element_total_matrix(node.get_id()));
    let (border_top, _, _, border_left) = node.get_border_width();

    //TODO maybe not inverted?
    let inverted_matrix = node_matrix.invert().unwrap();

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

pub fn frame_on_render_idle(frame_id: i32) {
    FRAMES.with_borrow_mut(|m| {
        if let Some(f) = m.get_mut(&frame_id) {
            f.renderer_idle = true;
            f.update();
        }
    });
}

pub fn frame_check_update(frame_id: i32) {
    FRAMES.with_borrow_mut(|m| {
        if let Some(f) = m.get_mut(&frame_id) {
            f.update();
        }
    });
}

