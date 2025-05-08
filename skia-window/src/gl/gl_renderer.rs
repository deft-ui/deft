use std::ffi::{CString};
use std::num::NonZeroU32;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::mpsc::channel;
use std::{thread};
use ::gl::GetIntegerv;
use gl::types::{GLint};
use glutin::config::{AsRawConfig, Config, ConfigTemplateBuilder};
use glutin::context::PossiblyCurrentContext;
use glutin::display::{Display};
use glutin::prelude::*;
use glutin::surface::{WindowSurface};
use log::info;
use measure_time::print_time;
use skia_safe::{ColorType, gpu, Surface};
use skia_safe::gpu::{backend_render_targets, SurfaceOrigin};
use skia_safe::gpu::gl::FramebufferInfo;
#[cfg(glx_backend)]
use winit::platform::x11;
use winit::window::Window;
use crate::context::{RenderContext, UserContext};
use crate::gl::context::GlRenderContext;
use crate::renderer::Renderer;

enum RenderMsg {
    Updated,
    Resize(u32, u32),
}

pub struct GlRenderer {
    sender: mpsc::Sender<RenderMsg>,
    render_context_wrapper: RenderContextWrapper,
}


#[derive(Copy, Clone)]
struct SurfaceParams {
    num_samples: usize,
    stencil_size: usize,
    frame_buffer_info: FramebufferInfo,
}

struct GlContext {
    user_context: Option<UserContext>,
    surface: Surface,
    render_context: GlRenderContext,
    // surface_params: SurfaceParams,
    gl_surface: glutin::surface::Surface<WindowSurface>,
    context: PossiblyCurrentContext,
}

unsafe impl Send for GlContext {}

impl GlRenderer {
    pub fn new(
        gl_display: &Display,
        window: &Window,
        gl_surface: glutin::surface::Surface<WindowSurface>,
        context: PossiblyCurrentContext,
        gl_config: &Config,
    ) -> Option<Self> {
        unsafe {
            gl::load_with(|s| {
                gl_display
                    .get_proc_address(CString::new(s).unwrap().as_c_str())
            });

            let interface = gpu::gl::Interface::new_load_with(|name| {
                if name == "eglGetCurrentDisplay" {
                    return std::ptr::null();
                }
                gl_display
                    .get_proc_address(CString::new(name).unwrap().as_c_str())
            })?;

            let mut gr_context = gpu::direct_contexts::make_gl(interface, None)?;

            let fb_info = {
                let mut fboid: GLint = 0;
                GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid);


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

            let size = window.inner_size();
            let size = (
                size.width.try_into().expect("Could not convert width"),
                size.height.try_into().expect("Could not convert height"),
            );
            let surface = {
                // let context = context.lock().unwrap();
                Self::create_surface(size.0, size.1, &mut gr_context, &surface_params)
            };

            let context = context.make_not_current().ok()?.treat_as_possibly_current();
            let render_context = GlRenderContext { gr_context };
            let context =  GlContext {
                surface,
                gl_surface,
                render_context,
                context,
                user_context: Some(UserContext::new()),
            };
            let context = Arc::new(Mutex::new(context));
            let drawer = Arc::new(Mutex::new(None));

            let render_context_wrapper = RenderContextWrapper {
                context,
                surface_params,
                drawer,
            };
            let (sender, receiver) = channel();
            {
                let render_context_wrapper = render_context_wrapper.clone();
                thread::spawn(move || {
                    render_context_wrapper.make_current();
                    loop {
                        let msg = match receiver.recv() {
                            Ok(msg) => msg,
                            Err(_) => break,
                        };
                        match msg {
                            RenderMsg::Updated => {
                                render_context_wrapper.update();
                            }
                            RenderMsg::Resize(width, height) => {
                                render_context_wrapper.resize(width, height);
                            }
                        }
                    }
                    info!("GlRenderer thread stopped");
                });
            }

            Some(Self { sender, render_context_wrapper })
        }
    }

    pub fn draw(&mut self, drawer: Renderer, callback: Box<dyn FnOnce(bool) + Send + 'static>) {
        self.render_context_wrapper.render(drawer, Box::new(callback));
        self.sender.send(RenderMsg::Updated).unwrap();
    }

    pub fn resize(&self, _window: &Window, width: u32, height: u32) {
        self.sender.send(RenderMsg::Resize(width, height)).unwrap();
    }

    fn create_surface(
        width: i32,
        height: i32,
        gr_context: &mut gpu::DirectContext,
        surface_params: &SurfaceParams,
    ) -> Surface {
        let num_samples = surface_params.num_samples;
        let stencil_size = surface_params.stencil_size;
        let fb_info = surface_params.frame_buffer_info;
        let size = (width, height);

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

struct RenderTask {
    pub task: Renderer,
    pub callback: Box<dyn FnOnce(bool) + Send + 'static>,
}

#[derive(Clone)]
pub struct RenderContextWrapper {
    context: Arc<Mutex<GlContext>>,
    drawer: Arc<Mutex<Option<RenderTask>>>,
    surface_params: SurfaceParams,
}

impl RenderContextWrapper {

    pub fn make_current(&self) {
        let context = self.context.lock().unwrap();
        context.context.make_current(&context.gl_surface).unwrap();
    }

    pub fn resize(&self, width: u32, height: u32, ) {
        print_time!("resize time");
        let mut context = self.context.lock().unwrap();
        let sf_params = self.surface_params.clone();
        let sf = GlRenderer::create_surface(
            width as i32,
            height as i32,
            &mut context.render_context.gr_context,
            &sf_params,
        );
        context.surface = sf;
        /* First resize the opengl drawable */

        context.gl_surface.resize(
            &context.context,
            NonZeroU32::new(width.max(1)).unwrap(),
            NonZeroU32::new(height.max(1)).unwrap(),
        );
    }

    pub fn render(&self,  drawer: Renderer, callback: Box<dyn FnOnce(bool) + Send + 'static>) {
        // print_time!("replace drawer");
        let mut drawer_mg = self.drawer.lock().unwrap();
        if let Some(task) = drawer_mg.take() {
            (task.callback)(false);
        }
        drawer_mg.replace(RenderTask { task: drawer, callback });
    }

    fn update(&self) {
        // print_time!("gpu render time");
        let mut context = {
            // print_time!("lock time");
            self.context.lock().unwrap()
        };

        // let mut context = context.borrow_mut();
        let mut rc = context.render_context.clone();
        let callback = {
            // print_time!("draw time");
            let drawer = {
                let mut drawer_arc = self.drawer.lock().unwrap();
                drawer_arc.take()
            };
            if let Some(drawer) = drawer {
                let mut user_context = context.user_context.take().unwrap();
                let canvas = context.surface.canvas();
                let mut rc = RenderContext::new(&mut rc, &mut user_context);
                (drawer.task).render(canvas, &mut rc);
                context.user_context = Some(user_context);
                drawer.callback
            } else {
                return;
            }
        };

        {
            // measure_time::print_time!("submit time");
            context.render_context.gr_context.flush_and_submit();
        }

        {
            // measure_time::print_time!("swap buffers time");
            if let Err(err) = context.gl_surface.swap_buffers(&context.context) {
                log::error!("Failed to swap buffers after render: {}", err);
            }
        }
        callback(true);
    }
}

