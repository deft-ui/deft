use crate::base::Rect;
use crate::element::container::Container;
use crate::element::label::Label;
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
        let mut container = Container::new_with_tag("tooltip".to_string());
        let mut label = Label::create();
        label.set_text(text);
        let _ = container.add_child(&label, Some(0));
        let popup_holder = Mrc::new(None);
        let timer_handle = {
            let mut popup_holder = popup_holder.clone();
            let window_handle = window_handle.clone();
            set_timeout(
                move || {
                    if let Ok(w) = window_handle.upgrade() {
                        let p = w.popup_ex(&container, target, false);
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
