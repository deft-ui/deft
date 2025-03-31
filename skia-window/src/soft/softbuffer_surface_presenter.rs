use crate::soft::surface_presenter::SurfacePresenter;
use skia_safe::{AlphaType, Canvas, ColorSpace, ColorType, ImageInfo};
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::slice;
use winit::window::Window;

pub struct SoftBufferSurfacePresenter {
    width: u32,
    height: u32,
    win_surface: Surface<Rc<Window>, Rc<Window>>,
}

impl SoftBufferSurfacePresenter {
    pub fn new(window: Window) -> SoftBufferSurfacePresenter {
        let window = Rc::new(window);
        let context = Context::new(window.clone()).unwrap();
        let mut win_surface = Surface::new(&context, window.clone()).unwrap();
        let size = window.inner_size();
        win_surface.resize(
            NonZeroU32::new(size.width).unwrap(),
            NonZeroU32::new(size.height).unwrap(),
        );
        Self {
            win_surface,
            width: size.width,
            height: size.height,
        }
    }
}

impl SurfacePresenter for SoftBufferSurfacePresenter {
    fn window(&self) -> &Window {
        self.win_surface.window()
    }
    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.win_surface.resize(NonZeroU32::new(width).unwrap(), NonZeroU32::new(height).unwrap());
    }

    fn render(&mut self, renderer: Box<dyn FnOnce(&Canvas)>) {
        let mut buffer = self
            .win_surface
            .buffer_mut()
            .expect("Failed to get the softbuffer buffer");
        let buf_ptr = buffer.as_mut_ptr() as *mut u8;
        let buf_ptr = unsafe { slice::from_raw_parts_mut(buf_ptr, buffer.len() * 4) };

        let width = self.width;
        let height = self.height;
        let img_info = ImageInfo::new(
            (width as i32, height as i32),
            ColorType::BGRA8888,
            AlphaType::Premul,
            Some(ColorSpace::new_srgb())
        );
        let mut surface = skia_safe::surfaces::wrap_pixels(&img_info, buf_ptr, width as usize * 4, None).unwrap();
        renderer(&mut surface.canvas());
        buffer
            .present()
            .expect("Failed to present the softbuffer buffer");
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
