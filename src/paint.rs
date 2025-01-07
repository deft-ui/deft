use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use measure_time::print_time;
use skia_bindings::{SkClipOp, SkPaint_Style, SkPathOp};
use skia_safe::{scalar, ClipOp, Color, IRect, Image, Matrix, Paint, Path, Point, Rect, Vector};
use skia_safe::Canvas;
use skia_window::layer::Layer;
use crate::base::{Id, IdKey};
use crate::element::Element;
use crate::mrc::Mrc;
use crate::render::RenderFn;
use crate::renderer::CpuRenderer;
use crate::{some_or_break, some_or_continue, some_or_return};
use crate::style::border_path::BorderPath;
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

    pub fn with_snapshot_mut<F: FnOnce(&mut Snapshot)>(&self, id: u32, callback: F) {
        let mut store = self.store.lock().unwrap();
        if let Some(sn) = store.get_mut(&id) {
            callback(sn);
        }
    }

    pub fn get_snapshot_image(&self, id: u32) -> Option<Image> {
        let mut store = self.store.lock().unwrap();
        let mut sn = store.get_mut(&id)?;
        Some(sn.image.as_image())
    }

    pub fn take(&self, id: u32, expected_width: usize, expected_height: usize) -> Option<Snapshot> {
        let mut store = self.store.lock().unwrap();
        let mut sn = store.remove(&id)?;
        if sn.width == expected_width && sn.height == expected_height {
            Some(sn)
        } else {
            None
        }
    }
}

pub struct Snapshot {
    id: Id<Snapshot>,
    pub image: Layer,
    pub width: usize,
    pub height: usize,
}

impl Snapshot {
    pub fn new(image: Layer, width: usize, height: usize) -> Self {
        let id = Id::next(&SNAPSHOT_ID_KEY);
        println!("Creating snapshot: {}", id);
        Self { id, image, width, height }
    }
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        println!("Dropping snapshot: {}", self.id);
    }
}

pub struct LayoutNodeMeta {
    pub children: Vec<usize>,
}

impl LayoutNodeMeta {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }
}

//TODO rename to LayoutTree?
pub struct RenderTree {
    nodes: Vec<RenderNode>,
    element2node: HashMap<u32, usize>,
    node_meta_list: Vec<LayoutNodeMeta>,
    pub ops: Vec<RenderOp>,
    pub layers: Vec<RenderLayer>,
}

pub struct RenderLayer {
    pub root: RenderLayerNode,
    pub graphic_layer: Option<Layer>,
    invalid_area: InvalidArea,
}

impl RenderLayer {
    pub fn invalid(&mut self, rect: &Rect) {
        self.invalid_area.add_rect(Some(rect));
    }
    pub fn invalid_all(&mut self) {
        self.invalid_area = InvalidArea::Full;
    }
}

pub struct RenderLayerNode {
    pub node_idx: usize,
    pub children: Vec<RenderLayerNode>,
}

