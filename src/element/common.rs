use crate::base::{EventContext, Rect};
use crate::canvas_util::CanvasHelper;
use crate::element::ElementWeak;
use crate::event::{MouseDownEvent, MouseMoveEvent, MouseUpEvent};
use crate::render::RenderFn;
use skia_safe::{Color, Paint, PaintStyle};
use std::any::Any;
use crate::element::scroll::ScrollBarStrategy;

pub enum ScrollBarDirection {
    Horizontal,
    Vertical,
}
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
    scroll_callback: Box<dyn FnMut()>,
}

impl ScrollBar {
    pub fn new_horizontal() -> Self {
        Self::new(ScrollBarDirection::Horizontal)
    }

    pub fn new_vertical() -> Self {
        Self::new(ScrollBarDirection::Vertical)
    }

    fn new(direction: ScrollBarDirection) -> Self {
        Self {
            direction,
            thickness: 14.0,
            length: 0.0,
            scroll_length: 0.0,
            thumb_rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            scroll_offset: 0.0,
            scroll_begin_info: None,
            padding: 0.0,
            track_rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            strategy: ScrollBarStrategy::Auto,
            scroll_callback: Box::new(|| {}),
            thumb_background_color: Color::from_rgb(0xC1, 0xC1, 0xC1),
            track_background_color: Color::from_rgb(0xE1, 0xE1, 0xE1),
        }
    }

    fn is_scrollable(&self) -> bool {
        self.scroll_length > self.length
    }

    pub fn get_max_scroll_offset(&self) -> f32 {
        (self.scroll_length - self.length).max(0.0)
    }

    pub fn set_scroll_callback<F: FnMut() + 'static>(&mut self, mut cb: F) {
        self.scroll_callback = Box::new(cb);
    }

    pub fn set_length(&mut self, length: f32, scroll_length: f32, view_length: f32) {
        self.length = length;
        self.scroll_length = scroll_length;
        let visible_thickness = match self.strategy {
            ScrollBarStrategy::Never => {
                0.0
            }
            ScrollBarStrategy::Auto => {
                if self.is_scrollable() {
                    self.thickness
                } else {
                    0.0
                }
            }
            ScrollBarStrategy::Always => {
                self.thickness
            }
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

    pub fn on_event(
        &mut self,
        event: & Box<&mut dyn Any>,
        ctx: &mut EventContext<ElementWeak>,
    ) -> bool {
        if let Some(e) = event.downcast_ref::<MouseDownEvent>() {
            let d = e.0;
            self.on_mouse_down(d.offset_x, d.offset_y)
        } else if let Some(e) = event.downcast_ref::<MouseUpEvent>() {
            let d = e.0;
            self.on_mouse_up(d.offset_x, d.offset_y)
        } else if let Some(e) = event.downcast_ref::<MouseMoveEvent>() {
            let d = e.0;
            self.on_mouse_move(d.offset_x, d.offset_y)
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
        } else {
            self.track_rect.contains_point(x, y)
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

    pub fn on_mouse_up(&mut self, x: f32, y: f32) -> bool {
        if self.scroll_begin_info.is_some() {
            self.scroll_begin_info = None;
            true
        } else {
            false
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

        // println!("render scrollbar: {:?} {:?}", thumb_rect, rect);

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
            (self.scroll_callback)();
        }
    }
}
