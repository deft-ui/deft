use crate::font::Font;
use crate::mrc::Mrc;
use std::collections::HashMap;
use swash::scale::image::Image;
use swash::{CacheKey, GlyphId};

#[derive(Debug, Hash, PartialEq, Eq)]
struct RasterizeCacheKey {
    font_cache_key: CacheKey,
    glyph_id: GlyphId,
    font_size: u32,
}

pub struct RasterizeCache {
    cache: Mrc<HashMap<RasterizeCacheKey, Option<Image>>>,
}

impl RasterizeCache {
    pub fn new() -> Self {
        Self {
            cache: Mrc::new(HashMap::new()),
        }
    }
    pub fn get_image(
        &self,
        font: &Font,
        glyph_id: GlyphId,
        size: f32,
    ) -> Option<Image> {
        let key = RasterizeCacheKey {
            font_cache_key: font.as_ref().key,
            glyph_id,
            font_size: size as u32,
        };
        self.cache
            .clone()
            .entry(key)
            .or_insert_with(move || font.rasterize_glyph(glyph_id, size))
            .clone()
    }
}
