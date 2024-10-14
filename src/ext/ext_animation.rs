use std::str::FromStr;
use anyhow::{anyhow, Error};
use quick_js::JsValue;
use serde::{Deserialize, Serialize};
use crate::animation::{AnimationDef, AnimationInstance, SimpleFrameController};
use crate::define_ref_and_resource;
use crate::style::{parse_style_obj};
define_ref_and_resource!(AnimationResource, AnimationInstance);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationOptions {
    duration: f32,
    iteration_count: Option<f32>,
}

pub fn animation_create(key_frames: JsValue, options: AnimationOptions) -> Result<AnimationResource, Error> {
    let mut ad = AnimationDef::new();
    if let Some(ps) = key_frames.get_properties() {
        for (k, v) in ps {
            let p = f32::from_str(&k)?;
            let styles = parse_style_obj(v);
            // let styles = create_animation_style(styles);
            ad = ad.key_frame(p, styles);
        }
        let ani = ad.build();
        let frame_controller = SimpleFrameController::new();
        let duration = options.duration * 1000000.0;
        let iteration_count = options.iteration_count.unwrap_or(1.0);
        let ani_instance = AnimationInstance::new(ani, duration, iteration_count, Box::new(frame_controller));
        Ok(AnimationResource::new(ani_instance))
    } else {
        Err(anyhow!("invalid argument"))
    }
}

