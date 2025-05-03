use std::collections::HashMap;
use font_kit::loader::Loader;
use log::warn;
use measure_time::print_time;
use skia_safe::{FontStyle, Paint, Typeface, Unichar};
use skia_safe::wrapper::ValueWrapper;
use crate::element::font_manager::FontManager;
use crate::element::paragraph::{ParagraphParams, DEFAULT_FONT_NAME, ZERO_WIDTH_WHITESPACE};
use crate::element::text::simple_text_paragraph::{chars_to_glyphs_vec, str_to_glyphs_vec, SimpleTextParagraph, TextBlock};
use crate::element::text::text_paragraph::TextParams;
use crate::font::Font;
use crate::some_or_continue;
use crate::string::StringUtils;
use crate::text::TextStyle;

thread_local! {
    pub static FONT_MANAGER: FontManager = FontManager::new();
}

pub struct SimpleParagraphBuilder {
    styles: Vec<TextStyle>,
    text_blocks: Vec<TextBlock>,
    font_manager: FontManager,
    fallback_cache: HashMap<char, Option<Font>>,
    line_height: Option<f32>,
}

impl SimpleParagraphBuilder {
    pub fn new(style: &ParagraphParams) -> Self {
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
            line_height: style.line_height,
            styles: vec![text_style],
            text_blocks: Vec::new(),
            font_manager: FONT_MANAGER.with(|fm| fm.clone()),
            fallback_cache: HashMap::new(),
        }
    }

    pub fn push_style(&mut self, style: &TextStyle) {
        self.styles.push(style.clone());
    }

    pub fn add_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        let style = self.styles.last().unwrap().clone();
        let font_families_names: Vec<&str> = style.font_families();
        let mut text_blocks = self.resolve_font(&font_families_names, &style, &text);
        // debug!("text_blocks: {:?} {:?}", &text, &text_blocks);
        self.text_blocks.append(&mut text_blocks);
    }

    fn resolve_font(&mut self, font_families_names: &Vec<&str>, style: &TextStyle, text: &str) -> Vec<TextBlock> {
        let mut font_families_names = font_families_names.clone();
        if font_families_names.is_empty() {
            for f in DEFAULT_FONT_NAME.split(",") {
                font_families_names.push(f);
            }
        }
        let mut fonts = self.font_manager.match_best(&font_families_names, style.font_style());
        if fonts.is_empty() {
            warn!("No matching font found for {:?}", &font_families_names);
            return Vec::new();
        }
        let mut chars = text.chars().collect::<Vec<_>>();
        let (mut resolved_typefaces, unresolved_count) = Self::do_resolve_font(&chars, &fonts);
        for i in 0..chars.len() {
            if resolved_typefaces[i] != -1 {
                continue;
            }
            let ch = chars[i];
            if ch == '\n' {
                continue;
            }
            let cached_tf = match self.fallback_cache.get(&ch) {
                Some(tf) => tf,
                None => {
                    //TODO fix locale
                    // let tf = self.font_collection.default_fallback_char(chars[i] as Unichar, style.font_style(), "en");
                    let tf = None;
                    self.fallback_cache.insert(ch, tf);
                    self.fallback_cache.get(&ch).unwrap()
                }
            };

            let tf = some_or_continue!(cached_tf);
            //TODO fix equal check
            /*
            for tf_idx in 0..fonts.len() {
                if fonts[tf_idx] == tf {
                    resolved_typefaces[i] = tf_idx as i32;
                    break;
                }
            }
             */
            if resolved_typefaces[i] == -1 {
                fonts.push(tf.clone());
                resolved_typefaces[i] = (fonts.len() - 1) as i32;
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
            let font = fonts[tf_idx].clone();
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
        SimpleTextParagraph::new(self.text_blocks, self.line_height)
    }

    fn do_resolve_font(chars: &Vec<char>, fonts: &Vec<Font>) -> (Vec<i32>, usize) {
        let mut resolved_typefaces: Vec<i32> = Vec::new();
        resolved_typefaces.resize(chars.len(), -1);
        let mut unresolved_char_count = resolved_typefaces.len();
        for tf_idx in 0..fonts.len() {
            let glyphs_ids = chars_to_glyphs_vec(&fonts[tf_idx], chars);
            for i in 0..glyphs_ids.len() {
                if resolved_typefaces[i] == -1 && glyphs_ids[i] != 0 {
                    resolved_typefaces[i] = tf_idx as i32;
                    unresolved_char_count -= 1;
                }
            }
            if unresolved_char_count == 0 {
                break;
            }
        }
        (resolved_typefaces, unresolved_char_count)
    }

}

#[test]
fn test_performance() {
    for _ in 0..10 {
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
        let str = include_str!("../../../Cargo.lock");
        {
            print_time!("render");
            pb.add_text(str.to_string());
        }
        let b = pb.build();
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
    let layout = paragraph.layout.as_ref().unwrap();
    for offset in 0..3 {
        let bounds = layout.get_char_bounds(offset);
        assert!(bounds.is_some());
    }
    let bounds0 = layout.get_char_bounds(0).unwrap();
    let bounds1 = layout.get_char_bounds(1).unwrap();
    assert!(bounds1.left >= bounds0.right);
    assert!(paragraph.max_intrinsic_width() > 0.0);
}