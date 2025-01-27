use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use anyhow::Error;
use jni::objects::JValue;
use jni::sys::{jboolean, jlong};
use crate::js::loader::JsModuleLoader;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
#[cfg(target_os = "android")]
use winit::platform::android::ActiveEventLoopExtAndroid;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
use winit::window::WindowId;

use crate::event_loop::{init_event_loop_proxy, run_event_loop_task, run_with_event_loop, AppEventProxy};
use crate::ext::ext_frame::FRAMES;
use crate::ext::ext_localstorage::localstorage_flush;
use crate::frame::{frame_check_update, frame_ime_resize, frame_input, frame_on_render_idle, frame_send_key};
use crate::js::js_engine::JsEngine;
use crate::js::js_event_loop::{js_init_event_loop, JsEvent, JsEventLoopClosedError};
use crate::js::js_runtime::JsContext;
use crate::mrc::Mrc;
use crate::timer;

#[derive(Debug)]
pub struct AppEventPayload {
    pub event: AppEvent,
    pub lock: Arc<(Mutex<bool>, Condvar)>,
}

impl AppEventPayload {
    pub fn new(event: AppEvent) -> Self {
        let lock = Arc::new((Mutex::new(false), Condvar::new()));
        Self {
            event,
            lock,
        }
    }
}

pub enum AppEvent {
    Callback(Box<dyn FnOnce() + Send + Sync>),
    JsEvent(JsEvent),
    ShowSoftInput(i32),
    HideSoftInput(i32),
    CommitInput(i32, String),
    NamedKeyInput(i32, String, bool),
    ImeResize(i32, f32),
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

pub trait DeftApp {
    fn init_js_engine(&mut self, js_engine: &mut JsEngine) {
        let _ = js_engine;
    }
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static>;
}


pub struct App {
    pub js_engine: JsEngine,
    pub deft_app: Box<dyn DeftApp>,
}

impl App {
    pub fn new(mut deft_app: Box<dyn DeftApp>, event_loop_proxy: AppEventProxy) -> Self {
        let module_loader = deft_app.create_module_loader();
        let mut js_engine = JsEngine::new(module_loader);
        js_engine.init_api();
        init_event_loop_proxy(event_loop_proxy.clone());
        let js_event_loop = js_init_event_loop(move |js_event| {
            event_loop_proxy.send_event(AppEvent::JsEvent(js_event)).map_err(|_| JsEventLoopClosedError {});
            Ok(())
        });
        deft_app.init_js_engine(&mut js_engine);
        Self {
            js_engine,
            deft_app,
        }
    }

    fn execute_pending_jobs(&mut self) {
        self.js_engine.execute_pending_jobs();
    }

}

impl ApplicationHandler<AppEventPayload> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        run_event_loop_task(event_loop, move || {
            let uninitialized = FRAMES.with(|m| m.borrow().is_empty());
            if uninitialized {
                self.js_engine.execute_main();
                self.execute_pending_jobs();
            } else {
                FRAMES.with_borrow_mut(|m| {
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
                AppEvent::ShowSoftInput(frame_id) => {
                    println!("show soft input");
                    #[cfg(target_os = "android")]
                    show_hide_keyboard(event_loop.android_app().clone(), frame_id, true);
                },
                AppEvent::HideSoftInput(frame_id) => {
                    println!("hide soft input");
                    #[cfg(target_os = "android")]
                    show_hide_keyboard(event_loop.android_app().clone(), frame_id, false);
                },
                AppEvent::CommitInput(frame_id, content) => {
                    frame_input(frame_id, content);
                },
                AppEvent::NamedKeyInput(frame_id, key, pressed) => {
                    frame_send_key(frame_id, &key, pressed);
                }
                AppEvent::ImeResize(frame_id, height) => {
                    frame_ime_resize(frame_id, height);
                },
                AppEvent::RenderIdle(frame_id) => {
                    frame_on_render_idle(frame_id);
                },
                AppEvent::Update(frame_id) => {
                    frame_check_update(frame_id);
                }
            }
            let (lock, cvar) = &*event.lock;
            let mut done = lock.lock().unwrap();
            *done = true;
            cvar.notify_one();
            self.execute_pending_jobs();
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        run_event_loop_task(event_loop, move || {
            self.js_engine.handle_window_event(window_id, event);
            self.execute_pending_jobs();
        });
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        run_event_loop_task(event_loop, move || {
            self.js_engine.handle_device_event(device_id, event);
            self.execute_pending_jobs();
        });
    }

}

pub fn exit_app(code: i32) -> Result<(), Error> {
    localstorage_flush()?;
    run_with_event_loop(|el| {
        el.exit();
    });
    Ok(())
}

#[cfg(target_os = "android")]
fn show_hide_keyboard_fallible(app: AndroidApp, frame_id: i32, show: bool) -> Result<(), jni::errors::Error> {
    use jni::JavaVM;
    use jni::objects::JObject;
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as _)? };
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as _) };
    let mut env = vm.attach_current_thread()?;
    let frame_id = frame_id as jlong;
    let show = show as jboolean;
    env.call_method(&activity, "showInput", "(JZ)V", &[
        JValue::Long(frame_id), JValue::Bool(show)
    ])?.v()?;
    Ok(())
}

#[cfg(target_os = "android")]
fn show_hide_keyboard(app: AndroidApp, frame_id: i32, show: bool) {
    if let Err(e) = show_hide_keyboard_fallible(app, frame_id, show) {
       //tracing::error!("Showing or hiding the soft keyboard failed: {e:?}");
    };
}

