use std::collections::{BTreeMap, HashMap};
use std::ops::Bound::{Excluded, Included};
use std::time::SystemTime;
use ordered_float::OrderedFloat;
use yoga::StyleUnit;
use crate::mrc::Mrc;
use crate::style::{StyleProp, StylePropVal, StyleTransform, StyleTransformOp};
use crate::timer::{set_timeout, set_timeout_nanos, TimerHandle};
use std::cell::RefCell;
use anyhow::{anyhow, Error};
use quick_js::JsValue;
use crate::define_ref_and_resource;

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

define_ref_and_resource!(AnimationResource, AnimationInstance);

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
        if let Some((p_deg, n_deg)) = match_both!(StyleTransformOp::Rotate, p, n) {
            let deg = interpolate_f32(p_deg, n_deg, position)?;
            op_list.push(StyleTransformOp::Rotate(deg));
        }
    }
    Some(StyleTransform {
        op_list
    })
}

fn interpolate(pre_position: f32, pre_value: StyleProp, next_position: f32, next_value: StyleProp, current_position: f32) -> Option<StyleProp> {
    let duration = next_position - pre_position;
    let percent = (current_position - pre_position) / duration;
    interpolate_values!(
        &pre_value, &next_value, percent;
        Width => interpolate_style_unit,
        Height => interpolate_style_unit,

        PaddingTop => interpolate_style_unit,
        PaddingRight => interpolate_style_unit,
        PaddingBottom => interpolate_style_unit,
        PaddingLeft => interpolate_style_unit,

        MarginTop => interpolate_style_unit,
        MarginRight => interpolate_style_unit,
        MarginBottom => interpolate_style_unit,
        MarginLeft => interpolate_style_unit,

        BorderTopLeftRadius => interpolate_f32,
        BorderTopRightRadius => interpolate_f32,
        BorderBottomRightRadius => interpolate_f32,
        BorderBottomLeftRadius => interpolate_f32,

        Top => interpolate_style_unit,
        Right => interpolate_style_unit,
        Bottom => interpolate_style_unit,
        Left => interpolate_style_unit,

        Transform => interpolate_transform,
    );
    None
}


pub struct AnimationDef {
    key_frames: BTreeMap<OrderedFloat<f32>, Vec<StyleProp>>,
}

#[derive(Clone)]
pub struct Animation {
    styles: HashMap<String, BTreeMap<OrderedFloat<f32>, StyleProp>>,
}

pub trait FrameController {
    fn request_next_frame(&mut self, callback: Box<dyn FnOnce(f32)>);
}

pub struct AnimationState {
    animation: Animation,
    start_time: f32,
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
                let map = styles.entry(s.name().to_string()).or_insert_with(|| BTreeMap::new());
                map.insert(p.clone(), s.clone());
            }
        }
        Animation {
            styles
        }
    }
}

impl Animation {
    pub fn get_frame(&self, position: f32) -> Vec<StyleProp> {
        //TODO support loop
        if position > 1.0 {
            return Vec::new();
        }
        let position = f32::clamp(position, 0.0, 1.0);
        let mut result = Vec::new();
        let p = OrderedFloat(position);
        for (k, v) in &self.styles {
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
    pub fn new(animation: Animation, duration: f32, iteration_count: f32, frame_controller: Box<dyn FrameController>) -> Self {
        let state = AnimationState {
            animation,
            start_time: 0.0,
            duration,
            iteration_count,
            frame_controller,
            stopped: false,
        };
        Self {
            state: Mrc::new(state),
        }
    }

    pub fn run(&mut self, renderer: Box<dyn FnMut(Vec<StyleProp>)>) {
        let mut state = self.state.clone();
        self.state.frame_controller.request_next_frame(Box::new(move |t| {
            // println!("animation started:{}", t);
            state.start_time = t;
            Self::render_frame(state, t, renderer);
        }));
    }

    fn stop(&mut self) {
        // println!("stopped");
        self.state.stopped = true;
    }

    fn render_frame(mut state: Mrc<AnimationState>, now: f32, mut renderer: Box<dyn FnMut(Vec<StyleProp>)>) {
        let elapsed = now - state.start_time;
        let position = elapsed / state.duration;
        let frame = if position > state.iteration_count || state.stopped {
            Vec::new()
        } else {
            state.animation.get_frame(position - position as usize as f32)
        };
        let is_ended = frame.is_empty();
        renderer(frame);
        if !is_ended {
            let s = state.clone();
            state.frame_controller.request_next_frame(Box::new(|t| {
                Self::render_frame(s, t, renderer);
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
    fn request_next_frame(&mut self, callback: Box<dyn FnOnce(f32)>) {
        let now = self.timer.elapsed().unwrap().as_nanos();
        let next_frame_time = self.prev_frame_time + 16666667;
        self.prev_frame_time = next_frame_time;
        if next_frame_time > now {
            let sleep_time = (next_frame_time - now) as u64;
            self.timer_handle = Some(set_timeout_nanos(move || {
                callback(next_frame_time as f32);
            }, sleep_time));
        } else {
            self.timer_handle = Some(set_timeout(move || {
                callback(now as f32);
            }, 0));
        }
    }
}