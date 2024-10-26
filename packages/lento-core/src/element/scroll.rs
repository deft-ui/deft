use std::str::FromStr;

use quick_js::JsValue;
use skia_safe::{Canvas, Paint};
use yoga::Direction::LTR;

use crate::{backend_as_api, js_call};
use crate::base::{CaretDetail, ElementEvent, Rect};
use crate::color::parse_hex_color;
use crate::element::{ElementBackend, ElementRef};
use crate::element::container::Container;
use crate::element::scroll::ScrollBarStrategy::{Always, Auto, Never};
use crate::event::{AcceptCaretEvent, AcceptMouseDownEvent, AcceptMouseMoveEvent, AcceptMouseUpEvent, AcceptTouchCancelEvent, AcceptTouchEndEvent, AcceptTouchMoveEvent, AcceptTouchStartEvent, MouseDownEvent, MouseWheelDetail};
use crate::js::js_runtime::FromJsValue;

const BACKGROUND_COLOR: &str = "1E1F22";

const INDICATOR_COLOR: &str = "444446";

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

pub struct Scroll {
    scroll_bar_size: f32,
    element: ElementRef,
    base: Container,
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

    //TODO rename
    pub fn scroll_to_top(&mut self, top: f32) {
        self.element.set_scroll_top(top);
    }

    fn layout_content(&mut self) {
        let (width, height) = self.get_body_view_size();
        //TODO fix ltr
        self.element.layout.calculate_shadow_layout(width, height, LTR);

        for child in &mut self.element.get_children().clone() {
            //TODO remove?
            child.on_layout_update();
        }
    }

    fn get_body_view_size(&self) -> (f32, f32) {
        let (mut width, mut height) = self.element.get_size();
        // let (body_width, body_height) = self.body.get_size();
        width -= self.vertical_bar_rect.width;
        height -= self.horizontal_bar_rect.height;

        width = f32::max(0.0, width);
        height = f32::max(0.0, height);

        (width, height)
    }

    fn handle_default_mouse_wheel(&mut self, detail: &MouseWheelDetail) -> bool {
        if self.is_y_overflow {
            self.element.set_scroll_top(self.element.get_scroll_top() - 40.0 * detail.rows);
            true
        } else {
            false
        }
    }

    fn handle_caret_change(&mut self, detail: &CaretDetail) {
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


}

impl ElementBackend for Scroll {
    fn create(mut ele: ElementRef) -> Self {
        ele.create_shadow();
        let mut base = Container::create(ele.clone());

        let inst = Self {
            scroll_bar_size: 14.0,
            element: ele.clone(),
            base,
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
        };
        inst
    }

    fn get_name(&self) -> &str {
        "Scroll"
    }

    fn handle_origin_bounds_change(&mut self, bounds: &Rect) {
        self.layout_content();

        let (mut body_width, mut body_height) = self.get_body_view_size();
        let (mut real_content_width, mut real_content_height) = self.element.get_real_content_size();

        let old_vertical_bar_visible = !self.vertical_bar_rect.is_empty();
        self.is_y_overflow = real_content_height > body_height;
        let new_vertical_bar_visible = match self.vertical_bar_strategy {
            Never => false,
            Auto => self.is_y_overflow,
            Always => true,
        };
        if old_vertical_bar_visible != new_vertical_bar_visible {
            self.update_vertical_bar_rect(new_vertical_bar_visible, bounds.width, bounds.height);
            self.layout_content();
            (body_width, body_height) = self.get_body_view_size();
            (real_content_width, real_content_height) = self.element.get_real_content_size();
        } else if new_vertical_bar_visible {
            self.update_vertical_bar_rect(true, bounds.width, bounds.height);
        }

        let old_horizontal_bar_visible = !self.horizontal_bar_rect.is_empty();
        self.is_x_overflow = real_content_width > body_width;
        let new_horizontal_bar_visible = match self.horizontal_bar_strategy {
            Never => false,
            Auto => self.is_x_overflow,
            Always => true
        };
        if old_horizontal_bar_visible != new_horizontal_bar_visible {
            self.update_horizontal_bar_rect(new_horizontal_bar_visible, bounds.width, bounds.height);
            self.update_vertical_bar_rect(new_vertical_bar_visible, bounds.width, bounds.height);

            self.layout_content();
            (body_width, body_height) = self.get_body_view_size();
            (real_content_width, real_content_height) = self.element.get_real_content_size();
        } else if new_horizontal_bar_visible {
            self.update_horizontal_bar_rect(true, bounds.width, bounds.height);
        }

        // Update scroll offset
        self.element.set_scroll_left(self.element.get_scroll_left());
        self.element.set_scroll_top(self.element.get_scroll_top());
        self.real_content_width = real_content_width;
        self.real_content_height = real_content_height;
    }

