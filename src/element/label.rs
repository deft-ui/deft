use std::cell::RefCell;
use std::rc::Rc;
use anyhow::{Error};
use quick_js::JsValue;
use yoga::{Context, MeasureMode, Node, NodeRef, Size};
use skia_safe::{Canvas, Color, Color4f, Font, FontMgr, FontStyle, Paint, Typeface};
use skia_safe::textlayout::{FontCollection, Paragraph, ParagraphBuilder, ParagraphStyle, StrutStyle, TextAlign, TextStyle};
use crate::base::{PropertyValue, Rect, TextUpdateDetail};
use crate::color::parse_hex_color;
use crate::element::{ElementData, ElementBackend, Element};
use crate::event::TextUpdateEvent;
use crate::js::js_value_util::JsValueHelper;
use crate::number::DeNan;
use crate::string::StringUtils;
use crate::style::StylePropKey;

pub struct AttributeText {
    pub text: String,
    pub font: Font,
}

thread_local! {
    pub static DEFAULT_TYPE_FACE: Typeface = default_typeface();
    pub static FONT_MGR: FontMgr = FontMgr::new();
    pub static FONT_COLLECTION: FontCollection = FontCollection::new();
}


fn default_typeface() -> Typeface {
    let font_mgr = FontMgr::new();
    font_mgr.legacy_make_typeface(None, FontStyle::default()).unwrap()
}

pub fn parse_align(align: &str) -> TextAlign {
    match align {
        "left" => TextAlign::Left,
        "right" => TextAlign::Right,
        "center" => TextAlign::Center,
        _ => TextAlign::Left,
    }
}