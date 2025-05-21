pub mod animation;
pub mod border;
pub mod border_path;
pub mod color;
pub mod css_manager;
pub mod flex;
pub mod font;
pub mod length;
mod select;
pub mod styles;
pub mod transform;

use crate as deft;
use crate::animation::css_actor::CssAnimationActor;
use crate::animation::ANIMATIONS;
use crate::animation::{AnimationInstance, WindowAnimationController};
use crate::base::Rect;
use crate::element::ElementWeak;
use crate::event_loop::create_event_loop_callback;
use crate::font::family::FontFamilies;
use crate::mrc::{Mrc, MrcWeak};
use crate::number::DeNan;
use crate::style::animation::AnimationParams;
use crate::style::font::{FontStyle, LineHeightVal};
use crate::style::length::{Length, LengthContext, LengthOrPercent};
use crate::style::transform::StyleTransform;
use crate::style_list::ParsedStyleProp;
use crate::{ok_or_return, some_or_return};
use anyhow::{anyhow, Error};
use deft_macros::mrc_object;
use quick_js::JsValue;
use skia_safe::font_style::Weight;
use skia_safe::{Color, Image, Matrix};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use swash::Style;
use yoga::{
    Align, Direction, Display, Edge, FlexDirection, Justify, Node, Overflow, PositionType,
    StyleUnit, Wrap,
};

//TODO rename
pub trait PropValueParse: Sized {
    fn parse_prop_value(value: &str) -> Option<Self>;
    fn to_style_string(&self) -> String;
}

