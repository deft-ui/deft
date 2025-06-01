use crate as deft;
use crate::animation::AnimationDef;
use crate::animation::ANIMATIONS;
use crate::js::js_binding::JsError;
use crate::style::parse_style_obj;
use std::str::FromStr;

#[deft_macros::js_func]
pub fn animation_create(name: String, key_frames: deft::JsValue) -> Result<(), JsError> {
    let mut ad = AnimationDef::new();
    if let Some(ps) = key_frames.get_properties() {
        for (k, v) in ps {
            let p = f32::from_str(&k)?;
            let mut styles = Vec::new();
            let (parsed_styles, _) = parse_style_obj(v);
            for item in parsed_styles {
                styles.append(&mut item.fixed());
            }
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
