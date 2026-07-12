use crate as deft;
use crate::js::JsError;
use crate::timer::{set_interval, set_timeout, TimerHandle};
use deft_macros::js_methods;
use log::error;
use quick_js::JsValue;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use crate::js_module;

thread_local! {
    pub static NEXT_TIMER_ID: Cell<i32> = Cell::new(1);
    pub static TIMERS: RefCell<HashMap<i32, TimerHandle>> = RefCell::new(HashMap::new());
}

pub struct Timer {

}

js_module!(Timer, include_str!("./timer.js"));

#[js_methods]
impl Timer {
    #[js_func]
    pub fn set_timeout(callback: JsValue, timeout: Option<i32>) -> Result<i32, JsError> {
        let id = NEXT_TIMER_ID.get();
        NEXT_TIMER_ID.set(id + 1);

        let handle = set_timeout(
            move || {
                let r = callback.call_as_function(vec![]);
                match r {
                    Ok(_) => {}
                    Err(err) => {
                        error!("timeout callback error:{:?}", err);
                    }
                }
                TIMERS.with_borrow_mut(|m| m.remove(&id));
            },
            timeout.unwrap_or(0) as u64,
        );
        TIMERS.with_borrow_mut(move |m| {
            assert!(m.insert(id, handle).is_none());
        });
        Ok(id)
    }

    #[js_func]
    pub fn clear_timeout(id: i32) {
        TIMERS.with_borrow_mut(|m| m.remove(&id));
    }

    #[js_func]
    pub fn set_interval(callback: JsValue, interval: i32) -> Result<i32, JsError> {
        let id = NEXT_TIMER_ID.get();
        NEXT_TIMER_ID.set(id + 1);

        let handle = set_interval(
            move || {
                let _ = callback.call_as_function(vec![]);
            },
            interval as u64,
        );

        TIMERS.with_borrow_mut(|m| m.insert(id, handle));
        Ok(id)
    }

    #[js_func]
    pub fn clear_interval(id: i32) {
        TIMERS.with_borrow_mut(|m| m.remove(&id));
    }

}

