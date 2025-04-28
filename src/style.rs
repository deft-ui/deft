pub mod border_path;
pub mod css_manager;
mod select;

use crate as deft;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::f32::consts::PI;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use anyhow::{anyhow, Error};
use cssparser::parse_color_keyword;
use deft_macros::mrc_object;
use ordered_float::{Float, OrderedFloat};
use quick_js::JsValue;
use skia_safe::{Color, Image, Matrix, Path};
use yoga::{Align, Direction, Display, Edge, FlexDirection, Justify, Node, Overflow, PositionType, StyleUnit, Wrap};
use crate::base::Rect;
use crate::color::parse_hex_color;
use crate::{match_both, ok_or_return, send_app_event, some_or_return};
use crate::animation::{AnimationInstance, SimpleFrameController, WindowAnimationController};
use crate::border::build_border_paths;
use crate::cache::CacheValue;
use crate::animation::ANIMATIONS;
use crate::animation::css_actor::CssAnimationActor;
use crate::element::{Element, ElementWeak};
use crate::event_loop::create_event_loop_callback;
use crate::mrc::{Mrc, MrcWeak};
use crate::number::DeNan;
use crate::paint::MatrixCalculator;
use crate::string::StringUtils;
use crate::style_list::{ParsedStyleProp, StyleList};
use crate::timer::{set_timeout, TimerHandle};

#[derive(Clone, Debug, PartialEq)]
pub enum StylePropertyValue {
    Float(f32),
    String(String),
    Invalid,
}


//TODO rename
pub trait PropValueParse: Sized {
    fn parse_prop_value(value: &str) -> Option<Self>;
    fn to_style_string(&self) -> String;
}

impl PropValueParse for Length {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Self::from_str(value)
    }

    fn to_style_string(&self) -> String {
        self.to_str()
    }
}

impl PropValueParse for LengthOrPercent {

    fn parse_prop_value(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("auto") {
            return Some(LengthOrPercent::Auto)
        } else if let Some(v) = Length::from_str(value) {
            return Some(Self::Length(v))
        } else {
            let value = value.trim();
            if let Some(v) = value.strip_suffix("%") {
                let value = f32::from_str(v).ok()?;
                return Some(Self::Percent(value));
            }
        }
        Some(LengthOrPercent::Undefined)
    }

    fn to_style_string(&self) -> String {
        match self {
            LengthOrPercent::Length(v) => v.to_style_string(),
            LengthOrPercent::Percent(v) => format!("{}%", v),
            LengthOrPercent::Undefined => "".to_string(),
            LengthOrPercent::Auto => "auto".to_string(),
        }
    }
}

impl PropValueParse for Color {
    fn parse_prop_value(value: &str) -> Option<Self> {
        parse_color(value)
    }
    fn to_style_string(&self) -> String {
        format!("#{:02X}{:02X}{:02X}{:02X}", self.r(), self.g(), self.b(), self.a())
    }
}

impl PropValueParse for Display {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Display::from_str(value).ok()
    }
    fn to_style_string(&self) -> String {
        self.to_string()
    }
}

impl PropValueParse for f32 {
    fn parse_prop_value(value: &str) -> Option<Self> {
        f32::from_str(value).ok()
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
        Some(PositionType::from_str(value).ok().unwrap_or(PositionType::Static))
    }
    fn to_style_string(&self) -> String {
        self.to_string()
    }
}