impl RenderTree {
    pub fn new(predicate_count: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(predicate_count),
            node_meta_list: Vec::with_capacity(predicate_count),
            ops: Vec::with_capacity(predicate_count * 2),
            element2node: HashMap::with_capacity(predicate_count),
            layers: Vec::new(),
        }
    }

    pub fn get_by_element_id(&self, element_id: u32) -> Option<&RenderNode> {
        let node_idx = self.element2node.get(&element_id)?;
        Some(&self.nodes[*node_idx])
    }

    pub fn get_mut_by_element_id(&mut self, element_id: u32) -> Option<&mut RenderNode> {
        let node_idx = self.element2node.get(&element_id)?;
        Some(&mut self.nodes[*node_idx])
    }

    pub fn add_node(&mut self, node: RenderNode) -> usize {
        let idx = self.nodes.len();
        self.element2node.insert(node.element_id, idx);
        self.nodes.push(node);
        self.node_meta_list.push(LayoutNodeMeta::new());
        idx
    }

    pub fn rebuild_layers(&mut self, root: &mut Element) {
        let mut layer_roots = Vec::new();
        layer_roots.push(root.clone());
        let mut layers = Vec::new();
        loop {
            let mut element_root = some_or_break!(layer_roots.pop());
            let layer_idx = layers.len();
            let layer_root = self.build_layer_node(layer_idx, &mut element_root, &mut layer_roots, 0.0, 0.0).unwrap();
            layers.push(RenderLayer {
                root: layer_root,
                graphic_layer: None,
                invalid_area: InvalidArea::Full,
            });
        }
        self.layers = layers;
    }

    pub fn invalid_element(&mut self, element_id: u32) {
        let node = some_or_return!(self.get_by_element_id(element_id));
        let layer_idx = node.layer_idx;
        let bounds = Rect::from_xywh(node.layer_x, node.layer_y, node.width, node.height);
        self.layers[layer_idx].invalid(&bounds);
    }

    fn need_create_layer(element: &Element) -> bool {
        element.need_snapshot || element.style.transform.is_some()
    }

    fn build_layer_node(&mut self, layer_idx: usize, root: &mut Element, layer_roots: &mut Vec<Element>, layer_x: f32, layer_y: f32) -> Option<RenderLayerNode> {
        let node_idx = *self.element2node.get(&root.get_id())?;
        let node = &mut self.nodes[node_idx];
        node.layer_idx = layer_idx;
        node.layer_x = layer_x;
        node.layer_y = layer_y;
        let mut children = Vec::new();
        for mut c in root.get_children() {
            if Self::need_create_layer(&c) {
                layer_roots.push(c);
            } else {
                let bounds = c.get_bounds().translate(-root.scroll_left, -root.scroll_top);
                let layer_x = layer_x + bounds.x;
                let layer_y = layer_y + bounds.y;
                children.push(self.build_layer_node(layer_idx, &mut c, layer_roots, layer_x, layer_y)?);
            }
        }

        Some(RenderLayerNode {
            node_idx,
            children,
        })
    }

    pub fn update_layout_info_recurse(
        &mut self,
        root: &mut Element,
        matrix_calculator: &mut MatrixCalculator,
        bounds: Rect,
    ) {
        let node_idx = *some_or_return!(self.element2node.get(&root.get_id()));
        let origin_bounds = root.get_origin_bounds().to_skia_rect();

        let mut border_path = root.create_border_path();
        let border_box_path =  border_path.get_box_path();

        root.apply_transform(matrix_calculator);
        //TODO support overflow:visible
        matrix_calculator.intersect_clip_path(&ClipPath::from_path(border_box_path));

        let total_matrix = matrix_calculator.get_total_matrix();
        let clip_chain = matrix_calculator.get_clip_chain();


        let (transformed_bounds, _) = total_matrix.map_rect(Rect::from_xywh(0.0, 0.0, bounds.width(), bounds.height()));
        let node = &mut self.nodes[node_idx];
        node.absolute_transformed_bounds = transformed_bounds;
        node.width = bounds.width();
        node.height = bounds.height();
        node.total_matrix = total_matrix;
        node.clip_chain = clip_chain;
        node.border_path = border_path;
        for mut c in root.get_children() {
            let child_bounds = c.get_bounds().translate(-root.scroll_left, -root.scroll_top);
            matrix_calculator.save();
            matrix_calculator.translate((child_bounds.x, child_bounds.y));
            self.update_layout_info_recurse(&mut c, matrix_calculator, bounds);
            matrix_calculator.restore();
        }
    }

    pub fn bind_children(&mut self, node_idx: usize, children: Vec<usize>) {
        self.node_meta_list[node_idx].children = children;
    }

    pub fn get_node_mut(&mut self, node_idx: usize) -> Option<&mut RenderNode> {
        self.nodes.get_mut(node_idx)
    }

    pub fn get_node_mut_unchecked(&mut self, node_idx: usize) -> &mut RenderNode {
        self.nodes.get_mut(node_idx).unwrap()
    }

    pub fn nodes(&self) -> &Vec<RenderNode> {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut Vec<RenderNode> {
        &mut self.nodes
    }


}

#[derive(Clone)]
pub enum RenderOp {
    Render(usize),
    Finish(usize),
}

pub struct RenderPaintInfo {
    // pub absolute_transformed_visible_path: Option<Path>,
    pub invalid_rects_idx: usize,
    pub children_invalid_rects_idx: usize,
    pub border_color: [Color; 4],
    pub render_fn: Option<RenderFn>,
    pub background_image: Option<Image>,
    pub background_color: Color,
    pub scroll_delta: (f32, f32),
}

