use crate as deft;
use crate::element::Element;
use crate::ext::common::create_event_handler;
use crate::js::js_serde::JsValueSerializer;
use crate::js::{FromJsValue, ToJsValue};
use crate::number::DeNan;
use crate::{js_deserialize, js_serialize, some_or_return};
use anyhow::Error;
use log::error;
use quick_js::{JsValue, ValueError};
use serde::{Deserialize, Serialize};
use skia_safe::Path;
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::LocalKey;
use yoga::Layout;

pub struct IdKey {
    next_id: Cell<usize>,
}

impl IdKey {
    pub fn new() -> Self {
        Self {
            next_id: Cell::new(1),
        }
    }
}

pub struct Id<T> {
    id: usize,
    _phantom: PhantomData<T>,
}

unsafe impl<T> Send for Id<T> {}
unsafe impl<T> Sync for Id<T> {}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _phantom: PhantomData,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> Debug for Id<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.id, f)
    }
}

impl<T> Display for Id<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.id, f)
    }
}

impl<T> ToJsValue for Id<T> {
    fn to_js_value(self) -> Result<JsValue, ValueError> {
        Ok(JsValue::String(self.id.to_string()))
    }
}

impl<T> FromJsValue for Id<T> {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
        if let JsValue::Int(id) = value {
            Ok(Self {
                id: id as usize,
                _phantom: PhantomData,
            })
        } else if let JsValue::String(id) = value {
            let id = usize::from_str(id.as_str())
                .map_err(|e| ValueError::Internal(format!("Invalid number format: {:?}", e)))?;
            Ok(Self {
                id,
                _phantom: PhantomData,
            })
        } else {
            Err(ValueError::UnexpectedType)
        }
    }
}

impl<T> Id<T> {
    pub fn next(local_key: &'static LocalKey<IdKey>) -> Self {
        let id = {
            local_key.with(|k| {
                let id = k.next_id.get();
                k.next_id.set(id + 1);
                id
            })
        };
        Id {
            id,
            _phantom: PhantomData,
        }
    }
}

pub struct StateMarker {
    state: bool,
}

impl StateMarker {
    pub fn new() -> Self {
        Self { state: false }
    }
    pub fn mark(&mut self) {
        self.state = true
    }

