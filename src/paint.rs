use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use measure_time::print_time;
use skia_bindings::{SkClipOp, SkPaint_Style, SkPathOp};
use skia_safe::{Color, IRect, Image, Matrix, Paint, Path, Rect};
use skia_safe::Canvas;
use crate::base::{Id, IdKey};
use crate::element::Element;
use crate::mrc::Mrc;
use crate::render::RenderFn;
use crate::renderer::CpuRenderer;
use crate::style::ColorHelper;

thread_local! {
    pub static NEXT_UNIQUE_RECT_ID: Cell<u64> = Cell::new(1);
    pub static SNAPSHOT_ID_KEY: IdKey = IdKey::new();
}

#[derive(Clone)]
pub struct SnapshotManager {
    store: Arc<Mutex<HashMap<u32, Snapshot>>>,
}

impl SnapshotManager {
    pub fn new() -> SnapshotManager {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub fn insert(&self, id: u32, snapshot: Snapshot) {
        let mut store = self.store.lock().unwrap();
        store.insert(id, snapshot);
    }
    pub fn remove(&self, id: u32) -> Option<Snapshot> {
        let mut store = self.store.lock().unwrap();
        store.remove(&id)
    }
}

#[derive(Clone)]
pub struct Snapshot {
    id: Id<Snapshot>,
    pub rect: Rect,
    pub image: Image,
}

impl Snapshot {
    pub fn new(rect: Rect, image: Image) -> Self {
        let id = Id::next(&SNAPSHOT_ID_KEY);
        println!("Creating snapshot: {}", id);
        Self { id, rect, image }
    }
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        println!("Dropping snapshot: {}", self.id);
    }
}

pub struct RenderTree {
    pub viewport: Rect,
    pub invalid_rects_list: Vec<InvalidArea>,
    pub nodes: Vec<RenderNode>,
    pub ops: Vec<RenderOp>,
}

impl RenderTree {
    pub fn new() -> Self {
        Self {
            viewport: Rect::default(),
            invalid_rects_list: vec![],
            nodes: vec![],
            ops: vec![],
        }
    }
}

pub enum RenderOp {
    Render(usize),
    Finish(usize),
}

pub struct RenderNode {
    pub element_id: u32,
    pub bounds: Rect,
    pub origin_bounds: Rect,
    pub transformed_bounds: Rect,
    pub total_matrix: Matrix,
    pub invalid_rects_idx: usize,
    pub children_invalid_rects_idx: usize,
    pub need_snapshot: bool,
    pub parent_translate: Option<(f32, f32)>,
    pub parent_clip_path: Option<Path>,
    pub border_box_path: Path,
    pub border_width: (f32, f32, f32, f32),
    pub border_paths: [Path; 4],
    pub border_color: [Color; 4],
    pub render_fn: Option<RenderFn>,
    pub background_image: Option<Image>,
    pub background_color: Color,
    pub children_viewport: Rect,
    // relative bounds
    pub reuse_bounds: Option<(f32, f32, Rect)>
}

impl RenderNode {
    pub fn draw_background(&self, canvas: &Canvas) {
        if let Some(img) = &self.background_image {
            canvas.draw_image(img, (0.0, 0.0), Some(&Paint::default()));
        } else if !self.background_color.is_transparent() {
            let mut paint = Paint::default();
            let (bd_top, bd_right, bd_bottom, bd_left) = self.border_width;
            let rect = Rect::new(bd_left, bd_top, self.bounds.width() - bd_right, self.bounds.height() - bd_bottom);

            paint.set_color(self.background_color);
            paint.set_style(SkPaint_Style::Fill);
            canvas.draw_rect(&rect, &paint);
        }
    }

