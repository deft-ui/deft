use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use anyhow::Error;
#[cfg(target_os = "android")]
use jni::objects::JValue;
#[cfg(target_os = "android")]
use jni::sys::{jboolean, jlong};
use log::debug;
use measure_time::debug_time;
use skia_safe::Rect;
use crate::js::loader::JsModuleLoader;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
#[cfg(target_os = "android")]
use winit::platform::android::ActiveEventLoopExtAndroid;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
use winit::window::WindowId;
use crate::base::ResultWaiter;
use crate::event_loop::{init_event_loop_proxy, run_event_loop_task, run_with_event_loop, AppEventProxy};
use crate::ext::ext_localstorage::localstorage;
use crate::ext::ext_window::WINDOWS;
use crate::window::{window_check_update, window_input, window_on_render_idle, window_send_key, window_update_inset};
use crate::js::js_engine::JsEngine;
use crate::js::js_event_loop::{js_init_event_loop, JsEvent, JsEventLoopClosedError};
use crate::js::js_runtime::JsContext;
use crate::mrc::Mrc;
use crate::{platform, timer};

#[derive(Debug)]
pub struct AppEventPayload {
    pub event: AppEvent,
    pub result_waiter: ResultWaiter<()>,
}

#[derive(Copy, Clone, Debug)]
pub enum InsetType {
    StatusBar,
    Ime,
    Navigation,
}

impl InsetType {
    pub fn from_i32(ty: i32) -> Option<InsetType> {
        match ty {
            1 => Some(InsetType::StatusBar),
            2 => Some(InsetType::Navigation),
            8 => Some(InsetType::Ime),
            _ => None,
        }
    }
}

pub enum AppEvent {
    BindWindow(i32),
    Callback(Box<dyn FnOnce() + Send + Sync>),
    JsEvent(JsEvent),
    ShowSoftInput(i32),
    HideSoftInput(i32),
    CommitInput(i32, String),
    /// window_id,key,pressed
    NamedKeyInput(i32, String, bool),
    SetInset(i32, InsetType, Rect),
    Update(i32),
    RenderIdle(i32),
}

impl Debug for AppEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        //TODO impl debug
        f.write_str("AppEvent")?;
        Ok(())
    }
}

/// Deft app trait
pub trait IApp {
    /// Js engine initializing callback, you can add global functions or do some works to initialize the js engine.
    fn init_js_engine(&mut self, js_engine: &mut JsEngine) {
        let _ = js_engine;
    }
    /// Create js module loader
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static>;
}


pub struct WinitApp {
    pub js_engine: Mrc<JsEngine>,
}

#[derive(Clone)]
pub struct App {
    pub app_impl: Arc<Mutex<Box<dyn IApp + Send + Sync>>>,
}

impl App {
    pub fn new<A: IApp + Send + Sync + 'static>(app: A) -> Self {
        Self {
            app_impl: Arc::new(Mutex::new(Box::new(app))),
        }
    }
}

impl WinitApp {
    pub fn new(mut app: App, event_loop_proxy: AppEventProxy) -> Self {
        JsEngine::init(app.clone());
        let mut js_engine = JsEngine::get();
        js_engine.init_api();
        init_event_loop_proxy(event_loop_proxy.clone());
        let js_event_loop = js_init_event_loop(move |js_event| {
            event_loop_proxy.send_event(AppEvent::JsEvent(js_event)).map_err(|_| JsEventLoopClosedError {});
            Ok(())
        });
        {
            let mut app = app.app_impl.lock().unwrap();
            app.init_js_engine(&mut js_engine);
        }
        Self {
            js_engine,
        }
    }

    fn execute_pending_jobs(&mut self) {
        self.js_engine.execute_pending_jobs();
    }

}

