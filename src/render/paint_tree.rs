use crate::paint::RenderLayerKey;
use crate::render::paint_object::{LayerPaintObject, PaintObject};

pub struct PaintTreeNew {
    pub root: LayerPaintObject,
}