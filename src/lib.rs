#![allow(dead_code)]
#![allow(deprecated)]

use crate::app::{WinitApp, AppEvent, AppEventPayload, App};
use measure_time::debug_time;
use std::sync::OnceLock;
use anyhow::{anyhow, Error};
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

pub use quick_js::JsValue;
pub use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopBuilder, EventLoopProxy};
pub mod border;
pub mod base;
pub mod style;
pub mod mrc;
pub mod console;
pub mod color;
pub mod app;
// mod graphics;
pub mod renderer;
pub mod window;
pub mod element;
pub mod loader;
pub mod time;
pub mod resource_table;
#[cfg(feature = "websocket")]
pub mod websocket;
pub mod number;
pub mod timer;
pub mod event_loop;
pub mod string;
pub mod canvas_util;
pub mod event;
pub mod cursor;
pub mod img_manager;
pub mod data_dir;
pub mod macro_mod;
pub mod ext;
pub mod js;
pub mod performance;
mod trace;

pub mod cache;
pub mod animation;
#[cfg(target_os = "android")]
mod android;
mod id_hash_map;
mod id_generator;
mod typeface;
mod text;
mod frame_rate;
mod paint;
mod layout;
pub mod render;
mod computed;
mod style_list;
pub mod winit;
mod task_executor;
mod stylesheet;
mod font;
mod platform;

pub use deft_macros::*;
use crate::base::ResultWaiter;
use crate::console::init_console;
use crate::event_loop::{AppEventProxy};

pub static APP_EVENT_PROXY: OnceLock<AppEventProxy> = OnceLock::new();

fn run_event_loop(event_loop: EventLoop<AppEventPayload>, deft_app: App) {
    let el_proxy = AppEventProxy::new(event_loop.create_proxy());
    {
        let el_proxy = el_proxy.clone();
        APP_EVENT_PROXY.get_or_init(move || el_proxy);
    }
    let mut app = {
        debug_time!("init engine time");
        WinitApp::new(deft_app, el_proxy)
    };
    #[cfg(ohos)]
    platform::run_app(event_loop, app);
    #[cfg(not(ohos))]
    event_loop.run_app(&mut app).unwrap();
}

/// Boostrap for desktop apps
pub fn bootstrap(deft_app: App) {
    init_console();
    let event_loop: EventLoop<AppEventPayload> = EventLoop::with_user_event().build().unwrap();
    run_event_loop(event_loop, deft_app);
}

/// Send an app event. Could call from any thread.
pub fn send_app_event(event: AppEvent) -> Result<ResultWaiter<()>, Error> {
    let proxy = APP_EVENT_PROXY.get().ok_or_else(|| anyhow!("no app event proxy found"))?;
    let result = proxy.send_event(event)?;
    Ok(result)
}

/// Whether is mobile platform
pub fn is_mobile_platform() -> bool {
    #[cfg(mobile_platform)]
    return true;
    #[cfg(not(mobile_platform))]
    return false;
}

/// Show repaint area, just for debug
pub fn show_repaint_area() -> bool {
    false
}

/// Show focus hint, just for debug
pub fn show_focus_hint() -> bool {
    false
}

/// Show layer hint, just for debug
pub fn show_layer_hint() -> bool {
    false
}

/// Bootstrap for android apps
#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_bootstrap(app: AndroidApp, deft_app: App) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    android::init_android_app(&app);

    android_logger::init_once(android_logger::Config::default().with_min_level(log::Level::Warn));

    // info!("starting");
    if let Some(p) = app.internal_data_path() {
        let data_path = p.into_os_string().to_string_lossy().to_string();
        log::debug!("internal data_path:{}", data_path);
        unsafe {
            std::env::set_var(data_dir::ENV_KEY, data_path);
        }
    }
    log::debug!("data path: {:?}", data_dir::get_data_path(""));
    let event_loop = EventLoop::with_user_event().with_android_app(app).build().unwrap();
    run_event_loop(event_loop, deft_app);
}


#[cfg(ohos)]
pub fn ohos_bootstrap(openharmony_app: openharmony_ability::OpenHarmonyApp, deft_app: App) {
    use winit::platform::ohos::EventLoopBuilderExtOpenHarmony;
    let a = openharmony_app.clone();

    let event_loop = EventLoop::with_user_event().with_openharmony_app(a).build().unwrap();
    run_event_loop(event_loop, deft_app);
}