use std::cell::RefCell;
use std::ffi::CString;
use std::num::NonZeroU32;

use ::gl::GetIntegerv;
use gl::types::GLint;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::PossiblyCurrentContext;
use glutin::display::{Display, GetGlDisplay};
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use raw_window_handle::HasRawWindowHandle;
use skia_safe::{Canvas, ColorType, gpu, Surface};
use skia_safe::gpu::{backend_render_targets, SurfaceOrigin};
use skia_safe::gpu::gl::FramebufferInfo;
#[cfg(glx_backend)]
use winit::platform::x11;
use winit::window::Window;

pub struct GlRenderer {
    context: RefCell<RenderContext>,
}


#[derive(Copy, Clone)]
struct SurfaceParams {
    num_samples: usize,
    stencil_size: usize,
    frame_buffer_info: FramebufferInfo,
}

struct RenderContext {
    surface: Surface,
    gr_context: gpu::DirectContext,
    surface_params: SurfaceParams,
    gl_surface: glutin::surface::Surface<WindowSurface>,
    context: PossiblyCurrentContext,
}

impl GlRenderer {
    pub fn new(gl_display: &Display, window: &Window, gl_surface: glutin::surface::Surface<WindowSurface>, context: PossiblyCurrentContext) -> Self {
        unsafe {
            gl::load_with(|s| {
                gl_display
                    .get_proc_address(CString::new(s).unwrap().as_c_str())
            });

            let template = ConfigTemplateBuilder::new()
                .with_alpha_size(8)
                .with_transparency(true).build();

            let configs = gl_display.find_configs(template).unwrap();
            let gl_config = configs.reduce(|accum, config| {
                let transparency_check = config.supports_transparency().unwrap_or(false)
                    & !accum.supports_transparency().unwrap_or(false);

                if transparency_check || config.num_samples() < accum.num_samples() {
                    config
                } else {
                    accum
                }
            })
                .unwrap();


            let interface = gpu::gl::Interface::new_load_with(|name| {
                if name == "eglGetCurrentDisplay" {
                    return std::ptr::null();
                }
                gl_display
                    .get_proc_address(CString::new(name).unwrap().as_c_str())
            })
                .expect("Could not create interface");

            let mut gr_context = gpu::direct_contexts::make_gl(interface, None)
                .expect("Could not create direct context");

            let fb_info = {
                let mut fboid: GLint = 0;
                unsafe { GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };


                FramebufferInfo {
                    fboid: fboid.try_into().unwrap(),
                    format: gpu::gl::Format::RGBA8.into(),
                    ..Default::default()
                }
            };


            let num_samples = gl_config.num_samples() as usize;
            let stencil_size = gl_config.stencil_size() as usize;

            let surface_params = SurfaceParams {
                num_samples,
                stencil_size,
                frame_buffer_info: fb_info,
            };
            let surface = Self::create_surface(&window, &mut gr_context, &surface_params);
            let context = RenderContext { surface, gr_context, surface_params, gl_surface, context };

            Self { context: RefCell::new(context) }
        }
    }

    pub fn draw<F: FnOnce(&Canvas)>(&mut self, drawer: F) {
        self.make_current();

        let mut context = self.context.borrow_mut();
        let canvas = context.surface.canvas();
        drawer(canvas);
        context.gr_context.flush_and_submit();

        if let Err(err) = context.gl_surface.swap_buffers(&context.context) {
            log::error!("Failed to swap buffers after render: {}", err);
        }

    }

    pub fn resize(&self, window: &Window, width: u32, height: u32) {
        let mut context = self.context.borrow_mut();
        let sf_params = context.surface_params.clone();
        context.surface = Self::create_surface(
            &window,
            &mut context.gr_context,
            &sf_params,
        );
        /* First resize the opengl drawable */

        context.gl_surface.resize(
            &context.context,
            NonZeroU32::new(width.max(1)).unwrap(),
            NonZeroU32::new(height.max(1)).unwrap(),
        );
    }

    fn make_current(&mut self) {
        let context = self.context.borrow_mut();
        context.context.make_current(&context.gl_surface).unwrap();
    }

    fn create_surface(
        window: &Window,
        gr_context: &mut gpu::DirectContext,
        surface_params: &SurfaceParams,
    ) -> Surface {
        let num_samples = surface_params.num_samples;
        let stencil_size = surface_params.stencil_size;
        let fb_info = surface_params.frame_buffer_info;
        let size = window.inner_size();
        let size = (
            size.width.try_into().expect("Could not convert width"),
            size.height.try_into().expect("Could not convert height"),
        );
        let backend_render_target =
            backend_render_targets::make_gl(size, num_samples, stencil_size, fb_info);

        gpu::surfaces::wrap_backend_render_target(
            gr_context,
            &backend_render_target,
            SurfaceOrigin::BottomLeft,
            ColorType::RGBA8888,
            None,
            None,
        )
            .expect("Could not create skia surface")
    }
}