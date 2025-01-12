use std::collections::HashMap;
use std::mem;
use skia_safe::{Canvas, ClipOp, Color, Image, Matrix, Paint, PaintStyle, Rect};
use skia_window::context::RenderContext;
use crate::canvas_util::CanvasHelper;
use crate::paint::{InvalidRects, LayerState, RenderLayerKey, RenderState};
use crate::render::paint_layer::PaintLayer;
use crate::render::paint_object::{ElementPaintObject, LayerPaintObject, PaintObject};
use crate::render::paint_tree::PaintTree;
use crate::{show_repaint_area, some_or_continue};

pub struct ElementPainter {
    scale: f32,
    viewport: Rect,
    layer_state_map: HashMap<RenderLayerKey, LayerState>,
}

impl ElementPainter {

    fn new() -> Self {
        Self {
            scale: 1.0,
            viewport: Rect::new_empty(),
            layer_state_map: HashMap::new(),
        }
    }

    pub fn take(ctx: &mut RenderContext) -> Self {
        ctx.user_context.take::<Self>().unwrap_or_else(
            || Self::new()
        )
    }
    pub fn put(self, ctx: &mut RenderContext) {
        ctx.user_context.set(self);
    }

    pub fn update_viewport(&mut self, scale: f32, viewport: Rect) {
        self.scale = scale;
        self.viewport = viewport;
    }

    pub fn draw_root(&mut self, canvas: &Canvas, obj: &mut PaintTree, context: &mut RenderContext) {
        let mut state = mem::take(&mut self.layer_state_map);
        self.draw_paint_object(canvas, &mut obj.root, context, &mut state);
        for k in &obj.all_layer_keys {
            let layer = some_or_continue!(state.remove(k));
            self.layer_state_map.insert(k.clone(), layer);
        }
        for k in &obj.all_layer_keys {
            // println!("Merging layer {:?}", k);
            let layer = some_or_continue!(self.layer_state_map.get_mut(&k));
            let img = layer.layer.as_image();
            canvas.save();
            canvas.concat(&layer.total_matrix);
            canvas.scale((1.0 / self.scale, 1.0 / self.scale));
            canvas.draw_image(img, (0.0, 0.0), None);
            if show_repaint_area() {
                canvas.scale((self.scale, self.scale));
                let path = layer.invalid_rects.to_path();
                if !path.is_empty() {
                    let mut paint = Paint::default();
                    paint.set_style(PaintStyle::Stroke);
                    paint.set_color(Color::from_rgb(200, 0, 0));
                    canvas.draw_path(&path, &paint);
                }
            }
            canvas.restore();
            unsafe  {
                let sf = canvas.surface();
                let sf = sf.unwrap();
                sf.direct_context().unwrap().flush_and_submit();
            }
        }
    }

    fn draw_paint_object(&mut self, canvas: &Canvas, obj: &mut PaintObject, context: &mut RenderContext, layer_states: &mut HashMap<RenderLayerKey, LayerState>) {
        match obj {
            PaintObject::Normal(epo) => {
                // println!("Painting {}", epo.element_id);
                canvas.save();
                canvas.translate(epo.coord);
                canvas.clip_path(&epo.border_box_path, ClipOp::Intersect, false);
                self.draw_element_paint_object(canvas, epo);
                self.draw_paint_objects(canvas, &mut epo.children, context, layer_states);
                canvas.restore();
            }
            PaintObject::Layer(lpo) => {
                unsafe  {
                    let sf = canvas.surface();
                    let sf = sf.unwrap();
                    sf.direct_context().unwrap().flush_and_submit();
                }
                //TODO apply matrix
                canvas.save();
                // println!("Updating layer: {:?} , {:?}", lpo.key, lpo.invalid_rects);
                self.draw_layer(canvas, context, lpo, layer_states);
                canvas.restore();
                unsafe  {
                    let sf = canvas.surface();
                    let sf = sf.unwrap();
                    sf.direct_context().unwrap().flush_and_submit();
                }
            }
        }
    }

