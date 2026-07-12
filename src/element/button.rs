use crate as deft;
use crate::element::{Element, Widget};
use crate::js_module;
use deft_macros::{widget, js_methods};

#[widget]
pub struct Button {

}

impl Widget for Button {}

#[js_methods]
impl Button {

    #[js_func]
    pub fn create() -> Button {
        let mut element = Element::new("button");
        element.is_form_element = true;
        element.set_focusable(true);
        Button {
            el: element,
        }
    }

}

js_module!(Button);