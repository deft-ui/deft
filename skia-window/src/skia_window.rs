use std::ops::Deref;

use skia_safe::Canvas;
use winit::event_loop::ActiveEventLoop;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
use winit::window::{Window, WindowAttributes};
use crate::gl::SurfaceState;
use crate::renderer::Renderer;
use crate::softbuffer::gl_presenter::GlPresenter;
use crate::softbuffer::SoftSurface;
use crate::surface::RenderBackend;

pub struct SkiaWindow {
    //surface_state: SurfaceState,
    surface_state: Box<dyn RenderBackend>,
}

#[derive(Debug, Copy, Clone)]
pub enum RenderBackendType {
    SoftBuffer,
    GL,
    SoftGL,
}

impl SkiaWindow {
    pub fn new(event_loop: &ActiveEventLoop, attributes: WindowAttributes, backend: RenderBackendType) -> Option<Self> {
        let window = event_loop.create_window(attributes).unwrap();
        let surface_state: Box<dyn RenderBackend> = match backend {
            RenderBackendType::SoftBuffer => {
                #[cfg(target_os = "android")]
                return None;
                #[cfg(not(target_os = "android"))]
                {
                    use crate::softbuffer::softbuffer_surface_presenter::SoftBufferSurfacePresenter;
                    Box::new(SoftSurface::new(event_loop, SoftBufferSurfacePresenter::new(window)))
                }
            }
            RenderBackendType::SoftGL => {
                Box::new(SoftSurface::new(event_loop, GlPresenter::new(event_loop, window)?))
            }
            RenderBackendType::GL => {
                Box::new(SurfaceState::new(event_loop, window)?)
            }
        };
        Some(Self { surface_state })
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) {
        // self.surface_state.render.resize(&self.winit_window(), width, height);
        self.surface_state.resize(width, height);
    }

    pub fn scale_factor(&self) -> f64 {
        self.winit_window().scale_factor()
    }

}

impl Deref for SkiaWindow {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        &self.surface_state.window()
        //&self.surface_state.window
    }
}

impl SkiaWindow {

    pub fn winit_window(&self) -> &Window {
        &self.surface_state.window()
    }

    pub fn render_with_result<C: FnOnce(bool) + Send + 'static>(&mut self, renderer: Renderer, callback: C) {
        // self.surface_state.render.draw(renderer);
        self.surface_state.render(renderer, Box::new(callback));
    }

    pub fn render(&mut self, renderer: Renderer) {
        // self.surface_state.render.draw(renderer);
        self.render_with_result(renderer, |_| ());
    }
}