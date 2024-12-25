use measure_time::print_time;
use skia_bindings::{SkClipOp, SkPathOp};
use skia_safe::{Path, Rect};
use skia_safe::Canvas;
use crate::renderer::CpuRenderer;

#[derive(PartialEq, Debug, Clone)]
pub enum InvalidArea {
    Full,
    Partial(InvalidRects),
    None,
}

pub trait Painter {
    fn is_visible_origin(&self, bounds: &Rect) -> bool;
}

pub struct SkiaPainter<'a> {
    invalid_area: InvalidArea,
    canvas: &'a Canvas,
}

impl<'a> SkiaPainter<'a> {
    pub fn new(canvas: &'a Canvas, invalid_area: InvalidArea) -> SkiaPainter {
        if let InvalidArea::Partial(rects) = &invalid_area {
            let mut path = Path::new();
            for r in &rects.rects {
                path.add_rect(r, None);
            }
            canvas.clip_path(&path, SkClipOp::Intersect, false);
        }
        Self {
            invalid_area,
            canvas,
        }
    }
}

impl<'a> Painter for SkiaPainter<'a> {
    fn is_visible_origin(&self, bounds: &Rect) -> bool {
        match &self.invalid_area {
            //TODO optimize
            InvalidArea::Full => { true }
            InvalidArea::Partial(p) => {
                p.has_intersects(bounds)
            }
            InvalidArea::None => {
                false
            }
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct InvalidRects {
    rects: Vec<Rect>,
}

impl InvalidRects {
    pub fn new() -> InvalidRects {
        Self { rects: Vec::new() }
    }
    pub fn add_rect(&mut self, rect: &Rect) {
        self.rects.push(rect.clone());
    }
    fn has_intersects(&self, rect: &Rect) -> bool {
        for r in &self.rects {
            let intersect = f32::min(r.right, rect.right) > f32::max(r.left(), rect.left())
                && f32::min(r.bottom, rect.bottom()) > f32::max(r.top, rect.top);
            if intersect {
                return true;
            }
        }
        false
    }
}

#[test]
pub fn test_visible() {
    let mut render = CpuRenderer::new(100, 100);
    let mut rects = InvalidRects::new();
    rects.add_rect(&Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
    rects.add_rect(&Rect::from_xywh(10.0, 70.0, 100.0, 100.0));
    rects.add_rect(&Rect::from_xywh(20.0, 40.0, 100.0, 100.0));
    rects.add_rect(&Rect::from_xywh(30.0, 40.0, 100.0, 100.0));
    rects.add_rect(&Rect::from_xywh(40.0, 40.0, 100.0, 100.0));
    rects.add_rect(&Rect::from_xywh(50.0, 40.0, 100.0, 100.0));

    print_time!("check time");
    for _ in 0..20000 {
        rects.has_intersects(&Rect::new(40.0, 20.0, 80.0, 50.0));
    }
}

