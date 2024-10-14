use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use anyhow::{anyhow, Error};
use ordered_float::{Float, OrderedFloat};
use quick_js::JsValue;
use skia_safe::{Color, Image, Matrix, Path};
use yoga::{Align, Direction, Display, Edge, FlexDirection, Justify, Node, Overflow, PositionType, StyleUnit, Wrap};
use crate::base::Rect;
use crate::color::parse_hex_color;
use crate::{inherit_color_prop};
use crate::border::build_border_paths;
use crate::cache::CacheValue;
use crate::mrc::{Mrc, MrcWeak};
use crate::number::DeNan;

macro_rules! define_props {
    ($($str: expr => $key: ident, )*; $($union_str: expr => $union_key: ident, )*) => {
        #[derive(PartialEq, Eq, Hash, Clone, Debug)]
        pub enum StylePropertyKey {
            $($key,)*
        }

        #[derive(PartialEq, Eq, Hash, Clone, Debug)]
        pub enum CompoundStylePropertyKey {
            $($union_key,)*
        }

        #[derive(PartialEq, Eq, Hash, Clone, Debug)]
        pub enum AllStylePropertyKey {
            StylePropertyKey(StylePropertyKey),
            CompoundStylePropertyKey(CompoundStylePropertyKey),
        }

        impl StylePropertyKey {
            pub fn from_str(key: &str) -> Option<Self> {
                let key = key.to_ascii_lowercase();
                match key.as_str() {
                    $( $str => Some(StylePropertyKey::$key), )*
                    _ => {
                        // println!("invalid key:{}", key);
                        None
                    }
                }
            }

            pub fn name(&self) -> &'static str {
                match self {
                    $( StylePropertyKey::$key => $str , )*
                    _ => {
                        // println!("invalid key:{}", key);
                        unreachable!()
                    }
                }
            }
        }

        impl CompoundStylePropertyKey {
            pub fn from_str(key: &str) -> Option<Self> {
                let key = key.to_ascii_lowercase();
                match key.as_str() {
                    $( $union_str => Some(Self::$union_key), )*
                    _ => {
                        // println!("invalid key:{}", key);
                        None
                    }
                }
            }
        }

        impl AllStylePropertyKey {
            pub fn from_str(key: &str) -> Option<Self> {
                let key = key.to_ascii_lowercase();
                if let Some(v) = StylePropertyKey::from_str(&key) {
                    Some(AllStylePropertyKey::StylePropertyKey(v))
                } else if let Some(v) = CompoundStylePropertyKey::from_str(&key) {
                    Some(AllStylePropertyKey::CompoundStylePropertyKey(v))
                } else {
                    println!("invalid key:{}", key);
                    None
                }
            }
        }

        pub fn get_style_defaults() -> HashMap<StylePropertyKey, StylePropertyValue> {
            let mut m = HashMap::new();
            $(
                m.insert(StylePropertyKey::$key, StylePropertyValue::Invalid);
            )*
            m
        }
    };
}

define_props!(
    "color" => Color,

    "backgroundcolor" => BackgroundColor,

    "bordertop" => BorderTop,
    "borderright" => BorderRight,
    "borderbottom" => BorderBottom,
    "borderleft" => BorderLeft,

    "display" => Display,

    "width" => Width,
    "height" => Height,
    "maxwidth" => MaxWidth,
    "maxheight" => MaxHeight,
    "minwidth" => MinWidth,
    "minheight" => MinHeight,

    "margintop" => MarginTop,
    "marginright" => MarginRight,
    "marginbottom" => MarginBottom,
    "marginleft" => MarginLeft,

    "paddingtop" => PaddingTop,
    "paddingright" => PaddingRight,
    "paddingbottom" => PaddingBottom,
    "paddingleft" => PaddingLeft,

    "flex" => Flex,
    "flexbasis" => FlexBasis,
    "flexgrow" => FlexGrow,
    "flexshrink" => FlexShrink,
    "alignself" => AlignSelf,
    "direction" => Direction,
    "position" => Position,
    "overflow" => Overflow,

    "bordertopleftradius" => BorderTopLeftRadius,
    "bordertoprightradius" => BorderTopRightRadius,
    "borderbottomrightradius" => BorderBottomRightRadius,
    "borderbottomleftradius" => BorderBottomLeftRadius,

    "justifycontent" => JustifyContent,
    "flexdirection" => FlexDirection,
    "aligncontent" => AlignContent,
    "alignitems" => AlignItems,
    "flexwrap" => FlexWrap,
    "columngap" => ColumnGap,
    "rowgap" => RowGap,
    "left" => Left,
    "right" => Right,
    "top" => Top,
    "bottom" => Bottom,
    "transform" => Transform,
    ;
    "background" => Background,
    "gap" => Gap,
    "border" => Border,
    "margin" => Margin,
    "padding" => Padding,
    "borderradius" => BorderRadius,
);

