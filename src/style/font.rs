use crate::element::paragraph::parse_weight;
use crate::font::family::{FontFamilies, FontFamily};
use crate::style::length::{parse_percent, Length, LengthContext};
use crate::style::PropValueParse;
use skia_safe::font_style::{Slant, Weight};
use std::ops::Deref;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Copy, Hash, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

impl FontStyle {
    pub fn to_slant(&self) -> Slant {
        match self {
            FontStyle::Normal => Slant::Upright,
            FontStyle::Italic => Slant::Italic,
            FontStyle::Oblique => Slant::Oblique,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum LineHeightVal {
    Length(Length),
    Percent(f32),
    Number(f32),
    Normal,
}

impl LineHeightVal {
    pub fn to_px(&self, length_context: &LengthContext) -> Option<f32> {
        let v = match self {
            LineHeightVal::Length(l) => l.to_px(length_context),
            LineHeightVal::Percent(p) => length_context.font_size * p / 100.0,
            LineHeightVal::Number(n) => length_context.font_size * n,
            LineHeightVal::Normal => return None,
        };
        Some(v)
    }
}

impl PropValueParse for FontFamilies {
    fn parse_prop_value(value: &str) -> Option<Self> {
        let mut list = Vec::new();
        for p in value.trim().split(",") {
            if p.starts_with("\"") && p.ends_with("\"") {
                list.push(FontFamily::new(&p[1..p.len() - 1]));
            } else if p.starts_with("'") && p.ends_with("'") {
                list.push(FontFamily::new(&p[1..p.len() - 1]));
            } else {
                list.push(FontFamily::new(p));
            }
        }
        Some(Self::new(list))
    }

    fn to_style_string(&self) -> String {
        let list: Vec<String> = self
            .as_slice()
            .iter()
            .map(|it| format!("'{}'", it.name()))
            .collect();
        list.join(",")
    }
}

impl PropValueParse for FontStyle {
    fn parse_prop_value(value: &str) -> Option<Self> {
        let value = value.trim();
        match value {
            "normal" => Some(Self::Normal),
            "italic" => Some(Self::Italic),
            "oblique" => Some(Self::Oblique),
            _ => None,
        }
    }

    fn to_style_string(&self) -> String {
        match self {
            Self::Normal => String::from("normal"),
            Self::Italic => String::from("italic"),
            Self::Oblique => String::from("oblique"),
        }
    }
}

impl PropValueParse for Weight {
    fn parse_prop_value(value: &str) -> Option<Self> {
        parse_weight(value)
    }

    fn to_style_string(&self) -> String {
        format!("{}", self.deref())
    }
}

impl PropValueParse for LineHeightVal {
    fn parse_prop_value(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("normal") {
            Some(LineHeightVal::Normal)
        } else if let Ok(v) = f32::from_str(value) {
            Some(LineHeightVal::Number(v))
        } else if let Some(len) = Length::from_str(value) {
            Some(LineHeightVal::Length(len))
        } else if let Some(v) = parse_percent(value) {
            Some(LineHeightVal::Percent(v))
        } else {
            None
        }
    }

    fn to_style_string(&self) -> String {
        match self {
            LineHeightVal::Length(l) => l.to_style_string(),
            LineHeightVal::Percent(p) => format!("{}%", p),
            LineHeightVal::Number(n) => format!("{}", n),
            LineHeightVal::Normal => "normal".to_string(),
        }
    }
}
