use crate as deft;
use crate::app::exit_app;
use crate::is_mobile_platform;
use crate::js::js_engine::JsEngine;
use deft_macros::js_methods;
use log::error;
use quick_js::exception::HostPromiseRejectionTracker;
use quick_js::JsValue;
use std::env;

struct UserPromiseRejectionTracker {
    handler: JsValue,
}

impl HostPromiseRejectionTracker for UserPromiseRejectionTracker {
    fn track_promise_rejection(&mut self, promise: JsValue, reason: JsValue, _is_handled: bool) {
        if let Err(e) = self.handler.call_as_function(vec![reason, promise]) {
            error!("Failed to call user promise rejection handler: {:?}", e);
        }
    }
}

#[allow(nonstandard_style)]
pub struct process;

#[js_methods]
impl process {
    #[js_func]
    pub fn exit(code: i32) {
        let _ = exit_app(code);
    }

    #[js_func]
    pub fn argv() -> Vec<String> {
        env::args().collect()
    }

    #[js_func]
    pub fn is_mobile_platform() -> bool {
        is_mobile_platform()
    }

    #[js_func]
    pub fn set_promise_rejection_tracker(handler: JsValue) {
        let mut js_engine = JsEngine::get();
        let tracker = UserPromiseRejectionTracker { handler };
        js_engine.js_context.set_promise_rejection_tracker(tracker);
    }
}
