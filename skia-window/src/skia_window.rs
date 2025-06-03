use std::collections::HashSet;
use std::ops::Deref;

use crate::renderer::Renderer;
use crate::soft::SoftSurface;
use crate::surface::RenderBackend;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

pub struct SkiaWindow {
    surface_state: Box<dyn RenderBackend>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum RenderBackendType {
    SoftBuffer,
    #[cfg(feature = "gl")]
    GL,
    #[cfg(feature = "gl")]
    SoftGL,
}

impl RenderBackendType {
    pub fn all() -> Vec<Self> {
        let mut list = Vec::new();
        list.push(Self::SoftBuffer);
        #[cfg(feature = "gl")]
        list.push(Self::GL);
        #[cfg(feature = "gl")]
        list.push(Self::SoftGL);
        list
    }

    pub fn from_str(backend_type_str: &str) -> Option<Self> {
        match backend_type_str.to_lowercase().as_str() {
            "softbuffer" => Some(RenderBackendType::SoftBuffer),
            #[cfg(feature = "gl")]
            "softgl" => Some(RenderBackendType::SoftGL),
            #[cfg(feature = "gl")]
            "gl" => Some(RenderBackendType::GL),
            _ => None,
        }
    }
    pub fn from_split_str(backend_type_str: &str) -> Vec<RenderBackendType> {
        let list = backend_type_str.split(",").collect::<Vec<&str>>();
        Self::from_str_list(&list)
    }

    pub fn from_str_list(backend_type_list: &Vec<&str>) -> Vec<RenderBackendType> {
        let mut backend_types = Vec::new();
        for bt_str in backend_type_list {
            if let Some(bt) = Self::from_str(bt_str) {
                backend_types.push(bt);
            }
        }
        backend_types
    }

    pub fn merge(
        list1: &Vec<RenderBackendType>,
        list2: &Vec<RenderBackendType>,
    ) -> Vec<RenderBackendType> {
        let mut result = Vec::new();
        let mut added = HashSet::new();
        for list in [list1, list2] {
            for it in list {
                if added.insert(it) {
                    result.push(it.clone());
                }
            }
        }
        result
    }
}

impl SkiaWindow {
    #[allow(unreachable_code)]
    pub fn new(
        event_loop: &ActiveEventLoop,
        attributes: WindowAttributes,
        backend: RenderBackendType,
    ) -> Option<Self> {
        let window = event_loop.create_window(attributes).unwrap();
        let surface_state: Box<dyn RenderBackend> = match backend {
            RenderBackendType::SoftBuffer => {
                use crate::soft::softbuffer_surface_presenter::SoftBufferSurfacePresenter;
                let presenter = SoftBufferSurfacePresenter::new(window);
                let soft_surface = SoftSurface::new(event_loop, presenter);
                Box::new(soft_surface)
            }
            #[cfg(feature = "gl")]
            RenderBackendType::SoftGL => {
                let soft_surface = SoftSurface::new(
                    event_loop,
                    crate::soft::gl_presenter::GlPresenter::new(event_loop, window)?,
                );
                Box::new(soft_surface)
            }
            #[cfg(feature = "gl")]
            RenderBackendType::GL => {
                #[cfg(target_env = "ohos")]
                return None;
                Box::new(crate::gl::SurfaceState::new(event_loop, window)?)
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

    pub fn render_with_result<C: FnOnce(bool) + Send + 'static>(
        &mut self,
        renderer: Renderer,
        callback: C,
    ) {
        // self.surface_state.render.draw(renderer);
        self.surface_state.render(renderer, Box::new(callback));
    }

    pub fn render(&mut self, renderer: Renderer) {
        // self.surface_state.render.draw(renderer);
        self.render_with_result(renderer, |_| ());
    }
}
