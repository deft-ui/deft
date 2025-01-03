use crate::layer::ILayer;
use skia_safe::{Canvas, Image, Surface};

pub struct GlLayer {
    image: Image,
    surface: Surface,
}

impl GlLayer {
    pub fn new(image: Image, surface: Surface) -> Self {
        Self { image, surface }
    }
}

impl ILayer for GlLayer {
    fn canvas(&mut self) -> &Canvas {
        &mut self.surface.canvas()
    }

    fn as_image(&mut self) -> Image {
        self.image.clone()
    }
}