#[derive(Clone, Debug, PartialEq)]
pub enum StylePropertyValue {
    Float(f32),
    String(String),
    Invalid,
}

pub type StyleColor = ColorPropValue;

pub trait PropValueParse: Sized {
    fn parse_prop_value(value: &str) -> Option<Self>;
}

impl PropValueParse for StyleColor {
    fn parse_prop_value(value: &str) -> Option<Self> {
        if let Some(hex) = value.strip_prefix("#") {
            parse_hex_color(hex).map(|v| ColorPropValue::Color(v))
        } else {
            None
        }
    }
}

impl PropValueParse for StyleBorder {
    fn parse_prop_value(value: &str) -> Option<Self> {
        parse_border(value)
    }
}

impl PropValueParse for StyleUnit {
    fn parse_prop_value(value: &str) -> Option<Self> {
        parse_style_unit(value)
    }
}

impl PropValueParse for Display {
    fn parse_prop_value(value: &str) -> Option<Self> {
        parse_display2(value)
    }
}

impl PropValueParse for f32 {
    fn parse_prop_value(value: &str) -> Option<Self> {
        f32::from_str(value).ok()
    }
}

impl PropValueParse for FlexDirection {
    fn parse_prop_value(value: &str) -> Option<Self> {
        parse_flex_direction2(value)
    }
}

impl PropValueParse for Direction {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Some(parse_direction(value))
    }
}

impl PropValueParse for Align {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Some(parse_align(value))
    }
}

impl PropValueParse for PositionType {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Some(parse_position_type(value))
    }
}

impl PropValueParse for Overflow {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Some(parse_overflow(value))
    }
}

impl PropValueParse for Justify {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Some(parse_justify(value))
    }
}

impl PropValueParse for Wrap {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Some(parse_wrap(value))
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
}

#[derive(Clone, Debug)]
pub enum StyleTransformOp {
    Rotate(f32),
    Scale(f32, f32),
    Translate(f32, f32),
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
            "rotate" => parse_rotate_op(param_str),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct StyleTransform {
    pub op_list: Vec<StyleTransformOp>,
}

impl StyleTransform {
    pub fn empty() -> StyleTransform {
        Self {
            op_list: Vec::new(),
        }
    }

