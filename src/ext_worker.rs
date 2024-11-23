use crate as lento;
use crate::js::js_engine::JsEngine;
use crate::js::JsError;
use crate::{create_module_loader, js_weak_value};
use lento_core::js::js_event_loop::{js_init_event_loop, JsEvent, JsEventLoopClosedError};
use lento_macros::{js_func, js_methods, mrc_object};
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::thread;

thread_local! {
    pub static JS_WORKDERS: RefCell<HashMap<u32, Worker >> = RefCell::new(HashMap::new());
    pub static NEXT_WORKER_ID: Cell<u32> = Cell::new(1);
}

#[mrc_object]
pub struct Worker {
    id: u32,
}

js_weak_value!(Worker, WorkerWeak);

#[js_methods]
impl Worker {
    #[js_func]
    pub fn create(module_name: String) -> Result<Self, JsError> {
        let id = NEXT_WORKER_ID.get();
        NEXT_WORKER_ID.set(id + 1);

        thread::spawn(move || {
            let mut js_engine = JsEngine::new(create_module_loader());
            let (sender, receiver) = std::sync::mpsc::channel();
            js_init_event_loop(move |js_event| {
                sender.send(js_event).map_err(|_| JsEventLoopClosedError {})
            });
            let r = js_engine.execute_module(module_name.as_str());
            if let Err(err) = r {
                println!("Error executing module: {}", err);
                return;
            }
            js_engine.execute_pending_jobs();
            loop {
                let Ok(event) = receiver.recv() else {
                    break;
                };
                match event {
                    JsEvent::MacroTask(task) => {
                        task();
                    }
                }
                js_engine.execute_pending_jobs();
            }
        });

        let js_worker = WorkerData { id }.to_ref();
        {
            let js_worker = js_worker.clone();
            JS_WORKDERS.with_borrow_mut(|workers| workers.insert(id, js_worker));
        }

        Ok(js_worker)
    }
}