impl ApplicationHandler<AppEventPayload> for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        run_event_loop_task(event_loop, move || {
            let uninitialized = WINDOWS.with(|m| m.borrow().is_empty());
            if uninitialized {
                debug_time!("js init time");
                self.js_engine.execute_main();
                self.execute_pending_jobs();
            } else {
                WINDOWS.with_borrow_mut(|m| {
                    m.iter_mut().for_each(|(_, f)| {
                        f.resume();
                    })
                })
            }
        });
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEventPayload) {
        run_event_loop_task(event_loop, move || {
            match event.event {
                AppEvent::BindWindow(id) => {
                    debug!("bindWindow {} Start", id);
                    #[cfg(target_os = "android")]
                    bind_deft_window(event_loop.android_app().clone(), id).unwrap();
                    debug!("bindWindow {} Done", id);
                }
                AppEvent::Callback(callback) => {
                    callback();
                },
                AppEvent::JsEvent(js_event) => {
                    match js_event {
                        JsEvent::MacroTask(callback) => {
                            callback();
                        }
                    }
                }
                AppEvent::ShowSoftInput(window_id) => {
                    debug!("show soft input");
                    #[cfg(target_os = "android")]
                    show_hide_keyboard(event_loop.android_app().clone(), window_id, true);
                    #[cfg(ohos)]
                    platform::show_soft_keyboard(window_id);
                },
                AppEvent::HideSoftInput(window_id) => {
                    debug!("hide soft input");
                    #[cfg(target_os = "android")]
                    show_hide_keyboard(event_loop.android_app().clone(), window_id, false);
                    #[cfg(ohos)]
                    platform::hide_soft_keyboard(window_id);
                },
                AppEvent::CommitInput(window_id, content) => {
                    window_input(window_id, content);
                },
                AppEvent::NamedKeyInput(window_id, key, pressed) => {
                    window_send_key(window_id, &key, pressed);
                }
                AppEvent::SetInset(window_id, ty, rect) => {
                    window_update_inset(window_id, ty, rect);
                },
                AppEvent::RenderIdle(window_id) => {
                    window_on_render_idle(window_id);
                },
                AppEvent::Update(window_id) => {
                    window_check_update(window_id);
                }
            }
            event.result_waiter.finish(());
            self.execute_pending_jobs();
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        // debug!("onWindowEvent: {:?}, {:?}", &window_id, event);
        run_event_loop_task(event_loop, move || {
            self.js_engine.handle_window_event(window_id, event);
            self.execute_pending_jobs();
        });
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        // debug!("onDeviceEvent: {:?}", event);
        run_event_loop_task(event_loop, move || {
            self.js_engine.handle_device_event(device_id, event);
            self.execute_pending_jobs();
        });
    }

}

pub fn exit_app(code: i32) -> Result<(), Error> {
    localstorage::cleanup();
    run_with_event_loop(|el| {
        el.exit();
    });
    Ok(())
}

#[cfg(target_os = "android")]
pub fn bind_deft_window(app: AndroidApp, window_id: i32) -> Result<(), jni::errors::Error> {
    use jni::JavaVM;
    use jni::objects::JObject;
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as _)? };
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as _) };
    let mut env = vm.attach_current_thread()?;
    let window_id = window_id as jlong;
    env.call_method(&activity, "bindDeftWindow", "(J)V", &[
        JValue::Long(window_id)
    ])?.v()?;
    Ok(())
}

#[cfg(target_os = "android")]
fn show_hide_keyboard_fallible(app: AndroidApp, window_id: i32, show: bool) -> Result<(), jni::errors::Error> {
    use jni::JavaVM;
    use jni::objects::JObject;
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as _)? };
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as _) };
    let mut env = vm.attach_current_thread()?;
    let window_id = window_id as jlong;
    let show = show as jboolean;
    env.call_method(&activity, "showInput", "(JZ)V", &[
        JValue::Long(window_id), JValue::Bool(show)
    ])?.v()?;
    Ok(())
}

#[cfg(target_os = "android")]
fn show_hide_keyboard(app: AndroidApp, window_id: i32, show: bool) {
    if let Err(e) = show_hide_keyboard_fallible(app, window_id, show) {
       //tracing::error!("Showing or hiding the soft keyboard failed: {e:?}");
    };
}

