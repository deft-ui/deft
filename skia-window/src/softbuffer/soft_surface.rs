use std::num::{NonZeroU32};
use std::rc::Rc;
use std::slice;
use skia_safe::{Canvas, ColorType, ImageInfo};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window};
use crate::context::{RenderContext, UserContext};
use crate::renderer::Renderer;
use crate::softbuffer::context::SoftRenderContext;
use crate::softbuffer::soft_renderer::SoftRenderer;
use crate::softbuffer::surface_presenter::SurfacePresenter;
use crate::surface::RenderBackend;

pub struct SoftSurface {
    context: SoftRenderContext,
    width: u32,
    height: u32,
    renderer: SoftRenderer,
}

impl SoftSurface {
    pub fn new<P: SurfacePresenter + 'static>(_event_loop: &ActiveEventLoop, surface_presenter: P) -> Self {
        let (width, height) = surface_presenter.size();
        let context = SoftRenderContext::new(Box::new(surface_presenter));
        let renderer = SoftRenderer::new(width as i32, height as i32);
        Self {
            context,
            width,
            height,
            renderer,
        }
    }
}

impl RenderBackend for SoftSurface {
    fn window(&self) -> &Window {
        self.context.surface_presenter.window()
    }

    fn render(&mut self, draw: Renderer, callback: Box<dyn FnOnce(bool) + Send + 'static>) {
        let mut skia_surface = self.renderer.skia_surface().clone();
        let mut user_context = self.context.user_context.take().unwrap();
        let mut ctx = RenderContext::new(&mut self.context, &mut user_context);
        let canvas = skia_surface.canvas();
        draw.render(canvas, &mut ctx);
        self.context.surface_presenter.present_surface(&mut skia_surface);
        self.context.user_context = Some(user_context);
        callback(true);
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.context.surface_presenter.resize(width, height);
        self.renderer = SoftRenderer::new(width as i32, height as i32);
        self.width = width;
        self.height = height;
    }
}