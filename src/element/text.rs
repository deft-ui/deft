// pub mod skia_text_paragraph;
mod rasterize_cache;
pub mod simple_text_paragraph;

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
