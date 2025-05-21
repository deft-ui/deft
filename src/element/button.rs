use crate as deft;
use crate::base::{EventContext, Rect};
use crate::element::container::Container;
use crate::element::util::is_form_event;
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::ok_or_return;
use crate::render::RenderFn;
use crate::style::StylePropKey;
use deft_macros::{element_backend, js_methods};
use std::any::Any;

#[element_backend]
pub struct Button {
    element_weak: ElementWeak,
    base: Container,
    disabled: bool,
}

#[js_methods]
impl Button {
    #[js_func]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    #[js_func]
    pub fn set_disabled(&mut self, disabled: bool) {
        let mut ele = ok_or_return!(self.element_weak.upgrade());
        if disabled {
            ele.set_attribute("disabled".to_string(), "".to_string());
        } else {
            ele.remove_attribute("disabled".to_string());
        }
    }
}

impl ElementBackend for Button {
    fn create(element: &mut Element) -> Self {
        element.set_focusable(true);
        let base = Container::create(element);
        ButtonData {
            base,
            element_weak: element.as_weak(),
            disabled: false,
        }
        .to_ref()
    }

    fn get_name(&self) -> &str {
        "Button"
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

    fn on_event(&mut self, event: Box<&mut dyn Any>, ctx: &mut EventContext<ElementWeak>) {
        if self.disabled && is_form_event(&event) {
            ctx.propagation_cancelled = true;
        } else {
            self.base.on_event(event, ctx);
        }
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

    fn on_attribute_changed(&mut self, key: &str, value: Option<&str>) {
        match key {
            "disabled" => self.disabled = value.is_some(),
            _ => self.base.on_attribute_changed(key, value),
        }
    }

    fn can_focus(&mut self) -> bool {
        !self.disabled
    }
}
