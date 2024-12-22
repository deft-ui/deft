use skia_safe::{Canvas, FontMetrics, Point, Rect};
use crate::base;
use crate::base::{TextAlign, VerticalAlign};
use crate::canvas_util::CanvasHelper;
use crate::element::text::text_paragraph::TextParams;

pub struct SimpleTextParagraph {
    params: TextParams,
    text: String,
    line_height: f32,
    char_bounds: Vec<(char,f32, Rect, Point)>,
    height: f32,
    max_intrinsic_width: f32,
    font_metrics: FontMetrics,
}

impl SimpleTextParagraph {
    pub fn new(text: &str, params: &TextParams) -> Self {
        let chars_count = text.chars().count();
        let mut char_bounds = Vec::with_capacity(chars_count);
        for c in text.chars() {
            let (c_w, r) = params.font.measure_str(c.to_string(), Some(&params.paint));
            char_bounds.push((c, c_w, r, Point::new(0.0, 0.0)));
        }
        let (_, font_metrics) = params.font.metrics();
        let font_line_height = font_metrics.descent - font_metrics.ascent + font_metrics.leading;
        let line_height = params.line_height.unwrap_or(font_line_height);
        Self {
            text: text.to_string(),
            char_bounds,
            max_intrinsic_width: 0.0,
            height: 0.0,
            line_height,
            params: params.clone(),
            font_metrics,
        }
    }

    pub fn layout(&mut self, available_width: f32) {
        let mut left = 0.0;
        let mut top = 0.0;
        let mut max_intrinsic_width = 0.0;
        let line_height = self.line_height;
        let mut lines = 1;
        for (_, cw, cb, point) in &mut self.char_bounds {
            let width = *cw;
            if left > 0.0 && !available_width.is_nan() {
                let right = left + width;
                if right > available_width {
                    left = 0.0;
                    top += line_height;
                    lines += 1;
                }
            }
            point.x = left;
            point.y = top;

            left += width;
            max_intrinsic_width = f32::max(max_intrinsic_width, left);
        }
        self.height = lines as f32 * line_height;
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn max_intrinsic_width(&self) -> f32 {
        self.max_intrinsic_width
    }

    pub fn get_char_bounds(&mut self, char_offset: usize) -> Option<Rect> {
        if let Some((c, cw, r, p)) = &self.char_bounds.get(char_offset) {
            Some(r.with_offset(*p))
        } else {
            None
        }
    }

    pub fn get_char_offset_at_coordinate(&self, coord: (f32, f32)) -> usize {
        let (x, y) = coord;
        let mut idx = 0;
        for (_, cw, c, p) in &self.char_bounds {
            let c = c.with_offset(*p);
            if x > c.left && x < c.right && y > c.top && y < c.bottom {
                return idx
            }
            idx += 1;
        }
        return idx - 1;
    }

    pub fn get_soft_line_height(&self, char_offset: usize) -> f32 {
        //TODO fix
        if let Some((_,cw, c, _)) = self.char_bounds.get(char_offset) {
            c.height()
        } else {
            0.0
        }
    }

    pub fn paint(&self, canvas: &Canvas, p: impl Into<Point>) {
        let base_line = -self.font_metrics.ascent + self.font_metrics.leading;
        let p = p.into();
        canvas.translate((p.x, p.y + base_line));
        for (c, cw, b, p) in &self.char_bounds {
            canvas.draw_str(&c.to_string(), *p, &self.params.font, &self.params.paint);
        }
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

}