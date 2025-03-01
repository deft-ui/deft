use std::num::NonZeroU32;
use std::rc::Rc;
use winit::window::Window;
use crate::context::{IRenderContext, UserContext};
use crate::layer::ILayer;
use crate::softbuffer::layer::SoftLayer;
use crate::softbuffer::surface_presenter::SurfacePresenter;

pub struct SoftRenderContext {
    pub surface_presenter: Box<dyn SurfacePresenter>,
    pub user_context: Option<UserContext>,
}

impl SoftRenderContext {
    pub fn new(surface_presenter: Box<dyn SurfacePresenter>) -> SoftRenderContext {
        Self {
            surface_presenter,
            user_context: Some(UserContext::new()),
        }
    }
}

impl IRenderContext for SoftRenderContext {
    fn create_layer(&mut self, width: usize, height: usize) -> Option<Box<dyn ILayer>> {
        let layer = SoftLayer::new(width as u32, height as u32);
        Some(Box::new(layer))
    }

    fn flush(&mut self) {
        // Do nothing
    }
}
