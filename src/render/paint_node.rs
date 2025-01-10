use skia_bindings::SkPaint_Style;
use skia_safe::{Canvas, Color, Image, Paint, Path, Rect};
use crate::paint::RenderNode;
use crate::render::RenderFn;
use crate::some_or_return;
use crate::style::border_path::BorderPath;
use crate::style::ColorHelper;

pub struct PaintNode {
    pub children_viewport: Option<Rect>,
    pub border_path: [Path; 4],
    pub border_box_path: Path,
    pub layer_x: f32,
    pub layer_y: f32,
    pub border_color: [Color; 4],
    pub render_fn: Option<RenderFn>,
    pub background_image: Option<Image>,
    pub background_color: Color,
    pub children: Vec<PaintNode>,
    pub border_width: (f32, f32, f32, f32),
    pub width: f32,
    pub height: f32,
}

impl PaintNode {
    pub fn draw_background(&self, canvas: &Canvas) {
        // let pi = some_or_return!(&self.paint_info);
        if let Some(img) = &self.background_image {
            canvas.draw_image(img, (0.0, 0.0), Some(&Paint::default()));
        } else if !self.background_color.is_transparent() {
            let mut paint = Paint::default();
            let (bd_top, bd_right, bd_bottom, bd_left) = self.border_width;
            let width = self.width;
            let height = self.height;
            let rect = Rect::new(bd_left, bd_top, width - bd_right, height - bd_bottom);

            paint.set_color(self.background_color);
            paint.set_style(SkPaint_Style::Fill);
            canvas.draw_rect(&rect, &paint);
        }
    }

    pub fn draw_border(&mut self, canvas: &Canvas) {
        let paths = &self.border_path;
        let color = &self.border_color;
        for i in 0..4 {
            let p = &paths[i];
            if !p.is_empty() {
                let mut paint = Paint::default();
                paint.set_style(SkPaint_Style::Fill);
                paint.set_anti_alias(true);
                paint.set_color(color[i]);
                canvas.draw_path(&p, &paint);
            }
        }
    }
}