    pub fn draw_border(&self, canvas: &Canvas) {
        let paths = &self.border_paths;
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

pub enum PaintElement {
    Element(Element),
    Layer(Vec<PaintElement>),
}

#[derive(PartialEq, Debug, Clone)]
pub enum InvalidArea {
    Full,
    Partial(PartialInvalidArea),
    None,
}

#[derive(PartialEq, Debug, Clone)]
pub struct InvalidRects {
    is_full: bool,
    rects: Vec<Rect>,
}

impl Default for InvalidRects {
    fn default() -> Self {
        InvalidRects {
            is_full: true,
            rects: vec![],
        }
    }
}

impl InvalidRects {
    pub fn has_intersects(&self, rect: &Rect) -> bool {
        if self.is_full {
            return true;
        }
        for r in &self.rects {
            let intersect = f32::min(r.right, rect.right) >= f32::max(r.left(), rect.left())
                && f32::min(r.bottom, rect.bottom) >= f32::max(r.top, rect.top);
            if intersect {
                return true;
            }
        }
        false
    }
    pub fn to_path(&self, viewport: Rect) -> Path {
        let mut path = Path::new();
        if self.is_full {
            path.add_rect(viewport, None);
        } else {
            for r in &self.rects {
                path.add_rect(r, None);
            }
        }
        path
    }
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
                let mut p = PartialInvalidArea::new();
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

    pub fn build(&self, viewport: Rect) -> InvalidRects {
        let mut rects = Vec::new();
        let mut is_full = false;
        match self {
            InvalidArea::Full => {
                rects.push(viewport);
            }
            InvalidArea::Partial(pia) => {
                //Too many rect may make a poor performance
                if pia.rects.len() > 100 {
                    rects.push(viewport);
                } else {
                    for (_,r) in &pia.rects {
                        rects.push(*r);
                    }
                }
            }
            InvalidArea::None => {}
        }
        InvalidRects { is_full, rects }
    }

}

pub trait Painter {
    fn set_invalid_rects(&mut self, invalid_area: InvalidRects);
    fn is_visible_origin(&self, bounds: &Rect) -> bool;
}

pub struct SkiaPainter<'a> {
    invalid_rects: InvalidRects,
    canvas: &'a Canvas,
}

impl<'a> SkiaPainter<'a> {
    pub fn new(canvas: &'a Canvas) -> SkiaPainter {
        Self {
            invalid_rects: InvalidRects::default(),
            canvas,
        }
    }
}

impl<'a> Painter for SkiaPainter<'a> {
    fn set_invalid_rects(&mut self, invalid_rects: InvalidRects) {
        if !invalid_rects.is_full {
            let mut path = Path::new();
            for r in &invalid_rects.rects {
                path.add_rect(r, None);
            }
            self.canvas.clip_path(&path, SkClipOp::Intersect, false);
        }
        self.invalid_rects = invalid_rects;
    }
    fn is_visible_origin(&self, bounds: &Rect) -> bool {
        self.invalid_rects.has_intersects(bounds)
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct PartialInvalidArea {
    rects: HashMap<u64, Rect>,
}

impl PartialInvalidArea {
    pub fn new() -> PartialInvalidArea {
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
}

#[test]
pub fn test_visible() {
    let mut render = CpuRenderer::new(100, 100);
    let mut area = InvalidArea::None;
    area.add_rect(Some(&Rect::from_xywh(0.0, 0.0, 100.0, 100.0)));
    area.add_rect(Some(&Rect::from_xywh(10.0, 70.0, 100.0, 100.0)));
    area.add_rect(Some(&Rect::from_xywh(20.0, 40.0, 100.0, 100.0)));
    area.add_rect(Some(&Rect::from_xywh(30.0, 40.0, 100.0, 100.0)));
    area.add_rect(Some(&Rect::from_xywh(40.0, 40.0, 100.0, 100.0)));
    area.add_rect(Some(&Rect::from_xywh(50.0, 40.0, 100.0, 100.0)));

    let rects = area.build(Rect::from_xywh(0.0, 0.0, 1000.0, 1000.0));
    print_time!("check time");
    for _ in 0..20000 {
        rects.has_intersects(&Rect::new(40.0, 20.0, 80.0, 50.0));
    }
}

