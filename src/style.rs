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
use crate::element::{Element, ElementWeak};
use crate::event_loop::create_event_loop_callback;
use crate::mrc::{Mrc, MrcWeak};
use crate::number::DeNan;
use crate::paint::MatrixCalculator;
use crate::timer::{set_timeout, TimerHandle};

#[derive(Clone, Debug, PartialEq)]
pub enum StylePropertyValue {
    Float(f32),
    String(String),
    Invalid,
}

pub type StyleColor = ColorPropValue;

//TODO rename
pub trait PropValueParse: Sized {
    fn parse_prop_value(value: &str) -> Option<Self>;
    fn to_style_string(&self) -> String;
}

impl PropValueParse for StyleColor {
    fn parse_prop_value(value: &str) -> Option<Self> {
        parse_color(value).map(|v| ColorPropValue::Color(v))
    }
    fn to_style_string(&self) -> String {
        match self {
            StyleColor::Inherit => {
                "inherit".to_string()
            }
            StyleColor::Color(c) => {
                c.to_style_string()
            }
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


impl PropValueParse for StyleBorder {
    fn parse_prop_value(value: &str) -> Option<Self> {
        parse_border(value)
    }
    fn to_style_string(&self) -> String {
        format!("solid {} {}", self.0.to_style_string(), self.1.to_style_string())
    }
}

impl PropValueParse for StyleUnit {
    fn parse_prop_value(value: &str) -> Option<Self> {
        parse_style_unit(value)
    }
    fn to_style_string(&self) -> String {
        match self {
            StyleUnit::UndefinedValue => {
                "".to_string()
            }
            StyleUnit::Point(v) => {
                format!("{}px", v)
            }
            StyleUnit::Percent(v) => {
                format!("{}%", v)
            }
            StyleUnit::Auto => {
                "auto".to_string()
            }
        }
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
pub struct StyleBorder(StyleUnit, StyleColor);

#[derive(Clone, Debug, PartialEq)]
pub struct ComputedStyleBorder(f32, Color);

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
                    if key == stringify!($name).to_lowercase().as_str() {
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
    FontSize        => f32, f32;
    LineHeight      => f32, f32;

    BorderTop => StyleBorder, StyleBorder;
    BorderRight => StyleBorder, StyleBorder;
    BorderBottom => StyleBorder,StyleBorder;
    BorderLeft => StyleBorder,StyleBorder;

    Display => Display, Display;

    Width => StyleUnit, StyleUnit;
    Height => StyleUnit, StyleUnit;
    MaxWidth => StyleUnit, StyleUnit;
    MaxHeight => StyleUnit, StyleUnit;
    MinWidth => StyleUnit, StyleUnit;
    MinHeight => StyleUnit, StyleUnit;

    MarginTop => StyleUnit, StyleUnit;
    MarginRight => StyleUnit, StyleUnit;
    MarginBottom => StyleUnit, StyleUnit;
    MarginLeft => StyleUnit, StyleUnit;

    PaddingTop => StyleUnit, StyleUnit;
    PaddingRight => StyleUnit, StyleUnit;
    PaddingBottom => StyleUnit, StyleUnit;
    PaddingLeft => StyleUnit, StyleUnit;
    //
    Flex => f32, f32;
    FlexBasis => StyleUnit, StyleUnit;
    FlexGrow => f32, f32;
    FlexShrink => f32, f32;
    AlignSelf => Align, Align;
    Direction => Direction, Direction;
    Position => PositionType, PositionType;
    Overflow => Overflow, Overflow;

    BorderTopLeftRadius => AbsoluteLen, AbsoluteLen;
    BorderTopRightRadius => AbsoluteLen, AbsoluteLen;
    BorderBottomRightRadius => AbsoluteLen, AbsoluteLen;
    BorderBottomLeftRadius => AbsoluteLen, AbsoluteLen;

    JustifyContent => Justify, Justify;
    FlexDirection => FlexDirection, FlexDirection;
    AlignContent => Align, Align;
    AlignItems => Align, Align;
    FlexWrap => Wrap, Wrap;
    ColumnGap => AbsoluteLen, f32;
    RowGap => AbsoluteLen, f32;

    Top => StyleUnit, StyleUnit;
    Right => StyleUnit, StyleUnit;
    Bottom => StyleUnit, StyleUnit;
    Left => StyleUnit, StyleUnit;

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

pub struct Style {
    // (inherited, computed)
    pub color: ColorPropValue,
    pub border_radius: [f32; 4],
    pub border_color: [Color;4],
    pub background_color: ColorPropValue,
    pub background_image: Option<Image>,
}

pub struct ComputedStyle {
    pub color: Color,
    pub background_color: Color,
    pub font_size: f32,
    pub line_height: f32,
}

impl ComputedStyle {
    pub fn default() -> Self {
        Self {
            color: Color::new(0),
            background_color: Color::new(0),
            font_size: 12.0,
            line_height: 12.0,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ColorPropValue {
    Inherit,
    Color(Color),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PropValue<T> {
    Inherit,
    Custom(T),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AbsoluteLen(pub f32);

impl PropValueParse for AbsoluteLen {
    fn parse_prop_value(value: &str) -> Option<Self> {
        let v = value.strip_suffix("px").unwrap_or(value);
        f32::from_str(v).ok().map(AbsoluteLen)
    }

    fn to_style_string(&self) -> String {
        format!("{}px", self.0)
    }
}

impl<T: PropValueParse> PropValueParse for PropValue<T> {
    fn parse_prop_value(value: &str) -> Option<Self> {
        if value == "inherit" {
            Some(Self::Inherit)
        } else {
            Some(Self::Custom(T::parse_prop_value(value)?))
        }
    }
    fn to_style_string(&self) -> String {
        match self {
            PropValue::Inherit => "inherit".to_string(),
            PropValue::Custom(v) => v.to_style_string()
        }
    }
}

impl Style {
    pub fn default() -> Self {
        let transparent = Color::from_argb(0,0,0,0);
        Self {
            border_radius: [0.0, 0.0, 0.0, 0.0],
            border_color: [transparent, transparent, transparent, transparent],
            background_color: ColorPropValue::Color(Color::TRANSPARENT),
            color: ColorPropValue::Inherit,
            background_image: None,
        }
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
    pub computed_style: ComputedStyle,
    animation_params: AnimationParams,
    animation_instance: Option<AnimationInstance>,
    pub on_changed: Option<Box<dyn FnMut(StylePropKey)>>,
    pub animation_renderer: Option<Mrc<Box<dyn FnMut(Vec<StyleProp>)>>>,
    pub style_props: HashMap<StylePropKey, StyleProp>,
    pub resolved_style_props: HashMap<StylePropKey, ResolvedStyleProp>,
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
            computed_style: ComputedStyle::default(),
            on_changed: None,
            animation_renderer: None,
            resolved_style_props: HashMap::new(),
            style_props: HashMap::new(),
        };
        inner.yoga_node.set_position_type(PositionType::Static);
        let mut inst = inner.to_ref();
        inst.set_style(StyleProp::Position(StylePropVal::Custom(PositionType::Static)));
        inst.set_style(StyleProp::Color(StylePropVal::Inherit));
        inst.set_style(StyleProp::FontSize(StylePropVal::Inherit));
        inst.set_style(StyleProp::LineHeight(StylePropVal::Inherit));
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
        let default_border = StyleBorder(StyleUnit::UndefinedValue, StyleColor::Color(Color::TRANSPARENT));
        match key {
            StylePropKey::Color => {
                ResolvedStyleProp::Color(Color::BLACK)
            }
            StylePropKey::BackgroundColor  =>   {
                ResolvedStyleProp::BackgroundColor(Color::TRANSPARENT)
            }
            StylePropKey::FontSize => {
                ResolvedStyleProp::FontSize(12.0)
            }
            StylePropKey::LineHeight => {
                //TODO use font-size
                ResolvedStyleProp::LineHeight(12.0)
            }
            StylePropKey::BorderTop  =>   {
                ResolvedStyleProp::BorderTop(default_border)
            }
            StylePropKey::BorderRight  =>   {
                ResolvedStyleProp::BorderRight(default_border)
            }
            StylePropKey::BorderBottom  =>   {
                ResolvedStyleProp::BorderBottom(default_border)
            }
            StylePropKey::BorderLeft  =>   {
                ResolvedStyleProp::BorderLeft(default_border)
            }
            StylePropKey::Display  =>   {
                ResolvedStyleProp::Display(Display::Flex)
            }
            StylePropKey::Width  =>   {
                ResolvedStyleProp::Width(standard_node.get_style_width())
            },
            StylePropKey::Height  =>   {
                ResolvedStyleProp::Height(standard_node.get_style_height())
            },
            StylePropKey::MaxWidth  =>   {
                ResolvedStyleProp::MaxWidth(standard_node.get_style_max_width())
            },
            StylePropKey::MaxHeight  =>   {
                ResolvedStyleProp::MaxHeight(standard_node.get_style_max_height())
            },
            StylePropKey::MinWidth  =>   {
                ResolvedStyleProp::MinWidth(standard_node.get_style_min_width())
            },
            StylePropKey::MinHeight  =>   {
                ResolvedStyleProp::MinHeight(standard_node.get_style_min_height())
            },
            StylePropKey::MarginTop  =>   {
                ResolvedStyleProp::MarginTop(standard_node.get_style_margin_top())
            },
            StylePropKey::MarginRight  =>   {
                ResolvedStyleProp::MarginRight(standard_node.get_style_margin_right())
            },
            StylePropKey::MarginBottom  =>   {
                ResolvedStyleProp::MarginBottom(standard_node.get_style_margin_bottom())
            },
            StylePropKey::MarginLeft  =>   {
                ResolvedStyleProp::MarginLeft(standard_node.get_style_margin_left())
            },
            StylePropKey::PaddingTop  =>   {
                ResolvedStyleProp::PaddingTop(standard_node.get_style_padding_top())
            },
            StylePropKey::PaddingRight  =>   {
                ResolvedStyleProp::PaddingRight(standard_node.get_style_padding_right())
            },
            StylePropKey::PaddingBottom  =>   {
                ResolvedStyleProp::PaddingBottom(standard_node.get_style_padding_bottom())
            },
            StylePropKey::PaddingLeft  =>   {
                ResolvedStyleProp::PaddingLeft(standard_node.get_style_padding_left())
            },
            StylePropKey::Flex  =>   {
                ResolvedStyleProp::Flex(standard_node.get_flex())
            },
            StylePropKey::FlexBasis  =>   {
                ResolvedStyleProp::FlexBasis(standard_node.get_flex_basis())
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
                ResolvedStyleProp::Top(StyleUnit::UndefinedValue)
            },
            StylePropKey::Right  =>   {
                ResolvedStyleProp::Right(StyleUnit::UndefinedValue)
            },
            StylePropKey::Bottom  =>   {
                ResolvedStyleProp::Bottom(StyleUnit::UndefinedValue)
            },
            StylePropKey::Left  =>   {
                ResolvedStyleProp::Left(StyleUnit::UndefinedValue)
            },
            StylePropKey::Overflow  =>   {
                ResolvedStyleProp::Overflow(Overflow::Hidden)
            },
            StylePropKey::BorderTopLeftRadius  =>   {
                ResolvedStyleProp::BorderTopLeftRadius(AbsoluteLen(0.0))
            },
            StylePropKey::BorderTopRightRadius  =>   {
                ResolvedStyleProp::BorderTopRightRadius(AbsoluteLen(0.0))
            },
            StylePropKey::BorderBottomRightRadius  =>   {
                ResolvedStyleProp::BorderBottomRightRadius(AbsoluteLen(0.0))
            },
            StylePropKey::BorderBottomLeftRadius  =>   {
                ResolvedStyleProp::BorderBottomLeftRadius(AbsoluteLen(0.0))
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
                ResolvedStyleProp::ColumnGap(AbsoluteLen(0.0))
            },
            StylePropKey::RowGap  =>   {
                ResolvedStyleProp::RowGap(AbsoluteLen(0.0))
            },
            //TODO aspectratio
        }
    }

    /// return (need_repaint, need_layout)
    pub fn set_style(&mut self, p: StyleProp) -> (bool, bool, ResolvedStyleProp) {
        self.style_props.insert(p.key().clone(), p.clone());
        let v = p.resolve_value(|k| {
            self.get_default_value(k)
        }, |k| {
            if let Some(p) = self.get_parent() {
                p.get_resolved_value(k)
            } else {
                self.get_default_value(k)
            }
        });
        let (need_repaint, need_layout) = self.set_resolved_style_prop(v.clone());
        (need_repaint, need_layout, v)
    }

    fn compute_style_prop(&self, p: &ResolvedStyleProp) -> ComputedStyleProp {
        match p {
            ResolvedStyleProp::Color(v) => {
                ComputedStyleProp::Color(v.clone())
            }
            ResolvedStyleProp::BackgroundColor(v) => {
                ComputedStyleProp::BackgroundColor(v.clone())
            }
            ResolvedStyleProp::FontSize(v) => {
                ComputedStyleProp::FontSize(v.clone())
            }
            ResolvedStyleProp::LineHeight(v) => {
                ComputedStyleProp::LineHeight(v.clone())
            }
            ResolvedStyleProp::BorderTop(v) => {
                ComputedStyleProp::BorderTop(v.clone())
            }
            ResolvedStyleProp::BorderRight(v) => {
                ComputedStyleProp::BorderRight(v.clone())
            }
            ResolvedStyleProp::BorderBottom(v) => {
                ComputedStyleProp::BorderBottom(v.clone())
            }
            ResolvedStyleProp::BorderLeft(v) => {
                ComputedStyleProp::BorderLeft(v.clone())
            }
            ResolvedStyleProp::Display(v) => {
                ComputedStyleProp::Display(v.clone())
            }
            ResolvedStyleProp::Width(v) => {
                ComputedStyleProp::Width(v.clone())
            }
            ResolvedStyleProp::Height(v) => {
                ComputedStyleProp::Height(v.clone())
            }
            ResolvedStyleProp::MaxWidth(v) => {
                ComputedStyleProp::MaxWidth(v.clone())
            }
            ResolvedStyleProp::MaxHeight(v) => {
                ComputedStyleProp::MaxHeight(v.clone())
            }
            ResolvedStyleProp::MinWidth(v) => {
                ComputedStyleProp::MinWidth(v.clone())
            }
            ResolvedStyleProp::MinHeight(v) => {
                ComputedStyleProp::MinHeight(v.clone())
            }
            ResolvedStyleProp::MarginTop(v) => {
                ComputedStyleProp::MarginTop(v.clone())
            }
            ResolvedStyleProp::MarginRight(v) => {
                ComputedStyleProp::MarginRight(v.clone())
            }
            ResolvedStyleProp::MarginBottom(v) => {
                ComputedStyleProp::MarginBottom(v.clone())
            }
            ResolvedStyleProp::MarginLeft(v) => {
                ComputedStyleProp::MarginLeft(v.clone())
            }
            ResolvedStyleProp::PaddingTop(v) => {
                ComputedStyleProp::PaddingTop(v.clone())
            }
            ResolvedStyleProp::PaddingRight(v) => {
                ComputedStyleProp::PaddingRight(v.clone())
            }
            ResolvedStyleProp::PaddingBottom(v) => {
                ComputedStyleProp::PaddingBottom(v.clone())
            }
            ResolvedStyleProp::PaddingLeft(v) => {
                ComputedStyleProp::PaddingLeft(v.clone())
            }
            ResolvedStyleProp::Flex(v) => {
                ComputedStyleProp::Flex(v.clone())
            }
            ResolvedStyleProp::FlexBasis(v) => {
                ComputedStyleProp::FlexBasis(v.clone())
            }
            ResolvedStyleProp::FlexGrow(v) => {
                ComputedStyleProp::FlexGrow(v.clone())
            }
            ResolvedStyleProp::FlexShrink(v) => {
                ComputedStyleProp::FlexShrink(v.clone())
            }
            ResolvedStyleProp::AlignSelf(v) => {
                ComputedStyleProp::AlignSelf(v.clone())
            }
            ResolvedStyleProp::Direction(v) => {
                ComputedStyleProp::Direction(v.clone())
            }
            ResolvedStyleProp::Position(v) => {
                ComputedStyleProp::Position(v.clone())
            }
            ResolvedStyleProp::Overflow(v) => {
                ComputedStyleProp::Overflow(v.clone())
            }
            ResolvedStyleProp::BorderTopLeftRadius(v) => {
                ComputedStyleProp::BorderTopLeftRadius(v.clone())
            }
            ResolvedStyleProp::BorderTopRightRadius(v) => {
                ComputedStyleProp::BorderTopRightRadius(v.clone())
            }
            ResolvedStyleProp::BorderBottomRightRadius(v) => {
                ComputedStyleProp::BorderBottomRightRadius(v.clone())
            }
            ResolvedStyleProp::BorderBottomLeftRadius(v) => {
                ComputedStyleProp::BorderBottomLeftRadius(v.clone())
            }
            ResolvedStyleProp::JustifyContent(v) => {
                ComputedStyleProp::JustifyContent(v.clone())
            }
            ResolvedStyleProp::FlexDirection(v) => {
                ComputedStyleProp::FlexDirection(v.clone())
            }
            ResolvedStyleProp::AlignContent(v) => {
                ComputedStyleProp::AlignContent(v.clone())
            }
            ResolvedStyleProp::AlignItems(v) => {
                ComputedStyleProp::AlignItems(v.clone())
            }
            ResolvedStyleProp::FlexWrap(v) => {
                ComputedStyleProp::FlexWrap(v.clone())
            }
            ResolvedStyleProp::ColumnGap(v) => {
                ComputedStyleProp::ColumnGap(v.0)
            }
            ResolvedStyleProp::RowGap(v) => {
                ComputedStyleProp::RowGap(v.0)
            }
            ResolvedStyleProp::Top(v) => {
                ComputedStyleProp::Top(v.clone())
            }
            ResolvedStyleProp::Right(v) => {
                ComputedStyleProp::Right(v.clone())
            }
            ResolvedStyleProp::Bottom(v) => {
                ComputedStyleProp::Bottom(v.clone())
            }
            ResolvedStyleProp::Left(v) => {
                ComputedStyleProp::Left(v.clone())
            }
            ResolvedStyleProp::Transform(v) => {
                ComputedStyleProp::Transform(v.clone())
            }
            ResolvedStyleProp::AnimationName(v) => {
                ComputedStyleProp::AnimationName(v.clone())
            }
            ResolvedStyleProp::AnimationDuration(v) => {
                ComputedStyleProp::AnimationDuration(v.clone())
            }
            ResolvedStyleProp::AnimationIterationCount(v) => {
                ComputedStyleProp::AnimationIterationCount(v.clone())
            }
        }
    }

    pub fn set_resolved_style_prop(&mut self, p: ResolvedStyleProp) -> (bool, bool) {
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
                self.computed_style.color = v;
                need_layout = false;
            }
            ResolvedStyleProp::BackgroundColor (value) =>   {
                self.computed_style.background_color = value;
                need_layout = false;
            }
            ResolvedStyleProp::FontSize(value) => {
                self.computed_style.font_size = value;
            }
            ResolvedStyleProp::LineHeight(value) => {
                self.computed_style.line_height = value;
            }
            ResolvedStyleProp::BorderTop (value) =>   {
                self.set_border(&value, &vec![0])
            }
            ResolvedStyleProp::BorderRight (value) =>   {
                self.set_border(&value, &vec![1])
            }
            ResolvedStyleProp::BorderBottom (value) =>   {
                self.set_border(&value, &vec![2])
            }
            ResolvedStyleProp::BorderLeft (value) =>   {
                self.set_border(&value, &vec![3])
            }
            ResolvedStyleProp::Display (value) =>   {
                self.yoga_node.set_display(value)
            }
            ResolvedStyleProp::Width (value) =>   {
                self.yoga_node.set_width(value)
            },
            ResolvedStyleProp::Height (value) =>   {
                self.yoga_node.set_height(value)
            },
            ResolvedStyleProp::MaxWidth (value) =>   {
                self.yoga_node.set_max_width(value)
            },
            ResolvedStyleProp::MaxHeight (value) =>   {
                self.yoga_node.set_max_height(value)
            },
            ResolvedStyleProp::MinWidth (value) =>   {
                self.yoga_node.set_min_width(value)
            },
            ResolvedStyleProp::MinHeight (value) =>   {
                self.yoga_node.set_min_height(value)
            },
            ResolvedStyleProp::MarginTop (value) =>   {
                self.yoga_node.set_margin(Edge::Top, value)
            },
            ResolvedStyleProp::MarginRight (value) =>   {
                self.yoga_node.set_margin(Edge::Right, value)
            },
            ResolvedStyleProp::MarginBottom (value) =>   {
                self.yoga_node.set_margin(Edge::Bottom, value)
            },
            ResolvedStyleProp::MarginLeft (value) =>   {
                self.yoga_node.set_margin(Edge::Left, value)
            },
            ResolvedStyleProp::PaddingTop (value) =>   {
                self.with_container_node_mut(|n| {
                    n.set_padding(Edge::Top, value)
                })
            },
            ResolvedStyleProp::PaddingRight (value) =>   {
                self.with_container_node_mut(|n| {
                    n.set_padding(Edge::Right, value)
                })
            },
            ResolvedStyleProp::PaddingBottom (value) =>   {
                self.with_container_node_mut(|n| {
                    n.set_padding(Edge::Bottom, value)
                })
            },
            ResolvedStyleProp::PaddingLeft (value) =>   {
                self.with_container_node_mut(|n| {
                    n.set_padding(Edge::Left, value)
                })
            },
            ResolvedStyleProp::Flex (value) =>   {
                self.yoga_node.set_flex(value)
            },
            ResolvedStyleProp::FlexBasis (value) =>   {
                self.yoga_node.set_flex_basis(value)
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
                self.yoga_node.set_position(Edge::Top, value);
            },
            ResolvedStyleProp::Right (value) =>   {
                self.yoga_node.set_position(Edge::Right, value);
            },
            ResolvedStyleProp::Bottom (value) =>   {
                self.yoga_node.set_position(Edge::Bottom, value);
            },
            ResolvedStyleProp::Left (value) =>   {
                self.yoga_node.set_position(Edge::Left, value);
            },
            ResolvedStyleProp::Overflow (value) =>   {
                self.yoga_node.set_overflow(value)
            },
            ResolvedStyleProp::BorderTopLeftRadius (value) =>   {
                self.border_radius[0] = value.0;
            },
            ResolvedStyleProp::BorderTopRightRadius (value) =>   {
                self.border_radius[1] = value.0;
            },
            ResolvedStyleProp::BorderBottomRightRadius (value) =>   {
                self.border_radius[2] = value.0;
            },
            ResolvedStyleProp::BorderBottomLeftRadius (value) =>   {
                self.border_radius[3] = value.0;
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
                    layout.set_column_gap(value.0)
                });
            },
            ResolvedStyleProp::RowGap (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_row_gap(value.0)
                });
            },
            //TODO aspectratio
        }
        if !change_notified {
            if let Some(on_changed) = &mut self.on_changed {
                on_changed(prop_key);
            }
        }
        for mut c in self.get_children() {
            c.resolve_style_prop(prop_key);
        }

        return (repaint, need_layout)
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
                    let ani_instance = AnimationInstance::new(ani, duration, iteration_count, Box::new(frame_controller));
                    Some(ani_instance)
                })
            };
            let mut ar = me.animation_renderer.clone();
            if let Some(ai) = &mut me.animation_instance {
                if let Some(ar) = &mut ar {
                    let mut ar = ar.clone();
                    ai.run(Box::new(move |styles| {
                        ar(styles);
                    }));
                }
            }
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

    fn set_border(&mut self, value: &StyleBorder, edges: &Vec<usize>) {
        // let default_border = StyleBorder(StyleUnit::UndefinedValue, StyleColor::Color(Color::TRANSPARENT));
        // let value = value.resolve(&default_border);
        let color = match value.1 {
            //TODO fix inherited color?
            StyleColor::Inherit => {Color::TRANSPARENT}
            StyleColor::Color(c) => {c}
        };
        //TODO fix percent?
        let width = match value.0 {
            StyleUnit::Point(f) => {f.0},
            _ => 0.0,
        };
        for index in edges {
            self.border_color[*index] = color;
            let edges_list = [Edge::Top, Edge::Right, Edge::Bottom, Edge::Left];
            self.yoga_node.set_border(edges_list[*index], width);
        }
    }

    pub fn insert_child(&mut self, child: &mut StyleNode, index: u32) {
        self.inner.children.insert(index as usize, child.clone());
        child.parent = Some(self.inner.as_weak());
        self.with_container_node_mut(|n| {
            n.insert_child(&mut child.inner.yoga_node, index as usize)
        });
        child.resolve_style_props();
    }

    fn resolve_style_prop(&mut self, k: StylePropKey) {
        if let Some(p) = self.style_props.get(&k) {
            self.set_style(p.clone());
        }
    }

    fn resolve_style_props(&mut self) {
        for (_, p) in self.style_props.clone() {
            self.set_style(p);
        }
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

pub fn parse_style_obj(style: JsValue) -> Vec<StyleProp> {
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
                if let Some(p) = StyleProp::parse(key, value) {
                    result.push(p);
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

pub fn parse_style_unit(value: &str) -> Option<StyleUnit> {
    if let Some(v) = value.strip_suffix("%") {
        let width = f32::from_str(v).unwrap();
        Some(StyleUnit::Percent(OrderedFloat(width)))
    } else {
        let value = value.strip_suffix("px").unwrap_or_else(|| value);
        match f32::from_str(value) {
            Ok(v) => {
                Some(StyleUnit::Point(OrderedFloat(v)))
            }
            Err(err) => {
                eprintln!("Invalid value:{}", err);
                None
            }
        }
    }
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

fn parse_border(value: &str) -> Option<StyleBorder> {
    let parts = value.split(" ");
    let mut width = StyleUnit::Point(OrderedFloat(0.0));
    let mut color = Color::from_rgb(0, 0, 0);
    for p in parts {
        let p = p.trim();
        if let Some(c) = parse_color(p) {
            color = c;
        } else if let Some(w) = parse_style_unit(p) {
            width = w;
        }
    }
    Some(StyleBorder(width, StyleColor::Color(color)))
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
    p.set_style(StyleProp::Color(StylePropVal::Custom(color)));
    let mut c = StyleNode::new();
    p.insert_child(&mut c, 0);
    let child_color = c.get_resolved_value(StylePropKey::Color);
    assert_eq!(child_color, ResolvedStyleProp::Color(color));
}