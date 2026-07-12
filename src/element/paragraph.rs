#![allow(unused)]
pub mod simple_paragraph_builder;

use crate as deft;
use crate::base::{EventContext, MouseDetail, MouseEventType};
use crate::color::parse_hex_color;
use crate::element::paragraph::simple_paragraph_builder::SimpleParagraphBuilder;
use crate::element::text::simple_text_paragraph::SimpleTextParagraph;
use crate::element::text::{intersect_range, ColOffset};
use crate::element::{Element, Widget, ElementWeak};
use crate::event::{
    Event, FocusShiftEvent, KeyDownEvent, KeyEventDetail, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, SelectEndEvent, SelectMoveEvent, SelectStartEvent, KEY_MOD_CTRL,
};
use crate::font::family::{FontFamilies, FontFamily};
use crate::number::DeNan;
use crate::paint::Painter;
use crate::render::RenderFn;
use crate::string::StringUtils;
use crate::style::color::parse_optional_color_str;
use crate::style::font::FontStyle;
use crate::style::{PropValueParse, StylePropKey};
use crate::text::textbox::{TextCoord, TextUnit};
use crate::text::{TextAlign, TextDecoration, TextStyle};
use crate::{js_deserialize, js_serialize, some_or_continue};
use deft_macros::{widget, js_methods};
use measure_time::print_time;
use serde::{Deserialize, Serialize};
use skia_safe::font_style::{Weight, Width};
use skia_safe::wrapper::NativeTransmutableWrapper;
use skia_safe::{Color, Paint};
use std::any::Any;
use std::str::FromStr;
use yoga::{Context, MeasureMode, Node, NodeRef, Size};

const ZERO_WIDTH_WHITESPACE: &str = "\u{200B}";

#[derive(Clone)]
pub struct ParagraphParams {
    pub mask_char: Option<char>,
    pub text_wrap: Option<bool>,
    pub line_height: Option<f32>,
    pub align: TextAlign,
    pub color: Color,
    pub font_size: f32,
    pub font_families: FontFamilies,
    pub font_weight: Weight,
    pub font_style: FontStyle,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_measure() {
        let text_demo = include_str!("../../Cargo.lock");
        let mut text = String::new();
        for i in 0..200 {
            text.push_str(text_demo);
        }
        // let font = DEFAULT_TYPE_FACE.with(|tf| Font::from_typeface(tf, 14.0));
        // debug!("font {:?}", font.typeface().family_name());
        // print_time!("measure time");
        // let result = font.measure_text(text.as_str(), None);
    }
}
