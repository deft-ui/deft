use std::any::Any;
use anyhow::Error;
use ordered_float::OrderedFloat;
use quick_js::JsValue;
use skia_safe::{Canvas, Color};
use yoga::{Edge, StyleUnit};
use crate::base::{EventContext, PropertyValue, Rect};
use crate::element::{ElementBackend, Element, ElementWeak};
use crate::element::container::Container;
use crate::style::StylePropKey;

pub struct Button {
    base: Container,
    element: Element,
}

impl Button {}

impl ElementBackend for Button {
    fn create(mut element: Element) -> Self {
        let base = Container::create(element.clone());

        element.style.set_margin(Edge::Top, StyleUnit::Point(OrderedFloat(4.0)));
        element.style.set_margin(Edge::Right, StyleUnit::Point(OrderedFloat(4.0)));
        element.style.set_margin(Edge::Bottom, StyleUnit::Point(OrderedFloat(4.0)));
        element.style.set_margin(Edge::Left, StyleUnit::Point(OrderedFloat(4.0)));

        element.style.set_padding(Edge::Left, StyleUnit::Point(OrderedFloat(4.0)));
        element.style.set_padding(Edge::Right, StyleUnit::Point(OrderedFloat(4.0)));

        element.style.set_border(Edge::Top, 1.0);
        element.style.set_border(Edge::Right, 1.0);
        element.style.set_border(Edge::Bottom, 1.0);
        element.style.set_border(Edge::Left, 1.0);
        let color = Color::from_rgb(128, 128, 128);
        element.style.border_color = [color, color, color, color];
        Self {
            base,
            element: element.clone(),
        }
    }

    fn get_name(&self) -> &str {
        "Button"
    }

    fn handle_style_changed(&mut self, key: StylePropKey) {
        self.base.handle_style_changed(key);
    }


    fn draw(&self, canvas: &Canvas) {
        self.base.draw(canvas);
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