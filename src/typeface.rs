use crate as deft;
use crate::element::paragraph::parse_optional_weight;
use crate::element::text::FONT_MGR;
use crate::js_deserialize;
use deft_macros::js_func;
use serde::{Deserialize, Serialize};
use skia_safe::font_style::{Weight, Width};
use skia_safe::textlayout::TypefaceFontProvider;
use skia_safe::{FontMgr, FontStyle, Typeface};
use std::cell::RefCell;
use std::collections::HashMap;
use skia_safe::font_style::Slant::Upright;

thread_local! {
    pub static TYPEFACES: RefCell<TypefaceFontProvider> = RefCell::new(TypefaceFontProvider::new());
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TypefaceSource {
    family: String,
    weight: Option<String>,
}
js_deserialize!(TypefaceSource);

#[js_func]
pub fn typeface_create(name: String, source: TypefaceSource) -> bool {
    TYPEFACES.with_borrow_mut(|m| {
        let fm = FONT_MGR.with(|fm| fm.clone());
        let weight = parse_optional_weight(source.weight.as_ref()).unwrap_or(Weight::NORMAL);
        let mut font_style = FontStyle::new(weight, Width::NORMAL, Upright);
        if let Some(tf) = fm.match_family_style(&source.family, font_style) {
            m.register_typeface(tf, Some(name.as_str()));
            true
        } else {
            false
        }
    })
}

pub fn get_font_mgr() -> FontMgr {
    let provider = TYPEFACES.with_borrow(|m| m.clone());
    provider.into()
}
