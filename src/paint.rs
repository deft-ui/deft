use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use measure_time::print_time;
use skia_bindings::{SkClipOp, SkPaint_Style, SkPathOp};
use skia_safe::{scalar, ClipOp, Color, IRect, Image, Matrix, Paint, Path, Point, Rect, Vector};
use skia_safe::Canvas;
use skia_window::context::{RenderContext, UserContext};
use skia_window::layer::Layer;
use crate::base::{Id, IdKey};
use crate::element::Element;
use crate::mrc::Mrc;
use crate::render::RenderFn;
use crate::renderer::CpuRenderer;
use crate::{some_or_break, some_or_continue, some_or_return};
use crate::render::paint_layer::PaintLayer;
use crate::render::paint_node::PaintNode;
use crate::render::paint_tree::PaintTree;
use crate::style::border_path::BorderPath;
use crate::style::ColorHelper;

thread_local! {
    pub static NEXT_UNIQUE_RECT_ID: Cell<u64> = Cell::new(1);
}

pub struct LayerState {
    pub layer: Layer,
    pub surface_width: usize,
    pub surface_height: usize,
    pub last_scroll_left: f32,
    pub last_scroll_top: f32,
}

pub struct RenderState {
    pub layers: HashMap<RenderLayerKey, LayerState>,
}

impl RenderState {

    fn new() -> Self {
        Self { layers: HashMap::new() }
    }

    pub fn take(ctx: &mut RenderContext) -> Self {
        ctx.user_context.take::<RenderState>().unwrap_or_else(
            || RenderState::new()
        )
    }
    pub fn put(self, ctx: &mut RenderContext) {
        ctx.user_context.set(self);
    }
}

