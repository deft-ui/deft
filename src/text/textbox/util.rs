use crate::text::TextDecoration;
use skia_safe::font_style::Weight;
use std::str::FromStr;

pub fn parse_optional_weight(value: Option<&String>) -> Option<Weight> {
    if let Some(v) = value {
        parse_weight(v)
    } else {
        None
    }
}
pub fn parse_weight(value: &str) -> Option<Weight> {
    let w = match value.to_lowercase().as_str() {
        "invisible" => Weight::INVISIBLE,
        "thin" => Weight::THIN,
        "extra-light" => Weight::EXTRA_LIGHT,
        "light" => Weight::LIGHT,
        "normal" => Weight::NORMAL,
        "medium" => Weight::MEDIUM,
        "semi-bold" => Weight::SEMI_BOLD,
        "bold" => Weight::BOLD,
        "extra-bold" => Weight::EXTRA_BOLD,
        "black" => Weight::BLACK,
        "extra-black" => Weight::EXTRA_BLACK,
        _ => return i32::from_str(value).ok().map(|w| Weight::from(w)),
    };
    Some(w)
}

pub fn parse_optional_text_decoration(value: Option<&String>) -> TextDecoration {
    if let Some(v) = value {
        parse_text_decoration(v)
    } else {
        TextDecoration::default()
    }
}

pub fn parse_text_decoration(value: &str) -> TextDecoration {
    let mut decoration = TextDecoration::default();
    for ty in value.split(" ") {
        let t = match ty {
            "none" => TextDecoration::NO_DECORATION,
            "underline" => TextDecoration::UNDERLINE,
            "overline" => TextDecoration::OVERLINE,
            "line-through" => TextDecoration::LINE_THROUGH,
            _ => continue,
        };
        decoration.set(t, true);
    }
    decoration
}
