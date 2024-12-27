use crate::element::{ElementBackend, Element};

pub struct Container {
    dirty: bool,
    element: Element,
}

impl Container {

}

impl ElementBackend for Container {
    fn create(element: Element) -> Self {
        Self {
            dirty: false,
            element,
        }
    }

    fn get_name(&self) -> &str {
        "Container"
    }

}

