use crate::paint::RenderLayerKey;
use crate::render::paint_object::{LayerPaintObject, PaintObject};

pub struct PaintTree {
    pub all_layer_keys: Vec<RenderLayerKey>,
    pub root: PaintObject,
}

pub struct PaintTreeNew {
    pub root: LayerPaintObject,
}