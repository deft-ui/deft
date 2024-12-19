use crate as lento;
use std::any::Any;
use std::str::FromStr;
use bezier_rs::{Bezier, TValue};
use lento_macros::element_backend;
use quick_js::JsValue;
use skia_safe::{Canvas, Color, Paint};
use tokio::time::Instant;
use yoga::Direction::LTR;
use yoga::{Context, MeasureMode, Node, NodeRef, Size, StyleUnit};
use crate::{backend_as_api, is_mobile_platform, js_call};
use crate::animation::{AnimationDef, AnimationInstance, SimpleFrameController};
use crate::base::{CaretDetail, ElementEvent, EventContext, Rect};
use crate::color::parse_hex_color;
use crate::element::{ElementBackend, Element, ViewEvent, ElementWeak};
use crate::element::container::Container;
use crate::element::paragraph::ParagraphWeak;
use crate::element::scroll::ScrollBarStrategy::{Always, Auto, Never};
use crate::event::{CaretChangeEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent, MouseWheelEvent, TouchCancelEvent, TouchEndEvent, TouchMoveEvent, TouchStartEvent};
use crate::js::js_runtime::FromJsValue;
use crate::style::{StyleProp, StylePropVal};

const MOMENTUM_DURATION: u128 = 200;
const MOMENTUM_DISTANCE: f32 = 16.0;