    pub fn to_matrix(&self) -> Matrix {
        let mut matrix = Matrix::new_identity();
        for op in &self.op_list {
            match op {
                StyleTransformOp::Rotate(deg) => {
                    matrix.post_rotate(*deg, None);
                }
                StyleTransformOp::Scale(_, _) => {
                    //TODO impl
                }
                StyleTransformOp::Translate(_, _) => {
                    //TODO impl
                }
            }
        }
        matrix
    }

}

#[derive(Clone, Debug)]
pub struct StyleBorder(StyleUnit, StyleColor);

#[derive(Clone, Debug)]
pub enum StylePropVal<T> {
    Custom(T),
    Unset,
}

impl<T: Clone> StylePropVal<T> {
    pub fn resolve(&self, default: &T) -> T {
        match self {
            StylePropVal::Custom(v) => { v.clone() }
            StylePropVal::Unset => { default.clone() }
        }
    }
}

macro_rules! define_style_props {
    ($($name: ident => $type: ty, )*) => {
        #[derive(Clone, Debug)]
        pub enum StyleProp {
            $(
                $name(StylePropVal<$type>),
            )*
        }

        impl StyleProp {
            pub fn parse(key: &str, value: &str) -> Option<StyleProp> {
                let key = key.to_lowercase();
                let k = key.as_str();
                $(
                    if k == stringify!($name).to_lowercase().as_str() {
                        return <$type>::parse_prop_value(value).map(|v| StyleProp::$name(StylePropVal::Custom(v)));
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
            pub fn unset(&self) -> Self {
                match self {
                    $(
                       Self::$name(_) => Self::$name(StylePropVal::Unset),
                    )*
                }
            }
        }
    };
}

define_style_props!(
    Color => StyleColor,
    BackgroundColor => StyleColor,

    BorderTop => StyleBorder,
    BorderRight => StyleBorder,
    BorderBottom => StyleBorder,
    BorderLeft => StyleBorder,

    Display => Display,

    Width => StyleUnit,
    Height => StyleUnit,
    MaxWidth => StyleUnit,
    MaxHeight => StyleUnit,
    MinWidth => StyleUnit,
    MinHeight => StyleUnit,

    MarginTop => StyleUnit,
    MarginRight => StyleUnit,
    MarginBottom => StyleUnit,
    MarginLeft => StyleUnit,

    PaddingTop => StyleUnit,
    PaddingRight => StyleUnit,
    PaddingBottom => StyleUnit,
    PaddingLeft => StyleUnit,
    //
    Flex => f32,
    FlexBasis => StyleUnit,
    FlexGrow => f32,
    FlexShrink => f32,
    AlignSelf => Align,
    Direction => Direction,
    Position => PositionType,
    Overflow => Overflow,

    BorderTopLeftRadius => f32,
    BorderTopRightRadius => f32,
    BorderBottomRightRadius => f32,
    BorderBottomLeftRadius => f32,

    JustifyContent => Justify,
    FlexDirection => FlexDirection,
    AlignContent => Align,
    AlignItems => Align,
    FlexWrap => Wrap,
    ColumnGap => f32,
    RowGap => f32,

    Top => StyleUnit,
    Right => StyleUnit,
    Bottom => StyleUnit,
    Left => StyleUnit,

    Transform => StyleTransform,
);

pub fn expand_mixed_style(mixed: HashMap<AllStylePropertyKey, StylePropertyValue>) -> HashMap<StylePropertyKey, StylePropertyValue> {
    let mut result = HashMap::new();
    for (k, v) in mixed {
        match k {
            AllStylePropertyKey::StylePropertyKey(k) => {
                result.insert(k, v);
            }
            AllStylePropertyKey::CompoundStylePropertyKey(k) => {
                match k {
                    CompoundStylePropertyKey::Background => {
                        result.insert(StylePropertyKey::BackgroundColor, v);
                    }
                    CompoundStylePropertyKey::Gap => {
                        result.insert(StylePropertyKey::RowGap, v.clone());
                        result.insert(StylePropertyKey::ColumnGap, v);
                    }
                    CompoundStylePropertyKey::Border => {
                        result.insert(StylePropertyKey::BorderTop, v.clone());
                        result.insert(StylePropertyKey::BorderRight, v.clone());
                        result.insert(StylePropertyKey::BorderBottom, v.clone());
                        result.insert(StylePropertyKey::BorderLeft, v);
                    }
                    CompoundStylePropertyKey::Margin => {
                        let (t, r, b, l) = parse_box_prop(v);
                        result.insert(StylePropertyKey::MarginTop, t);
                        result.insert(StylePropertyKey::MarginRight, r);
                        result.insert(StylePropertyKey::MarginBottom, b);
                        result.insert(StylePropertyKey::MarginLeft, l);
                    }
                    CompoundStylePropertyKey::Padding => {
                        let (t, r, b, l) = parse_box_prop(v);
                        result.insert(StylePropertyKey::PaddingTop, t);
                        result.insert(StylePropertyKey::PaddingRight, r);
                        result.insert(StylePropertyKey::PaddingBottom, b);
                        result.insert(StylePropertyKey::PaddingLeft, l);
                    }
                    CompoundStylePropertyKey::BorderRadius => {
                        let (t, r, b, l) = parse_box_prop(v);
                        result.insert(StylePropertyKey::BorderTopLeftRadius, t);
                        result.insert(StylePropertyKey::BorderTopRightRadius, r);
                        result.insert(StylePropertyKey::BorderBottomRightRadius, b);
                        result.insert(StylePropertyKey::BorderBottomLeftRadius, l);
                    }
                }
            }
        }
    }
    result
}

fn parse_box_prop(value: StylePropertyValue) -> (StylePropertyValue, StylePropertyValue, StylePropertyValue, StylePropertyValue) {
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
            JsValue::__NonExhaustive => Self::Invalid,
        }
    }

    pub fn from_str(value: &str) -> Self {
        Self::String(value.to_string())
    }

    pub fn to_color(&self, default: ColorPropValue) -> ColorPropValue {
        match self {
            StylePropertyValue::String(c) => {
                parse_color(c)
            }
            _ => {
                default
            }
        }
    }
    pub fn to_str(&self, default: &str) -> String {
        match self {
            StylePropertyValue::Float(f) => { f.to_string() }
            StylePropertyValue::String(s) => { s.to_string() }
            StylePropertyValue::Invalid => default.to_string()
        }
    }

    pub fn to_len(&self, default: StyleUnit) -> StyleUnit {
        match self {
            StylePropertyValue::Float(f) => {
                StyleUnit::Point(OrderedFloat(*f))
            }
            StylePropertyValue::String(s) => {
                parse_length(s)
            }
            StylePropertyValue::Invalid => {
                default
            }
        }
    }

    pub fn to_f32(&self, default: f32) -> f32 {
        match self {
            StylePropertyValue::Float(f) => {*f}
            StylePropertyValue::String(s) => {parse_float(s)}
            StylePropertyValue::Invalid => default,
        }
    }

    pub fn to_matrix(&self) -> Option<Matrix> {
        match self {
            StylePropertyValue::String(str) => {
                parse_transform(str)
            }
            _ => None,
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
}

impl ComputedStyle {
    pub fn default() -> Self {
        Self {
            color: Color::new(0),
            background_color: Color::new(0)
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ColorPropValue {
    Inherit,
    Color(Color),
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

pub struct StyleNodeInner {
    yoga_node: Node,
    shadow_node: Option<Node>,

    parent: Option<MrcWeak<Self>>,
    children: Vec<StyleNode>,
    border_paths: CacheValue<BorderParams, [Path; 4]>,

    // (inherited, computed)
    pub color: ColorPropValue,
    pub border_radius: [f32; 4],
    pub border_color: [Color;4],
    pub background_color: ColorPropValue,
    pub background_image: Option<Image>,
    pub transform: Option<Matrix>,
    pub computed_style: ComputedStyle,
    pub on_changed: Option<Box<dyn FnMut(&str)>>,
}

#[derive(PartialEq)]
struct BorderParams {
    border_width: [f32; 4],
    border_radius: [f32; 4],
    width: f32,
    height: f32,
}

impl Deref for StyleNodeInner {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        &self.yoga_node
    }
}

impl DerefMut for StyleNodeInner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.yoga_node
    }
}

#[derive(Clone, PartialEq)]
pub struct StyleNode {
    inner: Mrc<StyleNodeInner>,
}

impl Deref for StyleNode {
    type Target = StyleNodeInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for StyleNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl StyleNode {
    pub fn new() -> Self {
        let transparent = Color::from_argb(0,0,0,0);
        let inner = StyleNodeInner {
            yoga_node: Node::new(),
            shadow_node: None,
            parent: None,
            children: Vec::new(),
            border_radius: [0.0, 0.0, 0.0, 0.0],
            border_color: [transparent, transparent, transparent, transparent],
            background_color: ColorPropValue::Color(Color::TRANSPARENT),
            color: ColorPropValue::Inherit,
            background_image: None,
            transform: None,
            computed_style: ComputedStyle::default(),
            on_changed: None,
            border_paths: CacheValue::new(|p: &BorderParams| {
                build_border_paths(p.border_width, p.border_radius, p.width, p.height)
            }),
        };
        Self { inner: Mrc::new(inner) }
    }

    pub fn new_with_shadow() -> Self {
        let mut sn = Self::new();
        sn.inner.shadow_node = Some(Node::new());
        sn
    }

    pub fn get_content_bounds(&self) -> Rect {
        let l = self.get_layout_padding_left().de_nan(0.0);
        let r = self.get_layout_padding_right().de_nan(0.0);
        let t = self.get_layout_padding_top().de_nan(0.0);
        let b = self.get_layout_padding_bottom().de_nan(0.0);
        let (width, height) = self.with_container_node(|n| {
            (n.get_layout_width().de_nan(0.0), n.get_layout_height().de_nan(0.0))
        });
        Rect::new(l, t, width - l - r, height - t - b)
    }

    pub fn set_style(&mut self, p: &StyleProp) -> (bool, bool) {
        let mut repaint = true;
        let mut need_layout = true;
        let standard_node = Node::new();

        match p {
            StyleProp::Color(v) => {
                self.color = v.resolve(&ColorPropValue::Inherit);
                self.compute_color();
                need_layout = false;
            }
            StyleProp::BackgroundColor (value) =>   {
                self.background_color = value.resolve(&ColorPropValue::Color(Color::TRANSPARENT));
                self.compute_background_color();
                need_layout = false;
            }
            StyleProp::BorderTop (value) =>   {
                self.set_border(&value, &vec![0])
            }
            StyleProp::BorderRight (value) =>   {
                self.set_border(&value, &vec![1])
            }
            StyleProp::BorderBottom (value) =>   {
                self.set_border(&value, &vec![2])
            }
            StyleProp::BorderLeft (value) =>   {
                self.set_border(&value, &vec![3])
            }
            StyleProp::Display (value) =>   {
                self.set_display(value.resolve(&Display::Flex))
            }
            StyleProp::Width (value) =>   {
                self.set_width(value.resolve(&standard_node.get_style_width()))
            },
            StyleProp::Height (value) =>   {
                self.set_height(value.resolve(&standard_node.get_style_height()))
            },
            StyleProp::MaxWidth (value) =>   {
                self.set_max_width(value.resolve(&standard_node.get_style_max_width()))
            },
            StyleProp::MaxHeight (value) =>   {
                self.set_max_height(value.resolve(&standard_node.get_style_max_height()))
            },
            StyleProp::MinWidth (value) =>   {
                self.set_min_width(value.resolve(&standard_node.get_style_min_width()))
            },
            StyleProp::MinHeight (value) =>   {
                self.set_min_height(value.resolve(&standard_node.get_style_min_height()))
            },
            StyleProp::MarginTop (value) =>   {
                self.set_margin(Edge::Top, value.resolve(&standard_node.get_style_margin_top()))
            },
            StyleProp::MarginRight (value) =>   {
                self.set_margin(Edge::Right, value.resolve(&standard_node.get_style_margin_right()))
            },
            StyleProp::MarginBottom (value) =>   {
                self.set_margin(Edge::Bottom, value.resolve(&standard_node.get_style_margin_bottom()))
            },
            StyleProp::MarginLeft (value) =>   {
                self.set_margin(Edge::Left, value.resolve(&standard_node.get_style_margin_left()))
            },
            StyleProp::PaddingTop (value) =>   {
                self.with_container_node_mut(|n| {
                    n.set_padding(Edge::Top, value.resolve(&standard_node.get_style_padding_top()))
                })
            },
            StyleProp::PaddingRight (value) =>   {
                self.with_container_node_mut(|n| {
                    n.set_padding(Edge::Right, value.resolve(&standard_node.get_style_padding_right()))
                })
            },
            StyleProp::PaddingBottom (value) =>   {
                self.with_container_node_mut(|n| {
                    n.set_padding(Edge::Bottom, value.resolve(&standard_node.get_style_padding_bottom()))
                })
            },
            StyleProp::PaddingLeft (value) =>   {
                self.with_container_node_mut(|n| {
                    n.set_padding(Edge::Left, value.resolve(&standard_node.get_style_padding_left()))
                })
            },
            StyleProp::Flex (value) =>   {
                self.set_flex(value.resolve(&standard_node.get_flex()))
            },
            StyleProp::FlexBasis (value) =>   {
                self.set_flex_basis(value.resolve(&standard_node.get_flex_basis()))
            },
            StyleProp::FlexGrow (value) =>   {
                self.set_flex_grow(value.resolve(&standard_node.get_flex_grow()))
            },
            StyleProp::FlexShrink (value) =>   {
                self.set_flex_shrink(value.resolve(&standard_node.get_flex_shrink()))
            },
            StyleProp::AlignSelf (value) =>   {
                self.set_align_self(value.resolve(&Align::FlexStart))
            },
            StyleProp::Direction (value) =>   {
                self.set_direction(value.resolve(&Direction::LTR))
            },
            StyleProp::Position (value) =>   {
                self.set_position_type(value.resolve(&PositionType::Static))
            },
            StyleProp::Top (value) =>   {
                self.yoga_node.set_position(Edge::Top, value.resolve(&StyleUnit::UndefinedValue));
            },
            StyleProp::Right (value) =>   {
                self.yoga_node.set_position(Edge::Right, value.resolve(&StyleUnit::UndefinedValue));
            },
            StyleProp::Bottom (value) =>   {
                self.yoga_node.set_position(Edge::Bottom, value.resolve(&StyleUnit::UndefinedValue));
            },
            StyleProp::Left (value) =>   {
                self.yoga_node.set_position(Edge::Left, value.resolve(&StyleUnit::UndefinedValue));
            },
            StyleProp::Overflow (value) =>   {
                self.set_overflow(value.resolve(&Overflow::Hidden))
            },
            StyleProp::BorderTopLeftRadius (value) =>   {
                self.border_radius[0] = value.resolve(&0.0)
            },
            StyleProp::BorderTopRightRadius (value) =>   {
                self.border_radius[1] = value.resolve(&0.0)
            },
            StyleProp::BorderBottomRightRadius (value) =>   {
                self.border_radius[2] = value.resolve(&0.0)
            },
            StyleProp::BorderBottomLeftRadius (value) =>   {
                self.border_radius[3] = value.resolve(&0.0)
            },
            StyleProp::Transform (value) =>   {
                if let StylePropVal::Custom(v) = value {
                    self.transform = Some(v.to_matrix());
                } else {
                    self.transform = None;
                }
            }

            // container node style
            StyleProp::JustifyContent (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_justify_content(value.resolve(&Justify::FlexStart))
                });
            },
            StyleProp::FlexDirection (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_flex_direction(value.resolve(&FlexDirection::Column))
                });
            },
            StyleProp::AlignContent (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_align_content(value.resolve(&Align::FlexStart))
                });
            },
            StyleProp::AlignItems (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_align_items(value.resolve(&Align::FlexStart))
                });
            },
            StyleProp::FlexWrap (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_flex_wrap(value.resolve(&Wrap::NoWrap))
                });
            },
            StyleProp::ColumnGap (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_column_gap(value.resolve(&0.0))
                });
            },
            StyleProp::RowGap (value) =>   {
                self.with_container_node_mut(|layout| {
                    layout.set_row_gap(value.resolve(&0.0))
                });
            },
            //TODO aspectratio
        }
        if let Some(on_changed) = &mut self.on_changed {
            on_changed(p.name());
        }

        return (repaint, need_layout)
    }

    inherit_color_prop!(
        compute_color, compute_children_color, color, "color", Color::from_rgb(0, 0, 0)
    );
    inherit_color_prop!(
        compute_background_color, compute_children_background_color, background_color, "backgroundcolor", Color::from_argb(0, 0, 0, 0)
    );

    pub fn get_border_paths(&self) -> [Path; 4] {
        let border_width = [
            self.get_layout_border_top().de_nan(0.0),
            self.get_layout_border_right().de_nan(0.0),
            self.get_layout_border_bottom().de_nan(0.0),
            self.get_layout_border_left().de_nan(0.0),
        ];
        let width = self.get_layout_width().de_nan(0.0);
        let height = self.get_layout_height().de_nan(0.0);
        self.border_paths.get(BorderParams {
            border_width,
            border_radius: self.border_radius,
            width,
            height
        })
    }

    fn get_parent(&self) -> Option<StyleNode> {
        if let Some(p) = &self.parent {
            if let Some(sn) = p.upgrade() {
                return Some(StyleNode {
                    inner: sn,
                })
            }
        }
        return None
    }

    fn set_border(&mut self, value: &StylePropVal<StyleBorder>, edges: &Vec<usize>) {
        let default_border = StyleBorder(StyleUnit::UndefinedValue, StyleColor::Color(Color::TRANSPARENT));
        let value = value.resolve(&default_border);
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
            self.inner.set_border(edges_list[*index], width);
        }
    }

    pub fn insert_child(&mut self, child: &mut StyleNode, index: u32) {
        self.inner.children.insert(index as usize, child.clone());
        child.parent = Some(self.inner.as_weak());
        self.with_container_node_mut(|n| {
            n.insert_child(&mut child.inner.yoga_node, index)
        })
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

    fn calculate_shadow_layout_auto(&mut self) {
        let width = self.inner.yoga_node.get_layout_width().de_nan(0.0);
        let height = self.inner.yoga_node.get_layout_height().de_nan(0.0);
        if let Some(sl) = &mut self.inner.shadow_node {
            //TODO fix direction
            sl.calculate_layout(width, height, Direction::LTR);
        } else {
            for c in &mut self.inner.children {
                c.calculate_shadow_layout_auto();
            }
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

pub fn parse_style(style: JsValue) -> HashMap<AllStylePropertyKey, StylePropertyValue> {
    let mut style_map = HashMap::new();// get_style_defaults();
    if let Some(obj) = style.get_properties() {
        //TODO use default style
        obj.into_iter().for_each(|(k, v)| {
            if let Some(key) = AllStylePropertyKey::from_str(&k) {
                style_map.insert(key, StylePropertyValue::from_js_value(v));
            }
        });
    }
    style_map
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
                    "borderRadius" => {
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

pub fn parse_display(str: &str) -> Display {
    if str.to_lowercase() == "none" {
        Display::None
    } else {
        Display::Flex
    }
}

pub fn parse_display2(str: &str) -> Option<Display> {
    match str.to_lowercase().as_str() {
        "none" => Some(Display::None),
        "flex" => Some(Display::Flex),
        _ => None
    }
}


pub fn parse_justify(str: &str) -> Justify {
    let key = str.to_lowercase();
    match key.as_str() {
        "flex-start" => Justify::FlexStart,
        "center" => Justify::Center,
        "flex-end" => Justify::FlexEnd,
        "space-between" => Justify::SpaceBetween,
        "space-around" => Justify::SpaceAround,
        "space-evenly" => Justify::SpaceEvenly,
        _ => Justify::FlexStart,
    }
}

pub fn parse_flex_direction(value: &str) -> FlexDirection {
    let key = value.to_lowercase();
    match key.as_str() {
        "column" => FlexDirection::Column,
        "column-reverse" => FlexDirection::ColumnReverse,
        "row" => FlexDirection::Row,
        "row-reverse" => FlexDirection::RowReverse,
        _ => FlexDirection::Row,
    }
}

pub fn parse_flex_direction2(value: &str) -> Option<FlexDirection> {
    let key = value.to_lowercase();
    let r = match key.as_str() {
        "column" => FlexDirection::Column,
        "column-reverse" => FlexDirection::ColumnReverse,
        "row" => FlexDirection::Row,
        "row-reverse" => FlexDirection::RowReverse,
        _ => return None,
    };
    Some(r)
}

pub fn parse_float(value: &str) -> f32 {
    f32::from_str(value).unwrap_or(0.0)
}

pub fn parse_length(value: &str) -> StyleUnit {
    //TODO no unwrap
    return if value.ends_with("%") {
        let width = f32::from_str(value.strip_suffix("%").unwrap()).unwrap();
        StyleUnit::Percent(OrderedFloat(width))
    } else {
        match f32::from_str(value) {
            Ok(v) => {
                StyleUnit::Point(OrderedFloat(v))
            }
            Err(err) => {
                eprintln!("Invalid value:{}", err);
                StyleUnit::UndefinedValue
            }
        }
    }
}

pub fn parse_style_unit(value: &str) -> Option<StyleUnit> {
    //TODO no unwrap
    return if value.ends_with("%") {
        let width = f32::from_str(value.strip_suffix("%").unwrap()).unwrap();
        Some(StyleUnit::Percent(OrderedFloat(width)))
    } else {
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

pub fn parse_color(value: &str) -> ColorPropValue {
    if let Some(hex) = value.strip_prefix("#") {
        match parse_hex_color(hex) {
            None => ColorPropValue::Inherit,
            Some(c) => ColorPropValue::Color(c),
        }
    } else {
        ColorPropValue::Inherit
    }
}

pub fn parse_align(value: &str) -> Align {
    let key = value.to_lowercase();
    match key.as_str() {
        "auto" => Align::Auto,
        "flex-start" => Align::FlexStart,
        "center" => Align::Center,
        "flex-end" => Align::FlexEnd,
        "stretch" => Align::Stretch,
        "baseline" => Align::Baseline,
        "space-between" => Align::SpaceBetween,
        "space-around" => Align::SpaceAround,
        _ => Align::FlexStart,
    }
}

pub fn parse_wrap(value: &str) -> Wrap {
    let key = value.to_lowercase();
    match key.as_str() {
        "no-wrap" => Wrap::NoWrap,
        "wrap" => Wrap::Wrap,
        "wrap-reverse" => Wrap::WrapReverse,
        _ => Wrap::NoWrap,
    }
}

pub fn parse_direction(value: &str) -> Direction {
    let key = value.to_lowercase();
    match key.as_str() {
        "inherit" => Direction::Inherit,
        "ltr" => Direction::LTR,
        "rtl" => Direction::RTL,
        _ => Direction::Inherit,
    }
}

pub fn parse_position_type(value: &str) -> PositionType {
    let key = value.to_lowercase();
    match key.as_str() {
        "static" => PositionType::Static,
        "relative" => PositionType::Relative,
        "absolute" => PositionType::Absolute,
        _ => PositionType::Static,
    }
}

pub fn parse_overflow(value: &str) -> Overflow {
    let key = value.to_lowercase();
    match key.as_str() {
        "visible" => Overflow::Visible,
        "hidden" => Overflow::Hidden,
        "scroll" => Overflow::Scroll,
        _ => Overflow::Visible,
    }
}

fn parse_transform(value: &str) -> Option<Matrix> {
    let value = value.trim();
    if !value.ends_with(")") {
        return None;
    }
    let left_p = value.find("(")?;
    let func = &value[0..left_p];
    let param_str = &value[left_p + 1..value.len() - 1];
    match func {
        "matrix" => parse_matrix(param_str).ok(),
        "rotate" => parse_rotate(param_str).ok(),
        _ => None,
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

fn parse_rotate(value: &str) -> Result<Matrix, Error> {
    if let Some(v) = value.strip_suffix("deg") {
        let v = f32::from_str(v)? / 180.0 * PI;
        Ok(create_matrix([v.cos(), v.sin(), -v.sin(), v.cos(), 0.0, 0.0]))
    } else {
        Err(anyhow!("invalid value"))
    }
}

fn parse_rotate_op(value: &str) -> Option<StyleTransformOp> {
    if let Some(v) = value.strip_suffix("deg") {
        let v = f32::from_str(v).ok()?;
        Some(StyleTransformOp::Rotate(v))
    } else {
        None
    }
}

fn parse_border(value: &str) -> Option<StyleBorder> {
    let parts = value.split(" ");
    let mut width = StyleUnit::Point(OrderedFloat(0.0));
    let mut color = Color::from_rgb(0, 0, 0);
    for p in parts {
        let p = p.trim();
        if p.starts_with("#") {
            if let Some(c) = parse_hex_color(p.strip_prefix('#').unwrap()) {
                color = c;
            }
        } else if let Some(w) = parse_style_unit(p) {
            width = w;
        }
    }
    Some(StyleBorder(width, StyleColor::Color(color)))
}