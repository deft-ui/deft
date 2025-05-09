use std::cell::RefCell;
use std::fmt::Debug;
use std::future::Future;
use std::panic::RefUnwindSafe;
use std::path::{Path, PathBuf};
use anyhow::anyhow;
use quick_js::loader::JsModuleLoader;
use quick_js::{Callback, Context, ExecutionError, JsPromise, JsValue, ValueError};
use tokio::runtime::Builder;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::window::{CursorGrabMode, WindowId};

use crate::app::{exit_app, IApp, App};
use crate::console::Console;
use crate::element::{Element, CSS_MANAGER};
use crate::element::entry::Entry;
use crate::element::image::Image;
use crate::element::paragraph::Paragraph;
use crate::element::text::Text;
use crate::event_loop::run_with_event_loop;
use crate::ext::ext_animation::animation_create;
use crate::ext::ext_appfs::appfs;
use crate::ext::ext_base64::Base64;
use crate::ext::ext_console::Console as ExtConsole;
use crate::ext::ext_env::env;
use crate::ext::ext_window::{handle_window_event, WINDOWS};
use crate::ext::ext_fs::{fs_create_dir, fs_create_dir_all, fs_delete_file, fs_exists, fs_read_dir, fs_remove_dir, fs_remove_dir_all, fs_rename, fs_stat};
use crate::ext::ext_localstorage::localstorage;
use crate::ext::ext_path::path;
use crate::ext::ext_process::process;
use crate::ext::ext_shell::shell;
use crate::ext::ext_timer::{timer_clear_interval, timer_clear_timeout, timer_set_interval, timer_set_timeout};
#[cfg(feature = "tray")]
use crate::ext::ext_tray::SystemTray;
use crate::ext::ext_worker::{SharedModuleLoader, Worker, WorkerInitParams};
use crate::window::{Window, WindowType};
use crate::js::js_binding::{JsCallError, JsFunc};
use crate::js::js_event_loop::js_create_event_loop_proxy;
use crate::js::js_runtime::{JsContext, PromiseResolver};
use crate::js::ToJsCallResult;
use crate::mrc::Mrc;
use crate::stylesheet::{stylesheet_add, stylesheet_remove, stylesheet_update};
use crate::typeface::typeface_create;

thread_local! {
    static JS_ENGINE: RefCell<Option<Mrc<JsEngine>>> = RefCell::new(None);
}

pub struct JsEngine {
    pub js_context: Mrc<JsContext>,
    pub app: App,
}

struct JsFuncCallback {
    js_context: Mrc<JsContext>,
    pub js_func: Box<dyn JsFunc + RefUnwindSafe>,
}

impl Callback<()> for JsFuncCallback {
    fn argument_count(&self) -> usize {
        self.js_func.args_count()
    }

    fn call(&self, args: Vec<JsValue>) -> Result<Result<JsValue, String>, ValueError> {
        let mut js_context = self.js_context.clone();
        match self.js_func.call(&mut js_context, args) {
            Ok(v) => {
                Ok(Ok(v))
            }
            Err(e) => {
                match e {
                    JsCallError::ConversionError(ce) => {
                        Err(ce)
                    }
                    JsCallError::ExecutionError(ee) => {
                        Ok(Err(ee.to_string()))
                    }
                }
            }
        }
    }
}

impl JsEngine {

    pub fn get() -> Mrc<JsEngine> {
        JS_ENGINE.with(|e| {
            let e = e.borrow();
            let js_engine = e.as_ref().expect("js engine not initialized");
            js_engine.clone()
        })
    }

    pub fn init(mut app: App) {
        let loader = {
            let mut app = app.app_impl.lock().unwrap();
            SharedModuleLoader::new(app.create_module_loader())
        };
        let runtime = Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();
        let js_context = Context::builder()
            .console(Console::new())
            .module_loader(loader.clone())
            .build().unwrap();
        let js_context = Mrc::new(JsContext::new(js_context, runtime));

        let engine = Self {
            js_context,
            app: app.clone(),
        };

        engine.add_global_functions(ExtConsole::create_js_apis());
        engine.add_global_functions(Element::create_js_apis());
        engine.add_global_functions(Entry::create_js_apis());
        engine.add_global_functions(Paragraph::create_js_apis());
        engine.add_global_functions(Text::create_js_apis());
        engine.add_global_functions(Image::create_js_apis());
        #[cfg(feature = "sqlite")]
        engine.add_global_functions(crate::ext::ext_sqlite::SqliteConn::create_js_apis());
        #[cfg(feature = "tray")]
        {
            engine.add_global_functions(SystemTray::create_js_apis());
        }
        engine.add_global_functions(process::create_js_apis());
        #[cfg(feature = "dialog")]
        engine.add_global_functions(crate::ext::ext_dialog::dialog::create_js_apis());
        engine.add_global_functions(Base64::create_js_apis());
        engine.add_global_functions(shell::create_js_apis());
        #[cfg(feature = "audio")]
        engine.add_global_functions(crate::ext::ext_audio::Audio::create_js_apis());
        engine.add_global_functions(path::create_js_apis());
        engine.add_global_functions(env::create_js_apis());
        #[cfg(feature = "http")]
        engine.add_global_functions(crate::ext::ext_http::http::create_js_apis());
        engine.add_global_functions(appfs::create_js_apis());
        engine.add_global_functions(localstorage::create_js_apis());
        // websocket
        #[cfg(feature = "websocket")]
        engine.add_global_functions(crate::ext::ext_websocket::WsConnection::create_js_apis());
        #[cfg(feature = "http")]
        engine.add_global_functions(crate::ext::ext_fetch::fetch::create_js_apis());

        engine.add_global_functions(Window::create_js_apis());
        engine.add_global_func(timer_set_timeout::new());
        engine.add_global_func(timer_clear_timeout::new());
        engine.add_global_func(timer_set_interval::new());
        engine.add_global_func(timer_clear_interval::new());

        engine.add_global_func(fs_read_dir::new());
        engine.add_global_func(fs_stat::new());
        engine.add_global_func(fs_exists::new());
        engine.add_global_func(fs_rename::new());
        engine.add_global_func(fs_delete_file::new());
        engine.add_global_func(fs_create_dir::new());
        engine.add_global_func(fs_create_dir_all::new());
        engine.add_global_func(fs_remove_dir::new());
        engine.add_global_func(fs_remove_dir_all::new());

        engine.add_global_func(animation_create::new());
        engine.add_global_func(typeface_create::new());

        #[cfg(feature = "clipboard")]
        {
            engine.add_global_func(crate::ext::ext_clipboard::clipboard_write_text::new());
            engine.add_global_func(crate::ext::ext_clipboard::clipboard_read_text::new());
        }
        engine.add_global_func(stylesheet_add::new());
        engine.add_global_func(stylesheet_remove::new());
        engine.add_global_func(stylesheet_update::new());

        Worker::init_js_api(WorkerInitParams { app });
        engine.add_global_functions(Worker::create_js_apis());
        JS_ENGINE.with(|e| *e.borrow_mut() = Some(Mrc::new(engine)));
    }

