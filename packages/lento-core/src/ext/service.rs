use std::cell::RefCell;
use crate::ext::ext_worker::{SharedModuleLoader, WorkerContext, JS_WORKER_CONTEXTS};
use crate::js::js_engine::JsEngine;
use crate::js::js_event_loop::{js_init_event_loop, JsEvent, JsEventLoopClosedError};
use crate::js::ToJsValue;
use quick_js::loader::JsModuleLoader;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use std::thread;

struct ServiceHolder {
    next_id: u32,
    services: HashMap<u32, Service>,
}

static SERVICES: LazyLock<Arc<Mutex<ServiceHolder>>> = LazyLock::new(|| {
    let holder = ServiceHolder {
        next_id: 1,
        services: HashMap::new(),
    };
    Arc::new(Mutex::new(holder))
});

#[derive(Clone)]
pub struct Service {
    pub id: u32,
    pub sender: Sender<JsEvent>,
    pub receivers: Arc<Mutex<Vec<Box<dyn FnMut(crate::ext::ext_worker::MessageData) + Send>>>>,
}

impl Service {

    pub fn get(id: u32) -> Option<Self> {
        let services = SERVICES.lock().unwrap();
        services.services.get(&id).cloned()
    }

    pub fn new(
        module_loader: Box<dyn JsModuleLoader + Send + Sync + 'static>,
        module_name: String,
    ) -> Self {
        let id = {
            let mut services = SERVICES.lock().unwrap();
            let id = services.next_id;
            services.next_id += 1;
            id
        };

        let (sender, receiver) = std::sync::mpsc::channel();
        let shared_module_loader = SharedModuleLoader::new(module_loader);

        let receivers = Arc::new(Mutex::new(Vec::new()));
        let service = Self {
            id,
            sender: sender.clone(),
            receivers: receivers.clone(),
        };
        {
            let mut services = SERVICES.lock().unwrap();
            services.services.insert(id, service.clone());
        }

        let module_loader = shared_module_loader.clone();
        let _ = thread::Builder::new()
            .name("js-worker".to_string())
            .spawn(move || {
                let mut js_engine = JsEngine::new(module_loader);

                js_init_event_loop(move |js_event| {
                    sender.send(js_event).map_err(|_| JsEventLoopClosedError {})
                });

                let worker_context = WorkerContext::create(Box::new(move |msg| {
                    let mut receivers = receivers.lock().unwrap();
                    for receiver in receivers.iter_mut() {
                        receiver(msg.clone());
                    }
                }));
                JS_WORKER_CONTEXTS.with_borrow_mut(|ctxs| {
                    ctxs.replace(worker_context);
                });

                let _ = js_engine
                    .js_context
                    .add_callback("WorkerContext_get", move || {
                        let ctx = WorkerContext::get();
                        ctx.unwrap().to_js_value().unwrap()
                    });
                js_engine.add_global_functions(WorkerContext::create_js_apis());

                js_engine.init_api();
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
        service
    }

    pub fn add_receiver(
        &self,
        receiver: Box<dyn FnMut(crate::ext::ext_worker::MessageData) + Send>,
    ) {
        let mut receivers = self.receivers.lock().unwrap();
        receivers.push(receiver);
    }
}
