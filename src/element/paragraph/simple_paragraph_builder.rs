use skia_safe::{Font, FontStyle, Paint, Unichar};
use skia_safe::textlayout::{FontCollection, FontFamilies, Paragraph, ParagraphStyle, TextAlign, TextStyle};
use skia_safe::wrapper::ValueWrapper;
use crate::element::paragraph::{ParagraphParams, DEFAULT_FONT_NAME, ZERO_WIDTH_WHITESPACE};
use crate::element::text::{FONT_COLLECTION, FONT_MGR};
use crate::element::text::simple_text_paragraph::{SimpleTextParagraph, TextBlock};
use crate::element::text::text_paragraph::TextParams;
use crate::some_or_continue;
use crate::string::StringUtils;
use crate::typeface::get_font_mgr;

pub struct SimpleParagraphBuilder {
    styles: Vec<TextStyle>,
    text_blocks: Vec<TextBlock>,
    font_collection: FontCollection,
}

impl SimpleParagraphBuilder {
    pub fn new(style: &ParagraphParams) -> Self {
        let mut font_collection = FONT_COLLECTION.with(|f| f.clone());
        FONT_MGR.with(|fm| {
            font_collection.set_default_font_manager(Some(fm.clone()), None);
        });
        font_collection.set_dynamic_font_manager(get_font_mgr());
        let mut font_families = style.font_families.clone();
        if font_families.is_empty() {
            for f in DEFAULT_FONT_NAME.split(",") {
                font_families.push(f.to_string());
            }
        }

        let mut text_style = TextStyle::new();
        text_style.set_color(style.color);
        text_style.set_font_size(style.font_size);
        text_style.set_font_families(&font_families);
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
        let text = text.into();
        let style = self.styles.last().unwrap().clone();
        let font_families = style.font_families();
        let font_families_names: Vec<&str> = font_families.iter().collect();
        let mut text_blocks = self.resolve_font(&font_families_names, &style, &text);
        // println!("text_blocks: {:?} {:?}", &text, &text_blocks);
        self.text_blocks.append(&mut text_blocks);
    }

    fn resolve_font(&mut self, font_families_names: &Vec<&str>, style: &TextStyle, text: &str) -> Vec<TextBlock> {
        let mut typefaces = self.font_collection.find_typefaces(&font_families_names, style.font_style());
        let mut chars = text.chars().collect::<Vec<_>>();
        let mut resolved_typefaces: Vec<i32> = Vec::new();
        resolved_typefaces.resize(chars.len(), -1);
        for tf_idx in 0..typefaces.len() {
            let tf = &typefaces[tf_idx];
            let font = Font::from_typeface(tf, style.font_size());
            let glyphs_ids = font.str_to_glyphs_vec(text);
            for i in 0..glyphs_ids.len() {
                if resolved_typefaces[i] == -1 && glyphs_ids[i] != 0 {
                    resolved_typefaces[i] = tf_idx as i32;
                }
            }
        }
        for i in 0..chars.len() {
            if resolved_typefaces[i] != -1 {
                continue;
            }
            //TODO fix locale
            let tf = some_or_continue!(self.font_collection.default_fallback_char(chars[i] as Unichar, style.font_style(), "en"));
            for tf_idx in 0..typefaces.len() {
                if typefaces[tf_idx].unique_id() == tf.unique_id() {
                    resolved_typefaces[i] = tf_idx as i32;
                    break;
                }
            }
            if resolved_typefaces[i] == -1 {
                typefaces.push(tf);
                resolved_typefaces[i] = (typefaces.len() - 1) as i32;
            }
        }
        for i in 0..resolved_typefaces.len() {
            if resolved_typefaces[i] == -1 {
                resolved_typefaces[i] = 0;
            }
        }
        let mut text_blocks = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            let begin = i;
            let tf_idx = resolved_typefaces[i] as usize;
            let tf = &typefaces[tf_idx];
            let font = Font::from_typeface(tf, style.font_size());
            i += 1;
            while i < chars.len() && resolved_typefaces[i] as usize == tf_idx {
                i += 1;
            }
            let end = i;
            text_blocks.push(TextBlock {
                text: text.substring(begin, end - begin).to_string(),
                style: style.clone(),
                font,
            });
        }
        text_blocks
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
        mask_char: None,
    };
    let mut pb = SimpleParagraphBuilder::new(&params);
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