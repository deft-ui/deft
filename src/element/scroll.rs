use crate as deft;
use crate::animation::actor::AnimationActor;
use crate::animation::{AnimationInstance, WindowAnimationController};
use crate::base::{EventContext, Rect};
use crate::color::parse_hex_color;
use crate::element::container::Container;
use crate::element::scroll::ScrollBarStrategy::{Always, Auto, Never};
use crate::element::{Element, ElementBackend, ElementWeak};
use crate::event::{
    CaretChangeEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent, MouseWheelEvent,
    TouchCancelEvent, TouchEndEvent, TouchMoveEvent, TouchStartEvent,
};
use crate::js::js_runtime::FromJsValue;
use crate::js::JsError;
use crate::layout::LayoutRoot;
use crate::render::RenderFn;
use crate::style::ResolvedStyleProp;
use crate::{
    backend_as_api, is_mobile_platform, js_deserialize, js_serialize, ok_or_return, some_or_return,
};
use bezier_rs::{Bezier, TValue};
use deft_macros::{element_backend, js_methods};
use log::debug;
use quick_js::JsValue;
use serde::{Deserialize, Serialize};
use skia_safe::{Color, Paint};
use std::any::Any;
use std::collections::HashMap;
use std::time::Instant;
use yoga::Direction::LTR;
use yoga::{Context, MeasureMode, Node, NodeRef, Size};

const MOMENTUM_DURATION: f32 = 200.0;
const MOMENTUM_DISTANCE: f32 = 16.0;

#[derive(Debug)]
struct MomentumInfo {
    start_time: Instant,
    start_left: f32,
    start_top: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ScrollBarStrategy {
    Never,
    Auto,
    Always,
}
js_serialize!(ScrollBarStrategy);
js_deserialize!(ScrollBarStrategy);

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
    fn from_js_value(value: &JsValue) -> Option<Self> {
        if let JsValue::String(str) = value {
            Self::from_str(str.as_str())
        } else {
            None
        }
    }
}

backend_as_api!(ScrollBackend, Scroll, as_scroll, as_scroll_mut);

extern "C" fn measure_scroll(
    node_ref: NodeRef,
    width: f32,
    _width_mode: MeasureMode,
    height: f32,
    _height_mode: MeasureMode,
) -> Size {
    if let Some(ctx) = Node::get_context(&node_ref) {
        if let Some(scroll) = ctx.downcast_ref::<ScrollWeak>() {
            if let Ok(mut s) = scroll.upgrade() {
                let width = s.default_width.unwrap_or(width);
                let height = s.default_height.unwrap_or(height);
                if s.default_width.is_none() || s.default_height.is_none() {
                    s.last_layout_size = (width, height);
                    s.do_layout_content();
                }
                let width = s.default_width.unwrap_or(s.real_content_width);
                let height = s.default_height.unwrap_or(s.real_content_height);
                return Size { width, height };
            }
        }
    }
    Size {
        width: 0.0,
        height: 0.0,
    }
}

#[element_backend]
pub struct Scroll {
    scroll_bar_size: f32,
    element: ElementWeak,
    base: Container,
    bar_background_color: Color,
    indicator_color: Color,
    vertical_bar_strategy: ScrollBarStrategy,
    horizontal_bar_strategy: ScrollBarStrategy,

    vertical_bar_rect: Rect,
    horizontal_bar_rect: Rect,
    /// (mouse_offset, scroll_offset)
    vertical_move_begin: Option<(f32, f32)>,
    /// (mouse_offset, scroll_offset)
    horizontal_move_begin: Option<(f32, f32)>,

    is_y_overflow: bool,
    is_x_overflow: bool,
    real_content_width: f32,
    real_content_height: f32,
    content_layout_dirty: bool,
    auto_height: bool,
    pub content_auto_width: bool,
    pub content_auto_height: bool,
    default_width: Option<f32>,
    default_height: Option<f32>,

    momentum_info: Option<MomentumInfo>,
    momentum_animation_instance: Option<AnimationInstance>,

    last_layout_size: (f32, f32),
}

