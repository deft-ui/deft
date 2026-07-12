use std::any::TypeId;
use quick_js::{Callback, Context, ExecutionError, JsValue, ValueError};
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::panic::RefUnwindSafe;
use std::path::PathBuf;
use anyhow::anyhow;
use tokio::runtime::Builder;
use winit::dpi::{PhysicalSize};
use winit::event::{DeviceEvent, DeviceId, ElementState, WindowEvent};
use winit::window::{WindowId};

use crate::app::App;
use crate::console::Console;
use crate::element::button::Button;
use crate::element::checkbox::Checkbox;
use crate::element::image::Image;
use crate::element::label::Label;
use crate::element::radio::{Radio, RadioGroup};
use crate::element::richtext::RichText;
use crate::element::select::Select;
use crate::element::textedit::TextEdit;
use crate::element::textinput::TextInput;
use crate::element::Element;
use crate::element::body::Body;
use crate::element::container::Container;
use crate::ext::ext_animation::animation_create;
use crate::ext::ext_app::{AppReopenEvent, JsApp};
#[cfg(fs_enabled)]
use crate::ext::ext_appfs::appfs;
use crate::ext::ext_base64::Base64;
use crate::ext::ext_console::Console as ExtConsole;
use crate::ext::ext_env::env;
#[cfg(fs_enabled)]
use crate::ext::ext_fs::FileSystem;
use crate::ext::ext_localstorage::localstorage;
use crate::ext::ext_path::path;
use crate::ext::ext_process::process;
use crate::ext::ext_shell::shell;
use crate::ext::ext_timer::Timer;
#[cfg(feature = "tray")]
use crate::ext::ext_tray::SystemTray;
use crate::ext::ext_window::{handle_window_event, WINDOWS};
use crate::ext::ext_worker::{SharedModuleLoader, Worker, WorkerInitParams};
use crate::js::js_binding::{JsCallError, JsFunc};
use crate::js::js_runtime::{JsContext, PromiseResolver};
use crate::js::ToJsCallResult;
use crate::menu::{Menu, StandardMenuItem};
use crate::mrc::Mrc;
use crate::stylesheet::{Stylesheet};
use crate::typeface::typeface_create;
use crate::window::page::Page;
use crate::window::popup::Popup;
use crate::window::{Window, WindowHandle, WindowType};

#[derive(Clone)]
pub struct Function {
    name: String,
    executor: Mrc<Box<dyn JsFunc + RefUnwindSafe>>,
}

pub trait JsModule {
    fn get_functions() -> Vec<Function>;
    fn get_init_scripts() -> Option<String>;
}

#[derive(Default, Clone)]
struct JsModuleData {
    functions: Vec<Function>,
    init_code: Option<String>,
}

thread_local! {
    pub(crate) static JS_MODULES: RefCell<HashMap<TypeId, JsModuleData >> = RefCell::new(HashMap::new());
    static JS_ENGINE: RefCell<Option<Mrc<JsEngine>>> = RefCell::new(None);
}

pub fn register_js_function<F: JsFunc + RefUnwindSafe + 'static>(type_id: TypeId, name: &str, func: F) {
    JS_MODULES.with_borrow_mut(|m| {
        let module = m.entry(type_id).or_default();
        module.functions.push(Function {
            name: name.to_string(),
            executor: Mrc::new(Box::new(func)),
        });
    });
}

pub fn collect_js_functions<T: 'static>() -> Vec<Function> {
    JS_MODULES.with_borrow(|m| {
        let type_id = TypeId::of::<T>();
        m.get(&type_id).cloned().map(|d| d.functions).unwrap_or_default()
    })
}

pub fn register_js_init(type_id: TypeId, init_code: &str) {
    JS_MODULES.with_borrow_mut(|m| {
        let module = m.entry(type_id).or_default();
        module.init_code = Some(init_code.to_string());
    });
}


pub struct JsEngine {
    pub js_context: Mrc<JsContext>,
    pub app: App,
    module_loader: SharedModuleLoader,
}

struct JsFuncCallback {
    js_context: Mrc<JsContext>,
    pub js_func: Mrc<Box<dyn JsFunc + RefUnwindSafe>>,
}

impl Callback<()> for JsFuncCallback {
    fn argument_count(&self) -> usize {
        self.js_func.args_count()
    }