impl PropValueParse for Overflow {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Some(Overflow::from_str(value).unwrap_or(Overflow::Visible))
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


impl PropValueParse for StyleTransform {
    fn parse_prop_value(value: &str) -> Option<Self> {
        //TODO support multiple op
        if let Some(op) = StyleTransformOp::parse(value) {
            Some(Self {
                op_list: vec![op]
            })
        } else {
            None
        }
    }
    fn to_style_string(&self) -> String {
        self.op_list.iter()
            .map(|it| it.to_style_string())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl PropValueParse for String {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Some(value.to_string())
    }
    fn to_style_string(&self) -> String {
        self.to_string()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TranslateLength {
    Point(f32),
    Percent(f32),
}

impl TranslateLength {
    pub fn adapt_zero(&mut self, other: &mut Self) {
        if self.is_zero() {
            match other {
                TranslateLength::Point(_) => {
                    *self = TranslateLength::Point(0.0);
                }
                TranslateLength::Percent(_) => {
                    *self = TranslateLength::Percent(0.0);
                }
            }
        } else if other.is_zero() {
            other.adapt_zero(self)
        }
    }
    pub fn to_absolute(&self, block_length: f32) -> f32 {
        match self {
            TranslateLength::Point(p) => { *p }
            TranslateLength::Percent(p) => { *p / 100.0 * block_length }
        }
    }

    pub fn to_style_string(&self) -> String {
        match self {
            TranslateLength::Point(v) => {
                v.to_string()
            }
            TranslateLength::Percent(p) => {
                format!("{}%", p)
            }
        }
    }

    fn is_zero(&self) -> bool {
        match self {
            TranslateLength::Point(v) => *v == 0.0,
            TranslateLength::Percent(v) => *v == 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TranslateParams(pub TranslateLength, pub TranslateLength);

#[derive(Clone, Debug, PartialEq)]
pub struct ScaleParams(pub f32, pub f32);

#[derive(Clone, Debug, PartialEq)]
pub enum StyleTransformOp {
    Rotate(f32),
    Scale(ScaleParams),
    Translate(TranslateParams),
}

impl StyleTransformOp {
    pub fn parse(str: &str) -> Option<Self> {
        let value = str.trim();
        if !value.ends_with(")") {
            return None;
        }
        let left_p = value.find("(")?;
        let func = &value[0..left_p];
        let param_str = &value[left_p + 1..value.len() - 1];
        //TODO support double params
        match func {
            //"matrix" => parse_matrix(param_str).ok(),
            "translate" => parse_translate_op(param_str),
            "rotate" => parse_rotate_op(param_str),
            "scale" => parse_scale_op(param_str),
            _ => None,
        }
    }
    pub fn to_style_string(&self) -> String {
        match self {
            StyleTransformOp::Rotate(v) => {
                format!("rotate({})", v)
            }
            StyleTransformOp::Scale(v) => {
                format!("scale({}, {})", v.0, v.1)
            }
            StyleTransformOp::Translate(p) => {
                format!("translate({}, {})", p.0.to_style_string(), p.1.to_style_string())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyleTransform {
    pub op_list: Vec<StyleTransformOp>,
}

impl StyleTransform {
    pub fn empty() -> StyleTransform {
        Self {
            op_list: Vec::new(),
        }
    }

    pub fn preprocess(&self) -> StyleTransform {
        let mut list = Vec::new();
        for op in self.op_list.clone() {
            if let StyleTransformOp::Translate(params) = op {
                let (mut tl, mut tl2) = (params.0, params.1);
                tl.adapt_zero(&mut tl2);
                list.push(StyleTransformOp::Translate(TranslateParams(tl, tl2)));
                continue;
            }
            list.push(op);
        }
        StyleTransform { op_list: list }
    }

    pub fn apply(&self, width: f32, height: f32, mc: &mut MatrixCalculator) {
        for op in &self.op_list {
            match op {
                StyleTransformOp::Rotate(deg) => {
                    mc.rotate(*deg, None);
                }
                StyleTransformOp::Scale(ScaleParams(x, y)) => {
                    mc.scale((*x, *y));
                }
                StyleTransformOp::Translate(params) => {
                    let (x, y) = (&params.0, &params.1);
                    let x = x.to_absolute(width);
                    let y = y.to_absolute(height);
                    mc.translate((x, y));
                }
            }
        }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct StyleBorder(LengthOrPercent, Color);

#[derive(Clone, Debug, PartialEq)]
pub enum StylePropVal<T: PropValueParse> {
    Custom(T),
    Inherit,
    Unset,
}

impl<T: Clone + PropValueParse> StylePropVal<T> {
    //TODO remove
    pub fn resolve(&self, default: &T) -> T {
        match self {
            StylePropVal::Custom(v) => { v.clone() }
            StylePropVal::Unset => { default.clone() }
            StylePropVal::Inherit => { todo!() }
        }
    }

    pub fn to_style_string(&self) -> String {
        match self {
            StylePropVal::Custom(v) => {
                v.to_style_string()
            }
            StylePropVal::Inherit => {
                "inherit".to_string()
            }
            StylePropVal::Unset => {
                "unset".to_string()
            }
        }
    }

}

macro_rules! define_style_props {
    ($($name: ident => $type: ty, $compute_type: ty; )*) => {
        #[derive(Clone, Debug, PartialEq)]
        pub enum StyleProp {
            $(
                $name(StylePropVal<$type>),
            )*
        }

        #[derive(Clone, Debug, PartialEq)]
        pub enum ResolvedStyleProp {
            $(
                $name($type),
            )*
        }

        impl ResolvedStyleProp {
             pub fn key(&self) -> StylePropKey {
                match self {
                    $(
                        Self::$name(_) => StylePropKey::$name,
                    )*
                }
            }
        }

        #[derive(Clone, Debug, PartialEq)]
        pub enum ComputedStyleProp {
            $(
                $name($compute_type),
            )*
        }

        #[derive(Clone, Hash, PartialEq, Eq, Copy, Debug)]
        pub enum StylePropKey {
            $(
                $name,
            )*
        }

        impl StylePropKey {
            pub fn parse(key: &str) -> Option<Self> {
                $(
                    if key.to_lowercase() == stringify!($name).to_lowercase() {
                        return Some(StylePropKey::$name);
                    }
                )*
                None
            }
        }

        impl StyleProp {
            pub fn parse_value(key: StylePropKey, value: &str) -> Option<StyleProp> {
                $(
                    if key == StylePropKey::$name {
                        return <$type>::parse_prop_value(value).map(|v| StyleProp::$name(StylePropVal::Custom(v)));
                    }
                )*
                return None
            }
            pub fn parse(key: &str, value: &str) -> Option<StyleProp> {
                let key = key.to_lowercase();
                let k = key.as_str();
                $(
                    if k == stringify!($name).to_lowercase().as_str() {
                        let value_lowercase = value.to_lowercase();
                        let value_lowercase = value_lowercase.as_str();
                        if value_lowercase == "inherit" {
                            return Some(StyleProp::$name(StylePropVal::Inherit));
                        } else if value_lowercase == "unset" {
                            return Some(StyleProp::$name(StylePropVal::Unset));
                        } else {
                            return <$type>::parse_prop_value(value).map(|v| StyleProp::$name(StylePropVal::Custom(v)));
                        }
                    }
                )*
                return None
            }
            pub fn name(&self) -> &str {
                match self {
                    $(
                        Self::$name(_) => stringify!($name),
                    )*
                }
            }
            pub fn key(&self) -> StylePropKey {
                match self {
                    $(
                        Self::$name(_) => StylePropKey::$name,
                    )*
                }
            }
            pub fn unset(&self) -> Self {
                match self {
                    $(
                       Self::$name(_) => Self::$name(StylePropVal::Unset),
                    )*
                }
            }

            pub fn is_inherited(&self) -> bool {
                match self {
                    $(
                       Self::$name(v) => *v == StylePropVal::Inherit,
                    )*
                }
            }

            pub fn to_style_string(&self) -> String {
                match self {
                    $(
                       Self::$name(v) => v.to_style_string(),
                    )*
                }
            }

            pub fn resolve_value<
                D: Fn(StylePropKey) -> ResolvedStyleProp,
                P: Fn(StylePropKey) -> ResolvedStyleProp
            >(
                &self,
                default_value: D,
                parent_value: P,
            ) -> ResolvedStyleProp {
                match self {
                    $(
                        Self::$name(v) => {
                            match v {
                                StylePropVal::Custom(v) => { ResolvedStyleProp::$name(v.clone()) }
                                StylePropVal::Unset => {
                                    default_value(self.key())
                                }
                                StylePropVal::Inherit => {
                                    parent_value(self.key())
                                }
                            }
                        },
                    )*
                }
            }
        }
    };
}

define_style_props!(
    Color => Color, Color;
    BackgroundColor => Color, Color;
    FontSize        => Length, f32;
    LineHeight      => f32, f32;

    BorderTopWidth => LengthOrPercent, f32;
    BorderRightWidth => LengthOrPercent, f32;
    BorderBottomWidth => LengthOrPercent, f32;
    BorderLeftWidth => LengthOrPercent, f32;

    BorderTopColor => Color, Color;
    BorderRightColor => Color, Color;
    BorderBottomColor => Color, Color;
    BorderLeftColor => Color, Color;

    Display => Display, Display;

    Width => LengthOrPercent, StyleUnit;
    Height => LengthOrPercent, StyleUnit;
    MaxWidth => LengthOrPercent, StyleUnit;
    MaxHeight => LengthOrPercent, StyleUnit;
    MinWidth => LengthOrPercent, StyleUnit;
    MinHeight => LengthOrPercent, StyleUnit;

    MarginTop => LengthOrPercent, StyleUnit;
    MarginRight => LengthOrPercent, StyleUnit;
    MarginBottom => LengthOrPercent, StyleUnit;
    MarginLeft => LengthOrPercent, StyleUnit;

    PaddingTop => LengthOrPercent, StyleUnit;
    PaddingRight => LengthOrPercent, StyleUnit;
    PaddingBottom => LengthOrPercent, StyleUnit;
    PaddingLeft => LengthOrPercent, StyleUnit;
    //
    Flex => f32, f32;
    FlexBasis => LengthOrPercent, StyleUnit;
    FlexGrow => f32, f32;
    FlexShrink => f32, f32;
    AlignSelf => Align, Align;
    Direction => Direction, Direction;
    Position => PositionType, PositionType;
    Overflow => Overflow, Overflow;

    BorderTopLeftRadius => Length, Length;
    BorderTopRightRadius => Length, Length;
    BorderBottomRightRadius => Length, Length;
    BorderBottomLeftRadius => Length, Length;

    JustifyContent => Justify, Justify;
    FlexDirection => FlexDirection, FlexDirection;
    AlignContent => Align, Align;
    AlignItems => Align, Align;
    FlexWrap => Wrap, Wrap;
    ColumnGap => Length, f32;
    RowGap => Length, f32;

    Top => LengthOrPercent, StyleUnit;
    Right => LengthOrPercent, StyleUnit;
    Bottom => LengthOrPercent, StyleUnit;
    Left => LengthOrPercent, StyleUnit;

    Transform => StyleTransform, StyleTransform;
    AnimationName => String, String;
    AnimationDuration => f32, f32;
    AnimationIterationCount => f32, f32;
);

pub fn parse_box_prop(value: StylePropertyValue) -> (StylePropertyValue, StylePropertyValue, StylePropertyValue, StylePropertyValue) {
    match value {
        StylePropertyValue::String(str) => {
            let parts: Vec<&str> = str.split(" ")
                .filter(|e| !e.is_empty())
                .collect();
            let top = if let Some(v) = parts.get(0) {
                StylePropertyValue::String((*v).to_string())
            } else {
                StylePropertyValue::Invalid
            };
            let right = if let Some(v) = parts.get(1) {
                StylePropertyValue::String((*v).to_string())
            } else {
                top.clone()
            };
            let bottom = if let Some(v) = parts.get(2) {
                StylePropertyValue::String((*v).to_string())
            } else {
                top.clone()
            };
            let left = if let Some(v) = parts.get(3) {
                StylePropertyValue::String((*v).to_string())
            } else {
                right.clone()
            };
            (top, right, bottom, left)
        }
        e => {
            (e.clone(), e.clone(), e.clone(), e.clone())
        }
    }
}

impl StylePropertyValue {
    pub fn from_js_value(js_value: JsValue) -> Self {
        // AllStylePropertyKey::CompoundStylePropertyKey(1);
        match js_value {
            JsValue::Undefined => Self::Invalid,
            JsValue::Null => Self::Invalid,
            JsValue::Bool(_) => Self::Invalid,
            JsValue::Int(i) => Self::Float(i as f32),
            JsValue::Float(f) => Self::Float(f as f32),
            JsValue::String(s) => Self::String(s),
            JsValue::Array(_) => Self::Invalid,
            JsValue::Object(_) => Self::Invalid,
            JsValue::Raw(_) => Self::Invalid,
            JsValue::Date(_) => Self::Invalid,
            JsValue::Resource(_) => Self::Invalid,
            // JsValue::BigInt(_) => Self::Invalid,
            _ => Self::Invalid,
        }
    }

    pub fn from_str(value: &str) -> Self {
        Self::String(value.to_string())
    }

    pub fn to_str(&self, default: &str) -> String {
        match self {
            StylePropertyValue::Float(f) => { f.to_string() }
            StylePropertyValue::String(s) => { s.to_string() }
            StylePropertyValue::Invalid => default.to_string()
        }
    }

}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct LengthContext {
    pub root: f32,
    pub font_size: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum Length {
    PX(f32),
    CM(f32),
    MM(f32),
    IN(f32),
    PT(f32),

    EM(f32),
    REM(f32),
    VH(f32),
    VW(f32),
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum LengthOrPercent {
    Length(Length),
    Percent(f32),
    Undefined,
    Auto,
}

impl LengthOrPercent {
    pub fn to_style_unit(&self, ctx: &LengthContext) -> StyleUnit {
        match self {
            LengthOrPercent::Length(v) => {
                let value = v.to_px(ctx);
                StyleUnit::Point(OrderedFloat(value))
            }
            LengthOrPercent::Percent(p) => {
                StyleUnit::Percent(OrderedFloat(*p))
            }
            LengthOrPercent::Undefined => {
                StyleUnit::UndefinedValue
            }
            LengthOrPercent::Auto => {
                StyleUnit::Auto
            }
        }
    }
}

impl Length {
    pub fn from_str(value: &str) -> Option<Self> {
        let value = value.trim();
        let result = if let Some(v) = value.strip_suffix("px") {
            Self::PX(Self::parse_f32(v)?)
        } else if let Some(v) = value.strip_suffix("cm") {
            Self::CM(Self::parse_f32(v)?)
        } else if let Some(v) = value.strip_suffix("mm") {
            Self::MM(Self::parse_f32(v)?)
        } else if let Some(v) = value.strip_suffix("in") {
            Self::IN(Self::parse_f32(v)?)
        } else if let Some(v) = value.strip_suffix("pt") {
            Self::PT(Self::parse_f32(v)?)
        } else if let Some(v) = value.strip_suffix("em") {
            Self::EM(Self::parse_f32(v)?)
        } else if let Some(v) = value.strip_suffix("rem") {
            Self::REM(Self::parse_f32(v)?)
        } else if let Some(v) = value.strip_suffix("vh") {
            Self::VH(Self::parse_f32(v)?)
        } else if let Some(v) = value.strip_suffix("vw") {
            Self::VW(Self::parse_f32(v)?)
        } else {
            let v = Self::parse_f32(value)?;
            return Some(Self::PX(v));
        };
        Some(result)
    }

    pub fn update_value(&mut self, value: f32) {
        match self {
            Length::PX(x) => *x = value,
            Length::CM(x) => *x = value,
            Length::MM(x) => *x = value,
            Length::IN(x) => *x = value,
            Length::PT(x) => *x = value,
            Length::EM(x) => *x = value,
            Length::REM(x) => *x = value,
            Length::VH(x) => *x = value,
            Length::VW(x) => *x = value,
        }
    }

    pub fn to_px(&self, ctx: &LengthContext) -> f32 {
        match self {
            Length::EM(em) => em * ctx.font_size,
            Length::REM(rem) => rem * ctx.root,
            Length::VH(vh) => vh / 100.0 * ctx.viewport_height,
            Length::VW(vw) => vw / 100.0 * ctx.viewport_width,
            Length::PX(px) => *px,
            Length::CM(cm) => cm * 96.0 / 2.54,
            Length::MM(mm) => mm * 96.0 / 2.54 / 10.0,
            Length::IN(i) => i * 96.0,
            Length::PT(pt) => pt * 96.0 / 72.0,
        }
    }

    pub fn to_str(&self) -> String {
        match self {
            Length::PX(v) => format!("{}px", v),
            Length::CM(v) => format!("{}cm", v),
            Length::MM(v) => format!("{}mm", v),
            Length::IN(v) => format!("{}in", v),
            Length::PT(v) => format!("{}pt", v),
            Length::EM(v) => format!("{}em", v),
           Length::REM(v) => format!("{}rem", v),
            Length::VH(v) => format!("{}vh", v),
            Length::VW(v) => format!("{}vw", v),
        }
    }

    fn parse_f32(value: &str) -> Option<f32> {
        let value = value.trim();
        f32::from_str(value).ok()
    }

}

pub trait ColorHelper {
    fn is_transparent(&self) -> bool;
}

impl ColorHelper for Color {
    fn is_transparent(&self) -> bool {
        self.a() == 0
    }
}

struct AnimationParams {
    name: String,
    duration: f32,
    iteration_count: f32,
}

impl AnimationParams {
    pub fn new() -> Self {
        Self {
            name: "".to_string(),
            duration: 0.0,
            iteration_count: 1.0,
        }
    }
}

#[derive(PartialEq)]
struct BorderParams {
    border_width: [f32; 4],
    border_radius: [f32; 4],
    width: f32,
    height: f32,
}

#[derive(PartialEq, Clone)]
pub struct YogaNode {
    node: Mrc<Node>,
}

impl YogaNode {
    pub fn new() -> Self {
        Self {
            node: Mrc::new(Node::new()),
        }
    }
}

impl Deref for YogaNode {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl DerefMut for YogaNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}


#[mrc_object]
pub struct StyleNode {
    element: ElementWeak,
    pub yoga_node: YogaNode,
    shadow_node: Option<YogaNode>,

    parent: Option<MrcWeak<Self>>,
    children: Vec<StyleNode>,

    // (inherited, computed)
    pub border_radius: [f32; 4],
    pub border_color: [Color;4],
    pub background_image: Option<Image>,
    pub transform: Option<StyleTransform>,
    animation_params: AnimationParams,
    animation_instance: Option<AnimationInstance>,
    pub on_changed: Option<Box<dyn FnMut(StylePropKey)>>,
    pub resolved_style_props: HashMap<StylePropKey, ResolvedStyleProp>,
    pub font_size: f32,
    pub color: Color,
    pub background_color: Color,
    pub line_height: f32,
}


impl StyleNode {
    pub fn new() -> Self {
        let transparent = Color::from_argb(0,0,0,0);
        let mut inner = StyleNodeData {
            element: ElementWeak::invalid(),
            yoga_node: YogaNode::new(),
            shadow_node: None,
            parent: None,
            children: Vec::new(),
            border_radius: [0.0, 0.0, 0.0, 0.0],
            border_color: [transparent, transparent, transparent, transparent],
            background_image: None,
            transform: None,
            animation_instance: None,
            animation_params: AnimationParams::new(),
            on_changed: None,
            resolved_style_props: HashMap::new(),
            font_size: 12.0,
            color: Color::new(0),
            background_color: Color::new(0),
            line_height: 12.0,
        };
        inner.yoga_node.set_position_type(PositionType::Static);
        let mut inst = inner.to_ref();
        //TODO fix length context
        let length_ctx = LengthContext {
            root: 0.0,
            font_size: 0.0,
            viewport_width: 0.0,
            viewport_height: 0.0,
        };
        inst
    }

    pub fn new_with_shadow() -> Self {
        let mut sn = Self::new();
        sn.inner.shadow_node = Some(YogaNode::new());
        sn
    }

    pub fn bind_element(&mut self, element: ElementWeak) {
        self.element = element;
    }

    pub fn get_padding(&self) -> (f32, f32, f32, f32) {
        self.with_container_node(|n| {
            (
                n.get_layout_padding_top().de_nan(0.0),
                n.get_layout_padding_right().de_nan(0.0),
                n.get_layout_padding_bottom().de_nan(0.0),
                n.get_layout_padding_left().de_nan(0.0),
            )
        })
    }

    pub fn get_content_bounds(&self) -> Rect {
        let l = self.yoga_node.get_layout_padding_left().de_nan(0.0);
        let r = self.yoga_node.get_layout_padding_right().de_nan(0.0);
        let t = self.yoga_node.get_layout_padding_top().de_nan(0.0);
        let b = self.yoga_node.get_layout_padding_bottom().de_nan(0.0);
        let width = self.yoga_node.get_layout_width();
        let height = self.yoga_node.get_layout_height();
        // let (width, height) = self.with_container_node(|n| {
        //     (n.get_layout_width().de_nan(0.0), n.get_layout_height().de_nan(0.0))
        // });
        Rect::new(l, t, width - l - r, height - t - b)
    }

    fn get_resolved_value(&self, key: StylePropKey) -> ResolvedStyleProp {
        if let Some(v) = self.resolved_style_props.get(&key) {
            v.clone()
        } else {
            self.get_default_value(key)
        }
    }

    fn get_default_value(&self, key: StylePropKey) -> ResolvedStyleProp {
        let standard_node = Node::new();
        let default_border_width = LengthOrPercent::Length(Length::PX(0.0));
        let default_border_color = Color::TRANSPARENT;
        match key {
            StylePropKey::Color => {
                ResolvedStyleProp::Color(Color::BLACK)
            }
            StylePropKey::BackgroundColor  =>   {
                ResolvedStyleProp::BackgroundColor(Color::TRANSPARENT)
            }
            StylePropKey::FontSize => {
                ResolvedStyleProp::FontSize(Length::PX(12.0))
            }
            StylePropKey::LineHeight => {
                //TODO use font-size
                ResolvedStyleProp::LineHeight(12.0)
            }
            StylePropKey::BorderTopWidth  =>   {
                ResolvedStyleProp::BorderTopWidth(default_border_width)
            }
            StylePropKey::BorderRightWidth  =>   {
                ResolvedStyleProp::BorderRightWidth(default_border_width)
            }
            StylePropKey::BorderBottomWidth  =>   {
                ResolvedStyleProp::BorderBottomWidth(default_border_width)
            }
            StylePropKey::BorderLeftWidth  =>   {
                ResolvedStyleProp::BorderLeftWidth(default_border_width)
            }
            StylePropKey::BorderTopColor => {
                ResolvedStyleProp::BorderTopColor(default_border_color)
            }
            StylePropKey::BorderRightColor => {
                ResolvedStyleProp::BorderRightColor(default_border_color)
            }
            StylePropKey::BorderBottomColor => {
                ResolvedStyleProp::BorderBottomColor(default_border_color)
            }
            StylePropKey::BorderLeftColor => {
                ResolvedStyleProp::BorderLeftColor(default_border_color)
            }
            StylePropKey::Display  =>   {
                ResolvedStyleProp::Display(Display::Flex)
            }
            StylePropKey::Width  =>   {
                // ResolvedStyleProp::Width(standard_node.get_style_width())
                //TODO fix
                ResolvedStyleProp::Width(LengthOrPercent::Undefined)
            },
            StylePropKey::Height  =>   {
                //TODO fix
                ResolvedStyleProp::Height(LengthOrPercent::Undefined)
            },
            StylePropKey::MaxWidth  =>   {
                //TODO fix
                ResolvedStyleProp::MaxWidth(LengthOrPercent::Undefined)
            },
            StylePropKey::MaxHeight  =>   {
                //TODO fix
                ResolvedStyleProp::MaxHeight(LengthOrPercent::Undefined)
            },
            StylePropKey::MinWidth  =>   {
                //TODO fix
                ResolvedStyleProp::MinWidth(LengthOrPercent::Undefined)
            },
            StylePropKey::MinHeight  =>   {
                //TODO fix
                ResolvedStyleProp::MinHeight(LengthOrPercent::Undefined)
            },
            StylePropKey::MarginTop  =>   {
                ResolvedStyleProp::MarginTop(LengthOrPercent::Undefined)
            },
            StylePropKey::MarginRight  =>   {
                ResolvedStyleProp::MarginRight(LengthOrPercent::Undefined)
            },
            StylePropKey::MarginBottom  =>   {
                ResolvedStyleProp::MarginBottom(LengthOrPercent::Undefined)
            },
            StylePropKey::MarginLeft  =>   {
                ResolvedStyleProp::MarginLeft(LengthOrPercent::Undefined)
            },
            StylePropKey::PaddingTop  =>   {
                ResolvedStyleProp::PaddingTop(LengthOrPercent::Undefined)
            },
            StylePropKey::PaddingRight  =>   {
                ResolvedStyleProp::PaddingRight(LengthOrPercent::Undefined)
            },
            StylePropKey::PaddingBottom  =>   {
                ResolvedStyleProp::PaddingBottom(LengthOrPercent::Undefined)
            },
            StylePropKey::PaddingLeft  =>   {
                ResolvedStyleProp::PaddingLeft(LengthOrPercent::Undefined)
            },
            StylePropKey::Flex  =>   {
                ResolvedStyleProp::Flex(standard_node.get_flex())
            },
            StylePropKey::FlexBasis  =>   {
                ResolvedStyleProp::FlexBasis(LengthOrPercent::Undefined)
            },
            StylePropKey::FlexGrow  =>   {
                ResolvedStyleProp::FlexGrow(standard_node.get_flex_grow())
            },
            StylePropKey::FlexShrink  =>   {
                ResolvedStyleProp::FlexShrink(standard_node.get_flex_shrink())
            },
            StylePropKey::AlignSelf  =>   {
                ResolvedStyleProp::AlignSelf(Align::FlexStart)
            },
            StylePropKey::Direction  =>   {
                ResolvedStyleProp::Direction(Direction::LTR)
            },
            StylePropKey::Position  =>   {
                ResolvedStyleProp::Position(PositionType::Static)
            },
            StylePropKey::Top  =>   {
                ResolvedStyleProp::Top(LengthOrPercent::Undefined)
            },
            StylePropKey::Right  =>   {
                ResolvedStyleProp::Right(LengthOrPercent::Undefined)
            },
            StylePropKey::Bottom  =>   {
                ResolvedStyleProp::Bottom(LengthOrPercent::Undefined)
            },
            StylePropKey::Left  =>   {
                ResolvedStyleProp::Left(LengthOrPercent::Undefined)
            },
            StylePropKey::Overflow  =>   {
                ResolvedStyleProp::Overflow(Overflow::Hidden)
            },
            StylePropKey::BorderTopLeftRadius  =>   {
                ResolvedStyleProp::BorderTopLeftRadius(Length::PX(0.0))
            },
            StylePropKey::BorderTopRightRadius  =>   {
                ResolvedStyleProp::BorderTopRightRadius(Length::PX(0.0))
            },
            StylePropKey::BorderBottomRightRadius  =>   {
                ResolvedStyleProp::BorderBottomRightRadius(Length::PX(0.0))
            },
            StylePropKey::BorderBottomLeftRadius  =>   {
                ResolvedStyleProp::BorderBottomLeftRadius(Length::PX(0.0))
            },
            StylePropKey::Transform  =>   {
                ResolvedStyleProp::Transform(StyleTransform::empty())
            }
            StylePropKey::AnimationName => {
                ResolvedStyleProp::AnimationName("".to_string())
            }
            StylePropKey::AnimationDuration => {
                ResolvedStyleProp::AnimationDuration(0.0)
            }
            StylePropKey::AnimationIterationCount => {
                ResolvedStyleProp::AnimationIterationCount(1.0)
            }

            StylePropKey::JustifyContent  =>   {
                ResolvedStyleProp::JustifyContent(Justify::FlexStart)
            },
            StylePropKey::FlexDirection  =>   {
                ResolvedStyleProp::FlexDirection(FlexDirection::Column)
            },
            StylePropKey::AlignContent  =>   {
                ResolvedStyleProp::AlignContent(Align::FlexStart)
            },
            StylePropKey::AlignItems  =>   {
                ResolvedStyleProp::AlignItems(Align::FlexStart)
            },
            StylePropKey::FlexWrap  =>   {
                ResolvedStyleProp::FlexWrap(Wrap::NoWrap)
            },
            StylePropKey::ColumnGap  =>   {
                ResolvedStyleProp::ColumnGap(Length::PX(0.0))
            },
            StylePropKey::RowGap  =>   {
                ResolvedStyleProp::RowGap(Length::PX(0.0))
            },
            //TODO aspectratio
        }
    }

    /// return (need_repaint, need_layout)
    pub fn set_style(&mut self, p: StyleProp, length_ctx: &LengthContext) -> (bool, bool, ResolvedStyleProp) {
        let v = p.resolve_value(|k| {
            self.get_default_value(k)
        }, |k| {
            if let Some(p) = self.get_parent() {
                p.get_resolved_value(k)
            } else {
                self.get_default_value(k)
            }
        });
        let (need_repaint, need_layout) = self.set_resolved_style_prop(v.clone(), length_ctx);
        (need_repaint, need_layout, v)
    }

    pub fn set_resolved_style_prop(&mut self, p: ResolvedStyleProp, length_ctx: &LengthContext) -> (bool, bool) {
        let prop_key = p.key();
        if self.resolved_style_props.get(&prop_key) == Some(&p) {
            return (false, false);
        }
        self.resolved_style_props.insert(prop_key, p.clone());
        let mut repaint = true;
        let mut need_layout = true;
        let mut change_notified = false;
        let standard_node = Node::new();

        match p {
            ResolvedStyleProp::Color(v) => {
                self.color = v;
                need_layout = false;
            }
            ResolvedStyleProp::BackgroundColor (value) =>   {
                self.background_color = value;
                need_layout = false;
            }
            ResolvedStyleProp::FontSize(value) => {
                //TODO remove
                // self.computed_style.font_size = value.0;
            }
            ResolvedStyleProp::LineHeight(value) => {
                self.line_height = value;
            }
            ResolvedStyleProp::BorderTopWidth (value) =>   {
                self.set_border_width(&value, &vec![0], length_ctx);
            }
            ResolvedStyleProp::BorderRightWidth (value) =>   {
                self.set_border_width(&value, &vec![1], length_ctx);
            }
            ResolvedStyleProp::BorderBottomWidth (value) =>   {
                self.set_border_width(&value, &vec![2], length_ctx);
            }
            ResolvedStyleProp::BorderLeftWidth (value) =>   {
                self.set_border_width(&value, &vec![3], length_ctx);
            }
            ResolvedStyleProp::BorderTopColor (value) =>   {
                self.set_border_color(&value, &vec![0], length_ctx);
                need_layout = false;
            }
            ResolvedStyleProp::BorderRightColor (value) =>   {
                self.set_border_color(&value, &vec![1], length_ctx);
                need_layout = false;
            }
            ResolvedStyleProp::BorderBottomColor (value) =>   {
                self.set_border_color(&value, &vec![2], length_ctx);
                need_layout = false;
            }
            ResolvedStyleProp::BorderLeftColor (value) =>   {
                self.set_border_color(&value, &vec![3], length_ctx);
                need_layout = false;
            }
            ResolvedStyleProp::Display (value) =>   {
                self.yoga_node.set_display(value)
            }
            ResolvedStyleProp::Width (value) =>   {
                self.yoga_node.set_width(value.to_style_unit(&length_ctx));
            },
            ResolvedStyleProp::Height (value) =>   {
                self.yoga_node.set_height(value.to_style_unit(&length_ctx))
            },
            ResolvedStyleProp::MaxWidth (value) =>   {
                self.yoga_node.set_max_width(value.to_style_unit(&length_ctx))
            },
            ResolvedStyleProp::MaxHeight (value) =>   {
                self.yoga_node.set_max_height(value.to_style_unit(&length_ctx))
            },
            ResolvedStyleProp::MinWidth (value) =>   {
                self.yoga_node.set_min_width(value.to_style_unit(&length_ctx))
            },
            ResolvedStyleProp::MinHeight (value) =>   {
                self.yoga_node.set_min_height(value.to_style_unit(&length_ctx))
            },
            ResolvedStyleProp::MarginTop (value) =>   {
                self.yoga_node.set_margin(Edge::Top, value.to_style_unit(&length_ctx))
            },
            ResolvedStyleProp::MarginRight (value) =>   {
                self.yoga_node.set_margin(Edge::Right, value.to_style_unit(&length_ctx))
            },
            ResolvedStyleProp::MarginBottom (value) =>   {
                self.yoga_node.set_margin(Edge::Bottom, value.to_style_unit(&length_ctx))
            },
            ResolvedStyleProp::MarginLeft (value) =>   {
                self.yoga_node.set_margin(Edge::Left, value.to_style_unit(&length_ctx))
            },
            ResolvedStyleProp::PaddingTop (value) =>   {
                self.with_container_node_mut(|n| {
                    n.set_padding(Edge::Top, value.to_style_unit(&length_ctx))
                })
            },
            ResolvedStyleProp::PaddingRight (value) =>   {
                self.with_container_node_mut(|n| {
                    n.set_padding(Edge::Right, value.to_style_unit(&length_ctx))
                })
            },
            ResolvedStyleProp::PaddingBottom (value) =>   {
                self.with_container_node_mut(|n| {
                    n.set_padding(Edge::Bottom, value.to_style_unit(&length_ctx))
                })
            },
            ResolvedStyleProp::PaddingLeft (value) =>   {
                self.with_container_node_mut(|n| {
                    n.set_padding(Edge::Left, value.to_style_unit(&length_ctx))
                })
            },
            ResolvedStyleProp::Flex (value) =>   {
                self.yoga_node.set_flex(value)
            },
            ResolvedStyleProp::FlexBasis (value) =>   {
                self.yoga_node.set_flex_basis(value.to_style_unit(&length_ctx))
            },
            ResolvedStyleProp::FlexGrow (value) =>   {
                self.yoga_node.set_flex_grow(value)
            },
            ResolvedStyleProp::FlexShrink (value) =>   {
                self.yoga_node.set_flex_shrink(value)
            },
            ResolvedStyleProp::AlignSelf (value) =>   {
                self.yoga_node.set_align_self(value)
            },
            ResolvedStyleProp::Direction (value) =>   {
                self.yoga_node.set_direction(value)
            },
            ResolvedStyleProp::Position (value) =>   {
                self.yoga_node.set_position_type(value)
            },
            ResolvedStyleProp::Top (value) =>   {
                self.yoga_node.set_position(Edge::Top, value.to_style_unit(&length_ctx));
            },
            ResolvedStyleProp::Right (value) =>   {
                self.yoga_node.set_position(Edge::Right, value.to_style_unit(&length_ctx));
            },
            ResolvedStyleProp::Bottom (value) =>   {
                self.yoga_node.set_position(Edge::Bottom, value.to_style_unit(&length_ctx));
            },
            ResolvedStyleProp::Left (value) =>   {
                self.yoga_node.set_position(Edge::Left, value.to_style_unit(&length_ctx));
            },
            ResolvedStyleProp::Overflow (value) =>   {
                self.yoga_node.set_overflow(value)
            },
            ResolvedStyleProp::BorderTopLeftRadius (value) =>   {
                self.border_radius[0] = value.to_px(&length_ctx);
            },
            ResolvedStyleProp::BorderTopRightRadius (value) =>   {
                self.border_radius[1] = value.to_px(&length_ctx);
            },
            ResolvedStyleProp::BorderBottomRightRadius (value) =>   {
                self.border_radius[2] = value.to_px(&length_ctx);
            },
            ResolvedStyleProp::BorderBottomLeftRadius (value) =>   {
                self.border_radius[3] = value.to_px(&length_ctx);
            },
            ResolvedStyleProp::Transform (value) =>   {
                need_layout = false;
                self.transform = Some(value);
            }
            ResolvedStyleProp::AnimationName(value) => {
                need_layout = false;
                let name = value;
                self.animation_params.name = name;
                self.update_animation();
            }
            ResolvedStyleProp::AnimationDuration(value) => {
                need_layout = false;
                let duration = value;
                self.animation_params.duration = duration;
                self.update_animation();
            }
            ResolvedStyleProp::AnimationIterationCount(value) => {
                need_layout = false;
                let ic = value;
                self.animation_params.iteration_count = ic;
                self.update_animation();
            }

            // container node style
            ResolvedStyleProp::JustifyContent (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_justify_content(value)
                });
            },
            ResolvedStyleProp::FlexDirection (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_flex_direction(value)
                });
            },
            ResolvedStyleProp::AlignContent (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_align_content(value)
                });
            },
            ResolvedStyleProp::AlignItems (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_align_items(value)
                });
            },
            ResolvedStyleProp::FlexWrap (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_flex_wrap(value)
                });
            },
            ResolvedStyleProp::ColumnGap (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_column_gap(value.to_px(&length_ctx))
                });
            },
            ResolvedStyleProp::RowGap (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_row_gap(value.to_px(&length_ctx))
                });
            },
            //TODO aspectratio
        }
        if !change_notified {
            if let Some(on_changed) = &mut self.on_changed {
                on_changed(prop_key);
            }
        }

        (repaint, need_layout)
    }

