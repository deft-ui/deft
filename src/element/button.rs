use crate as deft;
use crate::base::{EventContext, Rect};
use crate::element::container::Container;
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::render::RenderFn;
use crate::style::StylePropKey;
use deft_macros::{element_backend, js_methods};
use std::any::Any;

#[element_backend]
pub struct Button {
    element_weak: ElementWeak,
    base: Container,
}

#[js_methods]
impl Button {}

impl ElementBackend for Button {
    fn create(element: &mut Element) -> Self {
        element.is_form_element = true;
        element.set_focusable(true);
        let base = Container::create(element);
        ButtonData {
            base,
            element_weak: element.as_weak(),
        }
        .to_ref()
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        Some(&mut self.base)
    }

    fn handle_style_changed(&mut self, key: StylePropKey) {
        self.base.handle_style_changed(key);
    }

    fn render(&mut self) -> RenderFn {
        self.base.render()
    }

    fn execute_default_behavior(
        &mut self,
        event: &mut Box<dyn Any>,
        ctx: &mut EventContext<ElementWeak>,
    ) -> bool {
        self.base.execute_default_behavior(event, ctx)
    }

    fn handle_origin_bounds_change(&mut self, bounds: &Rect) {
        self.base.handle_origin_bounds_change(bounds)
    }
}