#[js_methods]
impl Scroll {
    #[js_func]
    pub fn set_scroll_y(&mut self, value: ScrollBarStrategy) {
        self.vertical_bar_strategy = value;
        self.element.mark_dirty(true);
    }

    #[js_func]
    pub fn set_scroll_x(&mut self, value: ScrollBarStrategy) {
        self.horizontal_bar_strategy = value;
        self.element.mark_dirty(true);
    }

    pub fn set_auto_height(&mut self, value: bool) {
        self.auto_height = value;
        self.element.mark_dirty(true);
    }

    pub fn set_default_height(&mut self, value: Option<f32>) {
        self.default_height = value;
        self.element.mark_dirty(true)
    }

    pub fn set_default_width(&mut self, value: Option<f32>) {
        self.default_width = value;
        self.element.mark_dirty(true);
    }

    //TODO rename
    pub fn scroll_to_top(&mut self, top: f32) -> Result<(), JsError> {
        self.element.upgrade_mut()?.set_scroll_top(top);
        Ok(())
    }

    fn mark_layout_dirty(&mut self) {
        self.content_layout_dirty = true;
        let auto_height = self.auto_height;
        self.element.mark_dirty(auto_height);
    }

    fn layout_content(&mut self, bounds_width: f32, bounds_height: f32) {
        let mut element = ok_or_return!(self.element.upgrade_mut());
        let (width, height) = self.get_body_view_size(bounds_width, bounds_height);
        //TODO fix ltr
        // self.element.style.calculate_shadow_layout(width, height, LTR);
        let layout_width = if self.content_auto_width {
            f32::NAN
        } else {
            width
        };
        let layout_height = if self.content_auto_height {
            f32::NAN
        } else {
            height
        };
        // self.element.style.calculate_shadow_layout(f32::NAN, f32::NAN, LTR);
        element.before_layout_recurse();
        element
            .style
            .calculate_shadow_layout(layout_width, layout_height, LTR);

        for child in &mut element.get_children().clone() {
            //TODO remove?
            child.on_layout_update();
        }
    }

    fn get_body_view_size(&self, mut width: f32, mut height: f32) -> (f32, f32) {
        // let (mut width, mut height) = self.element.get_size();
        // let (body_width, body_height) = self.body.get_size();
        if !is_mobile_platform() {
            width -= self.vertical_bar_rect.width;
            height -= self.horizontal_bar_rect.height;
        }

        width = f32::max(0.0, width);
        height = f32::max(0.0, height);

        (width, height)
    }

    fn handle_default_mouse_wheel(&mut self, detail: &MouseWheelEvent) -> bool {
        let mut element = ok_or_return!(self.element.upgrade_mut(), false);
        if self.is_y_overflow {
            let new_scroll_top = element.get_scroll_top() - 40.0 * detail.rows;
            element.set_scroll_top(new_scroll_top);
            true
        } else {
            false
        }
    }

    fn handle_caret_change(&mut self, detail: &CaretChangeEvent) {
        // debug!("caretchange:{:?}", detail.origin_bounds);
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
    }

    fn update_vertical_bar_rect(
        &mut self,
        is_visible: bool,
        container_width: f32,
        container_height: f32,
    ) {
        let bar_size = self.scroll_bar_size;
        self.vertical_bar_rect = if is_visible {
            let bar_height = if self.horizontal_bar_rect.is_empty() {
                container_height
            } else {
                container_height - bar_size
            };
            Rect::new(container_width - bar_size, 0.0, bar_size, bar_height)
        } else {
            Rect::empty()
        }
    }

    fn calculate_vertical_indicator_rect(&self) -> Rect {
        let element = ok_or_return!(self.element.upgrade_mut(), Rect::empty());
        let indicator = Indicator::new(
            self.real_content_height,
            self.vertical_bar_rect.height,
            element.get_scroll_top(),
        );
        if self.vertical_bar_rect.is_empty() {
            Rect::empty()
        } else {
            Rect::new(
                self.vertical_bar_rect.x,
                indicator.get_indicator_offset(),
                self.vertical_bar_rect.width,
                indicator.get_indicator_size(),
            )
        }
    }

