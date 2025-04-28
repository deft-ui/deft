
use std::collections::HashMap;

use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

use skia_safe::{Color, Paint};
use winit::application::ApplicationHandler;
use winit::window::{Fullscreen, WindowAttributes, WindowId};
use skia_window::renderer::Renderer;
use skia_window::skia_window::{RenderBackendType, SkiaWindow};


pub struct App {
    windows: HashMap<WindowId, SkiaWindow>,
}

impl App {
    pub fn new() -> Self {
        let windows = HashMap::new();
        Self {
            windows,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let win = SkiaWindow::new(event_loop, WindowAttributes::default(), RenderBackendType::SoftBuffer).unwrap();
        self.windows.insert(win.id(), win);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if let Some(win) = self.windows.get_mut(&window_id) {
            match event {
                WindowEvent::MouseInput { state, .. } => {
                    if state == ElementState::Pressed {
                        if win.fullscreen().is_some() {
                            win.set_fullscreen(None);
                        } else {
                            win.set_fullscreen(Some(Fullscreen::Borderless(event_loop.primary_monitor())));
                        }
                    }
                }
                WindowEvent::RedrawRequested {} => {
                    let render = Renderer::new(|canvas, ctx| {
                        canvas.clear(Color::from_rgb(0, 40, 0));
                        {
                            let mut layer = ctx.create_layer(100, 100).unwrap();
                            let c = layer.canvas();
                            c.clear(Color::from_rgb(255, 255, 255));
                            let mut paint = Paint::default();
                            paint.set_alpha(127);
                            canvas.draw_image(layer.as_image(), (100, 100), Some(&paint));
                        }
                    });
                    win.render(render);
                }
                WindowEvent::Resized(size) => {
                    win.resize_surface(size.width, size.height);
                    win.request_redraw();
                }
                _ => {}
            }
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        //self.app.suspended(event_loop)
    }
}

fn run(event_loop: EventLoop<()>) {
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
    // env_logger::builder()
    //     .filter_level(log::LevelFilter::Debug) // Default Log Level
    //     .parse_default_env()
    //     .init();

    let event_loop = EventLoop::with_user_event().build().unwrap();
    run(event_loop);
}