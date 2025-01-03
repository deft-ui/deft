use std::num::NonZeroU32;
use std::rc::Rc;
use softbuffer::{Context, Surface};
use winit::window::Window;
use crate::context::IRenderContext;
use crate::layer::ILayer;
use crate::softbuffer::layer::SoftLayer;

pub struct SoftRenderContext {
    pub win_surface: Surface<Rc<Window>, Rc<Window>>,
}

impl SoftRenderContext {
    pub fn new(window: Window) -> SoftRenderContext {
        let window = Rc::new(window);
        let context = Context::new(window.clone()).unwrap();
        let mut win_surface = Surface::new(&context, window.clone()).unwrap();
        let size = window.inner_size();
        win_surface.resize(NonZeroU32::new(size.width).unwrap(), NonZeroU32::new(size.height).unwrap());
        Self {
            win_surface
        }
    }
}

impl IRenderContext for SoftRenderContext {
    fn create_layer(&mut self, width: usize, height: usize) -> Option<Box<dyn ILayer>> {
        let layer = SoftLayer::new(width as u32, height as u32);
        Some(Box::new(layer))
    }
}
