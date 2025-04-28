pub mod actor;
pub mod css_actor;

use crate as deft;
use crate::mrc::Mrc;
use crate::style::{ResolvedStyleProp, ScaleParams, StyleProp, StylePropKey, StylePropVal, StyleTransform, StyleTransformOp, TranslateLength, TranslateParams};
use crate::timer::{set_timeout, set_timeout_nanos, TimerHandle};
use crate::{js_value};
use anyhow::{anyhow, Error};
use ordered_float::OrderedFloat;
use quick_js::JsValue;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::ops::Bound::{Excluded, Included};
use std::time::SystemTime;
use log::debug;
use tokio::time::Instant;
use yoga::StyleUnit;
use crate::animation::actor::{AnimationAction, AnimationActor};
use crate::base::Callback;
use crate::element::Element;
use crate::window::WindowWeak;

macro_rules! interpolate_values {
    ($prev: expr, $next: expr, $percent: expr; $($ty: ident => $handler: ident,)* ) => {
        $(
            if let StyleProp::$ty(pre) = $prev {
                if let StyleProp::$ty(next) = $next {
                    if let StylePropVal::Custom(p) = pre {
                        if let StylePropVal::Custom(n) = next {
                            if let Some(v) = $handler(p, n, $percent) {
                                return Some(StyleProp::$ty(StylePropVal::Custom(v)));
                            }
                        }
                    }
                }
                return None
            }
        )*
    };
}

#[macro_export]
macro_rules! match_both {
    ($def: path, $expr1: expr, $expr2: expr) => {
        if let $def(e1) = $expr1 {
            if let $def(e2) = $expr2 {
                Some((e1, e2))
            } else {
                None
            }
        } else {
            None
        }
    };
}


thread_local! {
    pub static  ANIMATIONS: RefCell<HashMap<String, Animation>> = RefCell::new(HashMap::new());
}


fn interpolate_f32(prev: &f32, next: &f32, position: f32) -> Option<f32> {
    let delta = (next - prev) * position;
    Some(prev + delta)
}

fn interpolate_style_unit(prev: &StyleUnit, next: &StyleUnit, position: f32) -> Option<StyleUnit> {
    //TODO use compute value?
    if let StyleUnit::Point(p) = prev {
        if let StyleUnit::Point(n) = next {
            let v = interpolate_f32(&p.0, &n.0, position).unwrap_or(0.0);
            return Some(StyleUnit::Point(OrderedFloat(v)));
        }
    } else if let StyleUnit::Percent(p) = prev {
        if let StyleUnit::Percent(n) = next {
            let v = interpolate_f32(&p.0, &n.0, position).unwrap_or(0.0);
            return Some(StyleUnit::Percent(OrderedFloat(v)));
        }
    }
    return None;
}

fn interpolate_transform(prev: &StyleTransform, next: &StyleTransform, position: f32) -> Option<StyleTransform> {
    let p_list = &prev.op_list;
    let n_list = &next.op_list;
    if p_list.len() != n_list.len() {
        return None
    }
    let mut op_list = Vec::new();
    for i in 0..p_list.len() {
        let p = unsafe { p_list.get_unchecked(i) };
        let n = unsafe { n_list.get_unchecked(i) };
        if let Some(v) = interpolate_transform_op(p, n, position) {
            op_list.push(v);
        } else {
            debug!("Unsupported animation value");
        }
        //TODO support other transform
    }
    Some(StyleTransform {
        op_list
    })
}

fn interpolate_translate_len(p: &TranslateLength, n: &TranslateLength, position: f32) -> Option<TranslateLength> {
    if let Some((px, nx)) = match_both!(TranslateLength::Point, p, n) {
        let x = interpolate_f32(&px, &nx, position)?;
        Some(TranslateLength::Point(x))
    } else if let Some((px, nx)) = match_both!(TranslateLength::Percent, p, n) {
        let x = interpolate_f32(&px, &nx, position)?;
        Some(TranslateLength::Percent(x))
    } else {
        None
    }
}

