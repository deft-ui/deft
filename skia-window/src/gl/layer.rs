use crate::layer::ILayer;
use skia_safe::gpu::{BackendTexture, DirectContext};
use skia_safe::{Canvas, Image, Surface};

pub struct GlLayer {
    context: DirectContext,
    image: Image,
    surface: Surface,
    backend_texture: BackendTexture,
}

impl GlLayer {
    pub fn new(
        context: DirectContext,
        backend_texture: BackendTexture,
        image: Image,
        surface: Surface,
    ) -> Self {
        Self {
            context,
            image,
            surface,
            backend_texture,
        }
    }
}

impl Drop for GlLayer {
    fn drop(&mut self) {
        self.context.delete_backend_texture(&self.backend_texture);
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
