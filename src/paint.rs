use crate::base::{Id, IdKey, Rect};
use crate::element::Element;
use crate::render::layout_tree::LayoutTree;
use crate::render::paint_object::{ElementPO, LayerPO};
use crate::render::RenderFn;
use crate::renderer::CpuRenderer;
use crate::{some_or_continue, some_or_return};
use skia_safe::Canvas;
use skia_safe::{scalar, Color, Image, Matrix, Path, PathOp, Point, Vector};
use skia_window::layer::Layer;
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::mem;
use yoga::PositionType;

thread_local! {
    pub static NEXT_UNIQUE_RECT_ID: Cell<u64> = Cell::new(1);
    pub static RENDER_TREE_ID_KEY: IdKey = IdKey::new();
}

#[derive(Debug, Clone)]
pub struct PaintContext {
    pub scale_factor: f32,
}

pub struct Painter<'a> {
    pub canvas: &'a Canvas,
    pub context: PaintContext,
}

impl<'a> Painter<'a> {
    pub fn new(canvas: &'a Canvas, context: PaintContext) -> Painter<'a> {
        Self { canvas, context }
    }
}

pub enum DrawLayer {
    Root,
    Sublayer(Layer),
}

pub struct LayerState {
    pub layer: DrawLayer,
    pub matrix: Matrix,
    pub total_matrix: Matrix,
    pub invalid_rects: InvalidRects,
    pub surface_width: usize,
    pub surface_height: usize,
    pub surface_bounds: Rect,
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
        Self {
            tree_id,
            root_element_id,
            layer_type,
        }
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
    pub layer_object_idx: Option<usize>,
    pub element_id: u32,
    pub element: Element,
}

#[derive(Clone)]
pub struct ElementRO {
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
    // Original position relative to viewport before transform
    pub origin_absolute_pos: (f32, f32),
    pub surface_bounds: Rect,
    pub visible_bounds: Rect,
    pub clip_rect: Option<Rect>,
}

#[derive(Clone)]
pub struct LayerRO {
    pub objects: Vec<RenderObject>,
    // pub root_element_id: u32,
    pub layer_object_idx: usize,
}

#[derive(Clone)]
pub enum RenderObject {
    Element(ElementRO),
    Layer(LayerRO),
}

#[derive(Clone)]
pub struct NormalNode {
    element_object_idx: usize,
    children: Vec<NormalNode>,
}

#[derive(Default, Clone)]
pub struct LayerNode {
    layer_object_idx: usize,
    // origin_bounds: Rect,
    normal_nodes: Vec<NormalNode>,
    layer_nodes: Vec<LayerNode>,
}

impl LayerObjectData {
    pub fn invalid(&mut self, rect: &Rect) {
        // error!("Invalid {:?}   {:?}", self.key, rect);
        self.invalid_area.add_rect(Some(rect));
    }
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
        let layer_object = self
            .layout_tree
            .layer_objects
            .get(element_object.layer_object_idx?)?;
        let mut mc = MatrixCalculator::new();
        mc.concat(&layer_object.total_matrix);
        mc.translate(element_object.layer_coord);
        Some(mc.get_total_matrix())
    }

    pub fn get_element_object_by_pos(
        &self,
        x: f32,
        y: f32,
    ) -> Option<(&ElementObjectData, f32, f32)> {
        self.get_element_object_by_pos_recurse(self.layout_tree.layer_node.as_ref()?, x, y)
    }

    fn get_element_object_by_pos_recurse(
        &self,
        lo: &LayerNode,
        abs_x: f32,
        abs_y: f32,
    ) -> Option<(&ElementObjectData, f32, f32)> {
        let lod = &self.layout_tree.layer_objects[lo.layer_object_idx];
        let im = lod.total_matrix.invert()?;
        let Point { x, y } = im.map_xy(abs_x, abs_y);
        let rect = lod
            .clip_rect
            .unwrap_or(Rect::from_xywh(0.0, 0.0, lod.width, lod.height));
        if !rect.contains(x, y) {
            return None;
        }
        for sub_lo in lo.layer_nodes.iter().rev() {
            let result =
                some_or_continue!(self.get_element_object_by_pos_recurse(sub_lo, abs_x, abs_y));
            return Some(result);
        }
        for eo in lo.normal_nodes.iter().rev() {
            let result =
                some_or_continue!(self.get_element_object_in_normal_nodes_by_pos_recurse(eo, x, y));
            return Some(result);
        }
        None
    }

