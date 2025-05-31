use crate as deft;
use crate::animation::actor::AnimationActor;
use crate::base::EventContext;
use crate::element::container::Container;
use crate::element::scroll::ScrollBarStrategy::{Always, Auto, Never};
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::event::CaretChangeEvent;
use crate::js::FromJsValue;
use crate::render::RenderFn;
use crate::style::ResolvedStyleProp;
use crate::{backend_as_api, ok_or_return};
use bezier_rs::{Bezier, TValue};
use deft_macros::{element_backend, js_methods};
use log::debug;
use quick_js::{JsValue, ValueError};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::time::Instant;
use yoga::{MeasureMode, NodeRef, Size};

pub const MOMENTUM_DURATION: f32 = 200.0;
pub const MOMENTUM_DISTANCE: f32 = 16.0;

#[derive(Debug)]
pub struct MomentumInfo {
    pub start_time: Instant,
    pub start_left: f32,
    pub start_top: f32,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum ScrollBarStrategy {
    Never,
    Auto,
    Always,
}

impl ScrollBarStrategy {
    pub fn from_str(str: &str) -> Option<ScrollBarStrategy> {
        let v = match str.to_lowercase().as_str() {
            "never" => Never,
            "auto" => Auto,
            "always" => Always,
            _ => return None,
        };
        Some(v)
    }
}

impl FromJsValue for ScrollBarStrategy {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
        if let JsValue::String(str) = value {
            Self::from_str(str.as_str()).ok_or(ValueError::UnexpectedType)
        } else {
            Err(ValueError::Internal(
                "ScrollBarStrategy is not a string".into(),
            ))
        }
    }
}

backend_as_api!(ScrollBackend, Scroll, as_scroll, as_scroll_mut);

extern "C" fn measure_scroll(
    _node_ref: NodeRef,
    _width: f32,
    _width_mode: MeasureMode,
    _height: f32,
    _height_mode: MeasureMode,
) -> Size {
    // if let Some(ctx) = Node::get_context(&node_ref) {
    // if let Some(scroll) = ctx.downcast_ref::<ScrollWeak>() {
    // if let Ok(s) = scroll.upgrade() {
    // let width = s.default_width.unwrap_or(width);
    // let height = s.default_height.unwrap_or(height);
    // if s.default_width.is_none() || s.default_height.is_none() {
    //     s.last_layout_size = (width, height);
    //     s.do_layout_content();
    // }
    // let width = s.default_width.unwrap_or(s.real_content_width);
    // let height = s.default_height.unwrap_or(s.real_content_height);
    // return Size { width, height };
    // }
    // }
    // }
    Size {
        width: 0.0,
        height: 0.0,
    }
}

#[element_backend]
pub struct Scroll {
    element: ElementWeak,
    base: Container,
    auto_height: bool,
}

#[js_methods]
impl Scroll {
    pub fn set_auto_height(&mut self, value: bool) {
        self.auto_height = value;
        self.element.mark_dirty(true);
    }

    //TODO rename
    // pub fn scroll_to_top(&mut self, top: f32) -> Result<(), JsError> {
    //     self.element.upgrade_mut()?.set_scroll_top(top);
    //     Ok(())
    // }

    fn mark_layout_dirty(&mut self) {
        let auto_height = self.auto_height;
        self.element.mark_dirty(auto_height);
    }

    fn handle_caret_change(&mut self, _detail: &CaretChangeEvent) {
        // debug!("caretchange:{:?}", detail.origin_bounds);
        /*
        let mut body = ok_or_return!(self.element.upgrade_mut());
        let scroll_origin_bounds = body.get_origin_content_bounds();

        let caret_bottom = detail.origin_bounds.bottom();
        let scroll_bottom = scroll_origin_bounds.bottom();
        if caret_bottom > scroll_bottom {
            let new_scroll_top = body.get_scroll_top() + (caret_bottom - scroll_bottom);
            body.set_scroll_top(new_scroll_top);
        } else if detail.origin_bounds.y < scroll_origin_bounds.y {
            let new_scroll_top =
                body.get_scroll_top() - (scroll_origin_bounds.y - detail.origin_bounds.y);
            body.set_scroll_top(new_scroll_top);
        }

        let caret_right = detail.origin_bounds.right();
        let scroll_right = scroll_origin_bounds.right();
        if caret_right > scroll_right {
            let new_scroll_left = body.get_scroll_left() + (caret_right - scroll_right);
            body.set_scroll_left(new_scroll_left);
        } else if detail.origin_bounds.x < scroll_origin_bounds.x {
            let new_scroll_left =
                body.get_scroll_left() - (scroll_origin_bounds.x - detail.origin_bounds.x);
            body.set_scroll_left(new_scroll_left);
        }
         */
    }

