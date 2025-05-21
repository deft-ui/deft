use skia_safe::{FontMgr, Typeface};
use std::collections::HashMap;

pub struct TypefaceMgr {
    system_font_mgr: FontMgr,
    typefaces: HashMap<String, Typeface>,
}

impl TypefaceMgr {
    pub fn new() -> Self {
        Self {
            system_font_mgr: FontMgr::new(),
            typefaces: HashMap::new(),
        }
    }

    pub fn get_typeface(&mut self, family_name: &str) -> Typeface {
        if let Some(tf) = self.typefaces.get(family_name) {
            return tf.clone();
        }
        let tf = Self::search_typeface(&self.system_font_mgr, family_name);
        self.typefaces.insert(family_name.to_string(), tf.clone());
        tf
    }

    fn search_typeface(system_font_mgr: &FontMgr, family_name: &str) -> Typeface {
        for i in 0..system_font_mgr.count_families() {
            let name = system_font_mgr.family_name(i);
            if name != family_name {
                continue;
            }
            let mut style_set = system_font_mgr.new_style_set(i);
            for style_index in 0..style_set.count() {
                // let (_, style_name) = style_set.style(style_index);
                let face = style_set.new_typeface(style_index);
                if let Some(face) = face {
                    if face.family_name() == family_name {
                        return face;
                    }
                }
            }
        }
        system_font_mgr
            .match_family(family_name)
            .new_typeface(0)
            .unwrap()
    }
}
