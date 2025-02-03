use std::cell::RefCell;
use crate::ext::ext_worker::{SharedModuleLoader, WorkerContext, WorkerParams, JS_WORKER_CONTEXTS};
use crate::js::js_engine::JsEngine;
use crate::js::js_event_loop::{js_init_event_loop, JsEvent, JsEventLoopClosedError};
use crate::js::ToJsValue;
use quick_js::loader::JsModuleLoader;
use std::collections::HashMap;
use std::sync::mpsc::{SendError, Sender};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use std::thread;
use crate::app::{IApp, App};
use crate::id_generator::IdGenerator;
use crate::id_hash_map::IdHashMap;

struct ServiceHolder {
    id_generator: IdGenerator,
    services: HashMap<u32, Service>,
}

static SERVICES: LazyLock<Arc<Mutex<ServiceHolder>>> = LazyLock::new(|| {
    let holder = ServiceHolder {
        id_generator: IdGenerator::new(),
        services: HashMap::new(),
    };
    Arc::new(Mutex::new(holder))
});

#[derive(Clone)]
pub struct Service {
    id: u32,
    sender: Arc<Mutex<Option<Sender<JsEvent>>>>,
    msg_handlers: Arc<Mutex<IdHashMap<Box<dyn FnMut(crate::ext::ext_worker::MessageData) + Send>>>>,
}

impl Service {

    pub fn get(id: u32) -> Option<Self> {
        let services = SERVICES.lock().unwrap();
        services.services.get(&id).cloned()
    }

    pub fn new() -> Self {
        let id = {
            let mut service_holder = SERVICES.lock().unwrap();
            service_holder.id_generator.generate_id()
        };

        let msg_handlers = Arc::new(Mutex::new(IdHashMap::new()));
        let service = Self {
            id,
            sender: Arc::new(Mutex::new(None)),
            msg_handlers: msg_handlers.clone(),
        };
        {
            let mut services = SERVICES.lock().unwrap();
            services.services.insert(id, service.clone());
        }

        service
    }

    pub fn start(&mut self, app: App, module_name: String) {
        let (sender, receiver) = std::sync::mpsc::channel();
        {
            let mut sender_holder = self.sender.lock().unwrap();
            sender_holder.replace(sender.clone());
        }
        let msg_handlers = self.msg_handlers.clone();
        let module_loader = {
            let mut app = app.app_impl.lock().unwrap();
            app.create_module_loader()
        };
        let shared_module_loader = SharedModuleLoader::new(module_loader);
        let module_loader = shared_module_loader.clone();
        let _ = thread::Builder::new()
            .name("js-worker".to_string())
            .spawn(move || {
                JsEngine::init(app.clone());
                let mut js_engine = JsEngine::get();

                js_init_event_loop(move |js_event| {
                    sender.send(js_event).map_err(|_| JsEventLoopClosedError {})
                });

                let worker_context = WorkerContext::create(Box::new(move |msg| {
                    let mut msg_handlers = msg_handlers.lock().unwrap();
                    msg_handlers.for_each_mut(|_, handler| {
                        handler(msg.clone());
                    });
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
                {
                    let mut app = app.app_impl.lock().unwrap();
                    app.init_js_engine(&mut js_engine);
                }

                let r = js_engine.execute_module(module_name.as_str());
                if let Err(err) = r {
                    println!("Error executing module: {}, error:{}", module_name, err);
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
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn send_event(&self, event: JsEvent) -> Result<(), SendError<JsEvent>> {
        let sender = self.sender.lock().unwrap();
        if let Some(sender) = sender.as_ref() {
            sender.send(event)
        } else {
            Err(SendError(event))
        }
    }

    pub fn add_msg_handler(
        &self,
        handler: Box<dyn FnMut(crate::ext::ext_worker::MessageData) + Send>,
    ) {
        let mut handlers = self.msg_handlers.lock().unwrap();
        handlers.insert(handler);
    }

    pub fn remove_msg_handler(&self, id: u32) {
        let mut handlers = self.msg_handlers.lock().unwrap();
        handlers.remove(id);
    }

}