impl PropValueParse for f32 {
    fn parse_prop_value(value: &str) -> Option<Self> {
        f32::from_str(value).ok()
    }
    fn to_style_string(&self) -> String {
        self.to_string()
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
pub enum StylePropVal<T: PropValueParse> {
    Custom(T),
    Inherit,
    Unset,
}

impl<T: Clone + PropValueParse> StylePropVal<T> {
    pub fn to_style_string(&self) -> String {
        match self {
            StylePropVal::Custom(v) => v.to_style_string(),
            StylePropVal::Inherit => "inherit".to_string(),
            StylePropVal::Unset => "unset".to_string(),
        }
    }
}

macro_rules! define_style_props {
    ($($name: ident => $type: ty, $compute_type: ty; )*) => {
        #[derive(Clone, Debug, PartialEq)]
        pub enum FixedStyleProp {
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

            pub fn to_unresolved(&self) -> FixedStyleProp {
                 match self {
                    $(
                        Self::$name(v) => FixedStyleProp::$name(StylePropVal::Custom(v.clone())),
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

        impl FixedStyleProp {
            pub fn parse_value(key: StylePropKey, value: &str) -> Option<FixedStyleProp> {
                $(
                    if key == StylePropKey::$name {
                        return <$type>::parse_prop_value(value).map(|v| FixedStyleProp::$name(StylePropVal::Custom(v)));
                    }
                )*
                return None
            }
            pub fn parse(key: &str, value: &str) -> Option<FixedStyleProp> {
                let key = key.to_lowercase();
                let k = key.as_str();
                $(
                    if k == stringify!($name).to_lowercase().as_str() {
                        let value_lowercase = value.to_lowercase();
                        let value_lowercase = value_lowercase.as_str();
                        if value_lowercase == "inherit" {
                            return Some(FixedStyleProp::$name(StylePropVal::Inherit));
                        } else if value_lowercase == "unset" {
                            return Some(FixedStyleProp::$name(StylePropVal::Unset));
                        } else {
                            return <$type>::parse_prop_value(value).map(|v| FixedStyleProp::$name(StylePropVal::Custom(v)));
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
    FontFamily      => FontFamilies, FontFamilies;
    FontWeight      => Weight, Weight;
    FontStyle       => FontStyle, Style;
    LineHeight      => LineHeightVal, f32;

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

pub fn parse_box_prop(str: &str, default: &str) -> (String, String, String, String) {
    let parts: Vec<&str> = str.split(" ").filter(|e| !e.is_empty()).collect();
    let top = if let Some(v) = parts.get(0) {
        v
    } else {
        default
    };
    let right = if let Some(v) = parts.get(1) { v } else { top };
    let bottom = if let Some(v) = parts.get(2) { v } else { top };
    let left = if let Some(v) = parts.get(3) { v } else { right };
    (
        top.to_string(),
        right.to_string(),
        bottom.to_string(),
        left.to_string(),
    )
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
    pub border_color: [Color; 4],
    pub background_image: Option<Image>,
    pub transform: Option<StyleTransform>,
    animation_params: AnimationParams,
    animation_instance: Option<AnimationInstance>,
    pub on_changed: Option<Box<dyn FnMut(StylePropKey)>>,
    pub resolved_style_props: HashMap<StylePropKey, ResolvedStyleProp>,
    pub font_size: f32,
    pub color: Color,
    pub background_color: Color,
    pub line_height: Option<f32>,
    pub font_family: FontFamilies,
    pub font_weight: Weight,
    pub font_style: FontStyle,
}

impl StyleNode {
    pub fn new() -> Self {
        let transparent = Color::from_argb(0, 0, 0, 0);
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
            line_height: None,
            font_family: FontFamilies::default(),
            font_weight: Weight::NORMAL,
            font_style: FontStyle::Normal,
        };
        inner.yoga_node.set_position_type(PositionType::Static);
        inner.to_ref()
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
        let bl = self.yoga_node.get_layout_border_left().de_nan(0.0);
        let br = self.yoga_node.get_layout_border_right().de_nan(0.0);
        let bt = self.yoga_node.get_layout_border_top().de_nan(0.0);
        let bb = self.yoga_node.get_layout_border_bottom().de_nan(0.0);
        let width = self.yoga_node.get_layout_width();
        let height = self.yoga_node.get_layout_height();
        // let (width, height) = self.with_container_node(|n| {
        //     (n.get_layout_width().de_nan(0.0), n.get_layout_height().de_nan(0.0))
        // });
        Rect::new(
            l + bl,
            t + bt,
            width - l - r - bl - br,
            height - t - b - bt - bb,
        )
    }

    pub fn get_resolved_value(&self, key: StylePropKey) -> ResolvedStyleProp {
        if let Some(v) = self.resolved_style_props.get(&key) {
            v.clone()
        } else {
            self.get_default_value(key)
        }
    }

    pub fn get_default_value(&self, key: StylePropKey) -> ResolvedStyleProp {
        let standard_node = Node::new();
        let default_border_width = LengthOrPercent::Length(Length::PX(0.0));
        let default_border_color = Color::TRANSPARENT;
        match key {
            StylePropKey::Color => ResolvedStyleProp::Color(Color::BLACK),
            StylePropKey::BackgroundColor => ResolvedStyleProp::BackgroundColor(Color::TRANSPARENT),
            StylePropKey::FontSize => ResolvedStyleProp::FontSize(Length::PX(12.0)),
            StylePropKey::FontFamily => ResolvedStyleProp::FontFamily(FontFamilies::default()),
            StylePropKey::FontWeight => ResolvedStyleProp::FontWeight(Weight::NORMAL),
            StylePropKey::FontStyle => ResolvedStyleProp::FontStyle(FontStyle::Normal),
            StylePropKey::LineHeight => ResolvedStyleProp::LineHeight(LineHeightVal::Normal),
            StylePropKey::BorderTopWidth => ResolvedStyleProp::BorderTopWidth(default_border_width),
            StylePropKey::BorderRightWidth => {
                ResolvedStyleProp::BorderRightWidth(default_border_width)
            }
            StylePropKey::BorderBottomWidth => {
                ResolvedStyleProp::BorderBottomWidth(default_border_width)
            }
            StylePropKey::BorderLeftWidth => {
                ResolvedStyleProp::BorderLeftWidth(default_border_width)
            }
            StylePropKey::BorderTopColor => ResolvedStyleProp::BorderTopColor(default_border_color),
            StylePropKey::BorderRightColor => {
                ResolvedStyleProp::BorderRightColor(default_border_color)
            }
            StylePropKey::BorderBottomColor => {
                ResolvedStyleProp::BorderBottomColor(default_border_color)
            }
            StylePropKey::BorderLeftColor => {
                ResolvedStyleProp::BorderLeftColor(default_border_color)
            }
            StylePropKey::Display => ResolvedStyleProp::Display(Display::Flex),
            StylePropKey::Width => {
                // ResolvedStyleProp::Width(standard_node.get_style_width())
                //TODO fix
                ResolvedStyleProp::Width(LengthOrPercent::Undefined)
            }
            StylePropKey::Height => {
                //TODO fix
                ResolvedStyleProp::Height(LengthOrPercent::Undefined)
            }
            StylePropKey::MaxWidth => {
                //TODO fix
                ResolvedStyleProp::MaxWidth(LengthOrPercent::Undefined)
            }
            StylePropKey::MaxHeight => {
                //TODO fix
                ResolvedStyleProp::MaxHeight(LengthOrPercent::Undefined)
            }
            StylePropKey::MinWidth => {
                //TODO fix
                ResolvedStyleProp::MinWidth(LengthOrPercent::Undefined)
            }
            StylePropKey::MinHeight => {
                //TODO fix
                ResolvedStyleProp::MinHeight(LengthOrPercent::Undefined)
            }
            StylePropKey::MarginTop => ResolvedStyleProp::MarginTop(LengthOrPercent::Undefined),
            StylePropKey::MarginRight => ResolvedStyleProp::MarginRight(LengthOrPercent::Undefined),
            StylePropKey::MarginBottom => {
                ResolvedStyleProp::MarginBottom(LengthOrPercent::Undefined)
            }
            StylePropKey::MarginLeft => ResolvedStyleProp::MarginLeft(LengthOrPercent::Undefined),
            StylePropKey::PaddingTop => ResolvedStyleProp::PaddingTop(LengthOrPercent::Undefined),
            StylePropKey::PaddingRight => {
                ResolvedStyleProp::PaddingRight(LengthOrPercent::Undefined)
            }
            StylePropKey::PaddingBottom => {
                ResolvedStyleProp::PaddingBottom(LengthOrPercent::Undefined)
            }
            StylePropKey::PaddingLeft => ResolvedStyleProp::PaddingLeft(LengthOrPercent::Undefined),
            StylePropKey::Flex => ResolvedStyleProp::Flex(standard_node.get_flex()),
            StylePropKey::FlexBasis => ResolvedStyleProp::FlexBasis(LengthOrPercent::Undefined),
            StylePropKey::FlexGrow => ResolvedStyleProp::FlexGrow(standard_node.get_flex_grow()),
            StylePropKey::FlexShrink => {
                ResolvedStyleProp::FlexShrink(standard_node.get_flex_shrink())
            }
            StylePropKey::AlignSelf => ResolvedStyleProp::AlignSelf(Align::FlexStart),
            StylePropKey::Direction => ResolvedStyleProp::Direction(Direction::LTR),
            StylePropKey::Position => ResolvedStyleProp::Position(PositionType::Static),
            StylePropKey::Top => ResolvedStyleProp::Top(LengthOrPercent::Undefined),
            StylePropKey::Right => ResolvedStyleProp::Right(LengthOrPercent::Undefined),
            StylePropKey::Bottom => ResolvedStyleProp::Bottom(LengthOrPercent::Undefined),
            StylePropKey::Left => ResolvedStyleProp::Left(LengthOrPercent::Undefined),
            StylePropKey::Overflow => ResolvedStyleProp::Overflow(Overflow::Hidden),
            StylePropKey::BorderTopLeftRadius => {
                ResolvedStyleProp::BorderTopLeftRadius(Length::PX(0.0))
            }
            StylePropKey::BorderTopRightRadius => {
                ResolvedStyleProp::BorderTopRightRadius(Length::PX(0.0))
            }
            StylePropKey::BorderBottomRightRadius => {
                ResolvedStyleProp::BorderBottomRightRadius(Length::PX(0.0))
            }
            StylePropKey::BorderBottomLeftRadius => {
                ResolvedStyleProp::BorderBottomLeftRadius(Length::PX(0.0))
            }
            StylePropKey::Transform => ResolvedStyleProp::Transform(StyleTransform::empty()),
            StylePropKey::AnimationName => ResolvedStyleProp::AnimationName("".to_string()),
            StylePropKey::AnimationDuration => ResolvedStyleProp::AnimationDuration(0.0),
            StylePropKey::AnimationIterationCount => {
                ResolvedStyleProp::AnimationIterationCount(1.0)
            }

            StylePropKey::JustifyContent => ResolvedStyleProp::JustifyContent(Justify::FlexStart),
            StylePropKey::FlexDirection => ResolvedStyleProp::FlexDirection(FlexDirection::Column),
            StylePropKey::AlignContent => ResolvedStyleProp::AlignContent(Align::FlexStart),
            StylePropKey::AlignItems => ResolvedStyleProp::AlignItems(Align::FlexStart),
            StylePropKey::FlexWrap => ResolvedStyleProp::FlexWrap(Wrap::NoWrap),
            StylePropKey::ColumnGap => ResolvedStyleProp::ColumnGap(Length::PX(0.0)),
            StylePropKey::RowGap => ResolvedStyleProp::RowGap(Length::PX(0.0)),
            //TODO aspectratio
        }
    }

    pub fn set_resolved_style_prop(
        &mut self,
        p: ResolvedStyleProp,
        length_ctx: &LengthContext,
    ) -> (bool, bool) {
        let prop_key = p.key();
        if self.resolved_style_props.get(&prop_key) == Some(&p) {
            return (false, false);
        }
        self.resolved_style_props.insert(prop_key, p.clone());
        let repaint = true;
        let mut need_layout = true;
        let mut change_notified = false;

        match p {
            ResolvedStyleProp::Color(v) => {
                self.color = v;
                need_layout = false;
            }
            ResolvedStyleProp::BackgroundColor(value) => {
                self.background_color = value;
                need_layout = false;
            }
            ResolvedStyleProp::FontSize(_) => {
                //Do nothing
                change_notified = true;
                //TODO need_layout = false?
            }
            ResolvedStyleProp::FontFamily(value) => {
                self.font_family = value;
            }
            ResolvedStyleProp::FontWeight(value) => {
                self.font_weight = value;
            }
            ResolvedStyleProp::FontStyle(value) => {
                self.font_style = value;
            }
            ResolvedStyleProp::LineHeight(value) => {
                self.line_height = value.to_px(length_ctx);
            }
            ResolvedStyleProp::BorderTopWidth(value) => {
                self.set_border_width(&value, &vec![0], length_ctx);
            }
            ResolvedStyleProp::BorderRightWidth(value) => {
                self.set_border_width(&value, &vec![1], length_ctx);
            }
            ResolvedStyleProp::BorderBottomWidth(value) => {
                self.set_border_width(&value, &vec![2], length_ctx);
            }
            ResolvedStyleProp::BorderLeftWidth(value) => {
                self.set_border_width(&value, &vec![3], length_ctx);
            }
            ResolvedStyleProp::BorderTopColor(value) => {
                self.set_border_color(&value, &vec![0]);
                need_layout = false;
            }
            ResolvedStyleProp::BorderRightColor(value) => {
                self.set_border_color(&value, &vec![1]);
                need_layout = false;
            }
            ResolvedStyleProp::BorderBottomColor(value) => {
                self.set_border_color(&value, &vec![2]);
                need_layout = false;
            }
            ResolvedStyleProp::BorderLeftColor(value) => {
                self.set_border_color(&value, &vec![3]);
                need_layout = false;
            }
            ResolvedStyleProp::Display(value) => self.yoga_node.set_display(value),
            ResolvedStyleProp::Width(value) => {
                self.yoga_node.set_width(value.to_style_unit(&length_ctx));
            }
            ResolvedStyleProp::Height(value) => {
                self.yoga_node.set_height(value.to_style_unit(&length_ctx))
            }
            ResolvedStyleProp::MaxWidth(value) => self
                .yoga_node
                .set_max_width(value.to_style_unit(&length_ctx)),
            ResolvedStyleProp::MaxHeight(value) => self
                .yoga_node
                .set_max_height(value.to_style_unit(&length_ctx)),
            ResolvedStyleProp::MinWidth(value) => self
                .yoga_node
                .set_min_width(value.to_style_unit(&length_ctx)),
            ResolvedStyleProp::MinHeight(value) => self
                .yoga_node
                .set_min_height(value.to_style_unit(&length_ctx)),
            ResolvedStyleProp::MarginTop(value) => self
                .yoga_node
                .set_margin(Edge::Top, value.to_style_unit(&length_ctx)),
            ResolvedStyleProp::MarginRight(value) => self
                .yoga_node
                .set_margin(Edge::Right, value.to_style_unit(&length_ctx)),
            ResolvedStyleProp::MarginBottom(value) => self
                .yoga_node
                .set_margin(Edge::Bottom, value.to_style_unit(&length_ctx)),
            ResolvedStyleProp::MarginLeft(value) => self
                .yoga_node
                .set_margin(Edge::Left, value.to_style_unit(&length_ctx)),
            ResolvedStyleProp::PaddingTop(value) => self.with_container_node_mut(|n| {
                n.set_padding(Edge::Top, value.to_style_unit(&length_ctx))
            }),
            ResolvedStyleProp::PaddingRight(value) => self.with_container_node_mut(|n| {
                n.set_padding(Edge::Right, value.to_style_unit(&length_ctx))
            }),
            ResolvedStyleProp::PaddingBottom(value) => self.with_container_node_mut(|n| {
                n.set_padding(Edge::Bottom, value.to_style_unit(&length_ctx))
            }),
            ResolvedStyleProp::PaddingLeft(value) => self.with_container_node_mut(|n| {
                n.set_padding(Edge::Left, value.to_style_unit(&length_ctx))
            }),
            ResolvedStyleProp::Flex(value) => self.yoga_node.set_flex(value),
            ResolvedStyleProp::FlexBasis(value) => self
                .yoga_node
                .set_flex_basis(value.to_style_unit(&length_ctx)),
            ResolvedStyleProp::FlexGrow(value) => self.yoga_node.set_flex_grow(value),
            ResolvedStyleProp::FlexShrink(value) => self.yoga_node.set_flex_shrink(value),
            ResolvedStyleProp::AlignSelf(value) => self.yoga_node.set_align_self(value),
            ResolvedStyleProp::Direction(value) => self.yoga_node.set_direction(value),
            ResolvedStyleProp::Position(value) => self.yoga_node.set_position_type(value),
            ResolvedStyleProp::Top(value) => {
                self.yoga_node
                    .set_position(Edge::Top, value.to_style_unit(&length_ctx));
            }
            ResolvedStyleProp::Right(value) => {
                self.yoga_node
                    .set_position(Edge::Right, value.to_style_unit(&length_ctx));
            }
            ResolvedStyleProp::Bottom(value) => {
                self.yoga_node
                    .set_position(Edge::Bottom, value.to_style_unit(&length_ctx));
            }
            ResolvedStyleProp::Left(value) => {
                self.yoga_node
                    .set_position(Edge::Left, value.to_style_unit(&length_ctx));
            }
            ResolvedStyleProp::Overflow(value) => self.yoga_node.set_overflow(value),
            ResolvedStyleProp::BorderTopLeftRadius(value) => {
                self.border_radius[0] = value.to_px(&length_ctx);
            }
            ResolvedStyleProp::BorderTopRightRadius(value) => {
                self.border_radius[1] = value.to_px(&length_ctx);
            }
            ResolvedStyleProp::BorderBottomRightRadius(value) => {
                self.border_radius[2] = value.to_px(&length_ctx);
            }
            ResolvedStyleProp::BorderBottomLeftRadius(value) => {
                self.border_radius[3] = value.to_px(&length_ctx);
            }
            ResolvedStyleProp::Transform(value) => {
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
            ResolvedStyleProp::JustifyContent(value) => {
                self.with_container_node_mut(|layout| layout.set_justify_content(value));
            }
            ResolvedStyleProp::FlexDirection(value) => {
                self.with_container_node_mut(|layout| layout.set_flex_direction(value));
            }
            ResolvedStyleProp::AlignContent(value) => {
                self.with_container_node_mut(|layout| layout.set_align_content(value));
            }
            ResolvedStyleProp::AlignItems(value) => {
                self.with_container_node_mut(|layout| layout.set_align_items(value));
            }
            ResolvedStyleProp::FlexWrap(value) => {
                self.with_container_node_mut(|layout| layout.set_flex_wrap(value));
            }
            ResolvedStyleProp::ColumnGap(value) => {
                self.with_container_node_mut(|layout| {
                    layout.set_column_gap(value.to_px(&length_ctx))
                });
            }
            ResolvedStyleProp::RowGap(value) => {
                self.with_container_node_mut(|layout| layout.set_row_gap(value.to_px(&length_ctx)));
            } //TODO aspectratio
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
            me.animation_instance =
                if p.name.is_empty() || p.duration <= 0.0 || p.iteration_count <= 0.0 {
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
                        let mut ani_instance = AnimationInstance::new(
                            actor,
                            duration,
                            iteration_count,
                            Box::new(frame_controller),
                        );
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
                return Some(StyleNode { inner: sn });
            }
        }
        return None;
    }

    fn set_border_width(
        &mut self,
        value: &LengthOrPercent,
        edges: &Vec<usize>,
        length_ctx: &LengthContext,
    ) {
        // let default_border = StyleBorder(StyleUnit::UndefinedValue, StyleColor::Color(Color::TRANSPARENT));
        // let value = value.resolve(&default_border);
        //TODO fix percent?
        let width = match value.to_style_unit(length_ctx) {
            StyleUnit::Point(f) => f.0,
            _ => 0.0,
        };
        for index in edges {
            let edges_list = [Edge::Top, Edge::Right, Edge::Bottom, Edge::Left];
            self.yoga_node.set_border(edges_list[*index], width);
        }
    }

    fn set_border_color(&mut self, color: &Color, edges: &Vec<usize>) {
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

    pub fn calculate_layout(
        &mut self,
        available_width: f32,
        available_height: f32,
        parent_direction: Direction,
    ) {
        self.inner
            .yoga_node
            .calculate_layout(available_width, available_height, parent_direction);
        // self.calculate_shadow_layout();
    }

    pub fn calculate_shadow_layout(
        &mut self,
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
            let mut list = ParsedStyleProp::parse(&k, &v_str);
            result.append(&mut list);
        });
    }
    result
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
    format!(
        "matrix({},{},{},{},{},{})",
        v.scale_x(),
        v.skew_y(),
        v.skew_x(),
        v.scale_y(),
        v.translate_x(),
        v.translate_y()
    )
}

fn create_matrix(values: [f32; 6]) -> Matrix {
    let scale_x = values[0];
    let skew_y = values[1];
    let skew_x = values[2];
    let scale_y = values[3];
    let trans_x = values[4];
    let trans_y = values[5];
    Matrix::new_all(
        scale_x, skew_x, trans_x, skew_y, scale_y, trans_y, 0.0, 0.0, 1.0,
    )
}
