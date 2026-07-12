use crate as deft;
use crate::base::EventRegistration;
use crate::js_module;
use crate::element::body::Body;
use crate::element::Element;
use crate::js_weak_value;
use crate::window::WindowHandle;
use deft_macros::{js_methods, mrc_object};

#[mrc_object]
pub struct Page {
    window_weak: WindowHandle,
    event_registration: EventRegistration,
    body: Body,
}

js_weak_value!(Page, PageWeak);

#[js_methods]
impl Page {
    pub fn new(window_weak: WindowHandle, content: Element) -> Page {
        let mut body = Body::create();
        body.add_child(&content, Some(0)).unwrap();
        PageData {
            body,
            window_weak,
            event_registration: EventRegistration::new(),
        }
        .to_ref()
    }

    pub fn get_body(&self) -> &Body {
        &self.body
    }

    #[js_func]
    pub fn close(&self) {
        if let Ok(mut window) = self.window_weak.upgrade() {
            window.close_page(self.clone());
        }
    }
}

js_module!(Page);