    pub fn draw_layer(
        &mut self,
        root_canvas: &Canvas,
        context: &mut RenderContext,
        layer: &mut LayerPaintObject,
        layer_state_map: &mut HashMap<RenderLayerKey, LayerState>,
    ) {
        let viewport = &self.viewport;
        let scale = self.scale;
        let max_len = (viewport.width() * viewport.width() + viewport.height() * viewport.height()).sqrt();
        let surface_width = (f32::min(layer.width, max_len) * scale) as usize;
        let surface_height = (f32::min(layer.height, max_len) * scale) as usize;
        if surface_width <= 0 || surface_height <= 0 {
            return;
        }
        let mut graphic_layer = if let Some(mut ogl_state) = layer_state_map.remove(&layer.key) {
            if ogl_state.surface_width != surface_width || ogl_state.surface_height != surface_height {
                None
            } else {
                //TODO fix scroll delta
                let scroll_delta_x = layer.scroll_left - ogl_state.last_scroll_left;
                let scroll_delta_y = layer.scroll_top - ogl_state.last_scroll_top;
                if scroll_delta_x != 0.0 || scroll_delta_y != 0.0 {
                    //TODO optimize size
                    let tp_width = (viewport.width() * scale) as usize;
                    let tp_height = (viewport.height() * scale) as usize;
                    let mut temp_gl = context.create_layer(tp_width, tp_height).unwrap();
                    temp_gl.canvas().session(|canvas| {
                        canvas.clip_rect(&Rect::new(0.0, 0.0, layer.width * scale, layer.height * scale), ClipOp::Intersect, false);
                        canvas.draw_image(&ogl_state.layer.as_image(), (-scroll_delta_x * scale, -scroll_delta_y * scale), None);
                    });
                    unsafe {
                        let sf = root_canvas.surface();
                        let sf = sf.unwrap();
                        sf.direct_context().unwrap().flush_and_submit();
                    }
                    ogl_state.layer.canvas().session(|canvas| {
                        canvas.clear(Color::TRANSPARENT);
                        canvas.clip_rect(&Rect::from_xywh(0.0, 0.0, layer.width, layer.height), ClipOp::Intersect, false);
                        canvas.scale((1.0 / scale, 1.0 / scale));
                        canvas.draw_image(&temp_gl.as_image(), (0.0, 0.0), None);
                    });
                    unsafe {
                        let sf = root_canvas.surface();
                        let sf = sf.unwrap();
                        sf.direct_context().unwrap().flush_and_submit();
                    }
                    ogl_state.last_scroll_left = layer.scroll_left;
                    ogl_state.last_scroll_top = layer.scroll_top;
                }
                Some(ogl_state)
            }
        } else {
            None
        }.unwrap_or_else(|| {
            let mut gl = context.create_layer(surface_width, surface_height).unwrap();
            gl.canvas().scale((scale, scale));
            LayerState {
                layer: gl,
                last_scroll_left: layer.scroll_left,
                last_scroll_top: layer.scroll_top,
                surface_width,
                surface_height,
                total_matrix: Matrix::default(),
                invalid_rects: InvalidRects::default(),
            }
        });
        graphic_layer.invalid_rects = layer.invalid_rects.clone();
        graphic_layer.total_matrix = layer.total_matrix.clone();
        let layer_canvas = graphic_layer.layer.canvas();
        layer_canvas.save();
        if (!layer.invalid_rects.is_empty()) {
            layer_canvas.clip_path(&layer.invalid_rects.to_path(), ClipOp::Intersect, false);
            layer_canvas.clip_rect(&Rect::from_xywh(0.0, 0.0, layer.width, layer.height), ClipOp::Intersect, false);
            layer_canvas.clear(Color::TRANSPARENT);
        }
        self.draw_paint_objects(layer_canvas, &mut layer.objects, context, layer_state_map);
        layer_canvas.restore();

        self.layer_state_map.insert(layer.key.clone(), graphic_layer);
    }

    fn draw_paint_objects(&mut self, canvas: &Canvas, objs: &mut Vec<PaintObject>, context: &mut RenderContext, layer_states: &mut HashMap<RenderLayerKey, LayerState>) {
        for obj in objs {
            self.draw_paint_object(canvas, obj, context, layer_states);
        }
    }

    fn draw_element_paint_object(&mut self, canvas: &Canvas, node: &mut ElementPaintObject) {
        let width = node.width;
        let height = node.height;
        //TODO fix clip
        // node.clip_chain.apply(canvas);
        // canvas.concat(&node.total_matrix);
        // node.clip_path.apply(canvas);

        canvas.session(move |canvas| {

            // draw background and border
            node.draw_background(&canvas);
            node.draw_border(&canvas);

            // draw padding box and content box
            canvas.save();
            if width > 0.0 && height > 0.0 {
                let (border_top_width, _, _, border_left_width) = node.border_width;
                // let (padding_top, _, _, padding_left) = element.get_padding();
                // draw content box
                canvas.translate((border_left_width, border_top_width));
                // let paint_info = some_or_return!(&mut node.paint_info);
                if let Some(render_fn) = node.render_fn.take() {
                    render_fn.run(canvas);
                }
            }
            canvas.restore();
        });
    }
}