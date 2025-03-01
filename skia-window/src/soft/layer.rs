use crate::layer::ILayer;
use crate::soft::soft_renderer::SoftRenderer;
use skia_safe::{Canvas, Image};

pub struct SoftLayer {
    renderer: SoftRenderer,
}

impl SoftLayer {
    pub fn new(width: u32, height: u32) -> SoftLayer {
        let mut renderer = SoftRenderer::new(width as i32, height as i32);
        Self { renderer }
    }
}

impl ILayer for SoftLayer {
    fn canvas(&mut self) -> &Canvas {
        self.renderer.canvas()
    }

    fn as_image(&mut self) -> Image {
        self.renderer.skia_surface().image_snapshot()
    }
}
