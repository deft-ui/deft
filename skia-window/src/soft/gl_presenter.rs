use crate::gl::SurfaceState;
use crate::soft::surface_presenter::SurfacePresenter;
use crate::surface::RenderBackend;
use skia_safe::{Canvas};
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;
use crate::renderer::Renderer;

pub struct GlPresenter {
    surface_state: SurfaceState,
    width: u32,
    height: u32,
}

impl GlPresenter {
    pub fn new(event_loop: &ActiveEventLoop, window: Window) -> Option<GlPresenter> {
        let size = window.inner_size();
        let width = size.width;
        let height = size.height;
        let surface_state = SurfaceState::new(event_loop, window)?;
        Some(Self {
            surface_state,
            width,
            height,
        })
    }
}

impl SurfacePresenter for GlPresenter {
    fn window(&self) -> &Window {
        self.surface_state.window()
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.surface_state.resize(width, height);
    }

    fn render(&mut self, renderer: Box<dyn FnOnce(&Canvas) + Send>, callback: Box<dyn FnOnce(bool) + Send + 'static>) {
        let gl_renderer = Renderer::new(|canvas, _ctx| {
            renderer(canvas);
        });
        self.surface_state.render(gl_renderer, callback);
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
