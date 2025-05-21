use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use deft_macros::js_func;
use log::error;
use quick_js::{JsValue};
use crate::timer::{set_interval, set_timeout, TimerHandle};
use crate as deft;
use crate::js::JsError;

thread_local! {
    pub static NEXT_TIMER_ID: Cell<i32> = Cell::new(1);
    pub static TIMERS: RefCell<HashMap<i32, TimerHandle>> = RefCell::new(HashMap::new());
}

#[js_func]
pub fn timer_set_timeout(callback: JsValue, timeout: Option<i32>) -> Result<i32, JsError> {
    let id = NEXT_TIMER_ID.get();
    NEXT_TIMER_ID.set(id + 1);

    let handle = set_timeout(move || {
        let r = callback.call_as_function(vec![]);
        match r {
            Ok(_) => {}
            Err(err) => {
                error!("timeout callback error:{:?}", err);
            }
        }
        TIMERS.with_borrow_mut(|m| m.remove(&id));
    }, timeout.unwrap_or(0) as u64);
    TIMERS.with_borrow_mut(move |m| {
        assert!(m.insert(id, handle).is_none());
    });
    Ok(id)
}

#[js_func]
pub fn timer_clear_timeout(id: i32)  {
    TIMERS.with_borrow_mut(|m| m.remove(&id));
}

#[js_func]
pub fn timer_set_interval(callback: JsValue, interval: i32) -> Result<i32, JsError> {
    let id = NEXT_TIMER_ID.get();
    NEXT_TIMER_ID.set(id + 1);

    let handle = set_interval(move || {
        let _ = callback.call_as_function(vec![]);
    }, interval as u64);

    TIMERS.with_borrow_mut(|m| m.insert(id, handle));
    Ok(id)
}

#[js_func]
pub fn timer_clear_interval(id: i32) {
    TIMERS.with_borrow_mut(|m| m.remove(&id));
}