    fn update_horizontal_bar_rect(&mut self, is_visible: bool, width: f32, height: f32) {
        let bar_size = self.scroll_bar_size;
        self.horizontal_bar_rect = if is_visible {
            let bar_width = if self.vertical_bar_rect.is_empty() {
                width
            } else {
                width - bar_size
            };
            Rect::new(0.0, height - bar_size, bar_width, bar_size)
        } else {
            Rect::empty()
        }
    }

    fn calculate_horizontal_indicator_rect(&self) -> Rect {
        let element = ok_or_return!(self.element.upgrade_mut(), Rect::empty());
        if self.horizontal_bar_rect.is_empty() {
            Rect::empty()
        } else {
            let indicator = Indicator::new(
                self.real_content_width,
                self.horizontal_bar_rect.width,
                element.get_scroll_left(),
            );
            Rect::new(
                indicator.get_indicator_offset(),
                self.horizontal_bar_rect.y,
                indicator.get_indicator_size(),
                self.horizontal_bar_rect.height,
            )
        }
    }

    fn begin_scroll_y(&mut self, y: f32) {
        let element = ok_or_return!(self.element.upgrade_mut());
        self.vertical_move_begin = Some((y, element.get_scroll_top()));
    }

    fn begin_scroll_x(&mut self, x: f32) {
        let element = ok_or_return!(self.element.upgrade_mut());
        self.horizontal_move_begin = Some((x, element.get_scroll_left()));
    }

    fn update_scroll_y(&mut self, y: f32, by_scroll_bar: bool) {
        if let Some((begin_y, begin_top)) = self.vertical_move_begin {
            let mouse_move_distance = y - begin_y;
            let distance = if by_scroll_bar {
                let indicator_rect = self.calculate_vertical_indicator_rect();
                mouse_move_distance / (self.vertical_bar_rect.height - indicator_rect.height)
                    * (self.real_content_height - self.vertical_bar_rect.height)
            } else {
                mouse_move_distance
            };
            let mut element = ok_or_return!(self.element.upgrade_mut());
            element.set_scroll_top(begin_top + distance)
        }
    }

    fn update_scroll_x(&mut self, x: f32, by_scroll_bar: bool) {
        if let Some((begin_x, begin_left)) = self.horizontal_move_begin {
            let mouse_move_distance = x - begin_x;
            let distance = if by_scroll_bar {
                let indicator_rect = self.calculate_horizontal_indicator_rect();
                mouse_move_distance / (self.horizontal_bar_rect.width - indicator_rect.width)
                    * (self.real_content_width - self.horizontal_bar_rect.width)
            } else {
                mouse_move_distance
            };
            let mut element = ok_or_return!(self.element.upgrade_mut());
            element.set_scroll_left(begin_left + distance)
        }
    }

    fn end_scroll(&mut self) {
        self.vertical_move_begin = None;
        self.horizontal_move_begin = None;
    }

    fn update_layout(&mut self) {
        let element = ok_or_return!(self.element.upgrade_mut());
        if let Some(mut p) = element.get_parent() {
            p.ensure_layout_update();
        }
        let bounds = element.get_bounds();
        let size = (bounds.width, bounds.height);
        if !element.is_layout_dirty() && size == self.last_layout_size {
            return;
        }
        self.last_layout_size = size;
        self.do_layout_content();
        let mut element = ok_or_return!(self.element.upgrade_mut());
        element.set_dirty_state_recurse(false);
    }

