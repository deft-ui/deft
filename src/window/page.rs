use crate as deft;
use crate::base::EventRegistration;
use crate::element::Element;
use crate::window::WindowWeak;
use crate::{js_weak_value, ok_or_return};
use deft_macros::{js_methods, mrc_object};

#[mrc_object]
pub struct Page {
    window_weak: WindowWeak,
    event_registration: EventRegistration<PageWeak>,
    body: Element,
}

js_weak_value!(Page, PageWeak);

#[js_methods]
impl Page {
    pub fn new(window_weak: WindowWeak, body: Element) -> Page {
        PageData {
            body,
            window_weak,
            event_registration: EventRegistration::new(),
        }
        .to_ref()
    }

    pub fn get_body(&self) -> &Element {
        &self.body
    }

    #[js_func]
    pub fn close(self) {
        let mut window = ok_or_return!(self.window_weak.upgrade());
        window.close_page(self);
    }
}
