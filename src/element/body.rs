use deft_macros::{widget, js_methods};
use crate as deft;
use crate::element::{Element, Widget};
use crate::js_module;
#[widget]
pub struct Body {

}

impl PartialEq<Self> for Body {
    fn eq(&self, other: &Self) -> bool {
        self.el.eq(&other.el)
    }
}

impl Eq for Body {

}

impl Clone for Body {
    fn clone(&self) -> Self {
        Self {
            el: self.el.clone_element()
        }
    }
}

impl Widget for Body {}

#[js_methods]
impl Body {
    #[js_func]
    pub fn create() -> Self {
        Body {
            el: Element::new("body"),
        }
    }
    
    pub(crate) fn element_mut(&mut self) -> &mut Element {
        &mut self.el
    }
    
}

js_module!(Body);
