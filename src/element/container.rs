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

}

