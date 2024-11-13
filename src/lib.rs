pub use lento_core::*;
pub use lento_macros::*;
pub mod ext_animation;
pub mod ext_clipboard;

use crate::app::{App, AppEvent, LentoApp};
use crate::event_loop::set_event_proxy;
use crate::ext_animation::animation_create;
use crate::ext_clipboard::{clipboard_read_text, clipboard_write_text};
#[cfg(not(feature = "production"))]
use crate::loader::DefaultModuleLoader;
#[cfg(feature = "production")]
use lento_core::loader::StaticModuleLoader;
use quick_js::loader::JsModuleLoader;
use std::env;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(not(feature = "production"))]
fn create_module_loader() -> DefaultModuleLoader {
    let mut loader = DefaultModuleLoader::new(true);
    loader.set_fs_base(".");
    let module_name = env::var("LENTO_ENTRY").unwrap_or("index.js".to_string());
    let start_time = std::time::Instant::now();
    while start_time.elapsed() < std::time::Duration::from_secs(60) {
        if loader.load(&module_name).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
        eprintln!("Failed to load {}, retrying...", module_name);
    }
    loader
}

#[cfg(feature = "production")]
fn create_module_loader() -> StaticModuleLoader {
    let mut loader = StaticModuleLoader::new();
    let source = String::from_utf8_lossy(include_bytes!(env!("LENTO_JS_BUNDLE"))).to_string();
    loader.add_module("index.js".to_string(), source);
    loader
}

fn init_app(app: &mut App) {
    app.js_engine.add_global_func(animation_create::new());

    app.js_engine.add_global_func(clipboard_write_text::new());
    app.js_engine.add_global_func(clipboard_read_text::new());
}

fn run_event_loop(event_loop: EventLoop<AppEvent>, lento_app: Box<dyn LentoApp>) {
    let el_proxy = event_loop.create_proxy();
    set_event_proxy(el_proxy.clone());
    let mut app = App::new(create_module_loader(), lento_app);
    init_app(&mut app);
    event_loop.run_app(&mut app).unwrap();
}

pub fn bootstrap(lento_app: Box<dyn LentoApp>) {
    let event_loop: EventLoop<AppEvent> = EventLoop::with_user_event().build().unwrap();
    run_event_loop(event_loop, lento_app);
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    // android_logger::init_once(android_logger::Config::default().with_min_level(log::Level::Debug));

    info!("starting");
    if let Some(p) = app.internal_data_path() {
        let data_path = p.into_os_string().to_string_lossy().to_string();
        println!("internal data_path:{}", data_path);
        unsafe {
            env::set_var(data_dir::ENV_KEY, data_path);
        }
    }
    println!("data path: {:?}", get_data_path(""));
    let event_loop = EventLoop::with_user_event().with_android_app(app).build().unwrap();
    run_event_loop(event_loop);
}
