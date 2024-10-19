use std::cell::RefCell;
use std::collections::HashMap;
use std::str::FromStr;
use anyhow::{anyhow, Error};
use quick_js::JsValue;
use serde::{Deserialize, Serialize};
use crate::animation::{Animation, AnimationDef, AnimationInstance, SimpleFrameController};
use crate::define_ref_and_resource;
use crate::style::{parse_style_obj};
define_ref_and_resource!(AnimationResource, AnimationInstance);

thread_local! {
    pub static  ANIMATIONS: RefCell<HashMap<String, Animation>> = RefCell::new(HashMap::new());
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationOptions {
    duration: f32,
    iteration_count: Option<f32>,
}

pub fn animation_create(name: String, key_frames: JsValue) -> Result<(), Error> {
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
        Err(anyhow!("invalid argument"))
    }
}

