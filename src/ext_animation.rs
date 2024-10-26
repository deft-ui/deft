use std::str::FromStr;
use quick_js::JsValue;
use lento_js::{JsError};
use lento_core::animation::{AnimationDef};
use lento_core::animation::ANIMATIONS;
use crate::style::{parse_style_obj};


#[lento_macros::js_func]
pub fn animation_create(name: String, key_frames: JsValue) -> Result<(), JsError> {
    let mut ad = AnimationDef::new();
    if let Some(ps) = key_frames.get_properties() {
        for (k, v) in ps {
            let p = f32::from_str(&k)?;
            let styles = parse_style_obj(v);
            // let styles = create_animation_style(styles);
            ad = ad.key_frame(p, styles);
        }
        let ani = ad.build();
        ANIMATIONS.with_borrow_mut(|m| {
            m.insert(name, ani);
        });
        Ok(())
    } else {
        Err(JsError::from_str("invalid argument"))
    }
}