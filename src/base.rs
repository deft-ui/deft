use std::any::Any;
use std::collections::HashMap;
use std::str::FromStr;
use quick_js::JsValue;
use serde::{Deserialize, Serialize};
use skia_safe::Path;
use yoga::Layout;
use crate::element::{ElementRef};
use crate::number::DeNan;

pub enum TextAlign {
    Left,
    Right,
    Center,
}

pub enum VerticalAlign {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Copy, Clone, Serialize)]
pub enum MouseEventType {
    MouseDown,
    MouseUp,
    MouseClick,
    MouseMove,
    MouseEnter,
    MouseLeave,
}

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseDetail {
    pub event_type: MouseEventType,
    pub button: i32,

    /// The offset in the X coordinate of the mouse pointer between that event and the padding edge of the target node.
    pub offset_x: f32,
    ///  The offset in the Y coordinate of the mouse pointer between that event and the padding edge of the target node.
    pub offset_y: f32,

    /// x-axis relative to frame(as clientX in web)
    pub frame_x: f32,
    /// y-axis relative to frame(as clientY in web)
    pub frame_y: f32,
    pub screen_x: f32,
    pub screen_y: f32,
}

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Touch {
    pub identifier: u64,
    /// The offset in the X coordinate of the mouse pointer between that event and the padding edge of the target node.
    pub offset_x: f32,
    ///  The offset in the Y coordinate of the mouse pointer between that event and the padding edge of the target node.
    pub offset_y: f32,

    /// x-axis relative to frame(as clientX in web)
    pub frame_x: f32,
    /// y-axis relative to frame(as clientY in web)
    pub frame_y: f32,
    // pub screen_x: f32,
    // pub screen_y: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TouchDetail {
    pub touches: Vec<Touch>,
}

pub trait EventDetail {
    fn raw(&self) -> Box<&dyn Any>;
    fn raw_mut(&mut self) -> Box<&mut dyn Any>;
    fn create_js_value(&self) -> JsValue;
}

impl<T> EventDetail for T where T: Serialize + Clone + 'static {
    fn raw(&self) -> Box<&dyn Any> {
        Box::new(self)
    }

    fn raw_mut(&mut self) -> Box<&mut dyn Any> {
        Box::new(self)
    }

    fn create_js_value(&self) -> JsValue {
        todo!()
    }
}

pub struct EventContext<T> {
    pub target: T,
    pub propagation_cancelled: bool,
    pub prevent_default: bool,
}
pub struct Event<T> {
    pub event_type: String,
    pub detail: Box<dyn Any>,
    pub context: EventContext<T>,
}

pub type ElementEvent = Event<ElementRef>;

pub type ElementEventContext = EventContext<ElementRef>;

impl<E> Event<E> {

    pub fn new<T: 'static>(event_type: &str, detail: T, target: E) -> Self {
        Self {
            event_type: event_type.to_string(),
            detail: Box::new(detail),
            context: EventContext {
                propagation_cancelled: false,
                prevent_default: false,
                target,
            }
        }
    }

    pub fn try_as_detail<T: 'static, F: FnMut(&T)>(&self, mut callback: F) {
        if let Some(d) = self.detail.downcast_ref::<T>() {
            callback(d);
        }
    }

}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaretDetail {
    pub position: usize,
    pub origin_bounds: Rect,
    pub bounds: Rect,
}

#[derive(Serialize)]
pub struct TextChangeDetail {
    pub value: String,
}

#[derive(Serialize)]
pub struct TextUpdateDetail {
    pub value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollEventDetail {
    pub scroll_top: f32,
    pub scroll_left: f32,
}

impl CaretDetail {

    pub fn new(position: usize, origin_bounds: Rect, bounds: Rect) -> Self {
        Self { position, origin_bounds, bounds }
    }

}

pub type EventHandler<E> = dyn FnMut(&mut Event<E>);

pub type ElementEventHandler = EventHandler<ElementRef>;

pub struct EventRegistration<E> {
    listeners: HashMap<String, Vec<(u32, Box<EventHandler<E>>)>>,
    next_listener_id: u32,
}

impl<E> EventRegistration<E> {
    pub fn new() -> Self {
        Self {
            next_listener_id: 1,
            listeners: HashMap::new(),
        }
    }
    pub fn add_event_listener(&mut self, event_type: &str, handler: Box<EventHandler<E>>) -> u32 {
        let id = self.next_listener_id;
        self.next_listener_id += 1;
        if !self.listeners.contains_key(event_type) {
            let lst = Vec::new();
            self.listeners.insert(event_type.to_string(), lst);
        }
        let listeners = self.listeners.get_mut(event_type).unwrap();
        listeners.push((id, handler));
        id
    }

