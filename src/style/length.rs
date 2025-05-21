use crate::style::PropValueParse;
use ordered_float::OrderedFloat;
use std::str::FromStr;
use yoga::StyleUnit;

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
            LengthOrPercent::Percent(p) => StyleUnit::Percent(OrderedFloat(*p)),
            LengthOrPercent::Undefined => StyleUnit::UndefinedValue,
            LengthOrPercent::Auto => StyleUnit::Auto,
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

impl PropValueParse for LengthOrPercent {
    fn parse_prop_value(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("auto") {
            return Some(LengthOrPercent::Auto);
        } else if let Some(v) = Length::from_str(value) {
            return Some(Self::Length(v));
        } else {
            let value = value.trim();
            if let Some(v) = parse_percent(value) {
                return Some(Self::Percent(v));
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

impl PropValueParse for Length {
    fn parse_prop_value(value: &str) -> Option<Self> {
        Self::from_str(value)
    }

    fn to_style_string(&self) -> String {
        self.to_str()
    }
}

pub fn parse_percent(value: &str) -> Option<f32> {
    if let Some(v) = value.strip_suffix("%") {
        let v = f32::from_str(v.trim()).ok()?;
        Some(v)
    } else {
        None
    }
}
