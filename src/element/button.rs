use std::any::Any;
use anyhow::Error;
use ordered_float::OrderedFloat;
use quick_js::JsValue;
use skia_safe::{Canvas, Color};
use yoga::{Edge, StyleUnit};
use crate::base::{EventContext, Rect};
use crate::element::{ElementBackend, Element, ElementWeak};
use crate::element::container::Container;
use crate::render::RenderFn;
use crate::style::StylePropKey;

pub struct Button {
    base: Container,
}

impl Button {}

impl ElementBackend for Button {
    fn create(element: &mut Element) -> Self {
        element.set_focusable(true);
        let base = Container::create(element);

        element.style.yoga_node.set_margin(Edge::Top, StyleUnit::Point(OrderedFloat(4.0)));
        element.style.yoga_node.set_margin(Edge::Right, StyleUnit::Point(OrderedFloat(4.0)));
        element.style.yoga_node.set_margin(Edge::Bottom, StyleUnit::Point(OrderedFloat(4.0)));
        element.style.yoga_node.set_margin(Edge::Left, StyleUnit::Point(OrderedFloat(4.0)));

        element.style.yoga_node.set_padding(Edge::Left, StyleUnit::Point(OrderedFloat(4.0)));
        element.style.yoga_node.set_padding(Edge::Right, StyleUnit::Point(OrderedFloat(4.0)));

        element.style.yoga_node.set_border(Edge::Top, 1.0);
        element.style.yoga_node.set_border(Edge::Right, 1.0);
        element.style.yoga_node.set_border(Edge::Bottom, 1.0);
        element.style.yoga_node.set_border(Edge::Left, 1.0);
        let color = Color::from_rgb(128, 128, 128);
        element.style.border_color = [color, color, color, color];
        Self {
            base,
        }
    }

    fn get_name(&self) -> &str {
        "Button"
    }

    fn handle_style_changed(&mut self, key: StylePropKey) {
        self.base.handle_style_changed(key);
    }

    fn render(&mut self) -> RenderFn {
        self.base.render()
    }

    fn on_event(&mut self, event: Box<&mut dyn Any>, ctx: &mut EventContext<ElementWeak>) {
        self.base.on_event(event, ctx);
    }

    fn execute_default_behavior(&mut self, event: &mut Box<dyn Any>, ctx: &mut EventContext<ElementWeak>) -> bool {
        self.base.execute_default_behavior(event, ctx)
    }

    fn handle_origin_bounds_change(&mut self, bounds: &Rect) {
        self.base.handle_origin_bounds_change(bounds)
    }

}