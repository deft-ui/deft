pub mod editable;
pub mod image_object;
pub mod scrollable;
mod svg_object;

use crate as deft;
use crate::base::{EventContext, Rect};
use crate::canvas_util::CanvasHelper;
use crate::element::scroll::ScrollBarStrategy;
use crate::element::ElementWeak;
use crate::event::{Event, MouseDownEvent, MouseMoveEvent, MouseUpEvent, MouseWheelEvent};
use crate::render::RenderFn;
use crate::timer::{set_interval, set_timeout, TimerHandle};
use deft_macros::mrc_object;
use skia_safe::{Color, Paint, PaintStyle};

pub enum ScrollBarDirection {
    Horizontal,
    Vertical,
}

#[mrc_object]
pub struct ScrollBar {
    direction: ScrollBarDirection,
    length: f32,
    scroll_length: f32,
    thickness: f32,
    thumb_rect: Rect,
    thumb_background_color: Color,
    track_rect: Rect,
    track_background_color: Color,
    scroll_offset: f32,
    /// (mouse_offset, scroll_offset)
    scroll_begin_info: Option<(f32, f32)>,
    padding: f32,
    strategy: ScrollBarStrategy,
    scroll_callback: Box<dyn FnMut(f32)>,
    auto_scroll_timer: Option<TimerHandle>,
}

impl ScrollBar {
    pub fn new_horizontal() -> Self {
        Self::new(ScrollBarDirection::Horizontal)
    }

    pub fn new_vertical() -> Self {
        Self::new(ScrollBarDirection::Vertical)
    }

    fn new(direction: ScrollBarDirection) -> Self {
        ScrollBarData {
            direction,
            thickness: 14.0,
            length: 0.0,
            scroll_length: 0.0,
            thumb_rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            scroll_offset: 0.0,
            scroll_begin_info: None,
            padding: 0.0,
            track_rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            strategy: ScrollBarStrategy::Never,
            scroll_callback: Box::new(|_| {}),
            thumb_background_color: Color::from_rgb(0xC1, 0xC1, 0xC1),
            track_background_color: Color::from_rgb(0xE1, 0xE1, 0xE1),
            auto_scroll_timer: None,
        }
        .to_ref()
    }

    pub fn is_scrollable(&self) -> bool {
        self.scroll_length > self.length
    }

    pub fn get_max_scroll_offset(&self) -> f32 {
        (self.scroll_length - self.length).max(0.0)
    }