    pub fn map_element_window_xy(
        _element: &Element,
        window_x: f32,
        window_y: f32,
    ) -> Option<(f32, f32)> {
        //TODO fix or remove
        /*
        let window = element.get_window()?.upgrade().ok()?;
        let node_matrix = window.render_tree.get_element_total_matrix(&element)?;
        let p = node_matrix.invert()?.map_xy(window_x, window_y);
        Some((p.x, p.y))
         */
        Some((window_x, window_y))
    }
}

impl ElementBackend for Scroll {
    fn create(ele: &mut Element) -> Self {
        // ele.create_shadow();
        ele.need_snapshot = true;
        let base = Container::create(ele);

        let inst = ScrollData {
            // scroll_bar_size: if is_mobile_platform { 4.0 } else { 14.0 },
            element: ele.as_weak(),
            base,
            auto_height: false,
        }
        .to_ref();
        // ele.style.yoga_node.measure_func = (Some(measure_scroll));
        // let weak_ptr = inst.as_weak();
        // ele.style.yoga_node.context = (Some(Context::new(weak_ptr)));
        inst
    }

    /*
    fn execute_default_behavior(
        &mut self,
        event: &mut Box<dyn Any>,
        ctx: &mut EventContext<ElementWeak>,
    ) -> bool {
        let is_target_self = ctx.target == self.element;
        let element = self.element.clone();
        let element = ok_or_return!(element.upgrade_mut(), false);
        if let Some(e) = event.downcast_mut::<MouseDownEvent>() {
            let d = e.0;
            if !is_target_self {
                return false;
            }
            let is_in_vertical_bar = self
                .vertical_bar_rect
                .contains_point(d.offset_x, d.offset_y);
            if is_in_vertical_bar {
                let indicator_rect = self.calculate_vertical_indicator_rect();
                if indicator_rect.contains_point(d.offset_x, d.offset_y) {
                    self.begin_scroll_y(d.window_y);
                } else {
                    //TODO scroll page
                }
                return true;
            }
            let is_in_horizontal_bar = self
                .horizontal_bar_rect
                .contains_point(d.offset_x, d.offset_y);
            if is_in_horizontal_bar {
                let indicator_rect = self.calculate_horizontal_indicator_rect();
                if indicator_rect.contains_point(d.offset_x, d.offset_y) {
                    self.begin_scroll_x(d.window_x);
                } else {
                    //TODO scroll page
                }
                return true;
            }
        } else if let Some(_d) = event.downcast_mut::<MouseUpEvent>() {
            self.end_scroll();
            return true;
        } else if let Some(e) = event.downcast_mut::<MouseMoveEvent>() {
            let d = e.0;
            self.update_scroll_x(d.window_x, true);
            self.update_scroll_y(d.window_y, true);
            return true;
        } else if let Some(e) = event.downcast_mut::<TouchStartEvent>() {
            // debug!("touch start: {:?}", e.0);
            let d = &e.0;
            let touch = unsafe { d.touches.get_unchecked(0) };
            let (window_x, window_y) = match self.map_window_xy(touch.window_x, touch.window_y) {
                None => return false,
                Some(v) => v,
            };
            self.begin_scroll_x(-window_x);
            self.begin_scroll_y(-window_y);
            debug!("touch start: pos {:?}", (window_x, window_y));
            self.momentum_info = Some(MomentumInfo {
                start_time: Instant::now(),
                start_left: element.get_scroll_left(),
                start_top: element.get_scroll_top(),
            });
            self.momentum_animation_instance = None;
            return true;
        } else if let Some(e) = event.downcast_mut::<TouchMoveEvent>() {
            // debug!("touch move: {:?}", e.0);
            let d = &e.0;
            let touch = unsafe { d.touches.get_unchecked(0) };
            let (window_x, window_y) = match self.map_window_xy(touch.window_x, touch.window_y) {
                None => return false,
                Some(v) => v,
            };
            self.update_scroll_x(-window_x, false);
            self.update_scroll_y(-window_y, false);
            let left = element.get_scroll_left();
            let top = element.get_scroll_top();
            // debug!("touch updated: {:?}", (window_x, window_y));
            if let Some(momentum_info) = &mut self.momentum_info {
                if momentum_info.start_time.elapsed().as_millis() as f32 > MOMENTUM_DURATION {
                    momentum_info.start_time = Instant::now();
                    momentum_info.start_left = left;
                    momentum_info.start_top = top;
                }
            }
            return true;
        } else if let Some(e) = event.downcast_mut::<TouchEndEvent>() {
            debug!("touch end: {:?}", e.0);
            if let Some(momentum_info) = &self.momentum_info {
                let duration = momentum_info.start_time.elapsed().as_nanos() as f32 / 1000_000.0;
                let horizontal_distance = element.get_scroll_left() - momentum_info.start_left;
                let vertical_distance = element.get_scroll_top() - momentum_info.start_top;
                let max_distance = f32::max(horizontal_distance.abs(), vertical_distance.abs());
                // debug!("touch end: info{:?}", (duration, vertical_distance));
                if duration < MOMENTUM_DURATION && max_distance > MOMENTUM_DISTANCE {
                    let horizontal_speed = calculate_speed(horizontal_distance, duration);
                    let vertical_speed = calculate_speed(vertical_distance, duration);
                    // debug!("speed: {} {}", horizontal_speed, vertical_speed);
                    let old_left = element.get_scroll_left();
                    let old_top = element.get_scroll_top();
                    let left_dist = horizontal_speed / 0.003;
                    let top_dist = vertical_speed / 0.003;
                    let actor = ScrollAnimationActor::new(
                        element.as_weak(),
                        old_left,
                        old_top,
                        left_dist,
                        top_dist,
                    );
                    let window = some_or_return!(element.get_window(), false);
                    let fc = WindowAnimationController::new(window);
                    let mut ai =
                        AnimationInstance::new(actor, 1000.0 * 1000000.0, 1.0, Box::new(fc));
                    ai.run();
                    self.momentum_animation_instance = Some(ai);
                }
            }
            self.momentum_info = None;
            self.end_scroll();
            return true;
        } else if let Some(_e) = event.downcast_mut::<TouchCancelEvent>() {
            self.end_scroll();
            self.momentum_info = None;
            return true;
        } else if let Some(d) = event.downcast_mut::<CaretChangeEvent>() {
            self.handle_caret_change(d);
            return true;
        } else if let Some(e) = event.downcast_mut::<MouseWheelEvent>() {
            self.handle_default_mouse_wheel(e);
            return true;
        }
        false
    }
    */