    pub fn enable_localstorage(&mut self, p: PathBuf) {
        localstorage::init(p);
    }

    pub fn create_async_task<F, O>(&mut self, future: F) -> JsValue
    where
        F: Future<Output=O> + Send + 'static,
        O: ToJsCallResult,
    {
        self.js_context.create_async_task2(future)
    }

    pub fn create_promise(&mut self) -> (JsValue, PromiseResolver) {
        self.js_context.create_promise()
    }


    pub fn init_api(&self) {
        let default_css = include_str!("../../deft.css");
        CSS_MANAGER.with_borrow_mut(|mut manager| {
            manager.add(default_css);
        });
        let libjs = String::from_utf8_lossy(include_bytes!("../../lib.js"));
        self.js_context.eval_module(&libjs, "lib.js").unwrap();
    }

    pub fn add_global_functions(&self, functions: Vec<Box<dyn JsFunc + RefUnwindSafe + 'static>>) {
        for func in functions {
            let name = func.name().to_string();
            let js_context = self.js_context.clone();
            self.js_context.add_callback(name.as_str(), JsFuncCallback {
                js_func: func,
                js_context,
            }).unwrap();
        }
    }

    pub fn add_global_func(&self, func: impl JsFunc + RefUnwindSafe + 'static) {
        let name = func.name().to_string();
        let js_context = self.js_context.clone();
        self.js_context.add_callback(name.as_str(), JsFuncCallback {
            js_func: Box::new(func),
            js_context,
        }).unwrap();
    }

    pub fn execute_main(&mut self) {
        self.js_context.execute_main();
    }

    pub fn execute_module(&mut self, module_name: &str) -> Result<(), ExecutionError> {
        self.js_context.execute_module(module_name)
    }

    pub fn eval_module(&mut self, code: &str, filename: &str) -> Result<JsValue, ExecutionError> {
        self.js_context.eval_module(code, filename)
    }

    pub fn handle_window_event(&mut self, window_id: WindowId, event: WindowEvent) {
        handle_window_event(window_id, event);
    }

    pub fn handle_device_event(&mut self, device_id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::Button {..} = event {
            let close_windows = WINDOWS.with_borrow(|windows| {
                let mut result = Vec::new();
                let menu_windows: Vec<&Window> = windows.iter()
                    .filter(|(_, f)| f.window_type == WindowType::Menu)
                    .map(|(_, f)| f)
                    .collect();
                if menu_windows.is_empty() {
                    return result;
                }
                run_with_event_loop(|el| {
                    if let Some(pos) = el.query_pointer(device_id) {
                        menu_windows.iter().for_each(|window| {
                            let w_size = window.window.outer_size();
                            if let Some(wp) = window.window.outer_position().ok() {
                                let (wx, wy) = (wp.x as f32, wp.y as f32);
                                let (ww, wh) = (w_size.width as f32, w_size.height as f32);
                                let is_in_window = pos.0 >= wx && pos.0 <= wx + ww
                                                       && pos.1 >= wy && pos.1 <= wy + wh;
                                if !is_in_window {
                                    let _ = window.window.set_cursor_grab(CursorGrabMode::None);
                                    result.push(window.as_weak());
                                }
                            }
                        })
                    }
                });
                result
            });
            for window in close_windows {
                if let Ok(mut f) = window.upgrade() {
                    let _ = f.close();
                }
            }
        }
    }

    pub fn execute_pending_jobs(&self) {
        let jc = self.js_context.clone();
        loop {
            let job_res = jc.execute_pending_job();
            match job_res {
                Ok(res) => {
                    if !res {
                        break;
                    }
                }
                Err(e) => {
                    eprint!("job error:{:?}", e);
                    break;
                }
            }
        }
    }

}