use std::cell::{Cell};
use std::ptr::null_mut;
use std::sync::{Arc, Mutex, OnceLock};
use winit::event_loop::{ActiveEventLoop, EventLoopClosed, EventLoopProxy};
use crate::app::AppEvent;
use crate::base::{UnsafeFnMut, UnsafeFnOnce};

#[derive(Debug)]
struct  EventLoopProxyHolder {
    event_loop_proxy: Option<EventLoopProxy<AppEvent>>,
}

thread_local! {
    pub static ACTIVE_EVENT_LOOP: Cell<*const ActiveEventLoop> = Cell::new(null_mut());
}

unsafe impl Sync for EventLoopProxyHolder {}
unsafe impl Send for EventLoopProxyHolder {}



pub struct EventLoopCallback {
    callback: Option<UnsafeFnOnce>,
}

impl EventLoopCallback {
    pub fn call(mut self) {
        let mut callback = self.callback.take().unwrap();
        run_on_event_loop(|| {
            callback.call();
        })
    }
}

#[derive(Clone)]
pub struct EventLoopFnMutCallback<P> {
    callback: Arc<Mutex<UnsafeFnMut<P>>>,
}

impl<P: Send + Sync + 'static> EventLoopFnMutCallback<P> {
    pub fn call(&mut self, param: P) {
        let cb = self.callback.clone();
        run_on_event_loop(move || {
            let mut cb = cb.lock().unwrap();
            (cb.callback)(param);
        })
    }
}

static EVENT_LOOP_PROXY: OnceLock<EventLoopProxyHolder> = OnceLock::new();

fn get_event_loop_proxy_internal() -> &'static EventLoopProxyHolder {
    EVENT_LOOP_PROXY.get_or_init(|| EventLoopProxyHolder {
        event_loop_proxy: None,
    })
}

pub fn get_event_proxy() -> EventLoopProxy<AppEvent> {
    get_event_loop_proxy_internal().event_loop_proxy.clone().unwrap()
}

pub fn create_event_loop_callback<F: FnOnce() + 'static>(callback: F) -> EventLoopCallback {
    let callback = unsafe { UnsafeFnOnce::new(callback) };
    EventLoopCallback { callback: Some(callback) }
}

pub fn create_event_loop_fn_mut<P: Send + Sync, F: FnMut(P) + 'static>(callback: F) -> EventLoopFnMutCallback<P> {
    let fn_mut = UnsafeFnMut {
        callback: Box::new(callback)
    };
    EventLoopFnMutCallback {
        callback: Arc::new(Mutex::new(fn_mut)),
    }
}

pub fn run_on_event_loop<F: FnOnce() + 'static + Send + Sync>(callback: F) {
    let proxy = get_event_proxy();
    proxy.send_event(AppEvent::Callback(Box::new(callback))).unwrap();
}

pub fn set_event_proxy(proxy: EventLoopProxy<AppEvent>) {
    EVENT_LOOP_PROXY.set(EventLoopProxyHolder{
        event_loop_proxy: Some(proxy),
    }).unwrap();
}

pub fn send_event(event: AppEvent) -> Result<(), EventLoopClosed<AppEvent>> {
    let proxy = get_event_proxy();
    proxy.send_event(event)
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