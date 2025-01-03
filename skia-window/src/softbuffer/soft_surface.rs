use std::num::{NonZeroU32};
use std::rc::Rc;
use std::slice;
use skia_safe::{Canvas, ColorType, ImageInfo};
use softbuffer::{Context, Surface};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window};
use crate::context::RenderContext;
use crate::renderer::Renderer;
use crate::softbuffer::context::SoftRenderContext;
use crate::softbuffer::soft_renderer::SoftRenderer;
use crate::surface::RenderBackend;

pub struct SoftSurface {
    context: SoftRenderContext,
    width: u32,
    height: u32,
    renderer: SoftRenderer,
}

impl SoftSurface {
    pub fn new(_event_loop: &ActiveEventLoop, window: Window) -> Self {
        let size = window.inner_size();
        let context = SoftRenderContext::new(window);
        let renderer = SoftRenderer::new(size.width as i32, size.height as i32);
        Self {
            context,
            width: size.width,
            height: size.height,
            renderer,
        }
    }
}

impl RenderBackend for SoftSurface {
    fn window(&self) -> &Window {
        self.context.win_surface.window()
    }

    fn render(&mut self, draw: Renderer, callback: Box<dyn FnOnce(bool) + Send + 'static>) {
        let mut skia_surface = self.renderer.skia_surface().clone();
        let mut ctx = RenderContext::new(&mut self.context);
        let canvas = skia_surface.canvas();
        draw.render(canvas, &mut ctx);
        {
            let mut buffer = self.context.win_surface.buffer_mut().expect("Failed to get the softbuffer buffer");
            let buf_ptr = buffer.as_mut_ptr() as *mut u8;
            let buf_ptr = unsafe {
                slice::from_raw_parts_mut(buf_ptr, buffer.len() * 4)
            };

            let width = self.width;
            let height = self.height;

            let src_img_info = skia_surface.image_info();
            let img_info = ImageInfo::new((width as i32, height as i32), ColorType::BGRA8888, src_img_info.alpha_type(), src_img_info.color_space());
            let _ = skia_surface.canvas().read_pixels(&img_info, buf_ptr, width as usize * 4, (0, 0));
        }
        self.context.win_surface.buffer_mut().unwrap().present().expect("Failed to present the softbuffer buffer");
        callback(true);
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.context.win_surface.resize(NonZeroU32::new(width).unwrap(), NonZeroU32::new(height).unwrap()).unwrap();
        self.renderer = SoftRenderer::new(width as i32, height as i32);
        self.width = width;
        self.height = height;
    }
}