    pub fn bind_event_listener<T: 'static, F: FnMut(&mut EventContext<E>, &mut T) + 'static>(&mut self, event_type: &str, mut handler: F) -> u32 {
        self.add_event_listener(event_type, Box::new(move |e| {
            if let Some(me) = e.detail.downcast_mut::<T>() {
                handler(&mut e.context, me);
            }
        }))
    }

    pub fn remove_event_listener(&mut self, event_type: &str, id: u32) {
        if let Some(listeners) = self.listeners.get_mut(event_type) {
            listeners.retain(|(i, _)| *i != id);
        }
    }

    pub fn emit_event(&mut self, event: &mut Event<E>) {
        if let Some(listeners) = self.listeners.get_mut(&event.event_type) {
            for it in listeners {
                (it.1)(event);
            }
        }

    }

}


impl Rect {

    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_layout(layout: &Layout) -> Self {
        Self {
            x: layout.left().nan_to_zero(),
            y: layout.top().nan_to_zero(),
            width: layout.width().nan_to_zero(),
            height: layout.height().nan_to_zero(),
        }
    }

    pub fn empty() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn to_skia_rect(&self) -> skia_safe::Rect {
        skia_safe::Rect::new(self.x, self.y, self.x + self.width, self.y + self.height)
    }

    #[inline]
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    #[inline]
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    #[inline]
    pub fn translate(&self, x: f32, y: f32) -> Self {
        Self {
            x: self.x + x,
            y: self.y + y,
            width: self.width,
            height: self.height,
        }
    }

    #[inline]
    pub fn new_origin(&self, x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            width: self.width,
            height: self.height,
        }
    }

    #[inline]
    pub fn to_path(&self) -> Path {
        let mut p = Path::new();
        p.add_rect(&self.to_skia_rect(), None);
        p
    }

    //TODO rename
    #[inline]
    pub fn intersect(&self, other: &Rect) -> Self {
        let x = f32::max(self.x, other.x);
        let y = f32::max(self.y, other.y);
        let r = f32::min(self.right(), other.right());
        let b = f32::min(self.bottom(), other.bottom());
        return Self {
            x,
            y,
            width: f32::max(0.0, r - x),
            height: f32::max(0.0, b - y),
        }
    }

    #[inline]
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        let left = self.x;
        let top = self.y;
        let right = self.right();
        let bottom = self.bottom();
        x >= left && x <= right && y >= top && y <= bottom
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0.0 || self.height == 0.0
    }

    pub fn to_origin_bounds(&self, node: &ElementRef) -> Self {
        let origin_bounds = node.get_origin_bounds();
        self.translate(origin_bounds.x, origin_bounds.y)
    }

}

pub struct PaintContext {
    pub width: f32,
    pub height: f32,
}


pub enum PropertyValue {
    INT(u32),
    Str(String),
}

impl PropertyValue {
    pub fn as_string(&self) -> String {
        match self {
            PropertyValue::INT(v) => format!("{}", v),
            PropertyValue::Str(v) => v.to_string(),
        }
    }
    pub fn as_f32(&self) -> f32 {
        match self {
            PropertyValue::INT(v) => *v as f32,
            PropertyValue::Str(v) => f32::from_str(v).unwrap(),
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            PropertyValue::INT(v) => *v != 0,
            PropertyValue::Str(v) => bool::from_str(v).unwrap(),
        }
    }
}

pub struct UnsafeFnOnce {
    callback: Box<dyn FnOnce()>
}

impl UnsafeFnOnce {
    pub unsafe fn new<F: FnOnce() + 'static>(callback: F) -> Self {
        let callback: Box<dyn FnOnce()> = Box::new(callback);
        Self { callback }
    }

    pub fn call(self) {
        (self.callback)();
    }

    pub fn into_box(self) -> Box<dyn FnOnce() + Send + Sync + 'static> {
        Box::new(move || {
            self.call()
        })
    }
}

unsafe impl Send for UnsafeFnOnce {}
unsafe impl Sync for UnsafeFnOnce {}