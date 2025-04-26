use crate as deft;
use crate::element::paragraph::parse_optional_weight;
use crate::js_deserialize;
use deft_macros::js_func;
use serde::{Deserialize, Serialize};
use skia_safe::font_style::{Weight, Width};
use skia_safe::{FontMgr, FontStyle, Typeface};
use std::cell::RefCell;
use std::collections::HashMap;
use skia_safe::font_style::Slant::Upright;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TypefaceSource {
    family: String,
    weight: Option<String>,
}
js_deserialize!(TypefaceSource);

#[js_func]
pub fn typeface_create(name: String, source: TypefaceSource) -> bool {
    //TODO fix
    false
    /*
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
     */
}
