#![allow(dead_code)]

use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use std::time::{SystemTime};
use futures_util::StreamExt;
use measure_time::{info, print_time};
use memory_stats::memory_stats;
use quick_js::JsValue;
use quick_js::loader::FsJsModuleLoader;
use serde::{Deserialize, Serialize};
use skia_safe::{Font, Paint};
use skia_safe::textlayout::{paragraph, TextAlign};
use skia_window::skia_window::SkiaWindow;
use tokio_tungstenite::connect_async;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopBuilder, EventLoopProxy};
use yoga::Node;
use crate::app::{App, AppEvent};
use crate::element::ScrollByOption;
use crate::event_loop::set_event_proxy;
use crate::js::js_deserialze::JsDeserializer;
use crate::element::label::{AttributeText, DEFAULT_TYPE_FACE, Label};
use crate::element::text::Text;
use crate::element::text::text_paragraph::TextParams;
use crate::loader::{RemoteModuleLoader, StaticModuleLoader};
use crate::performance::MemoryUsage;
use crate::renderer::CpuRenderer;
use crate::websocket::WebSocketManager;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
use winit::window::{WindowAttributes, WindowId};
use crate::data_dir::get_data_path;

mod border;
mod base;
mod style;
mod mrc;
mod console;
mod color;
mod app;
// mod graphics;
mod renderer;
mod frame;
mod element;
mod loader;
mod time;
mod resource_table;
mod websocket;
mod number;
mod timer;
mod event_loop;
mod async_runtime;
mod string;
mod canvas_util;
mod event;
mod cursor;
mod img_manager;
mod data_dir;
mod macro_mod;
mod ext;
mod js;
mod performance;

mod cache;
#[cfg(target_os = "android")]
mod android;


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

#[cfg(not(feature = "production"))]
fn create_module_loader() -> FsJsModuleLoader {
    // RemoteModuleLoader::new("http://localhost:7800/".to_string())
   FsJsModuleLoader::new(".")
}

#[cfg(feature = "production")]
fn create_module_loader() -> StaticModuleLoader {
    let mut loader = StaticModuleLoader::new();
    let source = String::from_utf8_lossy(include_bytes!("../js/bundle.js")).to_string();
    loader.add_module("index.js".to_string(), source);
    loader
}

fn run_event_loop(mut event_loop: EventLoop<AppEvent>) {
    let el_proxy = event_loop.create_proxy();
    set_event_proxy(el_proxy.clone());
    let mut app = App::new(create_module_loader());
    event_loop.run_app(&mut app).unwrap();
}

fn main() {
    let event_loop: EventLoop<AppEvent> = EventLoop::with_user_event().build().unwrap();
    run_event_loop(event_loop);
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
    let text = include_str!("../Cargo.lock");
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
        Text::build_lines(&text, &params)
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
    let text = include_str!("../Cargo.lock");
    let start_mem_use = memory_stats().unwrap().physical_mem as f32;
    let font = DEFAULT_TYPE_FACE.with(|tf| Font::from_typeface(tf, 14.0));
    let paint = Paint::default();
    {
        print_time!("measure time");
        for ln in text.lines() {
            for ch in ln.chars() {
                font.measure_str(ch.to_string(), Some(&paint));
            }
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
            let mut skia_window = SkiaWindow::new(event_loop, WindowAttributes::default());
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