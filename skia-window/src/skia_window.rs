use std::num::NonZeroU32;
use std::ops::Deref;

use glutin::config::{Config, ConfigSurfaceTypes, ConfigTemplate, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentContext};
use glutin::display::{Display, DisplayApiPreference};
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle};
use skia_safe::Canvas;
use winit::event_loop::ActiveEventLoop;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
#[cfg(glx_backend)]
use winit::platform::x11;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::skia_renderer::SkiaRenderer;

pub struct SkiaWindow {
    surface_state: SurfaceState,
}

#[allow(dead_code)]
struct SurfaceState {
    glutin_display: Display,
    render: SkiaRenderer,
    window: Window,
}

impl SkiaWindow {
    pub fn new(event_loop: &ActiveEventLoop, attributes: WindowAttributes) -> Self {
        let surface_state = Self::ensure_surface_and_context(event_loop, attributes);
        Self { surface_state }
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) {
        self.surface_state.render.resize(&self.winit_window(), width, height);
    }

}

impl Deref for SkiaWindow {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        &self.surface_state.window
    }
}

impl SkiaWindow {
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
        let raw_window_handle = window.raw_window_handle();
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
            .with_alpha_size(8)
            .compatible_with_native_window(raw_window_handle)
            .with_surface_type(ConfigSurfaceTypes::WINDOW);

        #[cfg(cgl_backend)]
        let builder = builder.with_transparency(true).with_multisampling(8);

        builder.build()
    }


    fn ensure_surface_and_context(event_loop: &ActiveEventLoop, attributes: WindowAttributes) -> SurfaceState {
        let raw_display_handle = event_loop.raw_display_handle();
        let window = event_loop.create_window(attributes).unwrap();
        let raw_window_handle = window.raw_window_handle();

        let glutin_display = Self::ensure_glutin_display(raw_display_handle, &window);
        // Lazily initialize, egl, wgl, glx etc

        let template = Self::config_template(raw_window_handle);
        let config = unsafe {
            glutin_display
                .find_configs(template)
                .unwrap()
                .reduce(|accum, config| {
                    // Find the config with the maximum number of samples.
                    //
                    // In general if you're not sure what you want in template you can request or
                    // don't want to require multisampling for example, you can search for a
                    // specific option you want afterwards.
                    //
                    // XXX however on macOS you can request only one config, so you should do
                    // a search with the help of `find_configs` and adjusting your template.
                    if config.num_samples() > accum.num_samples() {
                        config
                    } else {
                        accum
                    }
                })
                .unwrap()
        };
        println!("Picked a config with {} samples", config.num_samples());

        // XXX: Winit is missing a window.surface_size() API and the inner_size may be the wrong
        // size to use on some platforms!
        let (width, height): (u32, u32) = window.inner_size().into();
        let raw_window_handle = window.raw_window_handle();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        );
        let surface = unsafe {
            glutin_display
                .create_window_surface(&config, &attrs)
                .unwrap()
        };

        let not_current_context =
            Self::create_compatible_gl_context(&glutin_display, raw_window_handle, &config);
        let context = not_current_context
            .make_current(&surface)
            .expect("Failed to make GL context current");
        let render = SkiaRenderer::new(&glutin_display, &window, surface, context);

        SurfaceState { window, glutin_display, render }
    }

    pub fn window_id(&self) -> WindowId {
        self.surface_state.window.id()
    }

    pub fn id(&self) -> WindowId {
        self.window_id()
    }

    pub fn winit_window(&self) -> &Window {
        &self.surface_state.window
    }

    pub fn request_redraw(&self) {
        self.surface_state.window.request_redraw();
    }

    pub fn render<F: FnOnce(&Canvas)>(&mut self, renderer: F) {
        self.surface_state.render.draw(renderer);
    }
}