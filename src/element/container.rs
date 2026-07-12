use deft_macros::{widget, js_methods};
use crate as deft;
use crate::element::{Element, Widget};
use crate::js_module;

#[widget]
pub struct Container {}

#[js_methods]
impl Container {
    #[js_func]
    pub fn create() -> Self {
        Self::new_with_tag("container".to_string())
    }

    #[js_func]
    pub fn new_with_tag(tag: String) -> Self {
        let el = Element::new(&tag);
        Container {
            el,
        }
    }

}

impl Widget for Container {

}

js_module!(Container);
