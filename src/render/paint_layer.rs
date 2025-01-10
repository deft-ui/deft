use skia_safe::{Matrix, Path};
use crate::paint::{RenderLayer, RenderLayerKey, RenderTree};
use crate::render::paint_node::PaintNode;

pub struct PaintLayer {
    pub roots: Vec<PaintNode>,
    pub scroll_left: f32,
    pub scroll_top: f32,
    pub key: RenderLayerKey,
    pub width: f32,
    pub height: f32,
    pub total_matrix: Matrix,
    pub invalid_path: Path,
}
