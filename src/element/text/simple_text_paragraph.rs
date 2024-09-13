use skia_safe::{Canvas, Point, Rect};
use crate::base;
use crate::base::{TextAlign, VerticalAlign};
use crate::canvas_util::CanvasHelper;
use crate::element::text::text_paragraph::TextParams;

pub struct SimpleTextParagraph {
    params: TextParams,
    text: String,
    char_bounds: Vec<(char, Rect)>,
    height: f32,
    max_intrinsic_width: f32,
}

impl SimpleTextParagraph {
    pub fn new(text: &str, params: &TextParams) -> Self {
        let chars_count = text.chars().count();
        let mut char_bounds = Vec::with_capacity(chars_count);
        for c in text.chars() {
            let (_, r) = params.font.measure_str(c.to_string(), Some(&params.paint));
            char_bounds.push((c, r));
        }
        Self {
            text: text.to_string(),
            char_bounds,
            max_intrinsic_width: 0.0,
            height: 0.0,
            params: params.clone()
        }
    }

    pub fn layout(&mut self, available_width: f32) {
        let mut left = 0.0;
        let mut top = 0.0;
        let mut last_line_height = 0.0;
        let mut max_intrinsic_width = 0.0;
        for (_, cb) in &mut self.char_bounds {
            let width = cb.width();
            let height = cb.height();
            if left > 0.0 && !available_width.is_nan() {
                let right = left + width;
                if right > available_width {
                    left = 0.0;
                    top += last_line_height;
                }
            }
            cb.left = left;
            cb.right = left + width;
            cb.top = top;
            cb.bottom = top + height;

            left += width;
            max_intrinsic_width = f32::max(max_intrinsic_width, left);
            //TODO fix last_line_height
            last_line_height = height;
        }
        self.height = top + last_line_height;
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn max_intrinsic_width(&self) -> f32 {
        self.max_intrinsic_width
    }

    pub fn get_char_bounds(&mut self, char_offset: usize) -> Option<Rect> {
        if let Some((c, r)) = &self.char_bounds.get(char_offset) {
            Some(r.clone())
        } else {
            None
        }
    }

    pub fn get_char_offset_at_coordinate(&self, coord: (f32, f32)) -> usize {
        let (x, y) = coord;
        let mut idx = 0;
        for (_, c) in &self.char_bounds {
            if x > c.left && x < c.right && y > c.top && y < c.bottom {
                return idx
            }
            idx += 1;
        }
        return idx - 1;
    }

    pub fn get_soft_line_height(&self, char_offset: usize) -> f32 {
        //TODO fix
        if let Some((_, c)) = self.char_bounds.get(char_offset) {
            c.height()
        } else {
            0.0
        }
    }

    pub fn paint(&self, canvas: &Canvas, p: impl Into<Point>) {
        canvas.translate(p);
        for (c, b) in &self.char_bounds {
            let rect = base::Rect::new(b.left, b.top, b.width(), b.height());
            canvas.draw_text(&rect, &c.to_string(), &self.params.font, &self.params.paint, TextAlign::Left, VerticalAlign::Bottom);
        }
        canvas.restore();
    }

}