use std::any::Any;
use crate::event::{ClickEvent, KeyDownEvent, KeyUpEvent, MouseDownEvent, MouseUpEvent, TextInputEvent};

pub fn is_form_event(event: &Box<&mut dyn Any>) -> bool {
    event.downcast_ref::<ClickEvent>().is_some()
    || event.downcast_ref::<MouseDownEvent>().is_some()
    || event.downcast_ref::<MouseUpEvent>().is_some()
    || event.downcast_ref::<TextInputEvent>().is_some()
    || event.downcast_ref::<KeyDownEvent>().is_some()
    || event.downcast_ref::<KeyUpEvent>().is_some()
}