    fn update_animation(&mut self) {
        let mut me = self.clone();
        let task = create_event_loop_callback(move || {
            let p = &me.animation_params;
            me.animation_instance = if p.name.is_empty() || p.duration <= 0.0 || p.iteration_count <= 0.0  {
                None
            } else {
                let element = ok_or_return!(me.element.upgrade());
                let window = some_or_return!(element.get_window());
                ANIMATIONS.with_borrow(|m| {
                    let ani = m.get(&p.name)?.preprocess();
                    let frame_controller = WindowAnimationController::new(window);
                    let duration = p.duration * 1000000.0;
                    let iteration_count = p.iteration_count;
                    let actor = CssAnimationActor::new(ani, element.as_weak());
                    let mut ani_instance = AnimationInstance::new(actor, duration, iteration_count, Box::new(frame_controller));
                    ani_instance.run();
                    Some(ani_instance)
                })
            };
        });
        task.call();
    }

    pub fn get_parent(&self) -> Option<StyleNode> {
        if let Some(p) = &self.parent {
            if let Ok(sn) = p.upgrade() {
                return Some(StyleNode {
                    inner: sn,
                })
            }
        }
        return None
    }

    fn set_border_width(&mut self, value: &LengthOrPercent, edges: &Vec<usize>, length_ctx: &LengthContext) {
        // let default_border = StyleBorder(StyleUnit::UndefinedValue, StyleColor::Color(Color::TRANSPARENT));
        // let value = value.resolve(&default_border);
        //TODO fix percent?
        let width = match value.to_style_unit(length_ctx) {
            StyleUnit::Point(f) => {f.0},
            _ => 0.0,
        };
        for index in edges {
            let edges_list = [Edge::Top, Edge::Right, Edge::Bottom, Edge::Left];
            self.yoga_node.set_border(edges_list[*index], width);
        }
    }

