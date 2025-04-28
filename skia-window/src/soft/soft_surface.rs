use std::ops::DerefMut;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window};
use crate::context::{RenderContext};
use crate::mrc::Mrc;
use crate::paint::Canvas;
use crate::renderer::Renderer;
use crate::soft::context::SoftRenderContext;
use crate::soft::surface_presenter::SurfacePresenter;
use crate::surface::RenderBackend;

pub struct SoftSurface {
    context: Mrc<SoftRenderContext>,
    width: u32,
    height: u32,
}

impl SoftSurface {
    pub fn new<P: SurfacePresenter + 'static>(_event_loop: &ActiveEventLoop, surface_presenter: P) -> Self {
        let (width, height) = surface_presenter.size();
        let context = SoftRenderContext::new(Box::new(surface_presenter));
        Self {
            context: Mrc::new(context),
            width,
            height,
        }
    }
}

impl RenderBackend for SoftSurface {
    fn window(&self) -> &Window {
        self.context.surface_presenter.window()
    }

    fn render(&mut self, draw: Renderer, callback: Box<dyn FnOnce(bool) + Send + 'static>) {
        let mut user_context = self.context.user_context.take().unwrap();
        let mut p_ctx = self.context.clone();
        let renderer: Box<dyn FnOnce(&Canvas)> = Box::new(move |canvas| {
            let mut ctx = RenderContext::new(p_ctx.deref_mut(), &mut user_context);
            draw.render(canvas, &mut ctx);
            p_ctx.user_context = Some(user_context);
        });
        self.context.surface_presenter.render(renderer);
        callback(true);
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.context.surface_presenter.resize(width, height);
        self.width = width;
        self.height = height;
    }
}