    pub fn unmark(&mut self) -> bool {
        if self.state {
            self.state = false;
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResultWaiter<T> {
    lock: Arc<(Mutex<Option<T>>, Condvar)>,
}

impl<T> ResultWaiter<T> {
    pub fn new() -> Self {
        Self {
            lock: Arc::new((Mutex::new(None), Condvar::new())),
        }
    }

    pub fn new_finished(value: T) -> Self {
        let waiter = Self::new();
        waiter.finish(value);
        waiter
    }
    pub fn finish(&self, value: T) {
        let (lock, cvar) = &*self.lock;
        let mut done = lock.lock().unwrap();
        *done = Some(value);
        cvar.notify_all();
    }

    pub fn wait_result<R, F: FnOnce(&T) -> R>(&self, callback: F) -> R {
        let (lock, cvar) = &*self.lock;
        let mut done = lock.lock().unwrap();
        while done.is_none() {
            done = cvar.wait(done).unwrap();
        }
        if let Some(value) = &*done {
            return callback(value);
        }
        unreachable!()
    }

    pub fn wait_finish(&self) {
        self.wait_result(|_| {});
    }
}

pub struct Callback {
    callback: Box<dyn FnOnce() + 'static>,
}

impl Callback {
    pub fn from_box(f: Box<dyn FnOnce()>) -> Callback {
        Self {
            callback: Box::new(f),
        }
    }
    pub fn new<F: FnOnce() + 'static>(callback: F) -> Self {
        Self {
            callback: Box::new(callback),
        }
    }
    pub fn call(self) {
        (self.callback)()
    }
}

pub struct JsValueContext {
    pub context: JsValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

js_deserialize!(Rect);
js_serialize!(Rect);

#[derive(Debug, Copy, Clone, Serialize)]
pub enum MouseEventType {
    MouseDown,
    MouseUp,
    MouseClick,
    ContextMenu,
    MouseMove,
    MouseEnter,
    MouseLeave,
}

pub struct FocusShiftDetail {
    element: u32,
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

    /// x-axis relative to window(as clientX in web)
    pub window_x: f32,
    /// y-axis relative to window(as clientY in web)
    pub window_y: f32,
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

    /// x-axis relative to window(as clientX in web)
    pub window_x: f32,
    /// y-axis relative to window(as clientY in web)
    pub window_y: f32,
    // pub screen_x: f32,
    // pub screen_y: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TouchDetail {
    pub touches: Vec<Touch>,
}

impl TouchDetail {
    pub fn only_one_touch(&self) -> Option<&Touch> {
        if self.touches.len() == 1 {
            Some(&self.touches[0])
        } else {
            None
        }
    }
}

pub trait EventDetail: 'static {
    fn raw(&self) -> Box<&dyn Any>;
    fn raw_mut(&mut self) -> Box<&mut dyn Any>;
    fn create_js_value(&self) -> Result<JsValue, Error>;
}

impl<T> EventDetail for T
where
    T: Serialize + 'static,
{
    fn raw(&self) -> Box<&dyn Any> {
        Box::new(self)
    }

    fn raw_mut(&mut self) -> Box<&mut dyn Any> {
        Box::new(self)
    }

    fn create_js_value(&self) -> Result<JsValue, Error> {
        let js_serializer = JsValueSerializer {};
        Ok(self.serialize(js_serializer)?)
    }
}

thread_local! {
    pub static NEXT_EVENT_ID: Cell<u64> = Cell::new(1);
}

pub struct EventContext<T> {
    id: u64,
    pub target: T,
    pub propagation_cancelled: bool,
    pub prevent_default: bool,
}

impl<T> EventContext<T> {
    pub fn new(target: T) -> Self {
        let id = NEXT_EVENT_ID.get();
        NEXT_EVENT_ID.set(id + 1);
        Self {
            id,
            target,
            propagation_cancelled: false,
            prevent_default: false,
        }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }
}

pub struct Event<T> {
    pub event_type: String,
    pub detail: Box<dyn EventDetail>,
    pub context: EventContext<T>,
}

impl<E> Event<E> {
    pub fn new<T: EventDetail>(event_type: &str, detail: T, target: E) -> Self {
        Self {
            event_type: event_type.to_string(),
            detail: Box::new(detail),
            context: EventContext::new(target),
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

js_deserialize!(Size);

impl CaretDetail {
    pub fn new(position: usize, origin_bounds: Rect, bounds: Rect) -> Self {
        Self {
            position,
            origin_bounds,
            bounds,
        }
    }
}

pub type EventHandler<E> = dyn FnMut(&mut Event<E>);

pub trait EventListener<T, E> {
    fn handle_event(&mut self, event: &mut T, ctx: &mut EventContext<E>);
}

pub struct EventRegistration<E> {
    listeners: HashMap<String, Vec<(u32, Box<EventHandler<E>>)>>,
    next_listener_id: u32,
    typed_listeners: HashMap<
        TypeId,
        Vec<(
            u32,
            Box<dyn FnMut(&mut Box<&mut dyn Any>, &mut EventContext<E>)>,
        )>,
    >,
    listener_types: HashMap<u32, TypeId>,
}

impl<E> EventRegistration<E> {
    pub fn new() -> Self {
        Self {
            next_listener_id: 1,
            listeners: HashMap::new(),
            typed_listeners: HashMap::new(),
            listener_types: HashMap::new(),
        }
    }

    pub fn register_event_listener<T: 'static, H: EventListener<T, E> + 'static>(
        &mut self,
        mut listener: H,
    ) -> u32 {
        let id = self.next_listener_id;
        self.next_listener_id += 1;
        let event_type_id = TypeId::of::<T>();
        if !self.typed_listeners.contains_key(&event_type_id) {
            let lst = Vec::new();
            self.typed_listeners.insert(event_type_id, lst);
        }
        let listeners = self.typed_listeners.get_mut(&event_type_id).unwrap();
        let wrapper_listener = Box::new(
            move |d: &mut Box<&mut dyn Any>, ctx: &mut EventContext<E>| {
                if let Some(t) = d.downcast_mut::<T>() {
                    listener.handle_event(t, ctx);
                }
            },
        );
        listeners.push((id, wrapper_listener));
        self.listener_types.insert(id, event_type_id);
        id
    }

    pub fn unregister_event_listener(&mut self, id: u32) {
        let event_type_id = some_or_return!(self.listener_types.remove(&id));
        if let Some(listeners) = self.typed_listeners.get_mut(&event_type_id) {
            listeners.retain(|(i, _)| *i != id);
        }
    }

    pub fn emit<T: 'static>(&mut self, event: &mut T, ctx: &mut EventContext<E>) {
        let event_type_id = TypeId::of::<T>();
        if let Some(listeners) = self.typed_listeners.get_mut(&event_type_id) {
            let mut event = Box::new(event as &mut dyn Any);
            for it in listeners {
                (it.1)(&mut event, ctx);
            }
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

    pub fn bind_event_listener<T: 'static, F: FnMut(&mut EventContext<E>, &mut T) + 'static>(
        &mut self,
        event_type: &str,
        mut handler: F,
    ) -> u32 {
        self.add_event_listener(
            event_type,
            Box::new(move |e| {
                if let Some(me) = e.detail.raw_mut().downcast_mut::<T>() {
                    handler(&mut e.context, me);
                }
            }),
        )
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

impl<E: ToJsValue + Clone + 'static> EventRegistration<E> {
    pub fn add_js_event_listener(&mut self, event_type: &str, callback: JsValue) -> i32 {
        let handler = create_event_handler(event_type, callback);
        let id = self.add_event_listener(
            event_type,
            Box::new(move |e| match e.detail.create_js_value() {
                Ok(ev) => {
                    handler(&mut e.context, ev);
                }
                Err(e) => {
                    error!("Failed to convert rust object to js value: {}", e);
                }
            }),
        );
        id as i32
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

    pub fn from_skia_rect(rect: skia_safe::Rect) -> Self {
        Self {
            x: rect.left,
            y: rect.top,
            width: rect.width(),
            height: rect.height(),
        }
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
        };
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

    pub fn to_origin_bounds(&self, node: &Element) -> Self {
        let origin_bounds = node.get_origin_bounds();
        self.translate(origin_bounds.x, origin_bounds.y)
    }
}

pub struct UnsafeFnOnce {
    callback: Box<dyn FnOnce()>,
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
        Box::new(move || self.call())
    }
}

unsafe impl Send for UnsafeFnOnce {}
unsafe impl Sync for UnsafeFnOnce {}

pub struct UnsafeFnMut<P> {
    pub callback: Box<dyn FnMut(P)>,
}

unsafe impl<P> Send for UnsafeFnMut<P> {}
unsafe impl<P> Sync for UnsafeFnMut<P> {}

#[cfg(test)]
mod tests {
    use crate::base::{EventContext, EventListener, EventRegistration};
    use log::debug;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_event_registration() {
        #[derive(Debug)]
        struct MyEvent {
            value: Rc<RefCell<i32>>,
        }
        struct MyEventListener {}
        impl EventListener<MyEvent, ()> for MyEventListener {
            fn handle_event(&mut self, event: &mut MyEvent, _ctx: &mut EventContext<()>) {
                debug!("handling {:?}", event);
                let mut v = event.value.borrow_mut();
                *v = 1;
            }
        }
        let value = Rc::new(RefCell::new(0));
        let mut er: EventRegistration<()> = EventRegistration::new();
        er.register_event_listener(MyEventListener {});
        er.emit(
            &mut MyEvent {
                value: Rc::clone(&value),
            },
            &mut EventContext::new(()),
        );

        assert_eq!(1, *value.borrow());
    }
}