    fn get_element_object_in_normal_nodes_by_pos_recurse(
        &self,
        lo: &NormalNode,
        x: f32,
        y: f32,
    ) -> Option<(&ElementObjectData, f32, f32)> {
        let eod = &self.element_objects[lo.element_object_idx];
        if x >= eod.coord.0
            && x <= eod.coord.0 + eod.width
            && y >= eod.coord.1
            && y <= eod.coord.1 + eod.height
        {
            for c in lo.children.iter().rev() {
                let r = some_or_continue!(self.get_element_object_in_normal_nodes_by_pos_recurse(
                    c,
                    x - eod.coord.0,
                    y - eod.coord.1
                ));
                return Some(r);
            }
            Some((eod, x - eod.coord.0, y - eod.coord.1))
        } else {
            None
        }
    }

    pub fn create_node(&mut self, element: &mut Element) {
        let bounds = element.get_bounds();
        let mut el = element.clone();
        let element_data = ElementObjectData {
            element: element.clone(),
            element_id: element.get_eid(),
            coord: (bounds.x, bounds.y),
            children_viewport: element.get_children_viewport(),
            border_color: element.style.border_color,
            renderer: Box::new(move || {
                RenderFn::merge(vec![el.scrollable.render(), el.get_backend_mut().render()])
            }),
            background_image: element.style.background_image.clone(),
            background_color: element.style.background_color,
            border_width: element.get_border_width(),
            width: bounds.width,
            height: bounds.height,

            layer_object_idx: None,
            layer_coord: (0.0, 0.0),
        };
        self.element_objects.push(element_data);
        element.render_object_idx = Some(self.element_objects.len() - 1);
    }

    pub fn rebuild_render_tree(&mut self, element: &mut Element, layer_cache_enabled: bool) {
        // print_time!("rebuild render object");
        let old_layout_tree = mem::take(&mut self.layout_tree);
        let mut matrix_calculator = MatrixCalculator::new();
        let bounds = element.get_bounds();
        let rro = self.build_render_object(
            element,
            0.0,
            0.0,
            None,
            &mut matrix_calculator,
            0.0,
            0.0,
            &bounds,
            true,
        );
        if let RenderObject::Layer(lo) = &rro {
            self.layout_tree.layer_node = Some(self.build_layer_tree(&lo));
        } else {
            self.layout_tree.layer_node = None;
        }
        if layer_cache_enabled {
            old_layout_tree.sync_invalid_area(&mut self.layout_tree, &rro);
        }
    }

    fn build_layer_tree(&mut self, layer_object: &LayerRO) -> LayerNode {
        let mut normal_nodes = Vec::new();
        let mut layer_objects = Vec::new();
        for c in &layer_object.objects {
            let nn = some_or_continue!(self.build_normal_node_recurse(c, &mut layer_objects));
            normal_nodes.push(nn);
        }
        if layer_objects.len() > 1 {
            layer_objects.sort_by(|a, b| {
                let la = &self.layout_tree.layer_objects[a.layer_object_idx];
                let lb = &self.layout_tree.layer_objects[b.layer_object_idx];
                if la.key.layer_type == RenderLayerType::Root
                    && lb.key.layer_type == RenderLayerType::Children
                {
                    return Ordering::Greater;
                }
                return a.layer_object_idx.cmp(&b.layer_object_idx);
            });
        }
        let mut layer_nodes = Vec::with_capacity(layer_objects.len());
        for lo in layer_objects {
            layer_nodes.push(self.build_layer_tree(&lo));
        }

        // let lo = &self.layout_tree.layer_objects[layer_object.layer_object_idx];
        // let origin_bounds = Rect::new(
        //     lo.origin_absolute_pos.0,
        //     lo.origin_absolute_pos.1,
        //     lo.width,
        //     lo.height
        // );

        LayerNode {
            layer_object_idx: layer_object.layer_object_idx,
            // origin_bounds,
            normal_nodes,
            layer_nodes,
        }
    }