pub struct RenderNode {
    pub element_id: u32,
    // Relative to viewport
    pub absolute_transformed_bounds: Rect,

    // Relative to viewport
    pub total_matrix: Matrix,
    pub need_snapshot: bool,
    pub width: f32,
    pub height: f32,

    pub clip_chain: ClipChain,
    pub border_width: (f32, f32, f32, f32),

    pub children_viewport: Option<Rect>,
    // relative bounds
    // pub reuse_bounds: Option<(f32, f32, Rect)>,
    pub paint_info: Option<RenderPaintInfo>,
    pub border_path: BorderPath,
    pub layer_idx: usize,
    pub layer_x: f32,
    pub layer_y: f32,
}

impl RenderNode {
    pub fn draw_background(&self, canvas: &Canvas) {
        let pi = some_or_return!(&self.paint_info);
        if let Some(img) = &pi.background_image {
            canvas.draw_image(img, (0.0, 0.0), Some(&Paint::default()));
        } else if !pi.background_color.is_transparent() {
            let mut paint = Paint::default();
            let (bd_top, bd_right, bd_bottom, bd_left) = self.border_width;
            let width = self.width;
            let height = self.height;
            let rect = Rect::new(bd_left, bd_top, width - bd_right, height - bd_bottom);

            paint.set_color(pi.background_color);
            paint.set_style(SkPaint_Style::Fill);
            canvas.draw_rect(&rect, &paint);
        }
    }

