use crate::element::{Element, ElementBackend, ElementWeak};

pub struct Body {}

impl ElementBackend for Body {
    fn create(element: &mut Element) -> Self {
        Self {}
    }

    fn get_name(&self) -> &str {
        "Body"
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        None
    }
}