    pub fn set_scroll_callback<F: FnMut(f32) + 'static>(&mut self, cb: F) {
        self.scroll_callback = Box::new(cb);
    }

    pub fn set_length(&mut self, length: f32, scroll_length: f32, view_length: f32) {
        self.length = length;
        self.scroll_length = scroll_length;
        let visible_thickness = match self.strategy {
            ScrollBarStrategy::Never => 0.0,
            ScrollBarStrategy::Auto => {
                if self.is_scrollable() {
                    self.thickness
                } else {
                    0.0
                }
            }
            ScrollBarStrategy::Always => self.thickness,
        };
        self.padding = view_length - visible_thickness;
        let (width, height, x, y) = match self.direction {
            ScrollBarDirection::Horizontal => (self.length, visible_thickness, 0.0, self.padding),
            ScrollBarDirection::Vertical => (visible_thickness, self.length, self.padding, 0.0),
        };
        self.track_rect = Rect::new(x, y, width, height);
        self.update_scroll_offset(self.scroll_offset.min(self.get_max_scroll_offset()));
        self.update_thumb_rect();
    }

    pub fn set_thickness(&mut self, thickness: f32) {
        self.thickness = thickness;
    }

    pub fn set_strategy(&mut self, strategy: ScrollBarStrategy) {
        self.strategy = strategy;
    }

    pub fn thickness(&self) -> f32 {
        self.thickness
    }

    pub fn visible_thickness(&self) -> f32 {
        match self.direction {
            ScrollBarDirection::Horizontal => self.track_rect.height,
            ScrollBarDirection::Vertical => self.track_rect.width,
        }
    }

    pub fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    pub fn set_scroll_offset(&mut self, scroll_offset: f32) {
        self.update_scroll_offset(scroll_offset);
    }

    pub fn scroll_into_view(&mut self, offset: f32, length: f32) -> bool {
        let start = offset - self.scroll_offset;
        if start < 0.0 {
            self.update_scroll_offset(offset);
            return true;
        }
        let end = offset + length - self.scroll_offset;
        if end > self.length {
            self.update_scroll_offset(self.scroll_offset + end - self.length);
            return true;
        }
        false
    }

    fn update_thumb_rect(&mut self) {
        let thumb_length = f32::max(20.0, self.length / self.scroll_length * self.length);
        let thumb_offset =
            self.scroll_offset / (self.scroll_length - self.length) * (self.length - thumb_length);
        let thumb_offset = thumb_offset.max(0.0).min(self.length - thumb_length);
        let (x, y, thumb_width, thumb_height) = match self.direction {
            ScrollBarDirection::Horizontal => {
                (thumb_offset, self.padding, thumb_length, self.thickness)
            }
            ScrollBarDirection::Vertical => {
                (self.padding, thumb_offset, self.thickness, thumb_length)
            }
        };
        self.thumb_rect = Rect::new(x, y, thumb_width, thumb_height);
    }

    pub fn is_mouse_over(&self, x: f32, y: f32) -> bool {
        self.track_rect.contains_point(x, y)
    }

    pub fn on_event(&mut self, event: &Event, _ctx: &mut EventContext<ElementWeak>) -> bool {
        if let Some(e) = MouseDownEvent::cast(event) {
            let d = e.0;
            self.on_mouse_down(d.offset_x, d.offset_y)
        } else if let Some(e) = MouseUpEvent::cast(event) {
            let d = e.0;
            self.on_mouse_up(d.offset_x, d.offset_y)
        } else if let Some(e) = MouseMoveEvent::cast(event) {
            let d = e.0;
            self.on_mouse_move(d.offset_x, d.offset_y)
        } else if let Some(e) = MouseWheelEvent::cast(event) {
            if self.is_scrollable() {
                let new_scroll_top = self.scroll_offset - 40.0 * e.rows;
                self.update_scroll_offset(new_scroll_top);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> bool {
        if self.thumb_rect.contains_point(x, y) {
            let mouse_offset = match self.direction {
                ScrollBarDirection::Horizontal => x,
                ScrollBarDirection::Vertical => y,
            };
            self.scroll_begin_info = Some((mouse_offset, self.scroll_offset));
            true
        } else if self.track_rect.contains_point(x, y) {
            let is_up = match self.direction {
                ScrollBarDirection::Horizontal => x < self.thumb_rect.left,
                ScrollBarDirection::Vertical => y < self.thumb_rect.top,
            };
            let pages = if is_up { -1.0 } else { 1.0 };
            self.scroll_page(pages);
            self.start_auto_scroll(pages * 3.0);
            true
        } else {
            false
        }
    }

    pub fn on_mouse_move(&mut self, x: f32, y: f32) -> bool {
        if let Some((mouse_offset, scroll_offset)) = self.scroll_begin_info {
            let (move_distance, thumb_length) = match self.direction {
                ScrollBarDirection::Horizontal => (x - mouse_offset, self.thumb_rect.width),
                ScrollBarDirection::Vertical => (y - mouse_offset, self.thumb_rect.height),
            };
            let scroll_distance =
                move_distance / (self.length - thumb_length) * (self.scroll_length - self.length);
            self.update_scroll_offset(scroll_offset + scroll_distance);
            self.update_thumb_rect();
            true
        } else {
            false
        }
    }

    pub fn on_mouse_up(&mut self, _x: f32, _y: f32) -> bool {
        if self.scroll_begin_info.is_some() {
            self.scroll_begin_info = None;
            true
        } else {
            self.auto_scroll_timer.take().is_some()
        }
    }

    pub fn set_track_background_color(&mut self, color: Color) {
        self.track_background_color = color;
    }

    pub fn set_thumb_background_color(&mut self, color: Color) {
        self.thumb_background_color = color;
    }

    pub fn render(&self) -> RenderFn {
        let bar_rect = self.track_rect.clone();
        if bar_rect.is_empty() {
            return RenderFn::empty();
        }

        let mut bar_paint = Paint::default();
        bar_paint.set_style(PaintStyle::Fill);
        bar_paint.set_color(self.track_background_color);

        let thumb_rect = self.thumb_rect.clone();
        let mut thumb_paint = Paint::default();
        thumb_paint.set_style(PaintStyle::Fill);
        thumb_paint.set_color(self.thumb_background_color);

        // println!("render scrollbar: {:?} {:?}", thumb_rect, bar_rect);

        RenderFn::new(move |painter| {
            painter.canvas.session(|c| {
                c.draw_rect(&bar_rect.to_skia_rect(), &bar_paint);
                c.draw_rect(&thumb_rect.to_skia_rect(), &thumb_paint);
            });
        })
    }

    fn update_scroll_offset(&mut self, scroll_offset: f32) {
        let new_scroll_offset = scroll_offset.clamp(0.0, self.get_max_scroll_offset());
        if self.scroll_offset != new_scroll_offset {
            self.scroll_offset = new_scroll_offset;
            self.update_thumb_rect();
            (self.scroll_callback)(new_scroll_offset);
        }
    }

    fn scroll_page(&mut self, pages: f32) {
        let new_scroll_offset = self.scroll_offset + self.length * pages;
        self.update_scroll_offset(new_scroll_offset);
    }

    fn start_auto_scroll(&mut self, speed: f32) {
        let mut me = self.clone();
        let timer_handle = set_timeout(
            move || {
                let interval_timer = {
                    let me = me.clone();
                    set_interval(
                        move || {
                            me.clone().scroll_page(speed / 10.0);
                        },
                        100,
                    )
                };
                me.auto_scroll_timer = Some(interval_timer);
            },
            300,
        );
        self.auto_scroll_timer = Some(timer_handle);
    }
}
