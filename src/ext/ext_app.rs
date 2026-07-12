use std::cell::RefCell;
use crate as deft;
use crate::js_module;
use deft_macros::{event, js_methods};
use quick_js::JsValue;
use crate::js::JsError;
use crate::{bind_js_event_listener};
use crate::base::{EventContext, EventListener, EventRegistration};

thread_local! {
    static INSTANCE: RefCell<JsApp>  = RefCell::new(JsApp::new());
}

pub struct JsApp {
    event_registration: EventRegistration,
}

impl JsApp {
    fn new() -> Self {
        Self {
            event_registration: EventRegistration::new(),
        }
    }

}

#[event]
pub struct AppReopenEvent {
    pub has_visible: bool,
}

#[js_methods]
impl JsApp {

    #[js_func]
    pub fn bind_js_event_listener(
        event_type: String,
        listener: JsValue,
    ) -> Result<u32, JsError> {
        INSTANCE.with_borrow_mut(|app| {
            let id = bind_js_event_listener!(
            app, event_type.as_str(), listener;
            "reopen" => AppReopenEventListener,
        );
            let id = id.ok_or_else(|| JsError::new(format!("unknown event_type:{}", event_type)))?;
            Ok(id)
        })
    }

    #[js_func]
    pub fn unbind_js_event_listener(id: u32) {
        INSTANCE.with_borrow_mut(|app| {
            app.event_registration.unregister_event_listener(id)
        })

    }

    pub fn emit<T: 'static>(e: T) {
        INSTANCE.with_borrow_mut(|app| {
            let mut ctx = EventContext::new();
            app.event_registration.emit(e, &mut ctx);
        })
    }

    pub fn register_event_listener<T: 'static, H: EventListener<T> + 'static>(
        &mut self,
        listener: H,
    ) -> u32 {
        self.event_registration.register_event_listener(listener)
    }

    pub fn unregister_event_listener(&mut self, id: u32) {
        self.event_registration.unregister_event_listener(id)
    }

    #[js_func]
    pub fn remove_event_listener(event_type: String, id: u32) {
        INSTANCE.with_borrow_mut(|app| {
            app.event_registration
                .remove_event_listener(&event_type, id)
        })
    }

}

js_module!(JsApp, include_str!("./jsapp.js"));