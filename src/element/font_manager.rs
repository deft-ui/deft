use crate as deft;
use crate::element::paragraph::DEFAULT_FALLBACK_FONTS;
use crate::font::Font;
use crate::style::FontStyle;
use deft_macros::mrc_object;
use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::source::{Source, SystemSource};
use skia_safe::font_style::Slant;
use skia_safe::wrapper::NativeTransmutableWrapper;
use std::collections::HashMap;
use swash::{ObliqueAngle, Style, Weight};

#[mrc_object]
pub struct FontManager {
    source: SystemSource,
    cache: HashMap<FontCacheKey, Option<Font>>,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct FontCacheKey {
    family_name: String,
    weight: u16,
    style: FontStyle,
}

impl FontManager {
    pub fn new() -> FontManager {
        let source = SystemSource::new();
        // source.all_families().unwrap().iter().for_each(|family| println!("{}", family));
        FontManagerData {
            source,
            cache: HashMap::new(),
        }
        .to_ref()
    }

    pub fn match_best(
        &self,
        family_names: &[impl AsRef<str>],
        style: &skia_safe::FontStyle,
    ) -> Vec<Font> {
        let mut result = Vec::new();
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

    fn get_by_family_name(
        &self,
        name: &str,
        expected_font_style: &skia_safe::FontStyle,
    ) -> Option<Font> {
        let weight = expected_font_style.weight().unwrap() as u16;
        let expected_style = match expected_font_style.slant() {
            Slant::Upright => FontStyle::Normal,
            Slant::Italic => FontStyle::Italic,
            //TODO support angle
            Slant::Oblique => FontStyle::Oblique,
        };
        let cache_key = FontCacheKey {
            family_name: name.to_string(),
            weight,
            style: expected_style,
        };
        let mut me = self.clone();
        me.cache
            .entry(cache_key)
            .or_insert_with(move || {
                let family_name = Self::str_to_family_name(name);
                let fh = self
                    .source
                    .select_family_by_generic_name(&family_name)
                    .ok()?;
                let expected_style = match expected_font_style.slant() {
                    Slant::Upright => Style::Normal,
                    Slant::Italic => Style::Italic,
                    //TODO support angle
                    Slant::Oblique => Style::Oblique(ObliqueAngle::default()),
                };
                let mut fonts = Vec::new();
                for h in fh.fonts() {
                    if let Some(font) = Self::load_font(h, name) {
                        fonts.push(font);
                    }
                }
                let mut result = Self::filter_fonts_by_style(&fonts, expected_style);
                if result.is_empty() && expected_style != Style::Normal {
                    result = Self::filter_fonts_by_style(&fonts, Style::Normal);
                }
                Self::select_best_font_by_weight(result, weight)
                    .map(|f| f.synthesize(Weight(weight), expected_style))
            })
            .clone()
    }

    fn filter_fonts_by_style(fonts: &Vec<Font>, style: Style) -> Vec<&Font> {
        let mut result = Vec::with_capacity(fonts.len());
        for f in fonts {
            if f.attributes().style() == style {
                result.push(f);
            }
        }
        result
    }

    fn select_best_font_by_weight(fonts: Vec<&Font>, expected_weight: u16) -> Option<Font> {
        let mut best_font = None;
        let mut best_weight_diff = i32::MAX;
        for f in fonts {
            let attrs = f.attributes();
            let weight_diff = (attrs.weight().0 as i32 - expected_weight as i32).abs();
            if weight_diff < best_weight_diff {
                best_font = Some(f);
                best_weight_diff = weight_diff;
            }
        }
        best_font.cloned()
    }

    fn load_font(h: &Handle, name: &str) -> Option<Font> {
        match h {
            Handle::Path { path, font_index } => {
                if let Some(font) = Font::from_file(path, *font_index as usize, name.to_string()) {
                    return Some(font);
                }
            }
            Handle::Memory { bytes, font_index } => {
                if let Some(font) =
                    Font::from_bytes(bytes.to_vec(), *font_index as usize, name.to_string())
                {
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

#[test]
fn test_font_weight() {
    let fm = FontManager::new();
    let font_style = skia_safe::FontStyle::bold();
    let fonts = fm.match_best(&["fangsong"], &font_style);
    println!("fonts count: {}", fonts.len());
    for font in fonts {
        println!("{}", font.attributes().weight().0);
    }
}