    pub fn draw_border(&mut self, canvas: &Canvas) {
        let pi = some_or_return!(&self.paint_info);
        let paths = self.border_path.get_paths();
        let color = &pi.border_color;
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

pub struct ClipChain {
    ops: Vec<CanvasOp>,
}

impl ClipChain {
    pub fn new() -> ClipChain {
        Self { ops: Vec::new() }
    }
    pub fn apply(&self, canvas: &Canvas) {
        for op in &self.ops {
            match op {
                CanvasOp::IntersectClipPath(cp) => {
                    cp.apply(canvas);
                }
                CanvasOp::Translate(v) => {
                    canvas.translate(*v);
                }
                CanvasOp::Rotate(degree, p) => {
                    canvas.rotate(*degree, p.clone());
                }
                CanvasOp::Scale(sx, sy) => {
                    canvas.scale((*sx, *sy));
                }
            }
        }
    }

    pub fn clip(&self, path: &Path) -> Path {
        //TODO cache
        let mut clip_path = ClipPath::unlimited();
        for op in &self.ops {
            match op {
                CanvasOp::IntersectClipPath(p) => {
                    clip_path.intersect(p);
                }
                CanvasOp::Translate(vector) => {
                    clip_path.offset((-vector.x, -vector.y));
                }
                CanvasOp::Rotate(degree, p) => {
                    if let Some(p) = p {
                        clip_path.transform(&Matrix::rotate_deg_pivot(-degree, *p));
                    } else {
                        clip_path.transform(&Matrix::rotate_deg(-degree));
                    }
                }
                CanvasOp::Scale(sx, sy) => {
                    clip_path.transform(&Matrix::scale((1.0 / sx, 1.0 / sy)));
                }
            }
        }
        clip_path.clip(path)
    }

}

#[derive(Clone)]
pub enum CanvasOp {
    IntersectClipPath(ClipPath),
    Translate(Vector),
    Rotate(f32, Option<Point>),
    Scale(f32, f32),
}


pub struct MatrixCalculator {
    matrix: Matrix,
    cpu_renderer: CpuRenderer,
    ops: Vec<CanvasOp>,
    saved_ops_sizes: Vec<usize>,
}

impl MatrixCalculator {
    pub fn new() -> MatrixCalculator {
        let cpu_renderer = CpuRenderer::new(1, 1);
        Self {
            matrix: Matrix::default(),
            cpu_renderer,
            saved_ops_sizes: Vec::new(),
            ops: Vec::new(),
        }
    }
    pub fn intersect_clip_path(&mut self, path: &ClipPath) {
        self.ops.push(CanvasOp::IntersectClipPath(path.clone()));
    }

    pub fn translate(&mut self, vector: impl Into<Vector>) {
        let vector = vector.into();
        self.cpu_renderer.canvas().translate(vector);
        self.ops.push(CanvasOp::Translate(vector));
    }
    pub fn rotate(&mut self, degree: f32, p: Option<Point>) {
        let p = p.into();
        self.cpu_renderer.canvas().rotate(degree, p);
        self.ops.push(CanvasOp::Rotate(degree, p));
    }
    pub fn scale(&mut self, (sx, sy): (scalar, scalar)) {
        self.cpu_renderer.canvas().scale((sx, sy));
        self.ops.push(CanvasOp::Scale(sx, sy));
    }
    pub fn get_total_matrix(&mut self) -> Matrix {
        self.cpu_renderer.canvas().total_matrix()
    }
    pub fn get_clip_chain(&mut self) -> ClipChain {
        ClipChain {
            ops: self.ops.clone(),
        }
    }
    pub fn save(&mut self) {
        self.cpu_renderer.canvas().save();
        self.saved_ops_sizes.push(self.ops.len());
    }
    pub fn restore(&mut self) {
        self.cpu_renderer.canvas().restore();
        let saved_size = self.saved_ops_sizes.pop().unwrap();
        while self.ops.len() > saved_size {
            self.ops.pop().unwrap();
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClipPath {
    unlimited: bool,
    path: Path,
}

impl ClipPath {
    pub fn unlimited() -> ClipPath {
        Self {
            unlimited: true,
            path: Path::new(),
        }
    }
    pub fn empty() -> ClipPath {
        Self {
            unlimited: false,
            path: Path::new(),
        }
    }
    pub fn from_wh(width: f32, height: f32) -> ClipPath {
        Self::from_rect(&Rect::from_xywh(0.0, 0.0, width, height))
    }
    pub fn from_rect(rect: &Rect) -> ClipPath {
        Self {
            unlimited: false,
            path: Path::rect(rect, None),
        }
    }

    pub fn from_path(path: &Path) -> ClipPath {
        Self {
            unlimited: false,
            path: path.clone(),
        }
    }

    pub fn intersect(&mut self, other: &ClipPath) {
        if other.unlimited {
            return;
        }
        if self.unlimited {
            *self = other.clone();
            return;
        }
        self.path = self.path.op(&other.path, SkPathOp::Intersect).unwrap_or(Path::new());
    }

    pub fn offset(&mut self, d: impl Into<Vector>) {
        if !self.unlimited {
            self.path.offset(d);
        }
    }

    pub fn transform(&mut self, matrix: &Matrix) {
        if self.unlimited {
            return;
        }
        self.path = self.path.with_transform(matrix);
    }

    pub fn with_offset(&self, d: impl Into<Vector>) -> ClipPath {
        let mut cp = self.clone();
        cp.offset(d);
        cp
    }

    pub fn apply(&self, canvas: &Canvas) {
        if !self.unlimited {
            canvas.clip_path(&self.path, SkClipOp::Intersect, false);
        }
    }

    pub fn clip(&self, path: &Path) -> Path {
        if self.unlimited {
            path.clone()
        } else {
            self.path.op(path, SkPathOp::Intersect).unwrap_or(Path::new())
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

#[test]
pub fn test_path() {
    let empty_path = Path::new();
    assert!(!empty_path.contains((0.0, 0.0)));

    let rect = Rect::from_xywh(10.0, 70.0, 100.0, 100.0);
    let mut path = Path::rect(rect, None);
    assert!(!path.contains((0.0, 10.0)));
    assert!(path.contains((30.0, 80.0)));
}

#[test]
pub fn test_matrix() {
    let mut matrix = Matrix::translate(Vector::new(100.0, 200.0));
    matrix.post_scale((3.0, 3.0), None);
    matrix.post_scale((2.0, -2.0), None);
    let sx = 4.0;
    let sy = 9.0;
    let d = matrix.map_xy(sx, sy);
    println!("s={},{}", sx, sy);
    println!("d={},{}", d.x, d.y);

    let invert_matrix = matrix.invert().unwrap();
    let r = invert_matrix.map_xy(d.x, d.y);
    println!("r={}, {}", r.x, r.y);

}