    fn build_normal_node_recurse(
        &mut self,
        render_object: &RenderObject,
        layer_objects: &mut Vec<LayerRO>,
    ) -> Option<NormalNode> {
        match render_object {
            RenderObject::Element(eo) => {
                let mut children = Vec::new();
                for c in &eo.children {
                    let child_node =
                        some_or_continue!(self.build_normal_node_recurse(c, layer_objects));
                    children.push(child_node);
                }
                let nn = NormalNode {
                    element_object_idx: eo.element_object_idx,
                    children,
                };
                Some(nn)
            }
            RenderObject::Layer(lo) => {
                layer_objects.push(lo.clone());
                None
            }
        }
    }

    fn build_children_objects(
        &mut self,
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
            let child_bounds = c.get_bounds();
            matrix_calculator.save();
            matrix_calculator.translate((child_bounds.x, child_bounds.y));
            let child_origin_x = origin_x + child_bounds.x;
            let child_origin_y = origin_y + child_bounds.y;
            let child_layer_x = layer_x + child_bounds.x;
            let child_layer_y = layer_y + child_bounds.y;
            children.push(self.build_render_object(
                &mut c,
                child_origin_x,
                child_origin_y,
                layer_object_idx,
                matrix_calculator,
                child_layer_x,
                child_layer_y,
                &child_bounds,
                false,
            ));
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
            let (scroll_left, scroll_top) = element.scrollable.scroll_offset();
            let clip_rect = bounds.translate(-bounds.x + scroll_left, -bounds.y + scroll_top);
            matrix_calculator.save();

            let mut matrix = Matrix::default();
            if scroll_left != 0.0 || scroll_top != 0.0 {
                matrix_calculator.translate((-scroll_left, -scroll_top));

                let mut mc = MatrixCalculator::new();
                mc.translate((-scroll_left, -scroll_top));
                matrix = mc.get_total_matrix();
            }

            let layer_object_idx = self.layout_tree.layer_objects.len();
            let (width, height) = element.get_real_content_size();
            let layer_object_data = LayerObjectData {
                matrix,
                total_matrix: matrix_calculator.get_total_matrix(),
                width,
                height,
                key: RenderLayerKey::new(self.id, element.get_eid(), RenderLayerType::Children),
                invalid_area: InvalidArea::Full,
                origin_absolute_pos: (origin_x, origin_y),
                //TODO init surface_bounds?
                surface_bounds: Rect::default(),
                visible_bounds: Rect::default(),
                clip_rect: Some(clip_rect),
            };
            self.layout_tree.layer_objects.push(layer_object_data);
            let children_layer_object = LayerRO {
                objects: self.build_children_objects(
                    element,
                    matrix_calculator,
                    origin_x,
                    origin_y,
                    0.0,
                    0.0,
                    Some(layer_object_idx),
                ),
                layer_object_idx,
            };
            matrix_calculator.restore();
            vec![RenderObject::Layer(children_layer_object)]
        } else {
            self.build_children_objects(
                element,
                matrix_calculator,
                origin_x,
                origin_y,
                layer_x,
                layer_y,
                layer_object_idx,
            )
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
            mc.translate((bounds.x, bounds.y));
            element.apply_transform(&mut mc);

            element.apply_transform(matrix_calculator);
            let layer_object_idx = self.layout_tree.layer_objects.len();
            let layer_object_data = LayerObjectData {
                matrix: mc.get_total_matrix(),
                total_matrix: matrix_calculator.get_total_matrix(),
                width: bounds.width,
                height: bounds.height,
                key: RenderLayerKey::new(self.id, element.get_eid(), RenderLayerType::Root),
                invalid_area: InvalidArea::Full,
                origin_absolute_pos: (origin_x, origin_y),
                surface_bounds: Rect::default(),
                visible_bounds: Rect::default(),
                clip_rect: None,
            };
            self.layout_tree.layer_objects.push(layer_object_data);
            let obj = self.create_normal_render_object(
                element,
                &bounds.with_offset((-bounds.x, -bounds.y)),
                origin_x,
                origin_y,
                0.0,
                0.0,
                layer_object_idx,
                matrix_calculator,
            );
            //TODO fix
            let layer_object = LayerRO {
                objects: vec![obj],
                layer_object_idx,
            };
            matrix_calculator.restore();
            RenderObject::Layer(layer_object)
        } else {
            self.create_normal_render_object(
                element,
                bounds,
                origin_x,
                origin_y,
                layer_x,
                layer_y,
                layer_object_idx.unwrap(),
                matrix_calculator,
            )
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
        element_data.background_color = element.style.background_color;
        element_data.border_width = element.get_border_width();
        element_data.coord = (bounds.x, bounds.y);
        element_data.layer_object_idx = Some(layer_object_idx);
        element_data.layer_coord = (layer_x, layer_y);
        let element_obj = ElementRO {
            element_object_idx,
            children: self.build_render_object_children(
                element,
                origin_x,
                origin_y,
                Some(layer_object_idx),
                matrix_calculator,
                layer_x,
                layer_y,
            ),
        };
        RenderObject::Element(element_obj)
    }

