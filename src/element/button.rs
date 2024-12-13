use ordered_float::OrderedFloat;
use quick_js::JsValue;
use skia_safe::{Canvas, Color};
use yoga::{Edge, StyleUnit};
use crate::base::PropertyValue;
use crate::element::{ElementBackend, Element};
use crate::element::label::Label;
use crate::style::StylePropKey;

pub struct Button {
    label: Label,
    element: Option<Element>,
}

impl Button {

    pub fn set_title(&mut self, title: String) {
        self.label.set_text(title);
    }

    pub fn get_title(&self) -> &str {
        self.label.get_text()
    }
}

impl ElementBackend for Button {
    fn create(mut context: Element) -> Self {
        let mut inst = Self {
            label: Label::create(context.clone()),
            element: None,
        };
        context.style.set_margin(Edge::Top, StyleUnit::Point(OrderedFloat(4.0)));
        context.style.set_margin(Edge::Right, StyleUnit::Point(OrderedFloat(4.0)));
        context.style.set_margin(Edge::Bottom, StyleUnit::Point(OrderedFloat(4.0)));
        context.style.set_margin(Edge::Left, StyleUnit::Point(OrderedFloat(4.0)));

        context.style.set_padding(Edge::Left, StyleUnit::Point(OrderedFloat(4.0)));
        context.style.set_padding(Edge::Right, StyleUnit::Point(OrderedFloat(4.0)));

        context.style.set_border(Edge::Top, 1.0);
        context.style.set_border(Edge::Right, 1.0);
        context.style.set_border(Edge::Bottom, 1.0);
        context.style.set_border(Edge::Left, 1.0);
        let color = Color::from_rgb(128, 128, 128);
        context.style.border_color = [color, color, color, color];
        inst.element = Some(context);
        inst
    }

    fn get_name(&self) -> &str {
        "Button"
    }

    fn draw(&self, canvas: &Canvas) {
        self.label.draw(canvas);
    }

    fn set_property(&mut self, property_name: &str, property_value: JsValue) {
        if let Some(str) = property_value.as_str() {
            match property_name {
                "title" => self.set_title(str.to_string()),
                _ => {}
            }
        }

    }

    fn handle_style_changed(&mut self, key: StylePropKey) {
        self.label.handle_style_changed(key)
    }

    fn before_origin_bounds_change(&mut self) {

    }
}