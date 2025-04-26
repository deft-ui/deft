use crate as deft;
use std::collections::HashMap;
use deft_macros::mrc_object;
use font_kit::family_name::FamilyName;
use crate::font::Font;
use font_kit::handle::Handle;
use font_kit::properties::{Properties, Weight};
use font_kit::source::{Source, SystemSource};
use skia_safe::FontStyle;
use skia_safe::wrapper::NativeTransmutableWrapper;
use crate::text::TextStyle;

#[mrc_object]
pub struct FontManager {
    source: SystemSource,
    cache: HashMap<FontCacheKey, Option<Font>>,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct FontCacheKey {
    family_name: String,
    weight: i32,
}

impl FontManager {
    pub fn new() -> FontManager {
        let source = SystemSource::new();
        FontManagerData {
            source,
            cache: HashMap::new(),
        }.to_ref()
    }

    pub fn match_best(&self, family_names: &[impl AsRef<str>], style: &FontStyle) -> Vec<Font> {
        let mut result = Vec::new();
        let mut me = self.clone();
        for name in family_names {
            if let Some(font) = self.get_by_family_name(name.as_ref(), &style) {
                result.push(font.clone());
            }
        }
        result
    }

    fn get_by_family_name(&self, name: &str, style: &FontStyle) -> Option<Font> {
        let weight = style.weight().unwrap();
        let cache_key = FontCacheKey {
            family_name: name.to_string(),
            weight,
        };
        let mut me = self.clone();
        me.cache.entry(cache_key).or_insert_with(move || {
            let mut properties = Properties::new();
            properties.weight(Weight(weight as f32));
            let family_name = FamilyName::Title(name.to_string());
            if let Ok(h) = self.source.select_best_match(&[family_name], &properties) {
                match h {
                    Handle::Path { path, font_index } => {
                        if let Some(font) = Font::from_file(path, font_index as usize) {
                            return Some(font);
                        }
                    }
                    Handle::Memory { bytes, font_index } => {
                        if let Some(font) = Font::from_bytes(bytes.to_vec(), font_index as usize) {
                            return Some(font);
                        }
                    }
                }
            }
            None
        }).clone()
    }

}
