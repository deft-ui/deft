use crate as lento;
use crate::js::js_engine::JsEngine;
use crate::js::JsError;
use crate::{js_weak_value};
use crate::js::js_event_loop::{js_create_event_loop_fn_mut, js_init_event_loop, js_is_in_event_loop, JsEvent, JsEventLoopClosedError};
use lento_macros::{js_methods, mrc_object, worker_context_event, worker_event};
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Error;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use crate::base::{EventContext, EventListener, EventRegistration};
use crate::{bind_js_event_listener};
use crate::js::ToJsValue;
use quick_js::{Callback, JsValue};
use quick_js::loader::JsModuleLoader;

thread_local! {
    pub static JS_WORKDERS: RefCell<HashMap<u32, Worker >> = RefCell::new(HashMap::new());
    pub static JS_WORKER_CONTEXTS: RefCell<Option<WorkerContext>> = RefCell::new(None);
    pub static NEXT_WORKER_ID: Cell<u32> = Cell::new(1);
    static WORKER_INIT_PARAMS: RefCell<Option<WorkerInitParams>> = RefCell::new(None);
}

#[mrc_object]
pub struct Worker {
    id: u32,
    event_registration: EventRegistration<WorkerWeak>,
    worker_event_sender: Sender<JsEvent>,
}

type MessageData = String;

#[worker_event]
pub struct MessageEvent {
    data: MessageData,
}

js_weak_value!(Worker, WorkerWeak);

#[worker_context_event]
pub struct WorkerContextMessageEvent {
    data: MessageData,
}

pub struct WorkerInitParams {
    pub module_loader_creator: Box<dyn FnMut() -> Box<dyn JsModuleLoader + Send + Sync + 'static>>,
}

#[derive(Clone)]
pub struct SharedModuleLoader {
    module_loader: Arc<Mutex<Box<dyn JsModuleLoader + Send + Sync>>>,
}

impl SharedModuleLoader {
    pub fn new(module_loader: Box<dyn JsModuleLoader + Send + Sync + 'static>) -> Self {
        Self {
            module_loader: Arc::new(Mutex::new(module_loader)),
        }
    }
}

impl JsModuleLoader for SharedModuleLoader {
    fn load(&self, module_name: &str) -> Result<String, Error> {
        let loader = self.module_loader.lock().unwrap();
        loader.load(module_name)
    }
}

pub struct Service {
    sender: Sender<JsEvent>,
    receivers: Arc<Mutex<Vec<Box<dyn FnMut(MessageData) + Send>>>>,
}

impl Service {