    pub fn invalid_element(&mut self, element: &Element) {
        let render_object_idx = some_or_return!(element.render_object_idx);
        let eo = some_or_return!(self.element_objects.get(render_object_idx));
        let bounds = Rect::from_xywh(eo.layer_coord.0, eo.layer_coord.1, eo.width, eo.height);
        let layer_idx = some_or_return!(eo.layer_object_idx);
        let lo = &mut self.layout_tree.layer_objects[layer_idx];
        lo.invalid(&bounds);
    }

    fn need_create_root_layer(element: &Element) -> bool {
        if element.style.transform.is_some() {
            return true;
        }
        let pos_type = element.style.yoga_node._yn.get_position_type();
        pos_type == PositionType::Absolute || pos_type == PositionType::Relative
    }

    fn need_create_children_layer(element: &Element) -> bool {
        element.scrollable.vertical_bar.is_scrollable()
            || element.scrollable.horizontal_bar.is_scrollable()
    }

    pub fn build_paint_tree(&mut self, viewport: &Rect) -> LayerPO {
        // print_time!("Building paint tree");
        // let invalid_rects = InvalidArea::Full.build(viewport.clone());
        self.build_paint_layer_node(
            self.layout_tree.layer_node.clone().as_mut().unwrap(),
            viewport,
        )
    }

    fn build_paint_normal_nodes(
        &mut self,
        nodes: &Vec<NormalNode>,
        viewport: &Rect,
        invalid_rects: &InvalidRects,
    ) -> Vec<ElementPO> {
        let mut result = Vec::with_capacity(nodes.len());
        for n in nodes {
            result.push(self.build_paint_normal_node(n, viewport, invalid_rects));
        }
        result
    }

    fn build_paint_normal_node(
        &mut self,
        eod: &NormalNode,
        viewport: &Rect,
        invalid_rects: &InvalidRects,
    ) -> ElementPO {
        let children =
            self.build_paint_normal_nodes(&mut eod.children.clone(), viewport, invalid_rects);
        let eo = &mut self.element_objects[eod.element_object_idx];

        let need_paint = invalid_rects.has_intersects(&Rect::from_xywh(
            eo.layer_coord.0,
            eo.layer_coord.1,
            eo.width,
            eo.height,
        ));
        let border_path_mut = eo.element.get_border_path_mut();
        let border_path = border_path_mut.get_paths().clone();
        let border_box_path = border_path_mut.get_box_path().clone().unwrap();
        let epo = ElementPO {
            coord: eo.coord,
            children,
            children_viewport: eo.children_viewport,
            border_path,
            border_box_path,
            border_color: eo.border_color,
            render_fn: if need_paint {
                Some((eo.renderer)())
            } else {
                None
            },
            background_image: eo.background_image.clone(),
            background_color: eo.background_color,
            border_width: eo.border_width,
            width: eo.width,
            height: eo.height,
            element_id: eo.element_id,
            need_paint,
            focused: eo.element.is_focused(),
        };
        epo
    }

