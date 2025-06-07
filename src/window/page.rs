use crate as deft;
use crate::base::EventRegistration;
use crate::element::body::Body;
use crate::element::{Element, ElementBackend, ElementType};
use crate::window::WindowHandle;
use crate::{js_weak_value, ok_or_return};
use deft_macros::{js_methods, mrc_object};

#[mrc_object]
pub struct Page {
    window_weak: WindowHandle,
    event_registration: EventRegistration<PageWeak>,
    body: Element,
}

js_weak_value!(Page, PageWeak);

#[js_methods]
impl Page {
    pub fn new(window_weak: WindowHandle, content: Element) -> Page {
        let mut body = Element::create(Body::create);
        body.set_tag("body".to_owned());
        body.set_element_type(ElementType::Widget);
        body.add_child(content, 0).unwrap();
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
    pub fn close(&self) {
        if let Ok(mut window) = self.window_weak.upgrade_mut() {
            window.close_page(self.clone());
        }

    }
}
