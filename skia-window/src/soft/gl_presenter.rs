use crate::gl::SurfaceState;
use crate::renderer::Renderer;
use crate::soft::surface_presenter::SurfacePresenter;
use crate::surface::RenderBackend;
use skia_safe::{Paint, Surface};
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

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
        let surface_state = SurfaceState::new(event_loop, window).unwrap();
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

    fn present_surface(&mut self, skia_surface: &mut Surface) {
        let img = skia_surface.image_snapshot();
        let renderer = Renderer::new(move |canvas, ctx| {
            canvas.draw_image(&img, (0.0, 0.0), Some(&Paint::default()));
        });
        self.surface_state.render(renderer, Box::new(|_| {}));
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
