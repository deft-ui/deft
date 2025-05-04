pub mod family;

use crate as deft;
use crate::element::text::simple_text_paragraph::TextBlock;
use deft_macros::mrc_object;
use memmap2::{Mmap, MmapOptions};
use skia_safe::Rect;
use std::fs::File;
use std::ops::Deref;
use std::path::Path;
use swash::scale::image::Image;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::Format;
use swash::{Attributes, CacheKey, Charmap, FontRef, GlyphId, Metrics};

enum FontContent {
    Mmap(Mmap),
    Mem(Vec<u8>),
}

impl FontContent {
    pub fn as_ref(&self) -> &[u8] {
        match self {
            FontContent::Mmap(mm) => mm.as_ref(),
            FontContent::Mem(data) => data.deref(),
        }
    }
}

#[mrc_object]
pub struct Font {
    // Full content of the font file
    data: FontContent,
    // Offset to the table directory
    offset: u32,
    // Cache key
    key: CacheKey,

    family_name: String,
}

unsafe impl Send for Font {}
unsafe impl Sync for Font {}

impl Font {
    pub fn from_file<P: AsRef<Path>>(path: P, index: usize, family_name: String) -> Option<Self> {
        let file = File::open(path).ok()?;
        let mmap = unsafe { MmapOptions::new().map(&file).ok()? };
        Self::new(FontContent::Mmap(mmap), index, family_name)
    }

    pub fn from_bytes(data: Vec<u8>, index: usize, family_name: String) -> Option<Self> {
        Self::new(FontContent::Mem(data), index, family_name)
    }

    fn new(data: FontContent, index: usize, family_name: String) -> Option<Self> {
        let font = FontRef::from_index(data.as_ref(), index)?;
        let (offset, key) = (font.offset, font.key);
        Some(
            FontData {
                data,
                offset,
                key,
                family_name,
            }
            .to_ref(),
        )
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
        let mut scaler = context
            .builder(self.as_ref())
            .size(font_size)
            .hint(true)
            .build();
        Render::new(&[
            // Color outline with the first palette
            Source::ColorOutline(0),
            Source::Bitmap(StrikeWith::BestFit),
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

    /// Just for debug
    pub fn name(&self) -> &str {
        &self.family_name
    }

    // Create the transient font reference for accessing this crate's
    // functionality.
    pub fn as_ref(&self) -> FontRef {
        // Note that you'll want to initialize the struct directly here as
        // using any of the FontRef constructors will generate a new key which,
        // while completely safe, will nullify the performance optimizations of
        // the caching mechanisms used in this crate.
        FontRef {
            data: self.data.as_ref(),
            offset: self.offset,
            key: self.key,
        }
    }
}
