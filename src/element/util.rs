use crate::element::Element;
use crate::event::{
    ClickEvent, KeyDownEvent, KeyUpEvent, MouseDownEvent, MouseUpEvent, TextInputEvent,
};
use std::any::Any;

pub fn is_form_event(event: &Box<&mut dyn Any>) -> bool {
    event.downcast_ref::<ClickEvent>().is_some()
        || event.downcast_ref::<MouseDownEvent>().is_some()
        || event.downcast_ref::<MouseUpEvent>().is_some()
        || event.downcast_ref::<TextInputEvent>().is_some()
        || event.downcast_ref::<KeyDownEvent>().is_some()
        || event.downcast_ref::<KeyUpEvent>().is_some()
}

pub fn get_tree_level(element: &Element) -> usize {
    if let Some(p) = element.get_parent() {
        get_tree_level(&p) + 1
    } else {
        0
    }
}
