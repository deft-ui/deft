use skia_safe::{Font, Paint};
use skia_safe::textlayout::{FontCollection, Paragraph, ParagraphStyle, TextAlign, TextStyle};
use skia_safe::wrapper::ValueWrapper;
use crate::element::paragraph::{ParagraphParams, ZERO_WIDTH_WHITESPACE};
use crate::element::text::{DEFAULT_TYPE_FACE, FONT_COLLECTION, FONT_MGR};
use crate::element::text::simple_text_paragraph::{SimpleTextParagraph, TextBlock};
use crate::element::text::text_paragraph::TextParams;
use crate::string::StringUtils;

pub struct SimpleParagraphBuilder {
    styles: Vec<TextStyle>,
    text_blocks: Vec<TextBlock>,
    font_collection: FontCollection,
}

impl SimpleParagraphBuilder {
    pub fn new(style: &ParagraphParams, font_collection: impl Into<FontCollection>) -> Self {
        let mut text_style = TextStyle::new();
        text_style.set_color(style.color);
        text_style.set_font_size(style.font_size);
        text_style.set_font_families(&style.font_families);
        Self {
            styles: vec![text_style],
            text_blocks: Vec::new(),
            font_collection: font_collection.into(),
        }
    }

    pub fn push_style(&mut self, style: &TextStyle) {
        self.styles.push(style.clone());
    }

    pub fn add_text(&mut self, text: impl Into<String>) {
        let style = self.styles.last().unwrap().clone();
        let font_families = style.font_families();
        let font_families_names: Vec<&str> = font_families.iter().collect();
        let typefaces = self.font_collection.find_typefaces(&font_families_names, style.font_style());
        let tf = typefaces.first().unwrap();
        let font_size = style.font_size();
        let font = Font::from_typeface(tf, font_size);
        self.text_blocks.push(TextBlock {
            text: text.into(),
            style,
            font,
        });
    }

    pub fn build(self) -> SimpleTextParagraph {
        let mut text = String::new();
        SimpleTextParagraph::new(self.text_blocks)
    }
}

#[test]
fn test_get_char_bounds() {
    let params = ParagraphParams {
        text_wrap: None,
        line_height: None,
        align: Default::default(),
        color: Default::default(),
        font_size: 14.0,
        font_families: vec!["monospace".to_string()],
    };
    let font_mgr = FONT_MGR.with(|fm| fm.clone());
    let mut fm = FONT_COLLECTION.with(|fm| fm.clone());
    fm.set_default_font_manager(font_mgr, None);
    let mut pb = SimpleParagraphBuilder::new(&params, fm);
    pb.add_text(format!("{}{}", "12", ZERO_WIDTH_WHITESPACE));
    let mut paragraph = pb.build();
    paragraph.layout(100.0);
    for offset in 0..3 {
        let bounds = paragraph.get_char_bounds(offset);
        assert!(bounds.is_some());
    }
    let bounds0 = paragraph.get_char_bounds(0).unwrap();
    let bounds1 = paragraph.get_char_bounds(1).unwrap();
    assert!(bounds1.left >= bounds0.right);
    assert!(paragraph.max_intrinsic_width() > 0.0);
}