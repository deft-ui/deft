#![allow(unused)]
#![allow(dead_code)]
#![allow(unused_variables)]

use crate::app::{App, AppEvent, AppEventPayload, LentoApp};
use crate::data_dir::get_data_path;
use crate::element::label::{AttributeText, Label, DEFAULT_TYPE_FACE};
use crate::element::text::text_paragraph::TextParams;
use crate::element::text::{Text, FONT_MGR};
use crate::element::ScrollByOption;
use crate::js::js_deserialze::JsDeserializer;
use crate::loader::{RemoteModuleLoader, StaticModuleLoader};
use crate::performance::MemoryUsage;
use crate::renderer::CpuRenderer;
use crate::websocket::WebSocketManager;
use futures_util::StreamExt;
use measure_time::{info, print_time};
use memory_stats::memory_stats;
use quick_js::loader::FsJsModuleLoader;
use serde::{Deserialize, Serialize};
use skia_safe::textlayout::{paragraph, TextAlign};
use skia_safe::{Font, FontMetrics, FontStyle, Paint};
use skia_window::skia_window::{RenderBackendType, SkiaWindow};
use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::SystemTime;
use anyhow::{anyhow, Error};
use tokio_tungstenite::connect_async;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
use winit::window::{WindowAttributes, WindowId};
use yoga::Node;

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
pub mod frame;
pub mod element;
pub mod loader;
pub mod time;
pub mod resource_table;
pub mod websocket;
pub mod number;
pub mod timer;
pub mod event_loop;
pub mod async_runtime;
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

pub use lento_macros::*;
use rodio::cpal::available_hosts;
use skia_bindings::SkFontStyle_Slant;
use skia_safe::font_style::{Weight, Width};
use skia_safe::wrapper::ValueWrapper;
use crate::event_loop::{AppEventProxy, AppEventResult};
use crate::string::StringUtils;
use crate::text::break_lines;

pub static APP_EVENT_PROXY: OnceLock<AppEventProxy> = OnceLock::new();

fn run_event_loop(event_loop: EventLoop<AppEventPayload>, lento_app: Box<dyn LentoApp>) {
    let el_proxy = AppEventProxy::new(event_loop.create_proxy());
    {
        let el_proxy = el_proxy.clone();
        APP_EVENT_PROXY.get_or_init(move || el_proxy);
    }
    let mut app = App::new(lento_app, el_proxy);
    event_loop.run_app(&mut app).unwrap();
}

pub fn bootstrap(lento_app: Box<dyn LentoApp>) {
    let event_loop: EventLoop<AppEventPayload> = EventLoop::with_user_event().build().unwrap();
    run_event_loop(event_loop, lento_app);
}

pub fn send_app_event(event: AppEvent) -> Result<AppEventResult, Error> {
    let proxy = APP_EVENT_PROXY.get().ok_or_else(|| anyhow!("no app event proxy found"))?;
    let result = proxy.send_event(event)?;
    Ok(result)
}

pub fn is_mobile_platform() -> bool {
    #[cfg(mobile_platform)]
    return true;
    #[cfg(not(mobile_platform))]
    return false;
}

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_bootstrap(app: AndroidApp, lento_app: Box<dyn LentoApp>) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    android::init_android_app(&app);

    // android_logger::init_once(android_logger::Config::default().with_min_level(log::Level::Debug));

    // info!("starting");
    if let Some(p) = app.internal_data_path() {
        let data_path = p.into_os_string().to_string_lossy().to_string();
        println!("internal data_path:{}", data_path);
        unsafe {
            std::env::set_var(data_dir::ENV_KEY, data_path);
        }
    }
    println!("data path: {:?}", data_dir::get_data_path(""));
    let event_loop = EventLoop::with_user_event().with_android_app(app).build().unwrap();
    run_event_loop(event_loop, lento_app);
}


fn main_js_deserializer() {
    let mut map = HashMap::new();
    map.insert("x".to_string(), JsValue::Int(1));
    map.insert("y".to_string(), JsValue::Int(2));
    let des = JsDeserializer {
        value: JsValue::Object(map)
    };
    let result = ScrollByOption::deserialize(des).unwrap();
    println!("result:{:?}", result);
}

