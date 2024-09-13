use skia_safe::{Canvas, Font, Paint, TextBlob};
use crate::base::{Rect, TextAlign, VerticalAlign};

pub trait CanvasHelper {
    fn draw_text(&self, rect: &Rect, text: &str, font: &Font, paint: &Paint, align: TextAlign, vertical_align: VerticalAlign);
    fn session<F: FnOnce(&Self)>(&self, callback: F);
}

impl CanvasHelper for Canvas {
    fn draw_text(&self, rect: &Rect, text: &str, font: &Font, paint: &Paint, align: TextAlign, vertical_align: VerticalAlign) {
        let text_blob = TextBlob::from_str(text, font).unwrap();
        let (_, bounds) = font.measure_text(text, Some(paint));
        let x = match align {
            TextAlign::Left => {
                rect.x
            }
            TextAlign::Right => {
                rect.right() - bounds.width()
            }
            TextAlign::Center => {
                rect.x + (rect.width - bounds.width()) / 2.0
            }
        };
        let y = match vertical_align {
            VerticalAlign::Top => {
                rect.y + bounds.height()
            }
            VerticalAlign::Bottom => {
                rect.bottom()
            }
            VerticalAlign::Middle => {
                rect.y + (rect.height - bounds.height()) / 2.0 + bounds.height()
            }
        };
        self.draw_text_blob(&text_blob, (x, y), &paint);
    }

    fn session<F: FnOnce(&Self)>(&self, callback: F) {
        self.save();
        callback(&self);
        self.restore();
    }
}