fn interpolate_transform_op(prev: &StyleTransformOp, next: &StyleTransformOp, position: f32) -> Option<StyleTransformOp> {
    if let Some((p_deg, n_deg)) = match_both!(StyleTransformOp::Rotate, prev, next) {
        let deg = interpolate_f32(p_deg, n_deg, position)?;
        Some(StyleTransformOp::Rotate(deg))
    } else if let Some((pv, nv)) = match_both!(StyleTransformOp::Translate, prev, next) {
        let (mut px, mut nx) = (pv.0.clone(), nv.0.clone());
        px.adapt_zero(&mut nx);
        let (mut py, mut ny) = (pv.1.clone(), nv.1.clone());
        py.adapt_zero(&mut ny);
        let x = interpolate_translate_len(&px, &nx, position)?;
        let y = interpolate_translate_len(&py, &ny, position)?;
        Some(StyleTransformOp::Translate(TranslateParams(x, y)))
    } else if let Some((pv, nv)) = match_both!(StyleTransformOp::Scale, prev, next) {
        let (px, nx) = (pv.0, nv.0);
        let (py, ny) = (pv.1, nv.1);
        let x = interpolate_f32(&px, &nx, position)?;
        let y = interpolate_f32(&py, &ny, position)?;
        Some(StyleTransformOp::Scale(ScaleParams(x, y)))
    } else {
        None
    }
}

fn interpolate(pre_position: f32, pre_value: StyleProp, next_position: f32, next_value: StyleProp, current_position: f32) -> Option<StyleProp> {
    let duration = next_position - pre_position;
    let percent = (current_position - pre_position) / duration;
    interpolate_values!(
        &pre_value, &next_value, percent;
        // TODO fix animation
        // Width => interpolate_style_unit,
        // TODO fix animation
        // Height => interpolate_style_unit,

        /*
        PaddingTop => interpolate_style_unit,
        PaddingRight => interpolate_style_unit,
        PaddingBottom => interpolate_style_unit,
        PaddingLeft => interpolate_style_unit,

        MarginTop => interpolate_style_unit,
        MarginRight => interpolate_style_unit,
        MarginBottom => interpolate_style_unit,
        MarginLeft => interpolate_style_unit,
         */

        /*
        BorderTopLeftRadius => interpolate_absolute_len,
        BorderTopRightRadius => interpolate_absolute_len,
        BorderBottomRightRadius => interpolate_absolute_len,
        BorderBottomLeftRadius => interpolate_absolute_len,

        Top => interpolate_style_unit,
        Right => interpolate_style_unit,
        Bottom => interpolate_style_unit,
        Left => interpolate_style_unit,


        RowGap => interpolate_absolute_len,
        ColumnGap => interpolate_absolute_len,
        */

        Transform => interpolate_transform,
    );
    None
}


pub struct AnimationDef {
    key_frames: BTreeMap<OrderedFloat<f32>, Vec<StyleProp>>,
}

#[derive(Clone)]
pub struct Animation {
    styles: HashMap<StylePropKey, BTreeMap<OrderedFloat<f32>, StyleProp>>,
}

pub trait FrameController {
    fn request_next_frame(&mut self, callback: Box<dyn FnOnce()>);
}

pub struct AnimationState {
    actor: Box<dyn AnimationActor>,
    start_time: Instant,
    duration: f32,
    iteration_count: f32,
    frame_controller: Box<dyn FrameController>,
    stopped: bool,
}

pub struct AnimationInstance {
    state: Mrc<AnimationState>,
}


impl AnimationDef {
    pub fn new() -> Self {
        Self { key_frames: BTreeMap::new() }
    }

    pub fn key_frame(mut self, position: f32, styles: Vec<StyleProp>) -> Self {
        self.key_frames.insert(OrderedFloat::from(position), styles);
        self
    }

    pub fn build(mut self) -> Animation {
        let mut styles = HashMap::new();
        for (p, key_styles) in &self.key_frames {
            for s in key_styles {
                let map = styles.entry(s.key()).or_insert_with(|| BTreeMap::new());
                map.insert(p.clone(), s.clone());
            }
        }
        Animation {
            styles
        }
    }
}

impl Animation {
    pub fn preprocess(&self) -> Animation {
        let mut styles = HashMap::new();
        for (p, m) in self.styles.clone() {
            let mut new_style = BTreeMap::new();
            for (k, v) in m {
                new_style.insert(k, Self::preprocess_style(v));
            }
            styles.insert(p, new_style);
        }
        Animation { styles }
    }