    pub fn build_paint_layer_node(&mut self, lod: &LayerNode, viewport: &Rect) -> LayerPO {
        let invalid_rects = {
            let lo = &mut self.layout_tree.layer_objects[lod.layer_object_idx];
            let mut invalid_area = lo.invalid_area.clone();
            let layer_path = Path::rect(
                &Rect::from_xywh(0.0, 0.0, lo.width, lo.height).to_skia_rect(),
                None,
            );
            let im = lo.total_matrix.invert().unwrap();
            let mut viewport_path = Path::rect(viewport.to_skia_rect(), None);
            let visible_path = viewport_path
                .transform(&im)
                .op(&layer_path, PathOp::Intersect)
                .unwrap_or(Path::new());
            let mut visible_bounds = Rect::from_skia(visible_path.bounds());
            visible_bounds.y = visible_bounds.y.floor();
            visible_bounds.height = visible_bounds.height.ceil();

            let max_len = (viewport.width * viewport.width + viewport.height * viewport.height)
                .sqrt()
                .ceil();
            let max_surface_width = f32::min(lo.width, max_len).ceil();
            let max_surface_height = f32::min(lo.height, max_len).ceil();
            if lo.surface_bounds.width() != max_surface_width
                || lo.surface_bounds.height() != max_surface_height
            {
                invalid_area = InvalidArea::Full;
                lo.surface_bounds = Rect::from_xywh(
                    visible_bounds.x,
                    visible_bounds.y,
                    max_surface_width,
                    max_surface_height,
                );
            } else {
                // Handle visible bound change
                let common_visible_bounds = visible_bounds.clone();
                if !common_visible_bounds
                    .intersect(&lo.visible_bounds)
                    .is_empty()
                {
                    if common_visible_bounds.x > visible_bounds.x {
                        let new_rect = Rect::from_xywh(
                            visible_bounds.x,
                            visible_bounds.y,
                            common_visible_bounds.x - visible_bounds.x,
                            visible_bounds.height(),
                        );
                        invalid_area.add_rect(Some(&new_rect));
                    }
                    if common_visible_bounds.y > visible_bounds.y {
                        let new_rect = Rect::from_xywh(
                            visible_bounds.x,
                            visible_bounds.y,
                            visible_bounds.width(),
                            common_visible_bounds.y - visible_bounds.y,
                        );
                        invalid_area.add_rect(Some(&new_rect));
                    }
                    if common_visible_bounds.right() < visible_bounds.right() {
                        let new_rect = Rect::from_xywh(
                            common_visible_bounds.right(),
                            visible_bounds.y,
                            visible_bounds.right() - common_visible_bounds.right(),
                            visible_bounds.height(),
                        );
                        invalid_area.add_rect(Some(&new_rect));
                    }
                    if common_visible_bounds.bottom() < visible_bounds.bottom() {
                        let new_rect = Rect::from_xywh(
                            visible_bounds.x,
                            common_visible_bounds.bottom(),
                            visible_bounds.width(),
                            visible_bounds.bottom() - common_visible_bounds.bottom(),
                        );
                        invalid_area.add_rect(Some(&new_rect));
                    }
                } else {
                    invalid_area.add_rect(Some(&visible_bounds));
                }
                // Handle surface change
                if lo.surface_bounds.x > visible_bounds.x {
                    let new_rect = Rect::from_xywh(
                        visible_bounds.x,
                        lo.surface_bounds.y,
                        lo.surface_bounds.x - visible_bounds.x,
                        lo.surface_bounds.height(),
                    );
                    invalid_area.add_rect(Some(&new_rect));
                    lo.surface_bounds
                        .offset((visible_bounds.x - lo.surface_bounds.x, 0.0));
                } else if lo.surface_bounds.right() < visible_bounds.right() {
                    let new_rect = Rect::from_xywh(
                        lo.surface_bounds.right(),
                        lo.surface_bounds.y,
                        visible_bounds.right() - lo.surface_bounds.right(),
                        lo.surface_bounds.height(),
                    );
                    invalid_area.add_rect(Some(&new_rect));
                    lo.surface_bounds
                        .offset((visible_bounds.right() - lo.surface_bounds.right(), 0.0));
                }
                if lo.surface_bounds.y > visible_bounds.y {
                    let new_rect = Rect::from_xywh(
                        lo.surface_bounds.x,
                        visible_bounds.y,
                        lo.surface_bounds.width(),
                        lo.surface_bounds.y - visible_bounds.y,
                    );
                    invalid_area.add_rect(Some(&new_rect));
                    lo.surface_bounds
                        .offset((0.0, visible_bounds.y - lo.surface_bounds.y));
                } else if lo.surface_bounds.bottom() < visible_bounds.bottom() {
                    let new_rect = Rect::from_xywh(
                        lo.surface_bounds.x,
                        lo.surface_bounds.bottom(),
                        lo.surface_bounds.width(),
                        visible_bounds.bottom() - lo.surface_bounds.bottom(),
                    );
                    invalid_area.add_rect(Some(&new_rect));
                    lo.surface_bounds
                        .offset((0.0, visible_bounds.bottom() - lo.surface_bounds.bottom()));
                }
            }
            lo.visible_bounds = visible_bounds.clone();
            invalid_area.build(visible_bounds.clone())
        };
        let normal_nodes =
            self.build_paint_normal_nodes(&lod.normal_nodes, viewport, &invalid_rects);
        let mut layers = Vec::new();
        for lo in &lod.layer_nodes {
            layers.push(self.build_paint_layer_node(lo, viewport));
        }

        let lo = &mut self.layout_tree.layer_objects[lod.layer_object_idx];
        let lpo = LayerPO {
            matrix: lo.matrix.clone(),
            total_matrix: lo.total_matrix.clone(),
            width: lo.width,
            height: lo.height,
            elements: normal_nodes,
            layers,
            key: lo.key.clone(),
            origin_absolute_pos: lo.origin_absolute_pos,
            visible_bounds: lo.visible_bounds.clone(),
            surface_bounds: lo.surface_bounds.clone(),
            invalid_rects,
            clip_rect: lo.clip_rect.clone(),
        };
        lo.invalid_area = InvalidArea::None;
        lpo
    }
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
        InvalidRects { rects: vec![] }
    }
}

