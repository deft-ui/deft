use std::fmt::Debug;
use std::panic::RefUnwindSafe;

use anyhow::anyhow;
use quick_js::loader::JsModuleLoader;
use quick_js::{Callback, Context, ExecutionError, JsValue, ValueError};
use tokio::runtime::Builder;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::window::{CursorGrabMode, WindowId};

use crate::app::exit_app;
use crate::console::Console;
use crate::element::Element;
use crate::element::paragraph::Paragraph;
use crate::element::text::Text;
use crate::event_loop::run_with_event_loop;
use crate::export_js_api;
use crate::ext::ext_animation::animation_create;
use crate::ext::ext_appfs::appfs;
use crate::ext::ext_audio::Audio;
use crate::ext::ext_base64::Base64;
use crate::ext::ext_clipboard::{clipboard_read_text, clipboard_write_text};
use crate::ext::ext_console::Console as ExtConsole;
use crate::ext::ext_dialog::dialog;
use crate::ext::ext_env::env;
use crate::ext::ext_fetch::fetch;
use crate::ext::ext_frame::{handle_window_event, FRAMES};
use crate::ext::ext_fs::{fs_create_dir, fs_create_dir_all, fs_delete_file, fs_exists, fs_read_dir, fs_remove_dir, fs_remove_dir_all, fs_rename, fs_stat};
use crate::ext::ext_http::http;
use crate::ext::ext_localstorage::localstorage;
use crate::ext::ext_path::path;
use crate::ext::ext_process::process;
use crate::ext::ext_shell::shell;
use crate::ext::ext_timer::{timer_clear_interval, timer_clear_timeout, timer_set_interval, timer_set_timeout};
#[cfg(feature = "tray")]
use crate::ext::ext_tray::SystemTray;
use crate::ext::ext_websocket::WsConnection;
use crate::ext::ext_worker::{SharedModuleLoader, Worker, WorkerInitParams};
use crate::frame::{Frame, FrameType};
use crate::js::js_binding::{JsCallError, JsFunc};
use crate::js::js_runtime::JsContext;
use crate::js::js_value_util::DeserializeFromJsValue;
use crate::mrc::Mrc;

pub struct JsEngine {
    pub js_context: Mrc<JsContext>,
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

    pub fn new<L: JsModuleLoader + Send + Sync>(loader: L) -> Self {
        let loader = SharedModuleLoader::new(Box::new(loader));
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
        };

        engine.add_global_functions(ExtConsole::create_js_apis());
        engine.add_global_functions(Element::create_js_apis());
        engine.add_global_functions(Paragraph::create_js_apis());
        engine.add_global_functions(Text::create_js_apis());
        #[cfg(feature = "tray")]
        {
            engine.add_global_functions(SystemTray::create_js_apis());
        }
        engine.add_global_functions(process::create_js_apis());
        engine.add_global_functions(dialog::create_js_apis());
        engine.add_global_functions(Base64::create_js_apis());
        engine.add_global_functions(shell::create_js_apis());
        engine.add_global_functions(Audio::create_js_apis());
        engine.add_global_functions(path::create_js_apis());
        engine.add_global_functions(env::create_js_apis());
        engine.add_global_functions(http::create_js_apis());
        engine.add_global_functions(appfs::create_js_apis());
        engine.add_global_functions(localstorage::create_js_apis());
        // websocket
        engine.add_global_functions(WsConnection::create_js_apis());
        engine.add_global_functions(fetch::create_js_apis());

        engine.add_global_functions(Frame::create_js_apis());
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

        engine.add_global_func(clipboard_write_text::new());
        engine.add_global_func(clipboard_read_text::new());

        Worker::init_js_api(WorkerInitParams {
            module_loader_creator: Box::new(move || {
                Box::new(loader.clone())
            })
        });
        engine.add_global_functions(Worker::create_js_apis());
        engine
    }

    pub fn init_api(&self) {
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

    pub fn handle_window_event(&mut self, window_id: WindowId, event: WindowEvent) {
        handle_window_event(window_id, event);
    }

    pub fn handle_device_event(&mut self, device_id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::Button {..} = event {
            let close_frames = FRAMES.with_borrow(|frames| {
                let mut result = Vec::new();
                let menu_frames: Vec<&Frame> = frames.iter()
                    .filter(|(_, f)| f.frame_type == FrameType::Menu)
                    .map(|(_, f)| f)
                    .collect();
                if menu_frames.is_empty() {
                    return result;
                }
                run_with_event_loop(|el| {
                    if let Some(pos) = el.query_pointer(device_id) {
                        menu_frames.iter().for_each(|frame| {
                            let w_size = frame.window.outer_size();
                            if let Some(wp) = frame.window.outer_position().ok() {
                                let (wx, wy) = (wp.x as f32, wp.y as f32);
                                let (ww, wh) = (w_size.width as f32, w_size.height as f32);
                                let is_in_frame = pos.0 >= wx && pos.0 <= wx + ww
                                                       && pos.1 >= wy && pos.1 <= wy + wh;
                                if !is_in_frame {
                                    let _ = frame.window.set_cursor_grab(CursorGrabMode::None);
                                    result.push(frame.as_weak());
                                }
                            }
                        })
                    }
                });
                result
            });
            for frame in close_frames {
                if let Ok(mut f) = frame.upgrade() {
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