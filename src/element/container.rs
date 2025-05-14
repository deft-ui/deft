use crate::element::{ElementBackend, Element, ElementWeak};

pub struct Container {
}

impl Container {

}

impl ElementBackend for Container {
    fn create(element: &mut Element) -> Self {
        Self {}
    }

    fn get_name(&self) -> &str {
        "Container"
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        None
    }
}

