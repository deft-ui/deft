use crate as deft;
use crate::animation::actor::AnimationActor;
use crate::animation::{AnimationInstance, WindowAnimationController};
use crate::base::{EventContext, Rect};
use crate::element::common::ScrollBar;
use crate::element::scroll::Scroll;
use crate::element::{Element, ElementWeak};
use crate::event::{Event, TouchCancelEvent, TouchEndEvent, TouchMoveEvent, TouchStartEvent};
use crate::number::DeNan;
use crate::render::RenderFn;
use crate::style::ResolvedStyleProp;
use crate::{is_mobile_platform, some_or_return};
use bezier_rs::{Bezier, TValue};
use deft_macros::mrc_object;
use log::debug;
use std::cell::Cell;
use std::collections::HashMap;
use std::time::Instant;
use yoga::Direction::LTR;

thread_local! {
    static CONSUMED_EVENT_ID: Cell<u64> = Cell::new(0);
}

#[mrc_object]
pub struct Scrollable {
    pub vertical_bar: ScrollBar,
    pub horizontal_bar: ScrollBar,
    momentum_info: Option<crate::element::scroll::MomentumInfo>,
    momentum_animation_instance: Option<AnimationInstance>,
    vertical_move_begin: Option<(f32, f32)>,
    /// (mouse_offset, scroll_offset)
    horizontal_move_begin: Option<(f32, f32)>,
    auto_scroll_callback: Option<Box<dyn FnOnce() -> Option<Rect>>>,
}

impl Scrollable {
    pub fn new() -> Self {
        let vertical_bar = ScrollBar::new_vertical();
        let horizontal_bar = ScrollBar::new_horizontal();
        ScrollableData {
            vertical_bar,
            horizontal_bar,
            momentum_animation_instance: None,
            momentum_info: None,
            vertical_move_begin: None,
            horizontal_move_begin: None,
            auto_scroll_callback: None,
        }
        .to_ref()
    }

    /// Scroll rect into view.
    /// Returns true if scroll occurs
    pub fn scroll_into_view(&mut self, rect: &Rect) -> bool {
        let scrolled_y = self.vertical_bar.scroll_into_view(rect.top, rect.height);
        let scrolled_x = self.horizontal_bar.scroll_into_view(rect.left, rect.width);
        scrolled_x || scrolled_y
    }

    pub fn execute_auto_scroll_callback(&mut self) {
        if let Some(auto_scroll_callback) = self.auto_scroll_callback.take() {
            if let Some(rect) = auto_scroll_callback() {
                self.scroll_into_view(&rect);
            }
        }
    }

    pub fn render(&mut self) -> RenderFn {
        let vertical_bar = self.vertical_bar.render();
        let horizontal_bar = self.horizontal_bar.render();
        RenderFn::merge(vec![vertical_bar, horizontal_bar])
    }

    pub fn scroll_offset(&self) -> (f32, f32) {
        let offset_y = self.vertical_bar.scroll_offset();
        let offset_x = self.horizontal_bar.scroll_offset();
        (offset_x, offset_y)
    }

