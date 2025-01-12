use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::mem;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use measure_time::print_time;
use sha1::digest::generic_array::functional::FunctionalSequence;
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
use crate::render::layout_tree::LayoutTree;
use crate::render::paint_layer::PaintLayer;
use crate::render::paint_node::PaintNode;
use crate::render::paint_object::{ElementPaintObject, LayerPaintObject, PaintObject};
use crate::render::paint_tree::PaintTree;
use crate::style::border_path::BorderPath;
use crate::style::ColorHelper;

thread_local! {
    pub static NEXT_UNIQUE_RECT_ID: Cell<u64> = Cell::new(1);
    pub static RENDER_TREE_ID_KEY: IdKey = IdKey::new();
}

pub struct LayerState {
    pub layer: Layer,
    pub total_matrix: Matrix,
    pub invalid_rects: InvalidRects,
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
    id: Id<RenderTree>,
    layout_tree: LayoutTree,
    pub element_objects: Vec<ElementObjectData>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum RenderLayerType {
    Root,
    Children,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct RenderLayerKey {
    tree_id: Id<RenderTree>,
    root_element_id: u32,
    pub layer_type: RenderLayerType,
}

impl RenderLayerKey {
    pub fn new(tree_id: Id<RenderTree>, root_element_id: u32, layer_type: RenderLayerType) -> Self {
        Self { tree_id, root_element_id, layer_type }
    }
}

pub struct ElementObjectData {
    pub coord: (f32, f32),
    pub layer_coord: (f32, f32),
    pub children_viewport: Option<Rect>,
    // pub layer_x: f32,
    // pub layer_y: f32,
    pub border_color: [Color; 4],
    pub renderer: Box<dyn FnMut() -> RenderFn>,
    pub background_image: Option<Image>,
    pub background_color: Color,
    pub border_width: (f32, f32, f32, f32),
    pub width: f32,
    pub height: f32,
    pub layer_object_idx: usize,
    pub element_id: u32,
    pub element: Element,
}

#[derive(Clone)]
pub struct ElementObject {
    pub element_object_idx: usize,
    pub children: Vec<RenderObject>,
}

pub struct LayerObjectData {
    pub matrix: Matrix,
    pub total_matrix: Matrix,
    pub width: f32,
    pub height: f32,
    pub key: RenderLayerKey,
    pub invalid_area: InvalidArea,
    pub scroll_left: f32,
    pub scroll_top: f32,
    // Original position relative to viewport before transform
    pub origin_absolute_pos: (f32, f32),
}

#[derive(Clone)]
pub struct LayerObject {
    pub objects: Vec<RenderObject>,
    // pub root_element_id: u32,
    pub layer_object_idx: usize,
}

#[derive(Clone)]
pub enum RenderObject {
    Normal(ElementObject),
    Layer(LayerObject),
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

impl LayerObjectData {
    pub fn invalid(&mut self, rect: &Rect) {
        // println!("Invalid {:?}   {:?}", self.key, rect);
        self.invalid_area.add_rect(Some(rect));
    }
    pub fn invalid_all(&mut self) {
        self.invalid_area = InvalidArea::Full;
    }
    //TODO remove
    pub fn update_scroll_left(&mut self, scroll_left: f32) {
        // self.scroll_left = scroll_left;
    }
    //TODO remove
    pub fn update_scroll_top(&mut self, scroll_top: f32) {
        // self.scroll_top = scroll_top;
    }
    pub fn build_invalid_path(&self, viewport: &Rect) -> Path {
        self.invalid_area.build(viewport.clone()).to_path()
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
            id: Id::next(&RENDER_TREE_ID_KEY),
            layout_tree: LayoutTree::new(),
            element_objects: Vec::with_capacity(predicate_count),
        }
    }

    pub fn get_element_total_matrix(&self, element: &Element) -> Option<Matrix> {
        let element_object = self.element_objects.get(element.render_object_idx?)?;
        let layer_object = self.layout_tree.layer_objects.get(element_object.layer_object_idx)?;
        let mut mc = MatrixCalculator::new();
        mc.concat(&layer_object.total_matrix);
        mc.translate(element_object.coord);
        Some(mc.get_total_matrix())
    }

    pub fn get_element_object_by_pos(&self, x: f32, y: f32) -> Option<(&ElementObjectData, f32, f32)> {
        self.get_element_object_by_pos_recurse(self.layout_tree.root_render_object.as_ref()?, x, y)
    }

    fn get_element_object_by_pos_recurse(&self, render_object: &RenderObject, x: f32, y: f32) -> Option<(&ElementObjectData, f32, f32)> {
        match render_object {
            RenderObject::Normal(eo) => {
                let eod = &self.element_objects[eo.element_object_idx];
                if x >= eod.coord.0 && x <= eod.coord.0 + eod.width
                    && y >= eod.coord.1 && y <= eod.coord.1 + eod.height {
                    for c in eo.children.iter().rev() {
                        let r = some_or_continue!(self.get_element_object_by_pos_recurse(c, x - eod.coord.0, y - eod.coord.1));
                        return Some(r);
                    }
                    Some((eod, x - eod.coord.0, y - eod.coord.1))
                } else {
                    None
                }
            }
            RenderObject::Layer(lo) => {
                let lod = &self.layout_tree.layer_objects[lo.layer_object_idx];
                let im = lod.total_matrix.invert()?;
                let Point {x, y} = im.map_xy(x, y);
                for ro in lo.objects.iter().rev() {
                    let eo = some_or_continue!(self.get_element_object_by_pos_recurse(ro, x, y));
                    return Some(eo);
                }
                None
            }
        }
    }

    pub fn create_node(&mut self, element: &mut Element) {
        let bounds = element.get_bounds();
        let mut el = element.clone();
        let element_data = ElementObjectData {
            element: element.clone(),
            element_id: element.get_id(),
            coord: (bounds.x, bounds.y),
            children_viewport: element.get_children_viewport(),
            border_color: element.style.border_color,
            renderer: Box::new(move || el.get_backend_mut().render()),
            background_image: element.style.background_image.clone(),
            background_color: element.style.computed_style.background_color,
            border_width: element.get_border_width(),
            width: bounds.width,
            height: bounds.height,

            layer_object_idx: 0,
            layer_coord: (0.0, 0.0),
        };
        self.element_objects.push(element_data);
        element.render_object_idx = Some(self.element_objects.len() - 1);
    }

    pub fn rebuild_render_object(&mut self, element: &mut Element) {
        print_time!("rebuild render object");
        let old_layout_tree = mem::take(&mut self.layout_tree);
        let mut matrix_calculator = MatrixCalculator::new();
        let bounds = element.get_bounds();
        self.layout_tree.root_render_object = Some(self.build_render_object(element, 0.0, 0.0, None, &mut matrix_calculator, 0.0, 0.0, &bounds.to_skia_rect(), true));
        old_layout_tree.sync_invalid_area(&mut self.layout_tree);
    }

    fn build_children_objects(&mut self,
                              element: &Element,
                              matrix_calculator: &mut MatrixCalculator,
                              origin_x: f32,
                              origin_y: f32,
                              layer_x: f32,
                              layer_y: f32,
                              layer_object_idx: Option<usize>,
    ) -> Vec<RenderObject> {
        let mut children = Vec::new();
        for mut c in element.get_children() {
            let child_bounds = c.get_bounds().translate(-element.get_scroll_left(), -element.get_scroll_top());
            matrix_calculator.save();
            matrix_calculator.translate((child_bounds.x, child_bounds.y));
            let child_origin_x = origin_x + child_bounds.x;
            let child_origin_y = origin_y + child_bounds.y;
            let child_layer_x = layer_x + child_bounds.x;
            let child_layer_y = layer_y + child_bounds.y;
            children.push(self.build_render_object(&mut c, child_origin_x, child_origin_y, layer_object_idx, matrix_calculator, child_layer_x, child_layer_y, &child_bounds.to_skia_rect(), false));
            matrix_calculator.restore();
        }
        children
    }

    pub fn build_render_object_children(
        &mut self,
        element: &mut Element,
        origin_x: f32,
        origin_y: f32,
        layer_object_idx: Option<usize>,
        matrix_calculator: &mut MatrixCalculator,
        layer_x: f32,
        layer_y: f32,
    ) -> Vec<RenderObject> {
        let bounds = element.get_bounds();
        let need_create_children_layer = Self::need_create_children_layer(element);
        if need_create_children_layer {
            let layer_object_idx = self.layout_tree.layer_objects.len();
            let layer_object_data = LayerObjectData {
                matrix: Matrix::default(),
                total_matrix: matrix_calculator.get_total_matrix(),
                width: bounds.width,
                height: bounds.height,
                key: RenderLayerKey::new(self.id, element.get_id(), RenderLayerType::Children),
                invalid_area: InvalidArea::Full,
                scroll_left: element.get_scroll_left(),
                scroll_top: element.get_scroll_top(),
                origin_absolute_pos: (origin_x, origin_y),
            };
            self.layout_tree.layer_objects.push(layer_object_data);
            let children_layer_object = LayerObject {
                objects: self.build_children_objects(element, matrix_calculator, origin_x, origin_y, 0.0, 0.0, Some(layer_object_idx)),
                layer_object_idx,
            };
            vec![RenderObject::Layer(children_layer_object)]
        } else {
            self.build_children_objects(element, matrix_calculator, origin_x, origin_y, layer_x, layer_y, layer_object_idx)
        }
    }

    pub fn build_render_object(
        &mut self,
        element: &mut Element,
        origin_x: f32,
        origin_y: f32,
        layer_object_idx: Option<usize>,
        matrix_calculator: &mut MatrixCalculator,
        layer_x: f32,
        layer_y: f32,
        bounds: &Rect,
        is_root: bool,
    ) -> RenderObject {
        let need_create_root_layer = is_root || Self::need_create_root_layer(element);
        if need_create_root_layer {
            matrix_calculator.save();
            let mut mc = MatrixCalculator::new();
            mc.translate((bounds.left, bounds.top));
            element.apply_transform(&mut mc);

            element.apply_transform(matrix_calculator);
            let layer_object_idx = self.layout_tree.layer_objects.len();
            let layer_object_data = LayerObjectData {
                matrix: mc.get_total_matrix(),
                total_matrix: matrix_calculator.get_total_matrix(),
                width: bounds.width(),
                height: bounds.height(),
                key: RenderLayerKey::new(self.id, element.get_id(), RenderLayerType::Root),
                invalid_area: InvalidArea::Full,
                scroll_left: 0.0,
                scroll_top: 0.0,
                origin_absolute_pos: (origin_x, origin_y),
            };
            self.layout_tree.layer_objects.push(layer_object_data);
            let obj = self.create_normal_render_object(
                element,
                &bounds.with_offset((-bounds.left, -bounds.top)),
                origin_x,
                origin_y,
                0.0,
                0.0,
                layer_object_idx,
                matrix_calculator,
            );
            //TODO fix
            let layer_object = LayerObject {
                objects: vec![obj],
                layer_object_idx,
            };
            matrix_calculator.restore();
            RenderObject::Layer(layer_object)
        } else {
            self.create_normal_render_object(element, bounds, origin_x, origin_y, layer_x, layer_y, layer_object_idx.unwrap(), matrix_calculator)
        }
    }

    fn create_normal_render_object(
        &mut self,
        element: &mut Element,
        bounds: &Rect,
        origin_x: f32,
        origin_y: f32,
        layer_x: f32,
        layer_y: f32,
        layer_object_idx: usize,
        matrix_calculator: &mut MatrixCalculator,
    ) -> RenderObject {
        let element_object_idx = element.render_object_idx.unwrap();
        // let mut border_path = element.create_border_path();
        let element_data = &mut self.element_objects[element_object_idx];
        element_data.border_color = element.style.border_color;
        element_data.background_image = element.style.background_image.clone();
        element_data.background_color = element.style.computed_style.background_color;
        element_data.border_width = element.get_border_width();
        element_data.coord = (bounds.left, bounds.top);
        element_data.layer_object_idx = layer_object_idx;
        element_data.layer_coord = (layer_x, layer_y);
        let element_obj = ElementObject {
            element_object_idx,
            children: self.build_render_object_children(element, origin_x, origin_y, Some(layer_object_idx), matrix_calculator, layer_x, layer_y),
        };
        RenderObject::Normal(element_obj)
    }

    pub fn invalid_element(&mut self, element: &Element) {
        let render_object_idx = some_or_return!(element.render_object_idx);
        let eo = some_or_return!(self.element_objects.get(render_object_idx));
        let bounds = Rect::from_xywh(eo.layer_coord.0, eo.layer_coord.1, eo.width, eo.height);
        let layer_idx = eo.layer_object_idx;
        let lo = &mut self.layout_tree.layer_objects[layer_idx];
        lo.invalid(&bounds);
    }

    pub fn update_scroll_left(&mut self, element: &Element, scroll_left: f32) {
        let render_object_idx = some_or_return!(element.render_object_idx);
        let layer_idx = self.element_objects[render_object_idx].layer_object_idx;
        let lo = &mut self.layout_tree.layer_objects[layer_idx];
        lo.update_scroll_left(scroll_left);
    }

    pub fn update_scroll_top(&mut self, element: &Element, scroll_top: f32) {
        let render_object_idx = some_or_return!(element.render_object_idx);
        let layer_idx = self.element_objects[render_object_idx].layer_object_idx;
        let lo = &mut self.layout_tree.layer_objects[layer_idx];
        lo.update_scroll_top(scroll_top);
    }

    fn need_create_root_layer(element: &Element) -> bool {
        element.style.transform.is_some()
    }

    fn need_create_children_layer(element: &Element) -> bool {
        element.need_snapshot
    }

    pub fn update_layout_info_recurse(
        &mut self,
        root: &mut Element,
        bounds: Rect,
    ) {
        //TODO support overflow:visible
        // matrix_calculator.intersect_clip_path(&ClipPath::from_path(border_box_path));

        // let (transformed_bounds, _) = total_matrix.map_rect(Rect::from_xywh(0.0, 0.0, bounds.width(), bounds.height()));
        //TODO update border path when border changed
        for mut c in root.get_children() {
            let child_bounds = c.get_bounds().translate(-root.scroll_left, -root.scroll_top);
            self.update_layout_info_recurse(&mut c, child_bounds.to_skia_rect());
        }
    }

    pub fn build_paint_tree(&mut self, viewport: &Rect) -> Option<PaintTree> {
        print_time!("Building paint tree");
        let invalid_rects = InvalidArea::Full.build(viewport.clone());
        let root = self.build_paint_object(self.layout_tree.root_render_object.clone().as_mut().unwrap(), viewport, &invalid_rects)?;
        Some(PaintTree {
            root,
            all_layer_keys: self.layout_tree.get_all_layer_keys(),
        })
    }

    pub fn build_paint_objects(&mut self, render_objects: &mut Vec<RenderObject>, viewport: &Rect, invalid_rects: &InvalidRects) -> Vec<PaintObject> {
        let mut results = Vec::new();
        for ro in render_objects {
            let po = some_or_continue!(self.build_paint_object(ro, viewport, invalid_rects));
            results.push(po);
        }
        results
    }

    pub fn build_paint_object(&mut self, render_object: &mut RenderObject, viewport: &Rect, invalid_rects: &InvalidRects) -> Option<PaintObject> {
        match render_object {
            RenderObject::Normal(eod) => {
                let children = self.build_paint_objects(&mut eod.children.clone(), viewport, invalid_rects);
                let eo = &mut self.element_objects[eod.element_object_idx];

                if children.is_empty() && !invalid_rects.has_intersects(&Rect::from_xywh(eo.layer_coord.0, eo.layer_coord.1, eo.width, eo.height)) {
                    // println!("skipping painting {}", eo.element_id);
                    return None;
                }
                let border_path_mut = eo.element.get_border_path_mut();
                let border_path = border_path_mut.get_paths().clone();
                let border_box_path = border_path_mut.get_box_path().clone();
                let epo = ElementPaintObject {
                    coord: eo.coord,
                    children,
                    children_viewport: eo.children_viewport,
                    border_path,
                    border_box_path,
                    border_color: eo.border_color,
                    render_fn: Some((eo.renderer)()),
                    background_image: eo.background_image.clone(),
                    background_color: eo.background_color,
                    border_width: eo.border_width,
                    width: eo.width,
                    height: eo.height,
                    element_id: eo.element_id,
                };
                Some(PaintObject::Normal(epo))
            }
            RenderObject::Layer(lod) => {
                let invalid_rects = {
                    let lo = &self.layout_tree.layer_objects[lod.layer_object_idx];
                    self.layout_tree.layer_objects[lod.layer_object_idx].invalid_area.build(Rect::from_xywh(0.0, 0.0, lo.width, lo.height))
                };
                let objects = self.build_paint_objects(&mut lod.objects, viewport, &invalid_rects);
                let lo = &mut self.layout_tree.layer_objects[lod.layer_object_idx];
                let lpo = LayerPaintObject {
                    matrix: lo.matrix.clone(),
                    total_matrix: lo.total_matrix.clone(),
                    width: lo.width,
                    height: lo.height,
                    objects,
                    key: lo.key.clone(),
                    scroll_left: lo.scroll_left,
                    scroll_top: lo.scroll_top,
                    origin_absolute_pos: lo.origin_absolute_pos,
                    invalid_rects,
                };
                lo.invalid_area = InvalidArea::None;
                Some(PaintObject::Layer(lpo))
            }
        }
    }

}

pub struct RenderNode {
}

#[derive(PartialEq, Debug, Clone)]
pub enum InvalidArea {
    Full,
    Partial(PartialInvalidArea),
    None,
}

#[derive(PartialEq, Debug, Clone)]
pub struct InvalidRects {
    rects: Vec<Rect>,
}

impl Default for InvalidRects {
    fn default() -> Self {
        InvalidRects {
            rects: vec![],
        }
    }
}

impl InvalidRects {
    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }
    pub fn has_intersects(&self, rect: &Rect) -> bool {
        for r in &self.rects {
            let intersect = f32::min(r.right, rect.right) >= f32::max(r.left(), rect.left())
                && f32::min(r.bottom, rect.bottom) >= f32::max(r.top, rect.top);
            if intersect {
                return true;
            }
        }
        false
    }
    pub fn to_path(&self) -> Path {
        let mut path = Path::new();
        for r in &self.rects {
            path.add_rect(r, None);
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
        InvalidRects { rects }
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
        let mut path = Path::new();
        for r in &invalid_rects.rects {
            path.add_rect(r, None);
        }
        self.canvas.clip_path(&path, SkClipOp::Intersect, false);
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