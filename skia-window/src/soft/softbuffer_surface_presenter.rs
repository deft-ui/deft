use crate::paint::Canvas;
use crate::soft::surface_presenter::SurfacePresenter;
use skia_safe::{AlphaType, ColorSpace, ColorType, ImageInfo};
use softbuffer::{Context, Surface};
use std::num::{NonZeroU32};
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, Mutex};
use std::{slice, thread};
use winit::window::Window;

struct WinSurface {
    surface: Surface<Arc<Window>, Arc<Window>>,
    width: u32,
    height: u32,
}

struct RenderTask {
    surface: Arc<Mutex<WinSurface>>,
    renderer: Box<dyn FnOnce(&Canvas) + Send>,
    callback: Box<dyn FnOnce(bool) + Send + 'static>,
}

unsafe impl Send for RenderTask {}

impl RenderTask {
    pub fn run(self) {
        let mut win_surface = self.surface.lock().unwrap();
        let width = win_surface.width;
        let height = win_surface.height;
        let _ = win_surface.surface.resize(
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        );
        let mut buffer = win_surface
            .surface
            .buffer_mut()
            .expect("Failed to get the softbuffer buffer");
        #[cfg(target_os = "android")]
        let color_type = ColorType::RGBA8888;
        #[cfg(not(target_os = "android"))]
        let color_type = ColorType::BGRA8888;
        let img_info = ImageInfo::new(
            (width as i32, height as i32),
            color_type,
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
        // Release buffer
        let _ = win_surface.surface.resize(NonZeroU32::new(1).unwrap(), NonZeroU32::new(1).unwrap());
        (self.callback)(true);
    }
}

pub struct SoftBufferSurfacePresenter {
    window: Arc<Window>,
    win_surface: Arc<Mutex<WinSurface>>,
    sender: Sender<RenderTask>,
}

impl SoftBufferSurfacePresenter {
    pub fn new(window: Window) -> SoftBufferSurfacePresenter {
        let window = Arc::new(window);
        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();
        let size = window.inner_size();
        let _ = surface.resize(
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
            win_surface: Arc::new(Mutex::new(WinSurface {
                width: size.width,
                height: size.height,
                surface,
            })),
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
        win_surface.width = width;
        win_surface.height = height;
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
        let surface = self.win_surface.lock().unwrap();
        (surface.width, surface.height)
    }
}