    pub fn set_autoscroll_callback<F: FnOnce() -> Option<Rect> + 'static>(
        &mut self,
        autoscroll_callback: F,
    ) {
        self.auto_scroll_callback = Some(Box::new(autoscroll_callback));
    }

    pub fn is_scrollable(&self) -> bool {
        self.vertical_bar.is_scrollable() || self.horizontal_bar.is_scrollable()
    }

    pub fn on_event(
        &mut self,
        event: &Event,
        ctx: &mut EventContext<ElementWeak>,
        element: &Element,
    ) -> bool {
        let event_id = ctx.get_id();
        if !self.is_scrollable() || CONSUMED_EVENT_ID.get() == event_id {
            return false;
        }
        let accepted =
            self.vertical_bar.on_event(&event, ctx) || self.horizontal_bar.on_event(&event, ctx);
        if accepted {
            CONSUMED_EVENT_ID.set(event_id);
        } else if event_id != CONSUMED_EVENT_ID.get() {
            if let Some(e) = TouchStartEvent::cast(event) {
                // debug!("touch start: {:?}", e.0);
                let d = &e.0;
                let touch = unsafe { d.touches.get_unchecked(0) };
                let (window_x, window_y) =
                    match Scroll::map_element_window_xy(element, touch.window_x, touch.window_y) {
                        None => return false,
                        Some(v) => v,
                    };
                self.begin_scroll_x(-window_x);
                self.begin_scroll_y(-window_y);
                debug!("touch start: pos {:?}", (window_x, window_y));
                self.momentum_info = Some(crate::element::scroll::MomentumInfo {
                    start_time: Instant::now(),
                    start_left: self.horizontal_bar.scroll_offset,
                    start_top: self.vertical_bar.scroll_offset,
                });
                self.momentum_animation_instance = None;
                CONSUMED_EVENT_ID.set(event_id);
                return false;
            } else if let Some(e) = TouchMoveEvent::cast(event) {
                // debug!("touch move: {:?}", e.0);
                let d = &e.0;
                let touch = unsafe { d.touches.get_unchecked(0) };
                let (window_x, window_y) =
                    match Scroll::map_element_window_xy(element, touch.window_x, touch.window_y) {
                        None => return false,
                        Some(v) => v,
                    };
                self.update_scroll_x(-window_x);
                self.update_scroll_y(-window_y);
                let left = self.horizontal_bar.scroll_offset;
                let top = self.vertical_bar.scroll_offset;
                // debug!("touch updated: {:?}", (window_x, window_y));
                if let Some(momentum_info) = &mut self.momentum_info {
                    if momentum_info.start_time.elapsed().as_millis() as f32
                        > crate::element::scroll::MOMENTUM_DURATION
                    {
                        momentum_info.start_time = Instant::now();
                        momentum_info.start_left = left;
                        momentum_info.start_top = top;
                    }
                }
                CONSUMED_EVENT_ID.set(event_id);
                return false;
            } else if let Some(e) = TouchEndEvent::cast(event) {
                debug!("touch end: {:?} {}", e.0, self.momentum_info.is_some());
                if let Some(momentum_info) = &self.momentum_info {
                    let duration =
                        momentum_info.start_time.elapsed().as_nanos() as f32 / 1000_000.0;
                    let horizontal_distance = self.scroll_offset().0 - momentum_info.start_left;
                    let vertical_distance = self.scroll_offset().1 - momentum_info.start_top;
                    let max_distance = f32::max(horizontal_distance.abs(), vertical_distance.abs());
                    debug!("touch end: info{:?}", (duration, vertical_distance));
                    if duration < crate::element::scroll::MOMENTUM_DURATION
                        && max_distance > crate::element::scroll::MOMENTUM_DISTANCE
                    {
                        let horizontal_speed =
                            crate::element::scroll::calculate_speed(horizontal_distance, duration);
                        let vertical_speed =
                            crate::element::scroll::calculate_speed(vertical_distance, duration);
                        debug!("speed: {} {}", horizontal_speed, vertical_speed);
                        let (old_left, old_top) = self.scroll_offset();
                        let left_dist = horizontal_speed / 0.003;
                        let top_dist = vertical_speed / 0.003;
                        debug!(
                            "scroll params: {} {} {} {} {}",
                            old_left, old_top, left_dist, top_dist, duration
                        );
                        let actor = ScrollAnimationActor::new(
                            self.clone(),
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
                CONSUMED_EVENT_ID.set(event_id);
                return false;
            } else if let Some(_e) = TouchCancelEvent::cast(event) {
                self.end_scroll();
                self.momentum_info = None;
                return false;
            }
        }
        accepted
    }

    pub fn is_mouse_over_bar(&self, x: f32, y: f32) -> bool {
        self.vertical_bar.is_mouse_over(x, y) || self.horizontal_bar.is_mouse_over(x, y)
    }

    pub fn accept_css_style(&mut self, styles: &HashMap<String, Vec<ResolvedStyleProp>>) -> bool {
        let mut accepted = false;
        if let Some(scrollbar_styles) = styles.get("scrollbar") {
            for style in scrollbar_styles {
                match style {
                    ResolvedStyleProp::BackgroundColor(color) => {
                        self.vertical_bar.set_track_background_color(*color);
                        self.horizontal_bar.set_track_background_color(*color);
                        accepted = true;
                    }
                    _ => {}
                }
            }
        }
        if let Some(thumb_styles) = styles.get("scrollbar-thumb") {
            for style in thumb_styles {
                match style {
                    ResolvedStyleProp::BackgroundColor(color) => {
                        self.vertical_bar.set_thumb_background_color(*color);
                        self.horizontal_bar.set_thumb_background_color(*color);
                        accepted = true;
                    }
                    _ => {}
                }
            }
        }
        accepted
    }

    pub fn update_size(&mut self, size: (f32, f32), content_size: (f32, f32)) {
        let box_width = size.0.de_nan(f32::INFINITY);
        let box_height = size.1.de_nan(f32::INFINITY);
        let content_height = content_size.1.de_nan(0.0);
        let content_width = content_size.0.de_nan(0.0);

        self.vertical_bar
            .set_length(box_height, content_height, box_width);
        self.horizontal_bar
            .set_length(box_width, content_width, box_height);
    }

    pub fn update_layout(&mut self, element: &mut Element) {
        let bounds = element.get_bounds();
        //TODO avoid recalculate
        // if size == self.last_layout_size {
        //     return;
        // }
        let border = element.get_border_width();
        self.do_layout_content(
            element,
            bounds.width - border.1 - border.3,
            bounds.height - border.0 - border.2,
        );
    }

    fn do_layout_content(&mut self, element: &mut Element, bounds_width: f32, bounds_height: f32) {
        // print_time!("scroll layout content time");
        self.layout_content(element, bounds_width, bounds_height);

        // let (mut body_width, body_height) = self.get_body_view_size(bounds_width, bounds_height);
        let (mut real_content_width, mut real_content_height) = element.get_real_content_size();

        //TODO optimize, do not emit events
        let old_vertical_bar_visible = self.vertical_bar.visible_thickness() > 0.0;
        let old_horizontal_bar_visible = self.horizontal_bar.visible_thickness() > 0.0;

        self.vertical_bar
            .set_length(bounds_height, real_content_height, bounds_width);
        let new_vertical_bar_visible = self.vertical_bar.visible_thickness() > 0.0;

        if old_vertical_bar_visible != new_vertical_bar_visible {
            if !is_mobile_platform() {
                self.layout_content(element, bounds_width, bounds_height);
            }
            // (body_width, _) = self.get_body_view_size(bounds_width, bounds_height);
            (real_content_width, real_content_height) = element.get_real_content_size();
            self.vertical_bar
                .set_length(bounds_height, real_content_height, bounds_width);
        }

        self.horizontal_bar
            .set_length(bounds_width, real_content_width, bounds_height);
        let new_horizontal_bar_visible = self.horizontal_bar.visible_thickness() > 0.0;
        if old_horizontal_bar_visible != new_horizontal_bar_visible {
            if !is_mobile_platform() {
                self.layout_content(element, bounds_width, bounds_height);
            }
            (real_content_width, real_content_height) = element.get_real_content_size();
        }
        self.vertical_bar
            .set_length(bounds_height, real_content_height, bounds_width);
        self.horizontal_bar
            .set_length(bounds_width, real_content_width, bounds_height);

        let vbw = self.vertical_bar.visible_thickness();
        let hbw = self.horizontal_bar.visible_thickness();
        element.set_child_decoration((0.0, vbw, hbw, 0.0));
    }

    pub fn layout_content(&mut self, element: &mut Element, bounds_width: f32, bounds_height: f32) {
        let (width, height) = self.get_body_view_size(bounds_width, bounds_height);
        //TODO fix ltr
        // self.element.style.calculate_shadow_layout(width, height, LTR);
        // let layout_width = width;
        let layout_height = height;
        // self.element.style.calculate_shadow_layout(f32::NAN, f32::NAN, LTR);
        element.before_layout_recurse();
        element
            .style
            .calculate_shadow_layout(width, layout_height, LTR);

        for child in &mut element.get_children().clone() {
            //TODO remove?
            child.on_layout_update();
        }
    }

    fn get_body_view_size(&self, mut width: f32, mut height: f32) -> (f32, f32) {
        if !is_mobile_platform() {
            width -= self.vertical_bar.visible_thickness();
            height -= self.horizontal_bar.visible_thickness();
        }

        width = f32::max(0.0, width);
        height = f32::max(0.0, height);

        (width, height)
    }

    fn begin_scroll_y(&mut self, y: f32) {
        self.vertical_move_begin = Some((y, self.vertical_bar.scroll_offset));
    }

    fn begin_scroll_x(&mut self, x: f32) {
        self.horizontal_move_begin = Some((x, self.horizontal_bar.scroll_offset));
    }

    fn update_scroll_y(&mut self, y: f32) {
        if let Some((begin_y, begin_top)) = self.vertical_move_begin {
            let mouse_move_distance = y - begin_y;
            let distance = mouse_move_distance;
            self.vertical_bar.update_scroll_offset(begin_top + distance);
        }
    }

    fn update_scroll_x(&mut self, x: f32) {
        if let Some((begin_x, begin_left)) = self.horizontal_move_begin {
            let mouse_move_distance = x - begin_x;
            let distance = mouse_move_distance;
            self.horizontal_bar
                .update_scroll_offset(begin_left + distance)
        }
    }

    fn end_scroll(&mut self) {
        self.vertical_move_begin = None;
        self.horizontal_move_begin = None;
    }
}

pub struct ScrollAnimationActor {
    old_left: f32,
    left_dist: f32,
    old_top: f32,
    top_dist: f32,
    //TODO use weak
    element: Scrollable,
    timing_func: Bezier,
}

impl ScrollAnimationActor {
    pub fn new(
        scrollable: Scrollable,
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
            element: scrollable,
            timing_func,
        }
    }
}

impl AnimationActor for ScrollAnimationActor {
    fn apply_animation(&mut self, position: f32, stop: &mut bool) {
        let mut left_stopped = self.left_dist == 0.0;
        let mut top_stooped = self.top_dist == 0.0;

        if !left_stopped {
            let new_left = self.old_left
                + self.left_dist
                    * self
                        .timing_func
                        .evaluate(TValue::Parametric(position as f64))
                        .y as f32;
            self.element.horizontal_bar.update_scroll_offset(new_left);
            // ele.set_scroll_left(new_left);
            left_stopped =
                new_left < 0.0 || new_left > self.element.horizontal_bar.get_max_scroll_offset();
        }
        if !top_stooped {
            let new_top = self.old_top
                + self.top_dist
                    * self
                        .timing_func
                        .evaluate(TValue::Parametric(position as f64))
                        .y as f32;
            self.element.vertical_bar.update_scroll_offset(new_top);
            top_stooped =
                new_top < 0.0 || new_top > self.element.vertical_bar.get_max_scroll_offset();
        }
        if left_stopped && top_stooped {
            debug!("animation stopped: {} {}", left_stopped, top_stooped);
            *stop = true;
        }
    }
}
