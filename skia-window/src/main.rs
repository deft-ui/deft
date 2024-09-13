mod skia_window;
mod skia_renderer;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::num::NonZeroU32;
use std::time::Instant;
use ::gl::GetIntegerv;
use gl::types::GLint;

use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder};
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
#[cfg(glx_backend)]
use winit::platform::x11;

use glutin::config::{Config, ConfigSurfaceTypes, ConfigTemplate, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext};
use glutin::display::{Display, DisplayApiPreference};
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use log::info;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle};
use skia_safe::{Color, Paint, Rect, Surface};
use skia_safe::gpu::{backend_render_targets, SurfaceOrigin};
use skia_safe::gpu::gl::FramebufferInfo;
use skia_safe::PaintStyle::Stroke;
use winit::application::ApplicationHandler;
use winit::window::{Window, WindowAttributes, WindowId};
use crate::skia_window::SkiaWindow;


pub struct App {
    windows: HashMap<WindowId, SkiaWindow>,
}

impl App {
    pub fn new() -> Self {
        let mut windows = HashMap::new();
        Self {
            windows,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let win = SkiaWindow::new(event_loop, WindowAttributes::default());
        self.windows.insert(win.window_id(), win);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if let WindowEvent::KeyboardInput { event, .. } = &event {
            if event.state == ElementState::Released {
                info!("create window");
                let mut app = SkiaWindow::new(event_loop);
                // app.resumed(event_loop);
                self.windows.insert(app.window_id(), app);
            }
        }
        if let Some(win) = self.windows.get_mut(&window_id) {
            match event {
                WindowEvent::RedrawRequested {} => {
                    win.render(|canvas| {
                        canvas.clear(Color::from_rgb(0, 0, 0));
                        let rect = Rect::new(500.0, 500.0, 700.0, 700.0);
                        let mut paint = Paint::default();
                        paint.set_style(Stroke);
                        paint.set_color(Color::from_rgb(255, 255, 255));
                        canvas.draw_rect(&rect, &paint);
                    });
                }
                WindowEvent::Resized(size) => {
                    win.request_redraw();
                }
                _ => {}
            }
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        //self.app.suspended(event_loop)
    }
}

fn run(mut event_loop: EventLoop<()>) {
    log::trace!("Running mainloop...");

    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    android_logger::init_once(android_logger::Config::default().with_min_level(log::Level::Debug));

    let event_loop = EventLoop::with_user_event().with_android_app(app).build().unwrap();
    run(event_loop);
}

// declared as pub to avoid dead_code warnings from cdylib target build
#[cfg(not(target_os = "android"))]
pub fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug) // Default Log Level
        .parse_default_env()
        .init();

    let event_loop = EventLoop::with_user_event().build().unwrap();
    run(event_loop);
}