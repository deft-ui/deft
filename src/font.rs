use crate as deft;
use std::path::Path;
use deft_macros::mrc_object;
use skia_safe::Rect;
use swash::{Attributes, CacheKey, Charmap, FontRef, GlyphId, Metrics};
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::scale::image::Image;
use swash::zeno::Format;
use crate::element::text::simple_text_paragraph::TextBlock;

#[mrc_object]
pub struct Font {
    // Full content of the font file
    data: Vec<u8>,
    // Offset to the table directory
    offset: u32,
    // Cache key
    key: CacheKey,
}

unsafe impl Send for Font {}
unsafe impl Sync for Font {}

impl Font {
    pub fn from_file<P: AsRef<Path>>(path: P, index: usize) -> Option<Self> {
        // Read the full font file
        let data = std::fs::read(path).ok()?;
        // Create a temporary font reference for the first font in the file.
        // This will do some basic validation, compute the necessary offset
        // and generate a fresh cache key for us.
        Self::from_bytes(data, index)
    }

    pub fn from_bytes(data: Vec<u8>, index: usize) -> Option<Self> {
        let font = FontRef::from_index(&data, index)?;
        let (offset, key) = (font.offset, font.key);
        // Return our struct with the original file data and copies of the
        // offset and key from the font reference
        Some(FontData { data, offset, key }.to_ref())
    }

    // As a convenience, you may want to forward some methods.
    pub fn attributes(&self) -> Attributes {
        self.as_ref().attributes()
    }

    pub fn charmap(&self) -> Charmap {
        self.as_ref().charmap()
    }

    pub fn metrics(&self) -> Metrics {
        self.as_ref().metrics(&[])
    }

    pub fn raster_bounds(&self, glyph_id: GlyphId, font_size: f32) -> Result<Rect, ()> {
        let metrics = self.as_ref().glyph_metrics(&[]);
        let scale = font_size / metrics.units_per_em() as f32;
        let width = metrics.advance_width(glyph_id) * scale;
        let left = metrics.lsb(glyph_id) * scale;
        let top = -metrics.tsb(glyph_id) * scale;
        let height = metrics.advance_height(glyph_id) * scale;
        Ok(Rect::from_xywh(left, top, width, height))
    }

    pub fn glyph_for_char(&self, c: char) -> Option<GlyphId> {
        Some(self.charmap().map(c))
    }

    pub fn rasterize_glyph(&self, glyph_id: GlyphId, font_size: f32) -> Option<Image> {
        let mut context = ScaleContext::new();
        let mut scaler = context.builder(self.as_ref())
            .size(font_size)
            .hint(true)
            .build();
        Render::new(&[
            // Color outline with the first palette
            Source::ColorOutline(0),
            // Color bitmap with best fit selection mode
            Source::ColorBitmap(StrikeWith::BestFit),
            // Standard scalable outline
            Source::Outline,
        ])
            // Select a subpixel format
            .format(Format::Alpha)
            // Render the image
            .render(&mut scaler, glyph_id)
    }

    // Create the transient font reference for accessing this crate's
    // functionality.
    pub fn as_ref(&self) -> FontRef {
        // Note that you'll want to initialize the struct directly here as
        // using any of the FontRef constructors will generate a new key which,
        // while completely safe, will nullify the performance optimizations of
        // the caching mechanisms used in this crate.
        FontRef {
            data: &self.data,
            offset: self.offset,
            key: self.key,
        }
    }
}