    fn call(&self, args: Vec<JsValue>) -> Result<Result<JsValue, String>, ValueError> {
        let mut js_context = self.js_context.clone();
        match self.js_func.call(&mut js_context, args) {
            Ok(v) => Ok(Ok(v)),
            Err(e) => match e {
                JsCallError::ConversionError(ce) => Err(ce),
                JsCallError::ExecutionError(ee) => Ok(Err(ee.to_string())),
            },
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

    pub fn init(app: App) {
        let loader = {
            let mut app = app.app_impl.lock().unwrap();
            SharedModuleLoader::new(app.create_module_loader())
        };
        #[cfg(not(emscripten_platform))]
        let runtime = {
            Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .unwrap()
        };
        #[cfg(emscripten_platform)]
        let runtime = { Builder::new_current_thread().enable_all().build().unwrap() };

        let js_context = Context::builder()
            .console(Console::new())
            .module_loader(loader.clone())
            .build()
            .unwrap();
        let js_context = Mrc::new(JsContext::new(js_context, runtime));

        let mut engine = Mrc::new(Self {
            js_context,
            app: app.clone(),
            module_loader: loader.clone(),
        });

        JS_ENGINE.with(|e| *e.borrow_mut() = Some(engine.clone()));

        // Init core module and console
        engine.eval_module(include_str!("./core.js"), "deft:core").unwrap();
        engine.register_module_and_load::<ExtConsole>("deft:core:console").unwrap();

        // Init app module
        engine.register_module_and_load::<JsApp>("deft:core:jsapp").unwrap();

        // Init env module
        engine.register_module::<env>("deft:env").unwrap();

        // Init process module
        engine.register_module_and_load::<process>("deft:process").unwrap();

        // Init menu modules
        engine.register_module::<Menu>("deft:core:menu").unwrap();
        engine.register_module::<StandardMenuItem>("deft:core:standardmenuitem").unwrap();
        engine.eval_module(include_str!("./menu.js"), "deft:menu").unwrap();

        // Init timer module
        engine.register_module_and_load::<Timer>("deft:core:timer").unwrap();

        // Init window modules
        engine.register_module::<Page>("deft:core:page").unwrap();
        engine.register_module::<Popup>("deft:core:popup").unwrap();
        engine.register_module_and_load::<Window>("deft:core:window").unwrap();

        // Init ui modules
        engine.register_module::<Element>("deft:core:element").unwrap();
        engine.register_module::<Body>("deft:core:body").unwrap();
        engine.register_module::<Checkbox>("deft:core:checkbox").unwrap();
        engine.register_module::<Container>("deft:core:container").unwrap();
        engine.register_module::<Button>("deft:core:button").unwrap();
        engine.register_module::<Checkbox>("deft:core:checkbox").unwrap();
        engine.register_module::<RadioGroup>("deft:core:radiogroup").unwrap();
        engine.register_module::<Radio>("deft:core:radio").unwrap();
        engine.register_module::<TextInput>("deft:core:textinput").unwrap();
        engine.register_module::<TextEdit>("deft:core:textedit").unwrap();
        engine.register_module::<RichText>("deft:core:richtext").unwrap();
        engine.register_module::<Label>("deft:core:label").unwrap();
        engine.register_module::<Image>("deft:core:image").unwrap();
        engine.register_module::<Select>("deft:core:select").unwrap();
        engine.eval_module(include_str!("./ui.js"), "deft:ui").unwrap();

        // Init sqlite module
        #[cfg(feature = "sqlite")]
        engine.register_mod::<crate::ext::ext_sqlite::SqliteConn>("deft:sqlite").unwrap();

        // Init system tray module
        #[cfg(feature = "tray")]
        {
            engine.register_module::<SystemTray>("deft:systemtray").unwrap();
        }

        // Init dialog module
        #[cfg(feature = "dialog")]
        engine.register_module::<crate::ext::ext_dialog::dialog>("deft:dialog").unwrap();

        // Init base64 module
        engine.register_module::<Base64>("deft:core:base64").unwrap();

        // Init shell module
        engine.register_module::<shell>("deft:core:shell").unwrap();

        // Init audio module
        #[cfg(feature = "audio")]
        engine.register_module::<crate::ext::ext_audio::Audio>("deft:audio").unwrap();

        // Init localstorage module
        engine.register_module_and_load::<localstorage>("deft:core:localstorage").unwrap();

        // Init fs module
        engine.register_module::<path>("deft:path").unwrap();
        #[cfg(fs_enabled)]
        {
            engine.register_module::<appfs>("deft:appfs").unwrap();
            engine.register_module::<FileSystem>("deft:fs").unwrap();
        }

        // Init net modules
        #[cfg(feature = "http")]
        {
            engine.register_module::<crate::ext::ext_http::http>("deft:core:http").unwrap();
            engine.register_module_and_load::<crate::ext::ext_fetch::fetch>("deft:core:fetch").unwrap();
        }
        #[cfg(feature = "websocket")]
        engine.register_module_and_load::<crate::ext::ext_websocket::WsConnection>("deft:core:wsconnection").unwrap();

        engine.add_global_func(animation_create::new());
        engine.add_global_func(typeface_create::new());

        // Init clipboard module
        #[cfg(feature = "clipboard")]
        engine.register_module_and_load::<crate::ext::ext_clipboard::Clipboard>("deft:clipboard").unwrap();
        engine.register_module_and_load::<Stylesheet>("deft:core:stylesheet").unwrap();

        // Init worker modules
        Worker::init_js_api(WorkerInitParams { app });
        engine.register_module_and_load::<Worker>("deft:core:worker").unwrap();
    }

    pub fn enable_localstorage(&mut self, p: PathBuf) {
        localstorage::init(p);
    }

    pub fn create_async_task<F, O>(&mut self, future: F) -> JsValue
    where
        F: Future<Output = O> + Send + 'static,
        O: ToJsCallResult,
    {
        self.js_context.create_async_task2(future)
    }

    pub fn create_promise(&mut self) -> (JsValue, PromiseResolver) {
        self.js_context.create_promise()
    }

    pub fn register_module<M: JsModule>(&mut self, module_name: &str) -> anyhow::Result<()> {
        let mut module = self.js_context.create_module(&format!("native://{}", module_name));
        for f in M::get_functions() {
            let js_context = self.js_context.clone();
            module = module.add_function(&f.name, JsFuncCallback {
                js_context,
                js_func: f.executor,
            });
        }
        module.build().map_err(|e| anyhow!("failed to build js module: {:?}", e))?;
        let module_init_code = M::get_init_scripts().unwrap_or("export * from 'native'".to_string());
        self.module_loader.register_memory_module(module_name, &module_init_code);
        Ok(())
    }

    pub fn register_module_and_load<M: JsModule>(&mut self, module_name: &str) -> anyhow::Result<()> {
        self.register_module::<M>(module_name)?;
        self.js_context.execute_module(module_name)
            .map_err(|e| anyhow!("Failed to init module: {:?}", e))?;
        Ok(())
    }

    pub fn add_global_functions(&self, functions: Vec<Box<dyn JsFunc + RefUnwindSafe + 'static>>) {
        for func in functions {
            let name = func.name().to_string();
            let js_context = self.js_context.clone();
            self.js_context
                .add_callback(
                    name.as_str(),
                    JsFuncCallback {
                        js_func: Mrc::new(func),
                        js_context,
                    },
                )
                .unwrap();
        }
    }

    pub fn add_global_func(&self, func: impl JsFunc + RefUnwindSafe + 'static) {
        let name = func.name().to_string();
        let js_context = self.js_context.clone();
        self.js_context
            .add_callback(
                name.as_str(),
                JsFuncCallback {
                    js_func: Mrc::new(Box::new(func)),
                    js_context,
                },
            )
            .unwrap();
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

    pub fn handle_device_event(&mut self, _device_id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::Button { state, .. } = event {
            if state == ElementState::Pressed {
                let close_windows: Vec<WindowHandle> = WINDOWS.with_borrow(|windows| {
                    windows
                        .iter()
                        .filter(|(_, f)| {
                            f.upgrade()
                                .ok()
                                .map(|f| f.window_type == WindowType::Menu)
                                .unwrap_or(false)
                        })
                        .map(|(_, f)| f.clone())
                        .filter(|w| {
                            if let Ok(window) = w.upgrade() {
                                let w_size: PhysicalSize<i32> = window.window.outer_size().cast();
                                if let Some(ptr_pos) = window.window.pointer_position() {
                                    let is_in_window = ptr_pos.x >= 0
                                        && ptr_pos.x <= w_size.width
                                        && ptr_pos.y >= 0
                                        && ptr_pos.y <= w_size.height;
                                    if !is_in_window {
                                        return true;
                                    }
                                }
                            }
                            false
                        })
                        .collect()
                });
                for f in close_windows {
                    if let Ok(mut f) = f.upgrade() {
                        let _ = f.close();
                    }
                }
            }
        }
    }

    pub fn handle_reopen(&mut self, has_visible: bool) {
        JsApp::emit(AppReopenEvent { has_visible });
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
