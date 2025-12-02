pub mod family;

use crate as deft;
use crate::mrc::Mrc;
use deft_macros::mrc_object;
use memmap2::{Mmap, MmapOptions};
use skia_safe::Rect;
use std::fs::File;
use std::ops::Deref;
use std::path::Path;
use swash::scale::image::Image;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::{Angle, Format, Transform};
use swash::{Attributes, CacheKey, Charmap, FontRef, GlyphId, Metrics, Style, Weight};

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
    data: Mrc<FontContent>,
    // Offset to the table directory
    offset: u32,
    // Cache key
    key: CacheKey,

    family_name: String,

    weight: Weight,

    style: Style,
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
        let weight = font.attributes().weight();
        let style = font.attributes().style();
        let (offset, key) = (font.offset, font.key);
        Some(
            FontData {
                data: Mrc::new(data),
                offset,
                key,
                family_name,
                weight,
                style,
            }
            .to_ref(),
        )
    }

    pub fn synthesize(self, weight: Weight, style: Style) -> Self {
        let data = self.data.clone();
        FontData {
            data,
            offset: self.offset,
            key: self.key,
            family_name: self.family_name.clone(),
            weight,
            style,
        }
        .to_ref()
    }

    // As a convenience, you may want to forward some methods.
    pub fn attributes(&self) -> Attributes {
        self.as_ref().attributes()
    }

    pub fn charmap(&self) -> Charmap<'_> {
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
        let mut render = Render::new(&[
            // Color outline with the first palette
            Source::ColorOutline(0),
            Source::Bitmap(StrikeWith::BestFit),
            // Color bitmap with best fit selection mode
            Source::ColorBitmap(StrikeWith::BestFit),
            // Standard scalable outline
            Source::Outline,
        ]);
        // Select a subpixel format
        let font_attrs = self.as_ref().attributes();
        if font_attrs.weight() != self.weight {
            render.embolden((self.weight.0 as f32 - font_attrs.weight().0 as f32) / 1000.0 * 2.0);
        }
        if font_attrs.style() == Style::Normal {
            if self.style == Style::Italic {
                render.transform(Some(Transform::skew(
                    Angle::from_degrees(14.0),
                    Angle::from_degrees(0.0),
                )));
            } else if let Style::Oblique(ang) = &self.style {
                render.transform(Some(Transform::skew(
                    Angle::from_degrees(ang.to_degrees()),
                    Angle::from_degrees(0.0),
                )));
            }
        }

        // Render the image
        render.format(Format::Alpha).render(&mut scaler, glyph_id)
    }

    /// Just for debug
    pub fn name(&self) -> &str {
        &self.family_name
    }

    // Create the transient font reference for accessing this crate's
    // functionality.
    pub fn as_ref(&self) -> FontRef<'_> {
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
