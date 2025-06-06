use std::num::NonZeroU32;
use glutin::config::{Config, ConfigSurfaceTypes, ConfigTemplate, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentContext};
use glutin::display::{Display, DisplayApiPreference};
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle};
use winit::event_loop::ActiveEventLoop;
#[cfg(glx_backend)]
use winit::platform::x11;
use winit::window::{Window};

use glutin::prelude::*;
use crate::gl::gl_renderer::GlRenderer;
use crate::renderer::Renderer;
use crate::surface::RenderBackend;


#[allow(dead_code)]
pub struct SurfaceState {
    glutin_display: Display,
    render: GlRenderer,
    window: Window,
}

impl SurfaceState {
    #[allow(unused_variables)]
    fn create_display(
        raw_display: RawDisplayHandle,
        raw_window_handle: RawWindowHandle,
    ) -> Display {
        #[cfg(egl_backend)]
        let preference = DisplayApiPreference::Egl;

        #[cfg(glx_backend)]
        let preference = DisplayApiPreference::Glx(Box::new(x11::register_xlib_error_hook));

        #[cfg(cgl_backend)]
        let preference = DisplayApiPreference::Cgl;

        #[cfg(wgl_backend)]
        let preference = DisplayApiPreference::Wgl(Some(raw_window_handle));

        #[cfg(all(egl_backend, wgl_backend))]
        let preference = DisplayApiPreference::WglThenEgl(Some(raw_window_handle));

        #[cfg(all(egl_backend, glx_backend))]
        let preference = DisplayApiPreference::GlxThenEgl(Box::new(x11::register_xlib_error_hook));

        // Create connection to underlying OpenGL client Api.
        unsafe { Display::new(raw_display, preference).unwrap() }
    }

    fn ensure_glutin_display(display_handle: RawDisplayHandle, window: &winit::window::Window) -> Display {
        //TODO no unwrap
        let raw_window_handle = window.raw_window_handle().unwrap();
        Self::create_display(display_handle, raw_window_handle)
    }

    fn create_compatible_gl_context(
        glutin_display: &Display,
        raw_window_handle: RawWindowHandle,
        config: &Config,
    ) -> NotCurrentContext {
        let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));

        // Since glutin by default tries to create OpenGL core context, which may not be
        // present we should try gles.
        let fallback_context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .build(Some(raw_window_handle));
        unsafe {
            glutin_display
                .create_context(&config, &context_attributes)
                .unwrap_or_else(|_| {
                    glutin_display
                        .create_context(config, &fallback_context_attributes)
                        .expect("failed to create context")
                })
        }
    }

    /// Create template to find OpenGL config.
    fn config_template(raw_window_handle: RawWindowHandle) -> ConfigTemplate {
        let builder = ConfigTemplateBuilder::new()
            .prefer_hardware_accelerated(None)
            .with_depth_size(0)
            .with_stencil_size(0)
            .compatible_with_native_window(raw_window_handle)
            .with_surface_type(ConfigSurfaceTypes::WINDOW);

        #[cfg(cgl_backend)]
        let builder = builder.with_transparency(true).with_multisampling(8);

        builder.build()
    }


    pub fn new(event_loop: &ActiveEventLoop, window: Window) -> Option<SurfaceState> {
        let raw_display_handle = event_loop.raw_display_handle().ok()?;
        let raw_window_handle = window.raw_window_handle().ok()?;

        let glutin_display = Self::ensure_glutin_display(raw_display_handle, &window);
        // Lazily initialize, egl, wgl, glx etc

        let template = Self::config_template(raw_window_handle);
        let config = unsafe {
            glutin_display
                .find_configs(template)
                .ok()?
                .reduce(|accum, config| {
                    // Find the config with the minimum number of samples.
                    //
                    // In general if you're not sure what you want in template you can request or
                    // don't want to require multisampling for example, you can search for a
                    // specific option you want afterwards.
                    //
                    // XXX however on macOS you can request only one config, so you should do
                    // a search with the help of `find_configs` and adjusting your template.
                    if config.num_samples() < accum.num_samples() {
                        config
                    } else if config.stencil_size() < accum.stencil_size() {
                        config
                    } else if config.depth_size() < accum.depth_size() {
                        config
                    } else {
                        accum
                    }
                })?
        };
        println!("Picked a config with {} samples", config.num_samples());

        // XXX: Winit is missing a window.surface_size() API and the inner_size may be the wrong
        // size to use on some platforms!
        let (width, height): (u32, u32) = window.inner_size().into();
        let raw_window_handle = window.raw_window_handle().ok()?;
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            NonZeroU32::new(width)?,
            NonZeroU32::new(height)?,
        );
        let surface = unsafe {
            glutin_display
                .create_window_surface(&config, &attrs)
                .ok()?
        };

        let not_current_context =
            Self::create_compatible_gl_context(&glutin_display, raw_window_handle, &config);
        let context = not_current_context
            .make_current(&surface)
            .expect("Failed to make GL context current");
        let render = GlRenderer::new(&glutin_display, &window, surface, context, &config)?;

        Some(SurfaceState { window, glutin_display, render })
    }
}

impl RenderBackend for SurfaceState {
    fn window(&self) -> &Window {
        &self.window
    }

    fn render(&mut self, renderer: Renderer, callback: Box<dyn FnOnce(bool) + Send + 'static>) {
        self.render.draw(renderer, callback);
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.render.resize(&self.window(), width, height);
    }
}