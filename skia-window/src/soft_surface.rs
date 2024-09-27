use std::num::{NonZeroU32};
use std::rc::Rc;
use std::slice;
use skia_safe::{Canvas, ColorType, ImageInfo};
use softbuffer::{Context, Surface};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window};
use crate::soft_renderer::SoftRenderer;
use crate::surface::RenderBackend;

pub struct SoftSurface {
    pub surface: Surface<Rc<Window>, Rc<Window>>,
    width: u32,
    height: u32,
}

impl SoftSurface {
    pub fn new(_event_loop: &ActiveEventLoop, window: Window) -> Self {
        let window = Rc::new(window);
        let context = Context::new(window.clone()).unwrap();
        let surface = Surface::new(&context, window.clone()).unwrap();
        Self {
            surface,
            width: 1,
            height: 1,
        }
    }
}

impl RenderBackend for SoftSurface {
    fn window(&self) -> &Window {
        &self.surface.window()
    }

    fn render(&mut self, draw: Box<dyn FnOnce(&Canvas)>) {
        {
            let mut buffer = self.surface.buffer_mut().expect("Failed to get the softbuffer buffer");
            let buf_ptr = buffer.as_mut_ptr() as *mut u8;
            let buf_ptr = unsafe {
                slice::from_raw_parts_mut(buf_ptr, buffer.len() * 4)
            };

            let width = self.width;
            let height = self.height;
            let mut renderer = SoftRenderer::new(width as i32, height as i32);

            draw(renderer.canvas());

            let src_img_info = renderer.surface().image_info();
            let img_info = ImageInfo::new((width as i32, height as i32), ColorType::BGRA8888, src_img_info.alpha_type(), src_img_info.color_space());
            let _ = renderer.canvas().read_pixels(&img_info, buf_ptr, width as usize * 4, (0, 0));
        }
        self.surface.buffer_mut().unwrap().present().expect("Failed to present the softbuffer buffer");
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.surface.resize(NonZeroU32::new(width).unwrap(), NonZeroU32::new(height).unwrap()).unwrap();
        self.width = width;
        self.height = height;
    }
}