struct MomentumInfo {
    start_time: Instant,
    start_left: f32,
    start_top: f32,
}

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
            _ => return None
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
    width_mode: MeasureMode,
    height: f32,
    height_mode: MeasureMode,
) -> Size {
    if let Some(ctx) = Node::get_context(&node_ref) {
        if let Some(scroll) = ctx.downcast_ref::<ScrollWeak>() {
            if let Ok(mut s) = scroll.upgrade() {
                let width = s.default_width.unwrap_or(width);
                let height = s.default_height.unwrap_or(height);
                if s.default_width.is_none() || s.default_height.is_none() {
                    s.do_layout_content(width, height);
                }
                let width = s.default_width.unwrap_or(s.real_content_width);
                let height = s.default_height.unwrap_or(s.real_content_height);
                let real_width = s.real_content_width;
                let real_height = s.real_content_height;
                return Size { width, height }
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
    element: Element,
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
    pub content_auto_width: bool,
    pub content_auto_height: bool,
    default_width: Option<f32>,
    default_height: Option<f32>,

    momentum_info: Option<MomentumInfo>,
    momentum_animation_instance: Option<AnimationInstance>,
}

impl Scroll {
    pub fn set_scroll_y(&mut self, value: ScrollBarStrategy) {
        self.vertical_bar_strategy = value;
        self.element.mark_dirty(true);
    }

    pub fn set_scroll_x(&mut self, value: ScrollBarStrategy) {
        self.horizontal_bar_strategy = value;
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
    pub fn scroll_to_top(&mut self, top: f32) {
        self.element.set_scroll_top(top);
    }

    fn layout_content(&mut self, bounds_width: f32, bounds_height: f32) {
        let (width, height) = self.get_body_view_size(bounds_width, bounds_height);
        //TODO fix ltr
        // self.element.style.calculate_shadow_layout(width, height, LTR);
        let layout_width = if self.content_auto_width { f32::NAN } else { width };
        let layout_height = if self.content_auto_height { f32::NAN } else { height };
        // self.element.style.calculate_shadow_layout(f32::NAN, f32::NAN, LTR);
        self.element.style.calculate_shadow_layout(layout_width, layout_height, LTR);

        for child in &mut self.element.get_children().clone() {
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
        if self.is_y_overflow {
            let new_scroll_top = self.element.get_scroll_top() - 40.0 * detail.rows;
            self.element.set_scroll_top(new_scroll_top);
            true
        } else {
            false
        }
    }

    fn handle_caret_change(&mut self, detail: &CaretChangeEvent) {
        // println!("caretchange:{:?}", detail.origin_bounds);
        let body = &mut self.element;
        let scroll_origin_bounds = body.get_origin_content_bounds();

        let caret_bottom = detail.origin_bounds.bottom();
        let scroll_bottom = scroll_origin_bounds.bottom();
        if  caret_bottom > scroll_bottom  {
            body.set_scroll_top(body.get_scroll_top() + (caret_bottom - scroll_bottom));
        } else if detail.origin_bounds.y < scroll_origin_bounds.y {
            body.set_scroll_top(body.get_scroll_top() - (scroll_origin_bounds.y - detail.origin_bounds.y));
        }

        let caret_right = detail.origin_bounds.right();
        let scroll_right = scroll_origin_bounds.right();
        if caret_right > scroll_right {
            body.set_scroll_left(body.get_scroll_left() + (caret_right - scroll_right));
        } else if detail.origin_bounds.x < scroll_origin_bounds.x {
            body.set_scroll_left(body.get_scroll_left() - (scroll_origin_bounds.x - detail.origin_bounds.x));
        }
    }


    fn update_vertical_bar_rect(&mut self, is_visible: bool, container_width: f32, container_height: f32) {
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
        let indicator = Indicator::new(self.real_content_height, self.vertical_bar_rect.height, self.element.get_scroll_top());
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
        if self.horizontal_bar_rect.is_empty() {
            Rect::empty()
        } else {
            let indicator = Indicator::new(self.real_content_width, self.horizontal_bar_rect.width, self.element.get_scroll_left());
            Rect::new(
                indicator.get_indicator_offset(),
                self.horizontal_bar_rect.y,
                indicator.get_indicator_size(),
                self.horizontal_bar_rect.height,
            )
        }
    }

    fn begin_scroll_y(&mut self, y: f32) {
        self.vertical_move_begin = Some((y, self.element.get_scroll_top()));
    }

    fn begin_scroll_x(&mut self, x: f32) {
        self.horizontal_move_begin = Some((x, self.element.get_scroll_left()));
    }

    fn update_scroll_y(&mut self, y: f32, by_scroll_bar: bool) {
        if let Some((begin_y, begin_top)) = self.vertical_move_begin {
            let mouse_move_distance = y - begin_y;
            let distance = if by_scroll_bar {
                let indicator_rect = self.calculate_vertical_indicator_rect();
                mouse_move_distance
                    / (self.vertical_bar_rect.height - indicator_rect.height)
                    * (self.real_content_height - self.vertical_bar_rect.height)
            } else {
                mouse_move_distance
            };
            self.element.set_scroll_top(begin_top + distance)
        }
    }

    fn update_scroll_x(&mut self, x: f32, by_scroll_bar: bool) {
        if let Some((begin_x, begin_left)) = self.horizontal_move_begin {
            let mouse_move_distance = x - begin_x;
            let distance = if by_scroll_bar {
                let indicator_rect = self.calculate_horizontal_indicator_rect();
                mouse_move_distance
                    / (self.horizontal_bar_rect.width - indicator_rect.width)
                    * (self.real_content_width - self.horizontal_bar_rect.width)
            } else {
                mouse_move_distance
            };
            self.element.set_scroll_left(begin_left + distance)
        }
    }

    fn end_scroll(&mut self) {
        self.vertical_move_begin = None;
        self.horizontal_move_begin = None;
    }

    fn do_layout_content(&mut self, bounds_width: f32, bounds_height: f32) {
        self.layout_content(bounds_width, bounds_height);

        let (mut body_width, mut body_height) = self.get_body_view_size(bounds_width, bounds_height);
        let (mut real_content_width, mut real_content_height) = self.element.get_real_content_size();

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
            (body_width, body_height) = self.get_body_view_size(bounds_width, bounds_height);
            (real_content_width, real_content_height) = self.element.get_real_content_size();
        } else if new_vertical_bar_visible {
            self.update_vertical_bar_rect(true, bounds_width, bounds_height);
        }

        let old_horizontal_bar_visible = !self.horizontal_bar_rect.is_empty();
        self.is_x_overflow = real_content_width > body_width;
        let new_horizontal_bar_visible = match self.horizontal_bar_strategy {
            Never => false,
            Auto => self.is_x_overflow,
            Always => true
        };
        if old_horizontal_bar_visible != new_horizontal_bar_visible {
            self.update_horizontal_bar_rect(new_horizontal_bar_visible, bounds_width, bounds_height);
            self.update_vertical_bar_rect(new_vertical_bar_visible, bounds_width, bounds_height);
            if !is_mobile_platform() {
                self.layout_content(bounds_width, bounds_height);
            }
            (body_width, body_height) = self.get_body_view_size(bounds_width, bounds_height);
            (real_content_width, real_content_height) = self.element.get_real_content_size();
        } else if new_horizontal_bar_visible {
            self.update_horizontal_bar_rect(true, bounds_width, bounds_height);
        }

        // Update scroll offset
        let scroll_left = self.element.get_scroll_left();
        self.element.set_scroll_left(scroll_left);
        let scroll_top = self.element.get_scroll_top();
        self.element.set_scroll_top(scroll_top);
        self.real_content_width = real_content_width;
        self.real_content_height = real_content_height;
        self.content_layout_dirty = false;
    }


}

impl ElementBackend for Scroll {
    fn create(mut ele: Element) -> Self {
        ele.create_shadow();
        let mut base = Container::create(ele.clone());
        let is_mobile_platform = is_mobile_platform();

        let mut inst = ScrollData {
            scroll_bar_size: if is_mobile_platform { 4.0 } else { 14.0 },
            element: ele.clone(),
            base,
            bar_background_color: parse_hex_color(if is_mobile_platform { "0000" } else { "1E1F22" } ).unwrap(),
            indicator_color: parse_hex_color(if is_mobile_platform { "66666644" } else { "444446" }).unwrap(),
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
        }.to_ref();
        inst.element.style.set_measure_func(Some(measure_scroll));
        let weak_ptr = inst.as_weak();
        inst.element.style.set_context(Some(Context::new(weak_ptr)));
        inst
    }

    fn get_name(&self) -> &str {
        "Scroll"
    }

    fn before_origin_bounds_change(&mut self) {
        self.content_layout_dirty = true;
    }

    fn handle_origin_bounds_change(&mut self, bounds: &Rect) {
        if self.content_layout_dirty {
            self.do_layout_content(bounds.width, bounds.height);
        }
    }

    fn add_child_view(&mut self, child: Element, position: Option<u32>) {
        self.base.add_child_view(child, position);
    }

    fn remove_child_view(&mut self, position: u32) {
        self.base.remove_child_view(position)
    }

    fn get_children(&self) -> Vec<Element> {
        self.base.get_children()
    }

    fn set_property(&mut self, p: &str, v: JsValue) {
        js_call!("scroll_y", ScrollBarStrategy, self, set_scroll_y, p, v);
        js_call!("scroll_x", ScrollBarStrategy, self, set_scroll_x, p, v);
    }

    fn execute_default_behavior(&mut self, event: &mut Box<dyn Any>, ctx: &mut EventContext<ElementWeak>) -> bool {
        let is_target_self = ctx.target.upgrade().ok().as_ref() == Some(&self.element);
        if let Some(e) = event.downcast_mut::<MouseDownEvent>() {
            let d = e.0;
            if !is_target_self {
                return false;
            }
            let is_in_vertical_bar = self.vertical_bar_rect.contains_point(d.offset_x, d.offset_y);
            if is_in_vertical_bar {
                let indicator_rect = self.calculate_vertical_indicator_rect();
                if indicator_rect.contains_point(d.offset_x, d.offset_y) {
                    self.begin_scroll_y(d.frame_y);
                } else {
                    //TODO scroll page
                }
                return true;
            }
            let is_in_horizontal_bar = self.horizontal_bar_rect.contains_point(d.offset_x, d.offset_y);
            if is_in_horizontal_bar {
                let indicator_rect = self.calculate_horizontal_indicator_rect();
                if indicator_rect.contains_point(d.offset_x, d.offset_y) {
                    self.begin_scroll_x(d.frame_x);
                } else {
                    //TODO scroll page
                }
                return true;
            }
        } else if let Some(d) = event.downcast_mut::<MouseUpEvent>() {
            self.end_scroll();
            return true;
        } else if let Some(e) = event.downcast_mut::<MouseMoveEvent>() {
            let d = e.0;
            self.update_scroll_x(d.frame_x, true);
            self.update_scroll_y(d.frame_y, true);
            return true;
        } else if let Some(e) = event.downcast_mut::<TouchStartEvent>() {
            let d = &e.0;
            let touch = unsafe { d.touches.get_unchecked(0) };
            self.begin_scroll_x(-touch.frame_x);
            self.begin_scroll_y(-touch.frame_y);
            self.momentum_info = Some(MomentumInfo {
                start_time: Instant::now(),
                start_left: self.element.get_scroll_left(),
                start_top: self.element.get_scroll_top(),
            });
            self.momentum_animation_instance = None;
            return true;
        } else if let Some(e) = event.downcast_mut::<TouchMoveEvent>() {
            let d = &e.0;
            let touch = unsafe { d.touches.get_unchecked(0) };
            self.update_scroll_x(-touch.frame_x, false);
            self.update_scroll_y(-touch.frame_y, false);
            let left = self.element.get_scroll_left();
            let top = self.element.get_scroll_top();
            if let Some(momentum_info) = &mut self.momentum_info {
                if momentum_info.start_time.elapsed().as_millis() > MOMENTUM_DURATION {
                    momentum_info.start_time = Instant::now();
                    momentum_info.start_left = left;
                    momentum_info.start_top = top;
                }
            }
            return true;
        } else if let Some(e) = event.downcast_mut::<TouchEndEvent>() {
            if let Some(momentum_info) = &self.momentum_info {
                let duration = momentum_info.start_time.elapsed().as_millis();
                let horizontal_distance = self.element.get_scroll_left() - momentum_info.start_left;
                let vertical_distance = self.element.get_scroll_top() - momentum_info.start_top;
                let max_distance = f32::max(horizontal_distance.abs(), vertical_distance.abs());
                if duration < MOMENTUM_DURATION && max_distance > MOMENTUM_DISTANCE {
                    let horizontal_speed = horizontal_distance / duration as f32;
                    let vertical_speed = vertical_distance / duration as f32;
                    // println!("speed: {} {}", horizontal_speed, vertical_speed);
                    let old_left = self.element.get_scroll_left();
                    let old_top = self.element.get_scroll_top();
                    let left_dist = horizontal_speed / 0.003;
                    let top_dist = vertical_speed / 0.003;

                    //TODO Don't use RowGap/ColumnGap
                    let animation = AnimationDef::new()
                        .key_frame(0.0, vec![StyleProp::RowGap(StylePropVal::Custom(0.0)), StyleProp::ColumnGap(StylePropVal::Custom(0.0))])
                        .key_frame(1.0, vec![StyleProp::RowGap(StylePropVal::Custom(1.0)), StyleProp::ColumnGap(StylePropVal::Custom(1.0))])
                        .build();
                    let frame_controller = SimpleFrameController::new();
                    let mut animation_instance = AnimationInstance::new(animation, 1000.0 * 1000000.0, 1.0, Box::new(frame_controller));
                    let mut ele = self.element.clone();
                    let timing_func = Bezier::from_cubic_coordinates(0.0, 0.0, 0.17, 0.89, 0.45, 1.0, 1.0, 1.0);
                    animation_instance.run(Box::new(move |styles| {
                        for style in styles {
                            match style {
                                StyleProp::RowGap(value) => {
                                    let new_left = old_left + left_dist * timing_func.evaluate(TValue::Parametric(value.resolve(&0.0) as f64)).y as f32;
                                    ele.set_scroll_left(new_left);
                                },
                                StyleProp::ColumnGap(value) => {
                                    let new_top = old_top + top_dist * timing_func.evaluate(TValue::Parametric(value.resolve(&0.0) as f64)).y as f32;
                                    ele.set_scroll_top(new_top);
                                },
                                _ => {}
                            }
                        }
                    }));
                    self.momentum_animation_instance = Some(animation_instance);
                }
            }
            self.momentum_info = None;
            self.end_scroll();
            return true;
        } else if let Some(e) = event.downcast_mut::<TouchCancelEvent>() {
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

    fn draw(&self, canvas: &Canvas) {
        let mut paint = Paint::default();
        paint.set_color(self.bar_background_color);

        let mut indicator_paint = Paint::default();
        indicator_paint.set_color(self.indicator_color);

        if !self.vertical_bar_rect.is_empty() {
            canvas.draw_rect(self.vertical_bar_rect.to_skia_rect(), &paint);
            let v_indicator_rect = self.calculate_vertical_indicator_rect();
            canvas.draw_rect(v_indicator_rect.to_skia_rect(), &indicator_paint);
        }
        if !self.horizontal_bar_rect.is_empty() {
            canvas.draw_rect(self.horizontal_bar_rect.to_skia_rect(), &paint);
            let h_indicator_rect = self.calculate_horizontal_indicator_rect();
            canvas.draw_rect(h_indicator_rect.to_skia_rect(), &indicator_paint);
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
        Self { bar_len, content_len, offset }
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