#[tokio::test]
async fn test_websocket() {
    let (client, _) = connect_async("ws://localhost:7800/ws").await.unwrap();
    let (w, mut r) = client.split();
    loop {
        let msg = r.next().await.unwrap().unwrap();
        println!("{:?}", msg);
    }
}

#[tokio::test]
async fn test_websocket_manager() {
    let mut ws_mgr = WebSocketManager::new();
    let conn = ws_mgr.create_connection("ws://localhost:7800/ws").await.unwrap();
    loop {
        let msg = ws_mgr.read_msg(conn).await.unwrap();
        println!("msg:{:?}", msg);
    }
}



// test layout performance
#[test]
fn test_layout() {
    let text = include_str!("../Cargo.lock").repeat(100);
    let start_mem_use = memory_stats().unwrap().physical_mem as f32;
    let font = DEFAULT_TYPE_FACE.with(|tf| Font::from_typeface(tf, 14.0));
    let paint = Paint::default();
    let params = TextParams {
        font,
        paint,
        line_height: Some(14.0),
        align: Default::default(),
    };
    let mut paragraph = {
        print_time!("build time");
        Text::build_lines(&text, &params, true)
    };
    {
        print_time!("layout time");
        for mut it in &mut paragraph {
            it.paragraph.layout(700.0);
        }
        let mem_use = memory_stats().unwrap().physical_mem as f32 - start_mem_use;
        println!("mem use:{}", mem_use / 1024.0 / 1024.0);
    }
    let mut renderer = CpuRenderer::new(1024, 1024);
    {
        print_time!("draw time");
        let mut lines = 0;
        for it in paragraph {
            it.paragraph.paint(renderer.canvas(), (0.0, 0.0));
            lines += 1;
            if lines >= 100 {
                break;
            }
        }
    }
}


#[test]
fn test_text_measure() {
    let text = include_str!("../Cargo.lock").repeat(100);
    let start_mem_use = memory_stats().unwrap().physical_mem as f32;
    // let font = DEFAULT_TYPE_FACE.with(|tf| Font::from_typeface(tf, 14.0));
    let paint = Paint::default();
    let fm = FONT_MGR.with(|fm| fm.clone());
    let mut font_style = FontStyle::new(Weight::NORMAL, Width::NORMAL, SkFontStyle_Slant::Upright);
    let tf = fm.match_family_style("monospace", font_style).unwrap();
    println!("font name: {}", &tf.family_name());
    let font = Font::from_typeface(tf, 14.0);
    {
        print_time!("measure time");
        for ln in text.lines() {
            let lines = break_lines(&font, ln, 100.0);
            // println!("lines:{:?}", lines);
            // for ch in ln.chars() {
            //     font.measure_str(ch.to_string(), Some(&paint));
            // }
        }
        let mem_use = memory_stats().unwrap().physical_mem as f32 - start_mem_use;
        println!("mem use:{}", mem_use / 1024.0 / 1024.0);
    }

}

fn test_border_performance_gl() {
    let event_loop: EventLoop<()> = EventLoopBuilder::default().build().unwrap();
    struct TestApp {
        window: Option<SkiaWindow>,
    }

    impl ApplicationHandler for TestApp {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            #[cfg(not(target_os = "android"))]
            let backend_type = RenderBackendType::SoftBuffer;
            #[cfg(target_os = "android")]
            let backend_type = RenderBackendType::GL;
            let mut skia_window = SkiaWindow::new(event_loop, WindowAttributes::default(), backend_type);
            skia_window.render(|canvas| {
                crate::renderer::test_border(canvas);
            });
            self.window = Some(skia_window);
        }

        fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
            if let WindowEvent::RedrawRequested = &event {
                let win = self.window.as_mut().unwrap();
                win.render(|canvas| {
                    crate::renderer::test_border(canvas);
                })
            } else if let WindowEvent::Resized(s) = &event {
                let win = self.window.as_mut().unwrap();
                win.resize_surface(s.width, s.height);
            }
        }
    }
    let mut app = TestApp { window: None };
    event_loop.run_app(&mut app).unwrap();
}