    fn preprocess_style(style: StyleProp) -> StyleProp {
        if let StyleProp::Transform(tf) = &style {
            if let StylePropVal::Custom(tf) = tf {
                return StyleProp::Transform(StylePropVal::Custom(tf.preprocess()));
            }
        }
        style
    }

    pub fn get_frame(&self, position: f32) -> Vec<StyleProp> {
        //TODO support loop
        if position > 1.0 {
            return Vec::new();
        }
        let position = f32::clamp(position, 0.0, 1.0);
        let mut result = Vec::new();
        let p = OrderedFloat(position);
        for (_k, v) in &self.styles {
            let begin = OrderedFloat::from(0.0);
            let end = OrderedFloat::from(1.0);
            let prev = v.range((Included(begin), Included(p))).last();
            let next = v.range((Excluded(p), Included(end))).next();
            if let Some((prev_position, prev_value)) = prev {
                if let Some((next_position, next_value)) = next {
                    if let Some(value) = interpolate(prev_position.0, prev_value.clone(), next_position.0, next_value.clone(), p.0) {
                        result.push(value);
                    }
                } else {
                    result.push(prev_value.clone());
                }
            }
        }
        result
    }
}

impl AnimationInstance {
    pub fn new<A: AnimationActor + 'static>(actor: A, duration: f32, iteration_count: f32, frame_controller: Box<dyn FrameController>) -> Self {
        let state = AnimationState {
            actor: Box::new(actor),
            start_time: Instant::now(),
            duration,
            iteration_count,
            frame_controller,
            stopped: false,
        };
        Self {
            state: Mrc::new(state),
        }
    }

    pub fn run(&mut self) {
        let mut state = self.state.clone();
        self.state.frame_controller.request_next_frame(Box::new(move || {
            // debug!("animation started:{}", t);
            state.start_time = Instant::now();
            Self::render_frame(state);
        }));
    }

    fn stop(&mut self) {
        // debug!("stopped");
        self.state.stopped = true;
    }

    fn render_frame(mut state: Mrc<AnimationState>) {
        let elapsed = state.start_time.elapsed().as_nanos() as f32;
        let position = elapsed / state.duration;
        let mut is_ended = false;
        if position >= state.iteration_count || state.stopped {
            state.actor.stop();
            is_ended = true;
        } else {
            state.actor.apply_animation(position - position as usize as f32, &mut is_ended);
        };
        if !is_ended {
            let s = state.clone();
            state.frame_controller.request_next_frame(Box::new(|| {
                Self::render_frame(s);
            }))
        } else {
            //TODO notify ended?
        }
    }
}

impl Drop for AnimationInstance {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct WindowAnimationController {
    frame: WindowWeak,
}

impl WindowAnimationController {
    pub fn new(frame: WindowWeak) -> Self {
        Self { frame }
    }
}

impl FrameController for WindowAnimationController {
    fn request_next_frame(&mut self, callback: Box<dyn FnOnce()>) {
        if let Ok(mut frame) = self.frame.upgrade() {
            frame.request_next_frame_callback(Callback::from_box(callback));
        }
    }
}

pub struct SimpleFrameController {
    timer: SystemTime,
    prev_frame_time: u128,
    timer_handle: Option<TimerHandle>,
}

impl SimpleFrameController {
    pub fn new() -> Self {
        Self {
            timer: SystemTime::now(),
            prev_frame_time: 0,
            timer_handle: None,
        }
    }
}

impl FrameController for SimpleFrameController {
    fn request_next_frame(&mut self, callback: Box<dyn FnOnce()>) {
        let now = self.timer.elapsed().unwrap().as_nanos();
        let next_frame_time = self.prev_frame_time + 16666667;
        self.prev_frame_time = next_frame_time;
        if next_frame_time > now {
            let sleep_time = (next_frame_time - now) as u64;
            self.timer_handle = Some(set_timeout_nanos(move || {
                callback();
            }, sleep_time));
        } else {
            self.timer_handle = Some(set_timeout(move || {
                callback();
            }, 0));
        }
    }
}