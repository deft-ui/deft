use crate::paint::Canvas;
use crate::soft::surface_presenter::SurfacePresenter;
use skia_safe::{AlphaType, ColorSpace, ColorType, ImageInfo};
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, Mutex};
use std::{slice, thread};
use winit::window::Window;

struct RenderTask {
    surface: Arc<Mutex<Surface<Arc<Window>, Arc<Window>>>>,
    renderer: Box<dyn FnOnce(&Canvas) + Send>,
    callback: Box<dyn FnOnce(bool) + Send + 'static>,
}

unsafe impl Send for RenderTask {}

impl RenderTask {
    pub fn run(self) {
        let mut win_surface = self.surface.lock().unwrap();
        let size = win_surface.window().inner_size();
        let mut buffer = win_surface
            .buffer_mut()
            .expect("Failed to get the softbuffer buffer");
        let width = size.width;
        let height = size.height;
        let img_info = ImageInfo::new(
            (width as i32, height as i32),
            ColorType::BGRA8888,
            AlphaType::Premul,
            Some(ColorSpace::new_srgb()),
        );
        let buf_ptr = buffer.as_mut_ptr() as *mut u8;
        let len = buffer.len() * 4;
        let row_bytes = width as usize * 4;

        let buf_ptr = unsafe { slice::from_raw_parts_mut(buf_ptr, len) };
        let mut surface =
            skia_safe::surfaces::wrap_pixels(&img_info, buf_ptr, row_bytes, None).unwrap();
        (self.renderer)(&mut surface.canvas());
        buffer
            .present()
            .expect("Failed to present the softbuffer buffer");
        (self.callback)(true);
    }
}

pub struct SoftBufferSurfacePresenter {
    width: u32,
    height: u32,
    window: Arc<Window>,
    win_surface: Arc<Mutex<Surface<Arc<Window>, Arc<Window>>>>,
    sender: Sender<RenderTask>,
}

impl SoftBufferSurfacePresenter {
    pub fn new(window: Window) -> SoftBufferSurfacePresenter {
        let window = Arc::new(window);
        let context = Context::new(window.clone()).unwrap();
        let mut win_surface = Surface::new(&context, window.clone()).unwrap();
        let size = window.inner_size();
        let _ = win_surface.resize(
            NonZeroU32::new(size.width).unwrap(),
            NonZeroU32::new(size.height).unwrap(),
        );
        let (sender, receiver) = mpsc::channel::<RenderTask>();
        thread::spawn(move || loop {
            if let Ok(task) = receiver.recv() {
                task.run();
            } else {
                break;
            }
        });
        Self {
            window,
            win_surface: Arc::new(Mutex::new(win_surface)),
            width: size.width,
            height: size.height,
            sender,
        }
    }
}

impl SurfacePresenter for SoftBufferSurfacePresenter {
    fn window(&self) -> &Window {
        self.window.as_ref()
    }
    fn resize(&mut self, width: u32, height: u32) {
        let mut win_surface = self.win_surface.lock().unwrap();
        self.width = width;
        self.height = height;
        let _ = win_surface.resize(
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        );
    }

    fn render(
        &mut self,
        renderer: Box<dyn FnOnce(&Canvas) + Send>,
        callback: Box<dyn FnOnce(bool) + Send + 'static>,
    ) {
        let render_task = RenderTask {
            surface: self.win_surface.clone(),
            renderer,
            callback,
        };
        let _ = self.sender.send(render_task);
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