    fn do_layout_content(&mut self) {
        let element = self.element.clone();
        let mut element = ok_or_return!(element.upgrade_mut());
        // print_time!("scroll layout content time");
        let (bounds_width, bounds_height) = self.last_layout_size;
        self.layout_content(bounds_width, bounds_height);

        let (mut body_width, body_height) = self.get_body_view_size(bounds_width, bounds_height);
        let (mut real_content_width, mut real_content_height) = element.get_real_content_size();

        let old_vertical_bar_visible = !self.vertical_bar_rect.is_empty();
        self.is_y_overflow = real_content_height > body_height;
        let new_vertical_bar_visible = match self.vertical_bar_strategy {
            Never => false,
            Auto => self.is_y_overflow,
            Always => true,
        };
        if old_vertical_bar_visible != new_vertical_bar_visible {
            self.update_vertical_bar_rect(new_vertical_bar_visible, bounds_width, bounds_height);
            if !is_mobile_platform() {
                self.layout_content(bounds_width, bounds_height);
            }
            (body_width, _) = self.get_body_view_size(bounds_width, bounds_height);
            (real_content_width, real_content_height) = element.get_real_content_size();
        } else if new_vertical_bar_visible {
            self.update_vertical_bar_rect(true, bounds_width, bounds_height);
        }

        let old_horizontal_bar_visible = !self.horizontal_bar_rect.is_empty();
        self.is_x_overflow = real_content_width > body_width;
        let new_horizontal_bar_visible = match self.horizontal_bar_strategy {
            Never => false,
            Auto => self.is_x_overflow,
            Always => true,
        };
        if old_horizontal_bar_visible != new_horizontal_bar_visible {
            self.update_horizontal_bar_rect(
                new_horizontal_bar_visible,
                bounds_width,
                bounds_height,
            );
            self.update_vertical_bar_rect(new_vertical_bar_visible, bounds_width, bounds_height);
            if !is_mobile_platform() {
                self.layout_content(bounds_width, bounds_height);
            }
            (real_content_width, real_content_height) = element.get_real_content_size();
        } else if new_horizontal_bar_visible {
            self.update_horizontal_bar_rect(true, bounds_width, bounds_height);
        }

        let vbw = self.vertical_bar_rect.width;
        let hbw = self.horizontal_bar_rect.width;
        element.set_child_decoration((0.0, vbw, hbw, 0.0));
        // Update scroll offset
        let scroll_left = element.get_scroll_left();
        element.set_scroll_left(scroll_left);
        let scroll_top = element.get_scroll_top();
        element.set_scroll_top(scroll_top);
        self.real_content_width = real_content_width;
        self.real_content_height = real_content_height;
        self.content_layout_dirty = false;
    }

    /// invert transform effect
    fn map_window_xy(&self, window_x: f32, window_y: f32) -> Option<(f32, f32)> {
        let element = ok_or_return!(self.element.upgrade_mut(), None);
        let window = element.get_window()?.upgrade().ok()?;
        let node_matrix = window.render_tree.get_element_total_matrix(&element)?;
        let p = node_matrix.invert()?.map_xy(window_x, window_y);
        Some((p.x, p.y))
    }
}

impl ElementBackend for Scroll {
    fn create(ele: &mut Element) -> Self {
        ele.create_shadow();
        ele.need_snapshot = true;
        let base = Container::create(ele);
        let is_mobile_platform = is_mobile_platform();

        let inst = ScrollData {
            scroll_bar_size: if is_mobile_platform { 4.0 } else { 14.0 },
            element: ele.as_weak(),
            base,
            bar_background_color: parse_hex_color(if is_mobile_platform {
                "0000"
            } else {
                "1E1F22"
            })
            .unwrap(),
            indicator_color: parse_hex_color(if is_mobile_platform {
                "66666644"
            } else {
                "444446"
            })
            .unwrap(),
            horizontal_bar_strategy: Auto,
            vertical_bar_strategy: Auto,
            is_x_overflow: false,
            real_content_width: 0.0,
            is_y_overflow: false,
            horizontal_bar_rect: Rect::empty(),
            vertical_move_begin: None,
            vertical_bar_rect: Rect::empty(),
            real_content_height: 0.0,
            horizontal_move_begin: None,
            content_auto_height: false,
            content_auto_width: false,
            content_layout_dirty: true,
            default_width: None,
            default_height: None,
            momentum_info: None,
            momentum_animation_instance: None,
            last_layout_size: (f32::NAN, f32::NAN),
            auto_height: false,
        }
        .to_ref();
        ele.style.yoga_node.set_measure_func(Some(measure_scroll));
        let weak_ptr = inst.as_weak();
        ele.style
            .yoga_node
            .set_context(Some(Context::new(weak_ptr)));
        ele.set_as_layout_root(Some(Box::new(inst.as_weak())));
        inst
    }

