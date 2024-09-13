use skia_safe::{Canvas, Font, Paint, Point, Rect};
use skia_safe::textlayout::{Paragraph as SkParagraph, ParagraphBuilder, ParagraphStyle, StrutStyle, TextAlign, TextStyle};
use crate::element::text::{FONT_COLLECTION, FONT_MGR, ZERO_WIDTH_WHITESPACE};
use crate::element::text::text_paragraph::TextParams;

pub struct SkiaTextParagraph {
    text: String,
    paragraph: SkParagraph,
}

impl SkiaTextParagraph {
    pub fn new(text: String, params: &TextParams) -> Self {
        let paragraph = Self::build_paragraph(&text, params);
        Self {
            paragraph,
            text,
        }
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    pub fn layout(&mut self, available_width: f32) {
        self.paragraph.layout(available_width)
    }

    pub fn height(&self) -> f32 {
        self.paragraph.height()
    }

    pub fn max_intrinsic_width(&self) -> f32 {
        self.paragraph.max_intrinsic_width()
    }

    pub fn get_char_bounds(&mut self, char_offset: usize) -> Option<Rect> {
        let gc = self.paragraph.get_glyph_info_at_utf16_offset(char_offset);
        gc.map(|g| g.grapheme_layout_bounds)
    }

    pub fn get_char_offset_at_coordinate(&self, coord: (f32, f32)) -> usize {
        self.paragraph.get_glyph_position_at_coordinate(coord).position as usize
    }

    pub fn get_soft_line_height(&self, char_offset: usize) -> f32 {
        let ln = self.paragraph.get_line_number_at_utf16_offset(char_offset).unwrap();
        let lm = self.paragraph.get_line_metrics_at(ln).unwrap();
        lm.height as f32
    }

    pub fn paint(&self, canvas: &Canvas, p: impl Into<Point>) {
        self.paragraph.paint(canvas, p)
    }

    pub fn build_paragraph(text: &str, params: &TextParams) -> SkParagraph {
        let mut text = text.trim_end().to_string();
        text.push_str(ZERO_WIDTH_WHITESPACE);
        let mut font_collection = FONT_COLLECTION.with(|f| f.clone());
        FONT_MGR.with(|fm| {
            font_collection.set_default_font_manager(Some(fm.clone()), None);
        });
        let mut paragraph_style = ParagraphStyle::new();
        paragraph_style.set_text_align(params.align);

        if let Some(line_height) = params.line_height {
            let mut strut_style = StrutStyle::default();
            strut_style.set_font_families(&["Roboto"]);
            strut_style.set_strut_enabled(true);
            strut_style.set_font_size(line_height);
            strut_style.set_force_strut_height(true);
            paragraph_style.set_strut_style(strut_style);
        }

        let mut pb = ParagraphBuilder::new(&paragraph_style, font_collection);
        let mut text_style = TextStyle::new();
        text_style.set_foreground_paint(&params.paint);
        text_style.set_font_size(params.font.size());

        pb.push_style(&text_style);
        pb.add_text(&text);
        pb.build()
    }
}