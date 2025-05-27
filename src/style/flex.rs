use crate::style::PropValueParse;
use std::str::FromStr;
use yoga::{Align, Direction, Display, FlexDirection, Justify, PositionType, Wrap};
impl PropValueParse for Display {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Display::from_str(value).ok()
    }
    fn to_style_string(&self) -> String {
        self.to_string()
    }
}
impl PropValueParse for FlexDirection {
    fn parse_prop_value(value: &str) -> Option<Self> {
        FlexDirection::from_str(value).ok()
    }
    fn to_style_string(&self) -> String {
        self.to_string()
    }
}

impl PropValueParse for Direction {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Direction::from_str(value).ok()
    }
    fn to_style_string(&self) -> String {
        self.to_string()
    }
}

impl PropValueParse for Align {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Some(Align::from_str(value).unwrap_or(Align::FlexStart))
    }
    fn to_style_string(&self) -> String {
        self.to_string()
    }
}

impl PropValueParse for PositionType {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Some(
            PositionType::from_str(value)
                .ok()
                .unwrap_or(PositionType::Static),
        )
    }
    fn to_style_string(&self) -> String {
        self.to_string()
    }
}

impl PropValueParse for Justify {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Some(Justify::from_str(value).unwrap_or(Justify::FlexStart))
    }
    fn to_style_string(&self) -> String {
        self.to_string()
    }
}

impl PropValueParse for Wrap {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Some(Wrap::from_str(value).unwrap_or(Wrap::NoWrap))
    }
    fn to_style_string(&self) -> String {
        self.to_string()
    }
}
