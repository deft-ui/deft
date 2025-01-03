use crate::context::IRenderContext;
use crate::gl::layer::GlLayer;
use crate::layer::ILayer;
use skia_safe::gpu::surfaces::wrap_backend_texture;
use skia_safe::gpu::{Mipmapped, Protected, Renderable, SurfaceOrigin};
use skia_safe::{gpu, AlphaType, ColorType, Image};
use std::num::NonZeroUsize;

#[derive(Clone)]
pub struct GlRenderContext {
    pub gr_context: gpu::DirectContext,
}

impl IRenderContext for GlRenderContext {
    fn create_layer(
        &mut self,
        width: usize,
        height: usize,
    ) -> Option<Box<dyn ILayer>> {
        let bt = self
            .gr_context
            .create_backend_texture(
                width as i32,
                height as i32,
                ColorType::RGBA8888,
                Mipmapped::No,
                Renderable::No,
                Protected::No,
                "layer",
            )
            .unwrap();
        // println!("texture created: {:?}", bt.gl_texture_info());
        let img = Image::from_texture(
            &mut self.gr_context,
            &bt,
            SurfaceOrigin::BottomLeft,
            ColorType::RGBA8888,
            AlphaType::Premul,
            None,
        )?;
        let surface = wrap_backend_texture(
            &mut self.gr_context,
            &bt,
            SurfaceOrigin::BottomLeft,
            None,
            ColorType::RGBA8888,
            None,
            None,
        )?;
        let layer = GlLayer::new(img, surface);
        Some(Box::new(layer))
    }
}
