use crate::element::Element;
use crate::event::{
    ClickEvent, Event, KeyDownEvent, KeyUpEvent, MouseDownEvent, MouseUpEvent, TextInputEvent,
};

pub fn is_form_event(event: &Event) -> bool {
    ClickEvent::is(event)
        || MouseDownEvent::is(event)
        || MouseUpEvent::is(event)
        || TextInputEvent::is(event)
        || KeyDownEvent::is(event)
        || KeyUpEvent::is(event)
}

pub fn get_tree_level(element: &Element) -> usize {
    if let Some(p) = element.get_parent() {
        get_tree_level(&p) + 1
    } else {
        0
    }
}
