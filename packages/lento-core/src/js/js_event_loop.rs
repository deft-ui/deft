use crate::base::UnsafeFnMut;
use crate::event_loop::EventLoopFnMutCallback;
use quick_js::JsValue;
use std::cell::RefCell;
use std::sync::mpsc::{Receiver, RecvError, SendError, Sender, TryRecvError};
use std::sync::{mpsc, Arc, Mutex};

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

pub fn js_create_event_loop_proxy() -> JsEventLoopProxy {
    JS_EVENT_LOOP_PROXY.with_borrow(|cell| {
        let el = cell
            .as_ref()
            .expect("Attempting to use event loop in non-main thread");
        el.clone()
    })
}