    fn get_name(&self) -> &str {
        "Scroll"
    }

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

    fn render(&mut self) -> RenderFn {
        let mut me = self.clone();
        //TODO use ensure_layout_update?
        if me.content_layout_dirty {
            me.do_layout_content();
        }
        let mut paint = Paint::default();
        paint.set_color(self.bar_background_color);

        let mut indicator_paint = Paint::default();
        indicator_paint.set_color(self.indicator_color);

        let vertical_bar_rect_visible = !self.vertical_bar_rect.is_empty();

        let horizontal_bar_rect_visible = !self.horizontal_bar_rect.is_empty();

        let v_indicator_rect = self.calculate_vertical_indicator_rect();
        let h_indicator_rect = self.calculate_horizontal_indicator_rect();
        let vertical_bar_rect = self.vertical_bar_rect.to_skia_rect();
        let horizontal_bar_rect = self.horizontal_bar_rect.to_skia_rect();

        RenderFn::new(move |painter| {
            let canvas = painter.canvas;
            if vertical_bar_rect_visible {
                canvas.draw_rect(vertical_bar_rect, &paint);
                canvas.draw_rect(v_indicator_rect.to_skia_rect(), &indicator_paint);
            }
            if horizontal_bar_rect_visible {
                canvas.draw_rect(horizontal_bar_rect, &paint);
                canvas.draw_rect(h_indicator_rect.to_skia_rect(), &indicator_paint);
            }
        })
    }

    fn get_base_mut(&mut self) -> Option<&mut dyn ElementBackend> {
        Some(&mut self.base)
    }

    fn accept_pseudo_element_styles(&mut self, styles: HashMap<String, Vec<ResolvedStyleProp>>) {
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
    }
}

struct Indicator {
    content_len: f32,
    bar_len: f32,
    offset: f32,
}

impl Indicator {
    fn new(content_len: f32, bar_len: f32, offset: f32) -> Self {
        Self {
            bar_len,
            content_len,
            offset,
        }
    }

    fn get_indicator_size(&self) -> f32 {
        let size = self.bar_len / self.content_len * self.bar_len;
        f32::max(size, 20.0)
    }

    fn get_indicator_offset(&self) -> f32 {
        self.offset / (self.content_len - self.bar_len) * (self.bar_len - self.get_indicator_size())
    }

    fn get_indicator_end(&self) -> f32 {
        self.get_indicator_offset() + self.get_indicator_size()
    }
}

impl LayoutRoot for ScrollWeak {
    fn update_layout(&mut self) {
        if let Ok(mut scroll) = self.upgrade() {
            scroll.update_layout();
        }
    }

    fn should_propagate_dirty(&self) -> bool {
        if let Ok(scroll) = self.upgrade() {
            scroll.auto_height
        } else {
            true
        }
    }
}

fn calculate_speed(distance: f32, duration: f32) -> f32 {
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

struct ScrollAnimationActor {
    old_left: f32,
    left_dist: f32,
    old_top: f32,
    top_dist: f32,
    element: ElementWeak,
    timing_func: Bezier,
}

impl ScrollAnimationActor {
    fn new(
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
        let mut ele = ok_or_return!(self.element.upgrade_mut());

        if !left_stopped {
            let new_left = self.old_left
                + self.left_dist
                    * self
                        .timing_func
                        .evaluate(TValue::Parametric(position as f64))
                        .y as f32;
            ele.set_scroll_left(new_left);
            left_stopped = new_left < 0.0 || new_left > ele.get_max_scroll_left();
        }
        if !top_stooped {
            let new_top = self.old_top
                + self.top_dist
                    * self
                        .timing_func
                        .evaluate(TValue::Parametric(position as f64))
                        .y as f32;
            ele.set_scroll_top(new_top);
            top_stooped = new_top < 0.0 || new_top > ele.get_max_scroll_top();
        }
        if left_stopped && top_stooped {
            *stop = true;
        }
    }
}
