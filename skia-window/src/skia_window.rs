use std::ops::Deref;

use skia_safe::Canvas;
use winit::event_loop::ActiveEventLoop;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
use winit::window::{Window, WindowAttributes};

use crate::gl_surface::SurfaceState;
#[cfg(not(target_os = "android"))]
use crate::soft_surface::SoftSurface;
use crate::surface::RenderBackend;

pub struct SkiaWindow {
    //surface_state: SurfaceState,
    surface_state: Box<dyn RenderBackend>,
}

pub enum RenderBackendType {
    SoftBuffer,
    GL,
}

impl SkiaWindow {
    pub fn new(event_loop: &ActiveEventLoop, attributes: WindowAttributes, backend: RenderBackendType) -> Self {
        let window = event_loop.create_window(attributes).unwrap();
        let surface_state: Box<dyn RenderBackend> = match backend {
            RenderBackendType::SoftBuffer => {
                #[cfg(target_os = "android")]
                panic!("Android does not support this backend!");
                #[cfg(not(target_os = "android"))]
                Box::new(SurfaceState::new(event_loop, window))
            }
            RenderBackendType::GL => {
                Box::new(SurfaceState::new(event_loop, window))
            }
        };
        Self { surface_state }
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

    pub fn render<F: FnOnce(&Canvas) + Send + 'static>(&mut self, renderer: F) {
        // self.surface_state.render.draw(renderer);
        self.surface_state.render(Box::new(renderer))
    }
}