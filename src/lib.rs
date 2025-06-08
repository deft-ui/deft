#![allow(dead_code)]
#![allow(deprecated)]

use crate::app::{App, AppEvent, AppEventPayload, WinitApp};
use anyhow::{anyhow, Error};
use measure_time::debug_time;
use std::sync::OnceLock;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

pub use quick_js::JsValue;
pub use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopBuilder, EventLoopProxy};
pub mod app;
pub mod base;
pub mod border;
pub mod color;
pub mod console;
pub mod mrc;
pub mod style;
// mod graphics;
pub mod canvas_util;
pub mod cursor;
pub mod data_dir;
pub mod element;
pub mod event;
pub mod event_loop;
pub mod ext;
pub mod img_manager;
pub mod js;
pub mod loader;
pub mod macro_mod;
pub mod number;
pub mod performance;
pub mod renderer;
pub mod resource_table;
pub mod string;
pub mod time;
pub mod timer;
mod trace;
#[cfg(feature = "websocket")]
pub mod websocket;
pub mod window;

#[cfg(target_os = "android")]
mod android;
pub mod animation;
pub mod cache;
mod computed;
mod error;
mod font;
mod frame_rate;
mod id_generator;
mod id_hash_map;
mod paint;
mod platform;
pub mod render;
mod state;
mod style_list;
mod stylesheet;
mod task_executor;
mod text;
mod typeface;
pub mod winit;

use crate::base::ResultWaiter;
use crate::console::init_console;
use crate::event_loop::AppEventProxy;
pub use deft_macros::*;

pub static APP_EVENT_PROXY: OnceLock<AppEventProxy> = OnceLock::new();

fn run_event_loop(event_loop: EventLoop<AppEventPayload>, deft_app: App) {
    let el_proxy = AppEventProxy::new(event_loop.create_proxy());
    {
        let el_proxy = el_proxy.clone();
        APP_EVENT_PROXY.get_or_init(move || el_proxy);
    }
    #[allow(unused_mut)]
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
    let proxy = APP_EVENT_PROXY
        .get()
        .ok_or_else(|| anyhow!("no app event proxy found"))?;
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
    let event_loop = EventLoop::with_user_event()
        .with_android_app(app)
        .build()
        .unwrap();
    run_event_loop(event_loop, deft_app);
}

#[cfg(ohos)]
pub fn ohos_bootstrap(openharmony_app: openharmony_ability::OpenHarmonyApp, deft_app: App) {
    use winit::platform::ohos::EventLoopBuilderExtOpenHarmony;
    let a = openharmony_app.clone();

    let event_loop = EventLoop::with_user_event()
        .with_openharmony_app(a)
        .build()
        .unwrap();
    run_event_loop(event_loop, deft_app);
}