    pub fn new(module_loader: Box<dyn JsModuleLoader + Send + Sync + 'static>, module_name: String) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        let shared_module_loader = SharedModuleLoader::new(module_loader);

        let receivers = Arc::new(Mutex::new(Vec::new()));
        let service = Self {
            sender: sender.clone(),
            receivers: receivers.clone(),
        };

        let module_loader = shared_module_loader.clone();
        let _ = thread::Builder::new().name("js-worker".to_string()).spawn(move || {
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

            let _ = js_engine.js_context.add_callback("WorkerContext_get", move || {
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

    pub fn add_receiver(&self, receiver: Box<dyn FnMut(MessageData) + Send>) {
        let mut receivers = self.receivers.lock().unwrap();
        receivers.push(receiver);
    }

}

#[js_methods]
impl Worker {

    pub fn init_js_api(init_params: WorkerInitParams) {
        WORKER_INIT_PARAMS.with_borrow_mut(|m| {
            m.replace(init_params);
        });
    }

    #[js_func]
    pub fn create(module_name: String) -> Result<Self, JsError> {
        let loader = WORKER_INIT_PARAMS.with_borrow_mut(|p| {
            p.as_mut().map(|p| (p.module_loader_creator)())
        });
        if let Some(loader) = loader {
            Self::build(loader, module_name)
        } else {
            Err(JsError::from_str("No worker loader found"))
        }
    }

    pub fn new<L: JsModuleLoader + Send + Sync + 'static>(module_loader: L, module_name: String) -> Result<Self, JsError> {
        Self::build(Box::new(module_loader), module_name)
    }

    fn build(module_loader: Box<dyn JsModuleLoader + Send + Sync + 'static>, module_name: String) -> Result<Self, JsError>{
        let id = NEXT_WORKER_ID.get();
        NEXT_WORKER_ID.set(id + 1);

        let service = Service::new(module_loader, module_name);

        let js_worker = WorkerData {
            id,
            event_registration: EventRegistration::new(),
            worker_event_sender: service.sender.clone(),
        }.to_ref();

        if js_is_in_event_loop() {
            let js_worker = js_worker.clone();
            let mut cb = js_create_event_loop_fn_mut(move |msg: MessageData| {
                let mut js_worker = js_worker.clone();
                js_worker.receive_message(msg).unwrap();
            });
            service.add_receiver(Box::new(move |msg| {
                cb.call(msg);
            }));
        }

        {
            let js_worker = js_worker.clone();
            JS_WORKDERS.with_borrow_mut(|workers| workers.insert(id, js_worker));
        }

        Ok(js_worker)
    }

    pub fn register_event_listener<T: 'static, H: EventListener<T, WorkerWeak> + 'static>(&mut self, mut listener: H) -> u32 {
        self.event_registration.register_event_listener(listener)
    }

    pub fn unregister_event_listener(&mut self, id: u32) {
        self.event_registration.unregister_event_listener(id)
    }

    #[js_func]
    pub fn remove_js_event_listener(&mut self, id: u32) {
        self.unregister_event_listener(id);
    }

    #[js_func]
    pub fn bind_js_event_listener(&mut self, event_type: String, listener: JsValue) -> Result<u32, JsError> {
        let id = bind_js_event_listener!(
            self, event_type.as_str(), listener;
            "message" => MessageEventListener,
        );
        Ok(id)
    }

    #[js_func]
    pub fn post_message(&mut self, message: MessageData) -> Result<(), JsError> {
        self.worker_event_sender.send(JsEvent::MacroTask(Box::new(move || {
            if let Some(mut ctx) = WorkerContext::get() {
                ctx.receive_message(message);
            }
        }))).map_err(|e| JsError::new(format!("fail to send message:{}", e)))
    }

    fn receive_message(&mut self, data: MessageData) -> Result<(), JsError> {
        let mut event = MessageEvent {
            data,
        };
        let mut ctx = EventContext {
            target: self.as_weak(),
            propagation_cancelled: false,
            prevent_default: false,
        };
        self.event_registration.emit(&mut event, &mut ctx);
        Ok(())
    }
}

#[mrc_object]
pub struct WorkerContext {
    message_emitter: Box<dyn FnMut(MessageData)>,
    event_registration: EventRegistration<WorkerContextWeak>,
}

js_weak_value!(WorkerContext, WorkerContextWeak);

#[js_methods]
impl WorkerContext {

    pub fn get() -> Option<Self> {
        JS_WORKER_CONTEXTS.with_borrow(|m| m.as_ref().map(|it| it.clone()))
    }

    pub fn create(message_emitter: Box<dyn FnMut(MessageData)>) -> Self {
        WorkerContextData {
            message_emitter,
            event_registration: EventRegistration::new(),
        }.to_ref()
    }

    pub fn register_event_listener<T: 'static, H: EventListener<T, WorkerContextWeak> + 'static>(&mut self, mut listener: H) -> u32 {
        self.event_registration.register_event_listener(listener)
    }

    pub fn unregister_event_listener(&mut self, id: u32) {
        self.event_registration.unregister_event_listener(id)
    }

    #[js_func]
    pub fn remove_js_event_listener(&mut self, id: u32) {
        self.unregister_event_listener(id);
    }

    #[js_func]
    pub fn bind_js_event_listener(&mut self, event_type: String, listener: JsValue) -> Result<u32, JsError> {
        let id = bind_js_event_listener!(
            self, event_type.as_str(), listener;
            "message" => WorkerContextMessageEventListener,
        );
        Ok(id)
    }

    #[js_func]
    pub fn post_message(&mut self, data: MessageData) -> Result<(), JsError> {
        (self.message_emitter)(data);
        Ok(())
    }

    fn receive_message(&mut self, data: MessageData) {
        let mut event = WorkerContextMessageEvent { data };
        let mut ctx = EventContext {
            target: self.as_weak(),
            propagation_cancelled: false,
            prevent_default: false,
        };
        self.event_registration.emit(&mut event, &mut ctx);
    }

}
