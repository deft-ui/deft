use crate::base::{UnsafeFnMut, UnsafeFnOnce};
use crate::event_loop::{create_event_loop_proxy, EventLoopCallback, EventLoopFnMutCallback};
use quick_js::JsValue;
use std::cell::RefCell;
use std::sync::mpsc::{Receiver, RecvError, SendError, Sender, TryRecvError};
use std::sync::{mpsc, Arc, Mutex};
use winit::event_loop::EventLoopProxy;
use crate::app::AppEvent;

thread_local! {
    static JS_EVENT_LOOP_PROXY: RefCell<Option<JsEventLoopProxy>> = RefCell::new(None);
}

pub struct JsEventLoop {
    sender: Arc<
        Mutex<
            Box<dyn FnMut(JsEvent) -> Result<(), JsEventLoopClosedError> + Send + Sync + 'static>,
        >,
    >,
}

impl JsEventLoop {
    pub fn create_event_loop_proxy(&mut self) -> JsEventLoopProxy {
        JsEventLoopProxy {
            sender: self.sender.clone(),
        }
    }
}

#[derive(Clone)]
pub struct JsEventLoopProxy {
    sender: Arc<
        Mutex<
            Box<dyn FnMut(JsEvent) -> Result<(), JsEventLoopClosedError> + Send + Sync + 'static>,
        >,
    >,
}

impl JsEventLoopProxy {
    pub fn schedule_macro_task<F: FnOnce() + Send + Sync + 'static>(
        &self,
        callback: F,
    ) -> Result<(), JsEventLoopClosedError> {
        let event = JsEvent::MacroTask(Box::new(callback));
        let mut sender = self.sender.lock().unwrap();
        sender(event)
    }
}

pub enum JsEvent {
    MacroTask(Box<dyn FnOnce() + Send + Sync + 'static>),
}

#[derive(Debug)]
pub struct JsEventLoopClosedError {}

pub struct JsEventLoopCallback {
    event_loop_proxy: JsEventLoopProxy,
    callback: Option<UnsafeFnOnce>,
}

impl JsEventLoopCallback {
    pub fn call(mut self) {
        let mut callback = self.callback.take().unwrap();
        self.event_loop_proxy.schedule_macro_task(callback.into_box()).unwrap();
    }
}

#[derive(Clone)]
pub struct JsEventLoopFnMutCallback<P> {
    event_loop_proxy: JsEventLoopProxy,
    callback: Arc<Mutex<UnsafeFnMut<P>>>,
}

impl<P: Send + Sync + 'static> JsEventLoopFnMutCallback<P> {
    pub fn call(&mut self, param: P) {
        let cb = self.callback.clone();
        self.event_loop_proxy.schedule_macro_task(move || {
            let mut cb = cb.lock().unwrap();
            (cb.callback)(param);
        }).unwrap();
    }
}

pub fn js_init_event_loop<
    F: FnMut(JsEvent) -> Result<(), JsEventLoopClosedError> + Send + Sync + 'static,
>(
    sender: F,
) -> JsEventLoop {
    let mut js_event_loop = JsEventLoop {
        sender: Arc::new(Mutex::new(Box::new(sender))),
    };
    let proxy = js_event_loop.create_event_loop_proxy();
    JS_EVENT_LOOP_PROXY.with_borrow_mut(|cell| {
        if cell.is_some() {
            panic!("Attempting to initialize  js event loop twice");
        }
        cell.replace(proxy);
    });
    js_event_loop
}

pub fn js_is_in_event_loop() -> bool {
    JS_EVENT_LOOP_PROXY.with_borrow(|cell| cell.is_some())
}

pub fn js_create_event_loop_proxy() -> JsEventLoopProxy {
    JS_EVENT_LOOP_PROXY.with_borrow(|cell| {
        let el = cell
            .as_ref()
            .expect("Attempting to use event loop in non-main thread");
        el.clone()
    })
}

pub fn js_create_event_loop_callback<F: FnOnce() + 'static>(callback: F) -> JsEventLoopCallback {
    let callback = unsafe { UnsafeFnOnce::new(callback) };
    let event_loop_proxy = js_create_event_loop_proxy();
    JsEventLoopCallback {
        event_loop_proxy,
        callback: Some(callback),
    }
}

pub fn js_create_event_loop_fn_mut<P: Send + Sync, F: FnMut(P) + 'static>(callback: F) -> JsEventLoopFnMutCallback<P> {
    let fn_mut = UnsafeFnMut {
        callback: Box::new(callback)
    };
    let event_loop_proxy = js_create_event_loop_proxy();
    JsEventLoopFnMutCallback {
        event_loop_proxy,
        callback: Arc::new(Mutex::new(fn_mut)),
    }
}