    fn set_border_color(&mut self, color: &Color, edges: &Vec<usize>, length_ctx: &LengthContext) {
        for index in edges {
            self.border_color[*index] = *color;
        }
    }

    pub fn insert_child(&mut self, child: &mut StyleNode, index: u32) {
        self.inner.children.insert(index as usize, child.clone());
        child.parent = Some(self.inner.as_weak());
        self.with_container_node_mut(|n| {
            n.insert_child(&mut child.inner.yoga_node, index as usize)
        });
    }

    pub fn get_children(&self) -> Vec<StyleNode> {
        self.children.clone()
    }

    pub fn remove_child(&mut self, child: &mut StyleNode) {
        let idx = if let Some(p) = self.inner.children.iter().position(|it| it == child) {
            p
        } else {
            return;
        };
        self.with_container_node_mut(|n| {
            n.remove_child(&mut child.inner.yoga_node);
        });
        child.parent = None;
        self.inner.children.remove(idx);
    }

    pub fn child_count(&self) -> u32 {
        self.inner.children.len() as u32
    }

    pub fn calculate_layout(&mut self,
                            available_width: f32,
                            available_height: f32,
                            parent_direction: Direction,
    ) {
        self.inner.yoga_node.calculate_layout(available_width, available_height, parent_direction);
        // self.calculate_shadow_layout();
    }