    fn render(&mut self) -> RenderFn {
        // let scrollbar_renderer = self.scrollable.render();
        RenderFn::new(move |_painter| {
            // scrollbar_renderer.run(painter);
        })
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        Some(&mut self.base)
    }

    fn accept_pseudo_element_styles(&mut self, _styles: HashMap<String, Vec<ResolvedStyleProp>>) {
        /*
        if let Some(scrollbar_styles) = styles.get("scrollbar") {
            for style in scrollbar_styles {
                match style {
                    ResolvedStyleProp::BackgroundColor(color) => {
                        self.bar_background_color = color.clone();
                        self.element.mark_dirty(false);
                    }
                    _ => {}
                }
            }
        }
        if let Some(thumb_styles) = styles.get("scrollbar-thumb") {
            for style in thumb_styles {
                match style {
                    ResolvedStyleProp::BackgroundColor(color) => {
                        self.indicator_color = color.clone();
                        self.element.mark_dirty(false);
                    }
                    _ => {}
                }
            }
        }
         */
    }

    fn on_event(&mut self, event: Box<&mut dyn Any>, ctx: &mut EventContext<ElementWeak>) {
        let element = ok_or_return!(self.element.upgrade());
        element.clone().scrollable.on_event(&event, ctx, &element);
    }
}

pub fn calculate_speed(distance: f32, duration: f32) -> f32 {
    let max_speed = 5.0;
    if distance == 0.0 {
        return 0.0;
    }
    let speed = if duration == 0.0 {
        max_speed
    } else {
        f32::min(distance.abs() / duration as f32, max_speed)
    };
    if distance > 0.0 {
        speed
    } else {
        speed * -1.0
    }
}

pub struct ScrollAnimationActor {
    old_left: f32,
    left_dist: f32,
    old_top: f32,
    top_dist: f32,
    element: ElementWeak,
    timing_func: Bezier,
}

impl ScrollAnimationActor {
    pub fn new(
        element: ElementWeak,
        old_left: f32,
        old_top: f32,
        left_dist: f32,
        top_dist: f32,
    ) -> Self {
        let timing_func = Bezier::from_cubic_coordinates(0.0, 0.0, 0.17, 0.89, 0.45, 1.0, 1.0, 1.0);
        Self {
            old_left,
            left_dist,
            old_top,
            top_dist,
            element,
            timing_func,
        }
    }
}

impl AnimationActor for ScrollAnimationActor {
    fn apply_animation(&mut self, position: f32, stop: &mut bool) {
        let mut left_stopped = self.left_dist == 0.0;
        let mut top_stooped = self.top_dist == 0.0;
        let ele = ok_or_return!(self.element.upgrade_mut());

        if !left_stopped {
            let new_left = self.old_left
                + self.left_dist
                    * self
                        .timing_func
                        .evaluate(TValue::Parametric(position as f64))
                        .y as f32;
            // ele.set_scroll_left(new_left);
            left_stopped = new_left < 0.0 || new_left > ele.get_max_scroll_left();
        }
        if !top_stooped {
            let new_top = self.old_top
                + self.top_dist
                    * self
                        .timing_func
                        .evaluate(TValue::Parametric(position as f64))
                        .y as f32;
            // ele.scrollable.vertical_bar.scroll_offset() .set_scroll_top(new_top);
            top_stooped = new_top < 0.0 || new_top > ele.get_max_scroll_top();
        }
        if left_stopped && top_stooped {
            debug!("animation stopped: {} {}", left_stopped, top_stooped);
            *stop = true;
        }
    }
}
