use crate as deft;
use std::collections::HashMap;
use deft_macros::mrc_object;
use font_kit::family_name::FamilyName;
use crate::font::Font;
use font_kit::handle::Handle;
use font_kit::source::{Source, SystemSource};
use skia_safe::FontStyle;
use skia_safe::wrapper::NativeTransmutableWrapper;
use swash::Weight;
use crate::element::paragraph::{DEFAULT_FALLBACK_FONTS};
use crate::text::TextStyle;

#[mrc_object]
pub struct FontManager {
    source: SystemSource,
    cache: HashMap<FontCacheKey, Option<Font>>,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct FontCacheKey {
    family_name: String,
    weight: u16,
}

impl FontManager {
    pub fn new() -> FontManager {
        let source = SystemSource::new();
        // source.all_families().unwrap().iter().for_each(|family| println!("{}", family));
        FontManagerData {
            source,
            cache: HashMap::new(),
        }.to_ref()
    }

    pub fn match_best(&self, family_names: &[impl AsRef<str>], style: &FontStyle) -> Vec<Font> {
        let mut result = Vec::new();
        let mut me = self.clone();
        let mut family_names = family_names.iter().map(|s| s.as_ref()).collect::<Vec<_>>();
        for name in DEFAULT_FALLBACK_FONTS.split(",") {
            family_names.push(name);
        }
        for name in family_names {
            if let Some(font) = self.get_by_family_name(name.as_ref(), &style) {
                result.push(font.clone());
            }
        }
        result
    }

    fn get_by_family_name(&self, name: &str, style: &FontStyle) -> Option<Font> {
        let weight = style.weight().unwrap() as u16;
        let cache_key = FontCacheKey {
            family_name: name.to_string(),
            weight,
        };
        let mut me = self.clone();
        me.cache.entry(cache_key).or_insert_with(move || {
            let family_name = Self::str_to_family_name(name);
            let fh = self.source.select_family_by_generic_name(&family_name).ok()?;
            for h in fh.fonts() {
                if let Some(font) = Self::load_font(h, name) {
                    let attrs = font.attributes();
                    if attrs.weight() == Weight(weight) {
                        return Some(font)
                    }
                }
            }
            None
        }).clone()
    }

    fn load_font(h: &Handle, name: &str) -> Option<Font> {
        match h {
            Handle::Path { path, font_index } => {
                if let Some(font) = Font::from_file(path, *font_index as usize, name.to_string()) {
                    return Some(font);
                }
            }
            Handle::Memory { bytes, font_index } => {
                if let Some(font) = Font::from_bytes(bytes.to_vec(), *font_index as usize, name.to_string()) {
                    return Some(font);
                }
            }
        }
        None
    }

    fn str_to_family_name(family_name: &str) -> FamilyName {
        match family_name {
            "serif" => FamilyName::Serif,
            "sans-serif" => FamilyName::SansSerif,
            "monospace" => FamilyName::Monospace,
            "cursive" => FamilyName::Cursive,
            "fantasy" => FamilyName::Fantasy,
            _ => FamilyName::Title(family_name.to_string()),
        }
    }

}