//TODO rename to LayoutTree?
pub struct RenderTree {
    nodes: Vec<RenderNode>,
    element2node: HashMap<u32, usize>,
    pub layers: Vec<RenderLayer>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum RenderLayerType {
    Root,
    Children,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct RenderLayerKey {
    root_element_id: u32,
    pub layer_type: RenderLayerType,
}

impl RenderLayerKey {
    pub fn new(root_element_id: u32, layer_type: RenderLayerType) -> Self {
        Self { root_element_id, layer_type }
    }
}


pub struct RenderLayer {
    pub total_matrix: Matrix,
    pub width: f32,
    pub height: f32,
    pub nodes: Vec<RenderLayerNode>,
    // pub root_element_id: u32,
    pub key: RenderLayerKey,
    invalid_area: InvalidArea,
    pub scroll_left: f32,
    pub scroll_top: f32,
    pub last_scroll_left: f32,
    pub last_scroll_top: f32,
    // Original position relative to viewport before transform
    origin_absolute_pos: (f32, f32),
}

impl RenderLayer {
    pub fn invalid(&mut self, rect: &Rect) {
        self.invalid_area.add_rect(Some(rect));
    }
    pub fn invalid_all(&mut self) {
        self.invalid_area = InvalidArea::Full;
    }
    pub fn update_scroll_left(&mut self, scroll_left: f32) {
        self.scroll_left = scroll_left;
    }
    pub fn update_scroll_top(&mut self, scroll_top: f32) {
        self.scroll_top = scroll_top;
    }
    pub fn build_invalid_path(&self, viewport: &Rect) -> Path {
        self.invalid_area.build(viewport.clone()).to_path(viewport.clone())
    }
    pub fn build_invalid_rects(&self, viewport: &Rect) -> InvalidRects {
        self.invalid_area.build(viewport.clone())
    }
    pub fn reset_invalid_area(&mut self) {
        self.invalid_area = InvalidArea::None;
    }
}

#[derive(Clone)]
pub struct RenderLayerNode {
    pub node_idx: usize,
    pub children: Vec<RenderLayerNode>,
}

impl RenderTree {
    pub fn new(predicate_count: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(predicate_count),
            element2node: HashMap::with_capacity(predicate_count),
            layers: Vec::new(),
        }
    }

    pub fn get_by_element_id(&self, element_id: u32) -> Option<&RenderNode> {
        let node_idx = self.element2node.get(&element_id)?;
        Some(&self.nodes[*node_idx])
    }

    pub fn get_element_total_matrix(&self, element_id: u32) -> Option<Matrix> {
        let node = self.get_by_element_id(element_id)?;
        let mut matrix = self.layers[node.layer_idx].total_matrix;
        let mut mc = MatrixCalculator::new();
        mc.concat(&matrix);
        mc.translate((node.layer_x, node.layer_y));
        Some(mc.get_total_matrix())
    }

    pub fn get_mut_by_element_id(&mut self, element_id: u32) -> Option<&mut RenderNode> {
        let node_idx = self.element2node.get(&element_id)?;
        Some(&mut self.nodes[*node_idx])
    }

    pub fn get_layer_idx(&mut self, element_id: u32) -> Option<usize> {
        self.get_by_element_id(element_id).map(|rn| rn.layer_idx)
    }

    pub fn add_node(&mut self, node: RenderNode) -> usize {
        let idx = self.nodes.len();
        self.element2node.insert(node.element_id, idx);
        self.nodes.push(node);
        idx
    }

    pub fn rebuild_layers(&mut self, root: &mut Element) {
        let mut layer_roots = Vec::new();
        layer_roots.push((RenderLayerType::Root, Matrix::default(), 0.0, 0.0, root.clone()));
        let mut layers = Vec::new();
        let mut old_layers_map = {
            let mut map = HashMap::new();
            loop {
                let ol = some_or_break!(self.layers.pop());
                map.insert(ol.key.clone(), ol);
            }
            map
        };
        loop {
            let mut layer_root = some_or_break!(layer_roots.pop());
            let (render_layer_type, matrix, origin_x, origin_y, mut element_root) = layer_root;
            let layer_key = RenderLayerKey::new(element_root.get_id(), render_layer_type.clone());
            let layer_idx = layers.len();
            let mut ol = if let Some(mut ol) = old_layers_map.remove(&layer_key) {
                let scroll_delta_x = ol.scroll_left - ol.last_scroll_left;
                let scroll_delta_y = ol.scroll_top - ol.last_scroll_top;
                if scroll_delta_x != 0.0 || scroll_delta_y != 0.0 {
                    ol.invalid_area.offset(-scroll_delta_x, -scroll_delta_y);
                    if scroll_delta_y > 0.0 {
                        ol.invalid_area.add_rect(Some(&Rect::new(0.0, ol.height - scroll_delta_y, ol.width, ol.height)));
                    } else if scroll_delta_y < 0.0 {
                        ol.invalid_area.add_rect(Some(&Rect::new(0.0, 0.0, ol.width, -scroll_delta_y)));
                    }

                    if scroll_delta_x > 0.0 {
                        ol.invalid_area.add_rect(Some(&Rect::new(ol.width - scroll_delta_x, 0.0, ol.width, ol.height)));
                    } else if scroll_delta_x < 0.0 {
                        ol.invalid_area.add_rect(Some(&Rect::new(0.0, 0.0, -scroll_delta_x, ol.height)));
                    }
                }
                ol.last_scroll_left = ol.scroll_left;
                ol.last_scroll_top = ol.scroll_top;
                ol
            } else {
                RenderLayer {
                    total_matrix: Matrix::default(),
                    width: 0.0,
                    height: 0.0,
                    nodes: Vec::new(),
                    key: layer_key,
                    invalid_area: InvalidArea::Full,
                    scroll_left: 0.0,
                    scroll_top: 0.0,
                    last_scroll_left: 0.0,
                    last_scroll_top: 0.0,
                    origin_absolute_pos: (0.0, 0.0),
                }
            };
            // Create new layers
            let mut nodes = Vec::new();
            let mut matrix_calculator = MatrixCalculator::new();
            matrix_calculator.concat(&matrix);
            self.build_layer_node(
                layer_idx,
                &mut element_root,
                &mut layer_roots,
                0.0,
                0.0,
                &mut ol.invalid_area,
                &mut old_layers_map,
                origin_x,
                origin_y,
                &mut nodes,
                render_layer_type,
                &mut matrix_calculator,
            );
            let root_node = self.get_by_element_id(element_root.get_id()).unwrap();
            ol.nodes = nodes;
            ol.total_matrix = matrix;
            ol.width = root_node.width;
            ol.height = root_node.height;
            ol.origin_absolute_pos = (origin_x, origin_y);
            layers.push(ol);
        }
        self.layers = layers;
    }

    pub fn invalid_element(&mut self, element_id: u32) {
        let node = some_or_return!(self.get_by_element_id(element_id));
        let layer_idx = node.layer_idx;
        let bounds = Rect::from_xywh(node.layer_x, node.layer_y, node.width, node.height);
        self.layers[layer_idx].invalid(&bounds);
    }

    pub fn update_scroll_left(&mut self, element_id: u32, scroll_left: f32) {
        // let node = some_or_return!(self.get_by_element_id(element_id));
        let key = RenderLayerKey::new(element_id, RenderLayerType::Children);
        for l in &mut self.layers {
            if l.key == key {
                l.update_scroll_left(scroll_left);
            }
        }
    }

    pub fn update_scroll_top(&mut self, element_id: u32, scroll_top: f32) {
        // let node = some_or_return!(self.get_by_element_id(element_id));
        let key = RenderLayerKey::new(element_id, RenderLayerType::Children);
        for l in &mut self.layers {
            if l.key == key {
                l.update_scroll_top(scroll_top);
            }
        }
    }

    fn need_create_root_layer(element: &Element) -> bool {
        element.style.transform.is_some()
    }

    fn need_create_children_layer(element: &Element) -> bool {
        element.need_snapshot
    }

    fn build_layer_node(
        &mut self,
        layer_idx: usize,
        root: &mut Element,
        layer_roots: &mut Vec<(RenderLayerType, Matrix, f32, f32, Element)>,
        layer_x: f32,
        layer_y: f32,
        layer_invalid_area: &mut InvalidArea,
        old_layers: &HashMap<RenderLayerKey, RenderLayer>,
        origin_x: f32,
        origin_y: f32,
        result: &mut Vec<RenderLayerNode>,
        layer_type: RenderLayerType,
        matrix_calculator: &mut MatrixCalculator,
    ) {
        let node_idx = *some_or_return!(self.element2node.get(&root.get_id()));
        let node = &mut self.nodes[node_idx];
        let node_width = node.width;
        let node_height = node.height;
        let visit_children = if layer_type == RenderLayerType::Root {
            node.layer_idx = layer_idx;
            node.layer_x = layer_x;
            node.layer_y = layer_y;
            let need_create_children_layer = Self::need_create_children_layer(root);
            if need_create_children_layer {
                matrix_calculator.save();
                matrix_calculator.translate((layer_x, layer_y));
                let total_matrix = matrix_calculator.get_total_matrix();
                //TODO layer_xy should be origin_xy?
                layer_roots.push((RenderLayerType::Children, total_matrix, layer_x, layer_y, root.clone()));
                matrix_calculator.restore();
                false
            } else {
                true
            }
        } else {
            true
        };
        let mut children = Vec::new();
        if visit_children {
            for mut c in root.get_children() {
                let bounds = c.get_bounds().translate(-root.scroll_left, -root.scroll_top);
                let child_layer_x = layer_x + bounds.x;
                let child_layer_y = layer_y + bounds.y;
                let child_origin_x = origin_x + bounds.x;
                let child_origin_y = origin_y + bounds.y;
                let key = RenderLayerKey::new(c.get_id(), RenderLayerType::Root);
                if Self::need_create_root_layer(&c) {
                    if !old_layers.contains_key(&key) {
                        //Split layer
                        let rect = Rect::from_xywh(bounds.x, bounds.y, node_width, node_height);
                        layer_invalid_area.add_rect(Some(&rect));
                    }
                    matrix_calculator.save();
                    matrix_calculator.translate((child_layer_x, child_layer_y));
                    c.apply_transform(matrix_calculator);
                    let total_matrix = matrix_calculator.get_total_matrix();
                    layer_roots.push((RenderLayerType::Root, total_matrix, child_origin_x, child_origin_y, c));
                    matrix_calculator.restore();
                } else {
                    if let Some(old_layer) = old_layers.get(&key) {
                        // Merge layer
                        let mut old_invalid_area = old_layer.invalid_area.clone();
                        old_invalid_area.offset(old_layer.origin_absolute_pos.0 - origin_x, old_layer.origin_absolute_pos.1 - origin_y);
                        layer_invalid_area.merge(&old_invalid_area);
                    }
                    self.build_layer_node(
                        layer_idx,
                        &mut c,
                        layer_roots,
                        child_layer_x,
                        child_layer_y,
                        layer_invalid_area,
                        old_layers,
                        child_origin_x,
                        child_origin_y,
                        &mut children,
                        RenderLayerType::Root,
                        matrix_calculator,
                    );
                }
            }
        }
        if layer_type == RenderLayerType::Root {
            result.push(RenderLayerNode {
                node_idx,
                children,
            });
        } else {
            result.append(&mut children);
        }
    }

    pub fn update_layout_info_recurse(
        &mut self,
        root: &mut Element,
        bounds: Rect,
    ) {
        let node_idx = *some_or_return!(self.element2node.get(&root.get_id()));

        let mut border_path = root.create_border_path();
        // let border_box_path =  border_path.get_box_path();

        //TODO support overflow:visible
        // matrix_calculator.intersect_clip_path(&ClipPath::from_path(border_box_path));

        // let (transformed_bounds, _) = total_matrix.map_rect(Rect::from_xywh(0.0, 0.0, bounds.width(), bounds.height()));
        let node = &mut self.nodes[node_idx];
        node.width = bounds.width();
        node.height = bounds.height();
        //TODO update border path when border changed
        node.border_path = border_path;
        for mut c in root.get_children() {
            let child_bounds = c.get_bounds().translate(-root.scroll_left, -root.scroll_top);
            self.update_layout_info_recurse(&mut c, child_bounds.to_skia_rect());
        }
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

    fn build_repaint_node(&mut self, n: &RenderLayerNode) -> Option<PaintNode> {
        let node = &mut self.nodes[n.node_idx];
        if !node.need_repaint {
            //TODO maybe children need to repaint?
            return None;
        }
        node.need_repaint = false;
        let border_path = node.border_path.get_paths().clone();
        let children_viewport = node.children_viewport.clone();
        let layer_x = node.layer_x;
        let layer_y = node.layer_y;
        let border_box_path = node.border_path.get_box_path().clone();
        let border_width = node.border_width;
        let width = node.width;
        let height = node.height;

        let pi = some_or_return!(&mut node.paint_info, None);
        let border_color = pi.border_color;
        let render_fn = pi.render_fn.take();
        let background_image = pi.background_image.clone();
        let background_color = pi.background_color;


        let mut children = Vec::new();
        for c in &n.children {
            let cn = some_or_continue!(self.build_repaint_node(c));
            children.push(cn);
        }
        let pn = PaintNode {
            children_viewport,
            border_path,
            border_box_path,
            layer_x,
            layer_y,
            border_color,
            render_fn,
            background_image,
            background_color,
            children,
            border_width,
            width,
            height,
        };
        Some(pn)
    }

    fn build_repaint_layer(&mut self, layer_idx: usize, viewport: &Rect) -> PaintLayer {
        let layer = &mut self.layers[layer_idx];
        let scroll_left = layer.scroll_left;
        let scroll_top = layer.scroll_top;
        let key = layer.key.clone();
        let width = layer.width;
        let height = layer.height;
        let total_matrix = layer.total_matrix;
        let invalid_path = layer.invalid_area.build(viewport.clone()).to_path(viewport.clone());
        layer.reset_invalid_area();

        let mut paint_nodes = Vec::new();
        //TODO do not clone
        let layer_nodes = layer.nodes.clone();
        for n in &layer_nodes {
            let pn = some_or_continue!(self.build_repaint_node(n));
            paint_nodes.push(pn);
        }
        PaintLayer {
            roots: paint_nodes,
            scroll_left,
            scroll_top,
            key,
            width,
            height,
            total_matrix,
            invalid_path,
        }
    }

    pub fn build_repaint_tree(&mut self, viewport: &Rect) -> PaintTree {
        print_time!("build repaint tree");
        let mut layers = Vec::new();
        for idx in 0..self.layers.len() {
            layers.push(self.build_repaint_layer(idx, viewport));
        }
        PaintTree {
            layers
        }
    }

}

#[derive(Clone)]
pub enum RenderOp {
    Render(usize),
    Finish(usize),
}

pub struct RenderPaintInfo {
    // pub absolute_transformed_visible_path: Option<Path>,
    pub border_color: [Color; 4],
    pub render_fn: Option<RenderFn>,
    pub background_image: Option<Image>,
    pub background_color: Color,
    pub scroll_delta: (f32, f32),
}

pub struct RenderNode {
    pub element_id: u32,

    pub need_snapshot: bool,
    pub width: f32,
    pub height: f32,

    pub border_width: (f32, f32, f32, f32),

    pub children_viewport: Option<Rect>,
    // relative bounds
    // pub reuse_bounds: Option<(f32, f32, Rect)>,
    pub paint_info: Option<RenderPaintInfo>,
    pub border_path: BorderPath,
    pub layer_idx: usize,
    //TODO fix root value
    pub layer_x: f32,
    pub layer_y: f32,
    pub need_repaint: bool,
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
    pub fn merge(&mut self, other: &Self) {
        match self {
            InvalidArea::Full => {}
            InvalidArea::Partial(p) => {
                match other {
                    InvalidArea::Full => {
                        *self = InvalidArea::Full;
                    }
                    InvalidArea::Partial(o) => {
                        p.merge(o);
                    }
                    InvalidArea::None => {}
                }
            }
            InvalidArea::None => {
                *self = other.clone();
            }
        }
    }
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
    pub fn merge(&mut self, other: &PartialInvalidArea) {
        for other_rect in &other.rects {
            let or = UniqueRect::from_rect(other_rect.1.clone());
            self.add_unique_rect(&or);
        }
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
    cpu_renderer: CpuRenderer,
}

impl MatrixCalculator {
    pub fn new() -> MatrixCalculator {
        let cpu_renderer = CpuRenderer::new(1, 1);
        Self {
            cpu_renderer,
        }
    }

    pub fn concat(&mut self, matrix: &Matrix) {
        self.cpu_renderer.canvas().concat(matrix);
    }

    pub fn translate(&mut self, vector: impl Into<Vector>) {
        let vector = vector.into();
        self.cpu_renderer.canvas().translate(vector);
    }
    pub fn rotate(&mut self, degree: f32, p: Option<Point>) {
        let p = p.into();
        self.cpu_renderer.canvas().rotate(degree, p);
    }
    pub fn scale(&mut self, (sx, sy): (scalar, scalar)) {
        self.cpu_renderer.canvas().scale((sx, sy));
    }
    pub fn get_total_matrix(&mut self) -> Matrix {
        self.cpu_renderer.canvas().total_matrix()
    }
    pub fn save(&mut self) {
        self.cpu_renderer.canvas().save();
    }
    pub fn restore(&mut self) {
        self.cpu_renderer.canvas().restore();
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

    {
        print_time!("map rect time");
        for i in 0..10000 {
            let rect=  Rect::from_xywh(0.0, 0.0, i as f32, i as f32);
            let p = matrix.map_rect(&rect);
        }
    }

}