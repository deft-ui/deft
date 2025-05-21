use crate::style::color::parse_color;
use crate::style::length::{Length, LengthOrPercent};
use crate::style::PropValueParse;
use skia_safe::Color;

pub fn parse_border(value: &str) -> (LengthOrPercent, Color) {
    let parts = value.split(" ");
    let mut width = LengthOrPercent::Length(Length::PX(0.0));
    let mut color = Color::from_rgb(0, 0, 0);
    for p in parts {
        let p = p.trim();
        if let Some(c) = parse_color(p) {
            color = c;
        } else if let Some(w) = LengthOrPercent::parse_prop_value(p) {
            width = w;
        }
    }
    (width, color)
}
