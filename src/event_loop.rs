use crate::app::{AppEvent, AppEventPayload};
use crate::base::{ResultWaiter, UnsafeFnMut, UnsafeFnOnce};
use std::cell::{Cell, RefCell};
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use winit::event_loop::{ActiveEventLoop, EventLoopClosed, EventLoopProxy};

thread_local! {
    pub static ACTIVE_EVENT_LOOP: Cell<*const ActiveEventLoop> = Cell::new(null_mut());
    pub static STATIC_EVENT_LOOP_PROXY: RefCell<Option<AppEventProxy>> = RefCell::new(None);
}

#[derive(Clone)]
pub struct AppEventProxy {
    proxy: EventLoopProxy<AppEventPayload>,
}

impl AppEventProxy {
    pub fn new(proxy: EventLoopProxy<AppEventPayload>) -> AppEventProxy {
        Self { proxy }
    }

    pub fn send_event(
        &self,
        event: AppEvent,
    ) -> Result<ResultWaiter<()>, EventLoopClosed<AppEventPayload>> {
        let result_waiter = ResultWaiter::new();
        self.proxy.send_event(AppEventPayload {
            event,
            result_waiter: result_waiter.clone(),
        })?;
        Ok(result_waiter)
    }
}

pub struct EventLoopCallback {
    event_loop_proxy: AppEventProxy,
    callback: Option<UnsafeFnOnce>,
}

impl EventLoopCallback {
    pub fn call(mut self) {
        let callback = self.callback.take().unwrap();
        self.event_loop_proxy
            .send_event(AppEvent::Callback(Box::new(|| {
                callback.call();
            })))
            .unwrap();
    }
}

pub struct EventLoopFnMutCallback<P> {
    event_loop_proxy: AppEventProxy,
    callback: Arc<Mutex<UnsafeFnMut<P>>>,
}

impl<P> Clone for EventLoopFnMutCallback<P> {
    fn clone(&self) -> Self {
        Self {
            event_loop_proxy: self.event_loop_proxy.clone(),
            callback: self.callback.clone(),
        }
    }
}

impl<P: Send + Sync + 'static> EventLoopFnMutCallback<P> {
    pub fn call(&mut self, param: P) {
        let cb = self.callback.clone();
        let _ = self
            .event_loop_proxy
            .send_event(AppEvent::Callback(Box::new(move || {
                let mut cb = cb.lock().unwrap();
                (cb.callback)(param);
            })));
    }
}

pub fn create_event_loop_callback<F: FnOnce() + 'static>(callback: F) -> EventLoopCallback {
    let callback = unsafe { UnsafeFnOnce::new(callback) };
    let event_loop_proxy = create_event_loop_proxy();
    EventLoopCallback {
        event_loop_proxy,
        callback: Some(callback),
    }
}

pub fn create_event_loop_fn_mut<P: Send + Sync, F: FnMut(P) + 'static>(
    callback: F,
) -> EventLoopFnMutCallback<P> {
    let fn_mut = UnsafeFnMut {
        callback: Box::new(callback),
    };
    let event_loop_proxy = create_event_loop_proxy();
    EventLoopFnMutCallback {
        event_loop_proxy,
        callback: Arc::new(Mutex::new(fn_mut)),
    }
}

pub fn run_event_loop_task<F: FnOnce()>(event_loop: &ActiveEventLoop, callback: F) {
    ACTIVE_EVENT_LOOP.set(event_loop as *const ActiveEventLoop);
    callback();
    ACTIVE_EVENT_LOOP.set(null_mut());
}

pub fn run_with_event_loop<R, F: FnOnce(&ActiveEventLoop) -> R>(callback: F) -> R {
    let el = ACTIVE_EVENT_LOOP.get();
    unsafe {
        if el == null_mut() {
            panic!("ActiveEventLoop not found");
        }
        callback(&*el)
    }
}

pub fn init_event_loop_proxy(elp: AppEventProxy) {
    STATIC_EVENT_LOOP_PROXY.with_borrow_mut(move |m| {
        m.replace(elp);
    })
}

pub fn create_event_loop_proxy() -> AppEventProxy {
    STATIC_EVENT_LOOP_PROXY.with_borrow(|p| {
        p.as_ref()
            .expect("Failed to create event loop proxy")
            .clone()
    })
}
