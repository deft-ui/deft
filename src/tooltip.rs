use crate::base::Rect;
use crate::element::container::Container;
use crate::element::label::Label;
use crate::element::{Element, ElementBackend, ElementType};
use crate::mrc::Mrc;
use crate::timer::{set_timeout, TimerHandle};
use crate::window::popup::Popup;
use crate::window::WindowHandle;
use std::ops::Deref;

pub struct Tooltip {
    timer_handle: Option<TimerHandle>,
    popup_holder: Mrc<Option<Popup>>,
}

impl Tooltip {
    pub fn new(window_handle: WindowHandle, text: String, target: Rect) -> Self {
        let mut container_el = Element::create(Container::create);
        container_el.tag = "tooltip".to_string();
        container_el.set_element_type(ElementType::Widget);
        let mut el = Element::create(Label::create);
        el.tag = "label".to_string();
        el.set_element_type(ElementType::Widget);
        let label = el.get_backend_mut_as::<Label>();
        label.set_text(text);
        let _ = container_el.add_child(el.clone(), 0);
        let popup_holder = Mrc::new(None);
        let timer_handle = {
            let mut popup_holder = popup_holder.clone();
            let window_handle = window_handle.clone();
            set_timeout(
                move || {
                    if let Ok(w) = window_handle.upgrade_mut() {
                        let p = w.popup_ex(container_el, target, false);
                        popup_holder.replace(p);
                    }
                },
                100,
            )
        };
        Tooltip {
            timer_handle: Some(timer_handle),
            popup_holder,
        }
    }
}

impl Drop for Tooltip {
    fn drop(&mut self) {
        if let Some(popup) = self.popup_holder.deref() {
            let _ = popup.close();
        }
    }
}