    fn add_child_view(&mut self, child: ElementRef, position: Option<u32>) {
        self.base.add_child_view(child, position);
    }

    fn remove_child_view(&mut self, position: u32) {
        self.base.remove_child_view(position)
    }

    fn get_children(&self) -> Vec<ElementRef> {
        self.base.get_children()
    }

    fn set_property(&mut self, p: &str, v: JsValue) {
        js_call!("scroll_y", ScrollBarStrategy, self, set_scroll_y, p, v);
        js_call!("scroll_x", ScrollBarStrategy, self, set_scroll_x, p, v);
    }

    fn handle_event_default_behavior(&mut self, _event_type: &str, event: &mut ElementEvent) -> bool {
        let is_target_self = &event.context.target == &self.element;
        event.accept_touch_start(|d| {
            let touch = unsafe { d.touches.get_unchecked(0) };
            self.begin_scroll_x(-touch.frame_x);
            self.begin_scroll_y(-touch.frame_y);
        }) || event.accept_touch_move(|d| {
            let touch = unsafe { d.touches.get_unchecked(0) };
            self.update_scroll_x(-touch.frame_x, false);
            self.update_scroll_y(-touch.frame_y, false);
        }) || event.accept_touch_end(|_| {
            self.end_scroll();
        }) || event.accept_touch_cancel(|_| {
            self.end_scroll();
        }) || event.accept_mouse_down(|d| {
            if !is_target_self {
                return;
            }
            let is_in_vertical_bar = self.vertical_bar_rect.contains_point(d.offset_x, d.offset_y);
            if is_in_vertical_bar {
                let indicator_rect = self.calculate_vertical_indicator_rect();
                if indicator_rect.contains_point(d.offset_x, d.offset_y) {
                    self.begin_scroll_y(d.frame_y);
                } else {
                    //TODO scroll page
                }
                return;
            }
            let is_in_horizontal_bar = self.horizontal_bar_rect.contains_point(d.offset_x, d.offset_y);
            if is_in_horizontal_bar {
                let indicator_rect = self.calculate_horizontal_indicator_rect();
                if indicator_rect.contains_point(d.offset_x, d.offset_y) {
                    self.begin_scroll_x(d.frame_x);
                } else {
                    //TODO scroll page
                }
                return;
            }
        }) || event.accept_mouse_up(|d| {
            self.end_scroll();
        }) || event.accept_mouse_move(|d| {
            self.update_scroll_x(d.frame_x, true);
            self.update_scroll_y(d.frame_y, true);
        }) || event.accept_caret_change(|d| {
            self.handle_caret_change(d);
        }) || if let Some(e) = event.detail.raw().downcast_ref::<MouseWheelDetail>() {
            self.handle_default_mouse_wheel(e)
        } else {
            false
        }
    }

    fn draw(&self, canvas: &Canvas) {
        let mut paint = Paint::default();
        paint.set_color(parse_hex_color(BACKGROUND_COLOR).unwrap());

        let mut indicator_paint = Paint::default();
        indicator_paint.set_color(parse_hex_color(INDICATOR_COLOR).unwrap());

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