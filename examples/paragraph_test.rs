use measure_time::print_time;
use skia_safe::textlayout::{FontCollection, TypefaceFontProvider};
use skia_safe::wrapper::PointerWrapper;
use skia_safe::{FontMgr, FontStyle, Typeface};
use std::collections::HashMap;
use deft::element::paragraph::typeface_mgr::TypefaceMgr;

pub fn main() {
    print_time!("font");
    let family_name = "Tlwg Mono";
    let mut font_mgr = TypefaceMgr::new();
    let ft = font_mgr.get_typeface(family_name);
    println!("Family name: {}", ft.family_name());
}
