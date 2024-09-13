use std::fmt::{Debug, Formatter};

use anyhow::Error;
use quick_js::loader::JsModuleLoader;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

use crate::event_loop::{get_event_proxy, run_event_loop_task};
use crate::ext::ext_frame::FRAMES;
use crate::ext::ext_localstorage::localstorage_flush;
use crate::js::js_engine::JsEngine;
use crate::timer;

pub enum AppEvent {
    Callback(Box<dyn FnOnce() + Send + Sync>),
    CallbackWithEventLoop(Box<dyn FnOnce(&ActiveEventLoop)>),
    CheckTimer,
}

impl Debug for AppEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        //TODO impl debug
        f.write_str("AppEvent")?;
        Ok(())
    }
}

pub struct App {
    js_engine: JsEngine,
}

impl App {
    pub fn new<L: JsModuleLoader>(module_loader: L) -> Self {
        let js_engine = JsEngine::new(module_loader);
        Self {
            js_engine,
        }
    }

}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        run_event_loop_task(event_loop, move || {
            let uninitialized = FRAMES.with(|m| m.borrow().is_empty());
            if uninitialized {
                self.js_engine.execute_main();
            } else {
                FRAMES.with_borrow_mut(|m| {
                    m.iter_mut().for_each(|(_, f)| {
                        f.resume();
                    })
                })
            }
        });
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        run_event_loop_task(event_loop, move || {
            match event {
                AppEvent::Callback(callback) => {
                    callback();
                },
                AppEvent::CallbackWithEventLoop(callback) => {
                    callback(event_loop);
                },
                AppEvent::CheckTimer => {
                    timer::check_task();
                },
            }
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        run_event_loop_task(event_loop, move || {
            self.js_engine.handle_window_event(window_id, event);
        });
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        run_event_loop_task(event_loop, move || {
            self.js_engine.execute_pending_jobs();
        });
    }

}

pub fn exit_app(code: i32) -> Result<(), Error> {
    localstorage_flush().unwrap();
    get_event_proxy().send_event(AppEvent::CallbackWithEventLoop(Box::new(|el| {
        el.exit();
    }))).unwrap();
    Ok(())
}