impl InvalidRects {
    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }
    pub fn has_intersects(&self, rect: &Rect) -> bool {
        for r in &self.rects {
            let intersect = f32::min(r.right(), rect.right()) >= f32::max(r.left(), rect.left())
                && f32::min(r.bottom(), rect.bottom()) >= f32::max(r.y, rect.y);
            if intersect {
                return true;
            }
        }
        false
    }
    pub fn to_path(&self) -> Path {
        let mut path = Path::new();
        for r in &self.rects {
            path.add_rect(r.to_skia_rect(), None);
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
        Self { id, rect }
    }
}

impl InvalidArea {
    pub fn merge(&mut self, other: &Self) {
        match self {
            InvalidArea::Full => {}
            InvalidArea::Partial(p) => match other {
                InvalidArea::Full => {
                    *self = InvalidArea::Full;
                }
                InvalidArea::Partial(o) => {
                    p.merge(o);
                }
                InvalidArea::None => {}
            },
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
                    for (_, r) in &pia.rects {
                        rects.push(r.clone());
                    }
                }
            }
            InvalidArea::None => {}
        }
        InvalidRects { rects }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct PartialInvalidArea {
    rects: HashMap<u64, Rect>,
}

impl PartialInvalidArea {
    pub fn new() -> PartialInvalidArea {
        Self {
            rects: HashMap::new(),
        }
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
        Self { cpu_renderer }
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

#[cfg(test)]
pub mod tests {
    use crate::base::Rect;
    use crate::paint::InvalidArea;
    use log::debug;
    use measure_time::print_time;
    use skia_safe::{Matrix, Path, Vector};

    #[test]
    pub fn test_visible() {
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
    pub fn test_rect_intersect() {
        let rect = Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
        let rect2 = Rect::from_xywh(50.0, 50.0, 100.0, 100.0);
        rect.intersect(&rect2);
        debug!("{:?}", rect);
    }

    #[test]
    pub fn test_path() {
        let empty_path = Path::new();
        assert!(!empty_path.contains((0.0, 0.0)));

        let rect = Rect::from_xywh(10.0, 70.0, 100.0, 100.0);
        let path = Path::rect(rect.to_skia_rect(), None);
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
        debug!("s={},{}", sx, sy);
        debug!("d={},{}", d.x, d.y);

        let invert_matrix = matrix.invert().unwrap();
        let r = invert_matrix.map_xy(d.x, d.y);
        debug!("r={}, {}", r.x, r.y);

        {
            print_time!("map rect time");
            for i in 0..10000 {
                let rect = Rect::from_xywh(0.0, 0.0, i as f32, i as f32);
                let _p = matrix.map_rect(&rect.to_skia_rect());
            }
        }
    }
}
