use std::cell::RefCell;
use std::collections::HashMap;
use std::{mem, slice};
use std::mem::ManuallyDrop;
use std::num::NonZeroU32;
use skia_safe::ImageInfo;

use softbuffer::{Context, Surface};
use winit::window::{Window, WindowId};
use crate::renderer::CpuRenderer;

thread_local! {
        // NOTE: You should never do things like that, create context and drop it before
        // you drop the event loop. We do this for brevity to not blow up examples. We use
        // ManuallyDrop to prevent destructors from running.
        //
        // A static, thread-local map of graphics contexts to open windows.
        pub static GC: ManuallyDrop<RefCell<Option<GraphicsContext>>> = const { ManuallyDrop::new(RefCell::new(None)) };
    }

/// The graphics context used to draw to a window.
pub struct GraphicsContext {
    /// The global softbuffer context.
    context: RefCell<Context<&'static Window>>,

    /// The hash map of window IDs to surfaces.
    surfaces: HashMap<WindowId, Surface<&'static Window, &'static Window>>,
}

impl GraphicsContext {
    pub fn new(w: &Window) -> Self {
        Self {
            context: RefCell::new(
                Context::new(unsafe { mem::transmute::<&'_ Window, &'static Window>(w) })
                    .expect("Failed to create a softbuffer context"),
            ),
            surfaces: HashMap::new(),
        }
    }

    pub fn create_surface(
        &mut self,
        window: &Window,
    ) -> &mut Surface<&'static Window, &'static Window> {
        self.surfaces.entry(window.id()).or_insert_with(|| {
            Surface::new(&self.context.borrow(), unsafe {
                mem::transmute::<&'_ Window, &'static Window>(window)
            })
                .expect("Failed to create a softbuffer surface")
        })
    }

    pub fn destroy_surface(&mut self, window: &Window) {
        self.surfaces.remove(&window.id());
    }
}

pub fn fill_window(window: &Window) {
    GC.with(|gc| {
        let size = window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
            else {
                return;
            };

        // Either get the last context used or create a new one.
        let mut gc = gc.borrow_mut();
        let surface =
            gc.get_or_insert_with(|| GraphicsContext::new(window)).create_surface(window);

        // Fill a buffer with a solid color.
        const DARK_GRAY: u32 = 0xff181818;

        surface.resize(width, height).expect("Failed to resize the softbuffer surface");

        let mut buffer = surface.buffer_mut().expect("Failed to get the softbuffer buffer");
        buffer.fill(DARK_GRAY);
        buffer.present().expect("Failed to present the softbuffer buffer");
    })
}

#[allow(dead_code)]
pub fn cleanup_window(window: &Window) {
    GC.with(|gc| {
        let mut gc = gc.borrow_mut();
        if let Some(context) = gc.as_mut() {
            context.destroy_surface(window);
        }
    });
}

fn with_surface<R, F: FnOnce(&mut softbuffer::Surface<&Window, &Window>) -> R>(window: &Window, callback: F) -> R {
    GC.with(|gc| {
        let mut gc = gc.borrow_mut();
        let surface =
            gc.get_or_insert_with(|| GraphicsContext::new(&window)).create_surface(&window);
        callback(surface)
    })
}

fn resize(window: &Window, width: u32, height: u32) {
    with_surface(window,move |surface| {
        surface.resize(NonZeroU32::new(width).unwrap(), NonZeroU32::new(height).unwrap())
            .expect("Failed to resize the softbuffer surface");
    });
}

fn draw(window: &Window) {
    let buf_ptr = with_surface(window, |surface| {
        let mut buffer = surface.buffer_mut().expect("Failed to get the softbuffer buffer");
        let buf_ptr = buffer.as_mut_ptr() as *mut u8;
        unsafe {
            slice::from_raw_parts_mut(buf_ptr, buffer.len() * 4)
        }
    });
    //TODO test
    // let mut renderer = CpuRenderer::new(1, 1);
    // let src_img_info = renderer.surface().image_info();
    // let img_info = ImageInfo::new((width as i32, height as i32), ColorType::BGRA8888, src_img_info.alpha_type(), src_img_info.color_space());
    // let _ = renderer.canvas().read_pixels(&img_info, buf_ptr , width as usize * 4, (0, 0));

    with_surface(window, |surface| {
        surface.buffer_mut().unwrap().present().expect("Failed to present the softbuffer buffer");
    });
}