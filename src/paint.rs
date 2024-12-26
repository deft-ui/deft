use std::cell::Cell;
use std::collections::HashMap;
use measure_time::print_time;
use skia_bindings::{SkClipOp, SkPathOp};
use skia_safe::{IRect, Image, Path, Rect};
use skia_safe::Canvas;
use crate::element::Element;
use crate::mrc::Mrc;
use crate::renderer::CpuRenderer;

thread_local! {
    pub static NEXT_UNIQUE_RECT_ID: Cell<u64> = Cell::new(1);
}

pub struct RenderTree {
    pub invalid_rects_list: Vec<InvalidArea>,
    pub nodes: Vec<RenderNode>,
}

impl RenderTree {
    pub fn new() -> Self {
        Self {
            invalid_rects_list: vec![],
            nodes: vec![],
        }
    }
}

pub struct RenderNode {
    pub element: Element,
    pub invalid_rects_idx: usize,
    pub snapshot: Option<(Rect, Image)>,
}

pub enum PaintElement {
    Element(Element),
    Layer(Vec<PaintElement>),
}

#[derive(PartialEq, Debug, Clone)]
pub enum InvalidArea {
    Full,
    Partial(InvalidRects),
    None,
}

#[derive(PartialEq, Debug, Clone)]
pub struct UniqueRect {
    id: u64,
    pub rect: Rect,
}

impl UniqueRect {
    pub fn from_rect(rect: Rect) -> Self {
        let id = NEXT_UNIQUE_RECT_ID.get();
        NEXT_UNIQUE_RECT_ID.replace(id + 1);
        Self {
            id,
            rect,
        }
    }
}

impl InvalidArea {
    pub fn add_rect(&mut self, rect: Option<&Rect>) {
        match rect {
            None => {
                *self = InvalidArea::Full;
            }
            Some(rect) => {
                self.add_unique_rect(&UniqueRect::from_rect(rect.clone()));
            }
        }
    }
    pub fn add_unique_rect(&mut self, unique_rect: &UniqueRect) {
        match self {
            InvalidArea::Full => {}
            InvalidArea::Partial(p) => {
                p.add_unique_rect(&unique_rect);
            }
            InvalidArea::None => {
                let mut p = InvalidRects::new();
                p.add_unique_rect(unique_rect);
                *self = InvalidArea::Partial(p);
            }
        }
    }
    pub fn remove_unique_rect(&mut self, unique_rect: &UniqueRect) {
        match self {
            InvalidArea::Full => {}
            InvalidArea::Partial(ir) => {
                ir.remove_unique_rect(unique_rect);
            }
            InvalidArea::None => {}
        }
    }
    pub fn offset(&mut self, x: f32, y: f32) {
        match self {
            InvalidArea::Full => {}
            InvalidArea::Partial(p) => {
                p.offset(x, y);
            }
            InvalidArea::None => {}
        }
    }
}

pub trait Painter {
    fn set_invalid_area(&mut self, invalid_area: InvalidArea);
    fn is_visible_origin(&self, bounds: &Rect) -> bool;
}

pub struct SkiaPainter<'a> {
    invalid_area: InvalidArea,
    canvas: &'a Canvas,
}

impl<'a> SkiaPainter<'a> {
    pub fn new(canvas: &'a Canvas) -> SkiaPainter {
        Self {
            invalid_area: InvalidArea::Full,
            canvas,
        }
    }
}

impl<'a> Painter for SkiaPainter<'a> {
    fn set_invalid_area(&mut self, invalid_area: InvalidArea) {
        if let InvalidArea::Partial(rects) = &invalid_area {
            let mut path = Path::new();
            for r in rects.rects.values() {
                path.add_rect(r, None);
            }
            self.canvas.clip_path(&path, SkClipOp::Intersect, false);
        }
        self.invalid_area = invalid_area;
    }
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
    rects: HashMap<u64, Rect>,
}

impl InvalidRects {
    pub fn new() -> InvalidRects {
        Self { rects: HashMap::new() }
    }
    pub fn add_rect(&mut self, rect: &Rect) {
        self.add_unique_rect(&UniqueRect::from_rect(rect.clone()))
    }
    pub fn add_unique_rect(&mut self, rect: &UniqueRect) {
        self.rects.insert(rect.id, rect.rect);
    }
    pub fn remove_unique_rect(&mut self, rect: &UniqueRect) {
        self.rects.remove(&rect.id);
    }
    pub fn offset(&mut self, x: f32, y: f32) {
        for (_, r) in &mut self.rects {
            r.offset((x, y));
        }
    }
    fn has_intersects(&self, rect: &Rect) -> bool {
        for r in self.rects.values() {
            let intersect = f32::min(r.right, rect.right) >= f32::max(r.left(), rect.left())
                && f32::min(r.bottom, rect.bottom) >= f32::max(r.top, rect.top);
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

