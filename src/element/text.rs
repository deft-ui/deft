// pub mod skia_text_paragraph;
mod rasterize_cache;
pub mod simple_text_paragraph;
pub mod text_paragraph;


use crate::text::TextAlign;

// zero-width space for caret
const ZERO_WIDTH_WHITESPACE: &str = "\u{200B}";

pub type AtomOffset = usize;
pub type RowOffset = usize;
pub type ColOffset = usize;

pub fn intersect_range<T: Ord>(range1: (T, T), range2: (T, T)) -> Option<(T, T)> {
    let start = T::max(range1.0, range2.0);
    let end = T::min(range1.1, range2.1);
    if end > start {
        Some((start, end))
    } else {
        None
    }
}
pub fn parse_align(align: &str) -> TextAlign {
    match align {
        "left" => TextAlign::Left,
        "right" => TextAlign::Right,
        "center" => TextAlign::Center,
        _ => TextAlign::Left,
    }
}