    pub fn calculate_shadow_layout(&mut self,
                               available_width: f32,
                               available_height: f32,
                               parent_direction: Direction,
    ) {
        if let Some(s) = &mut self.inner.shadow_node {
            s.calculate_layout(available_width, available_height, parent_direction);
        }
    }

    fn with_container_node_mut<R, F: FnOnce(&mut Node) -> R>(&mut self, callback: F) -> R {
        if let Some(sn) = &mut self.inner.shadow_node {
            callback(sn)
        } else {
            callback(&mut self.inner.yoga_node)
        }
    }

    fn with_container_node<R, F: FnOnce(&Node) -> R>(&self, callback: F) -> R {
        if let Some(sn) = &self.inner.shadow_node {
            callback(sn)
        } else {
            callback(&self.inner.yoga_node)
        }
    }
}

pub fn parse_style_obj(style: JsValue) -> Vec<ParsedStyleProp> {
    let mut result = Vec::new();
    if let Some(obj) = style.get_properties() {
        //TODO use default style
        obj.into_iter().for_each(|(k, v)| {
            let v_str = match v {
                JsValue::String(s) => s,
                JsValue::Int(i) => i.to_string(),
                JsValue::Float(f) => f.to_string(),
                _ => return,
            };
            let mut parse = |key: &str, value: &str| -> bool {
                let mut list = ParsedStyleProp::parse(key, value);
                if !list.is_empty() {
                    result.append(&mut list);
                    true
                } else {
                    false
                }
            };
            if !parse(&k, &v_str) {
                let key = k.to_lowercase();
                let k = key.as_str();
                match k {
                    "background" => {
                        parse("BackgroundColor", &v_str);
                    },
                    "gap" => {
                        parse("RowGap", &v_str);
                        parse("ColumnGap", &v_str);
                    },
                    "border" => {
                        parse("BorderTop", &v_str);
                        parse("BorderRight", &v_str);
                        parse("BorderBottom", &v_str);
                        parse("BorderLeft", &v_str);
                    },
                    "margin" => {
                        let (t, r, b, l) = parse_box_prop(StylePropertyValue::String(v_str.to_string()));
                        parse("MarginTop", &t.to_str("none"));
                        parse("MarginRight", &r.to_str("none"));
                        parse("MarginBottom", &b.to_str("none"));
                        parse("MarginLeft", &l.to_str("none"));
                    }
                    "padding" => {
                        let (t, r, b, l) = parse_box_prop(StylePropertyValue::String(v_str.to_string()));
                        parse("PaddingTop", &t.to_str("none"));
                        parse("PaddingRight", &r.to_str("none"));
                        parse("PaddingBottom", &b.to_str("none"));
                        parse("PaddingLeft", &l.to_str("none"));
                    }
                    "borderradius" => {
                        let (t, r, b, l) = parse_box_prop(StylePropertyValue::String(v_str.to_string()));
                        parse("BorderTopLeftRadius", &t.to_str("none"));
                        parse("BorderTopRightRadius", &r.to_str("none"));
                        parse("BorderBottomRightRadius", &b.to_str("none"));
                        parse("BorderBottomLeftRadius", &l.to_str("none"));
                    }
                    _ => {}
                }
            }
        });
    }
    result
}


pub fn parse_float(value: &str) -> f32 {
    f32::from_str(value).unwrap_or(0.0)
}

pub fn parse_color_str(value: &str) -> Option<Color> {
    //TODO support white,black and so on
    if let Some(hex) = value.strip_prefix("#") {
        parse_hex_color(hex)
    } else {
        None
    }
}

pub fn parse_optional_color_str(value: Option<&String>) -> Option<Color> {
    if let Some(str) = value {
        parse_color_str(str)
    } else {
        None
    }
}


fn parse_matrix(value: &str) -> Result<Matrix, Error> {
    let parts: Vec<&str> = value.split(",").collect();
    if parts.len() != 6 {
        return Err(anyhow!("invalid value"));
    }
    Ok(create_matrix([
        f32::from_str(parts.get(0).unwrap())?,
        f32::from_str(parts.get(1).unwrap())?,
        f32::from_str(parts.get(2).unwrap())?,
        f32::from_str(parts.get(3).unwrap())?,
        f32::from_str(parts.get(4).unwrap())?,
        f32::from_str(parts.get(5).unwrap())?,
    ]))
}

pub fn format_matrix(v: &Matrix) -> String {
    format!("matrix({},{},{},{},{},{})", v.scale_x(), v.skew_y(), v.skew_x(), v.scale_y(), v.translate_x(), v.translate_y())
}

fn create_matrix(values: [f32; 6]) -> Matrix {
    let scale_x = values[0];
    let skew_y =  values[1];
    let skew_x =  values[2];
    let scale_y = values[3];
    let trans_x = values[4];
    let trans_y = values[5];
    Matrix::new_all(
        scale_x, skew_x, trans_x,
        skew_y, scale_y, trans_y,
        0.0, 0.0, 1.0,
    )
}

fn parse_rotate_op(value: &str) -> Option<StyleTransformOp> {
    if let Some(v) = value.strip_suffix("deg") {
        let v = f32::from_str(v).ok()?;
        Some(StyleTransformOp::Rotate(v))
    } else {
        None
    }
}

fn parse_scale_op(value: &str) -> Option<StyleTransformOp> {
    let mut values = value.split(",").collect::<Vec<&str>>();
    if values.len() < 2 {
        values.push(values[0]);
    }
    let x = f32::from_str(values[0].trim()).ok()?;
    let y = f32::from_str(values[1].trim()).ok()?;
    Some(StyleTransformOp::Scale(ScaleParams(x, y)))
}

fn parse_translate_op(value: &str) -> Option<StyleTransformOp> {
    let mut values = value.split(",").collect::<Vec<&str>>();
    if values.len() < 2 {
        values.push(values[0]);
    }
    let x = parse_translate_length(values[0].trim())?;
    let y = parse_translate_length(values[1].trim())?;
    Some(StyleTransformOp::Translate(TranslateParams(x, y)))
}

fn parse_translate_length(value: &str) -> Option<TranslateLength> {
    if let Some(v) = value.strip_suffix("%") {
        let v = f32::from_str(v.trim()).ok()?;
        Some(TranslateLength::Percent(v))
    } else {
        let v = f32::from_str(value).ok()?;
        Some(TranslateLength::Point(v))
    }
}

pub fn parse_border(value: &str) -> (LengthOrPercent, Color) {
    let parts = value.split(" ");
    let mut width = LengthOrPercent::Length(Length::PX((0.0)));
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

fn parse_color(value: &str) -> Option<Color> {
    if let Some(hex) = value.strip_prefix("#") {
        parse_hex_color(hex)
    } else if let Ok(c) = parse_color_keyword(value) {
        match c {
            cssparser::Color::CurrentColor => None,
            cssparser::Color::RGBA(rgba) => {
                Some(Color::from_argb(rgba.alpha, rgba.red, rgba.green, rgba.blue))
            }
        }
    } else {
        None
    }
}

#[test]
fn test_inherit() {
    let color = Color::from_rgb(10, 20, 30);
    let mut p = StyleNode::new();
    let length_context = LengthContext::default();
    p.set_style(StyleProp::Color(StylePropVal::Custom(color)), &length_context);
    let mut c = StyleNode::new();
    p.insert_child(&mut c, 0);
    c.set_style(StyleProp::Color(StylePropVal::Inherit), &length_context);
    let child_color = c.get_resolved_value(StylePropKey::Color);
    assert_eq!(child_color, ResolvedStyleProp::Color(color));
}