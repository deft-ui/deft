use crate as deft;
use crate::app::{App, IApp};
use crate::base::{EventContext, EventListener, EventRegistration};
use crate::bind_js_event_listener;
use crate::ext::service::Service;
use crate::js::js_event_loop::{js_create_event_loop_fn_mut, js_is_in_event_loop, JsEvent};
use crate::js::JsError;
use crate::js_weak_value;
use deft_macros::{js_methods, mrc_object, worker_context_event, worker_event};
use quick_js::loader::JsModuleLoader;
use quick_js::JsValue;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Error;
use std::sync::{Arc, Mutex};

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
    service: Service,
}

pub type MessageData = String;

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
    pub app: App,
}

pub struct WorkerParams {
    pub worker_app: Option<Box<dyn IApp + Send + 'static>>,
    pub module_loader: Box<dyn JsModuleLoader + Send + Sync + 'static>,
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
    fn load(&mut self, module_name: &str) -> Result<String, Error> {
        let mut loader = self.module_loader.lock().unwrap();
        loader.load(module_name)
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
        let init_params = WORKER_INIT_PARAMS.with_borrow_mut(|p| p.as_mut().map(|p| p.app.clone()));
        if let Some(app) = init_params {
            Self::build(app, module_name)
        } else {
            Err(JsError::from_str("No worker loader found"))
        }
    }

    #[js_func]
    pub fn bind(service_id: u32) -> Result<Self, JsError> {
        let service = Service::get(service_id).ok_or(JsError::from_str("No service found"))?;
        Self::bind_service(service)
    }

    fn build(app: App, module_name: String) -> Result<Self, JsError> {
        let mut service = Service::new();
        service.start(app, module_name);
        Self::bind_service(service)
    }

    fn bind_service(service: Service) -> Result<Self, JsError> {
        let id = NEXT_WORKER_ID.get();
        NEXT_WORKER_ID.set(id + 1);

        let js_worker = WorkerData {
            id,
            event_registration: EventRegistration::new(),
            service: service.clone(),
        }
        .to_ref();

        if js_is_in_event_loop() {
            let js_worker = js_worker.clone();
            let mut cb = js_create_event_loop_fn_mut(move |msg: MessageData| {
                let mut js_worker = js_worker.clone();
                let _ = js_worker.receive_message(msg);
            });
            service.add_msg_handler(Box::new(move |msg| {
                cb.call(msg);
            }));
        }

        {
            let js_worker = js_worker.clone();
            JS_WORKDERS.with_borrow_mut(|workers| workers.insert(id, js_worker));
        }

        Ok(js_worker)
    }

    pub fn register_event_listener<T: 'static, H: EventListener<T, WorkerWeak> + 'static>(
        &mut self,
        listener: H,
    ) -> u32 {
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
    pub fn bind_js_event_listener(
        &mut self,
        event_type: String,
        listener: JsValue,
    ) -> Result<u32, JsError> {
        let id = bind_js_event_listener!(
            self, event_type.as_str(), listener;
            "message" => MessageEventListener,
        );
        let id = id.ok_or_else(|| JsError::new(format!("unknown event_type:{}", event_type)))?;
        Ok(id)
    }

    #[js_func]
    pub fn post_message(&mut self, message: MessageData) -> Result<(), JsError> {
        self.service
            .send_event(JsEvent::MacroTask(Box::new(move || {
                if let Some(mut ctx) = WorkerContext::get() {
                    ctx.receive_message(message);
                }
            })))
            .map_err(|e| JsError::new(format!("fail to send message:{}", e)))
    }

    fn receive_message(&mut self, data: MessageData) -> Result<(), JsError> {
        let event = MessageEvent { data };
        let mut ctx = EventContext::new(self.as_weak());
        self.event_registration.emit(event, &mut ctx);
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
        }
        .to_ref()
    }

    pub fn register_event_listener<T: 'static, H: EventListener<T, WorkerContextWeak> + 'static>(
        &mut self,
        listener: H,
    ) -> u32 {
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
    pub fn bind_js_event_listener(
        &mut self,
        event_type: String,
        listener: JsValue,
    ) -> Result<u32, JsError> {
        let id = bind_js_event_listener!(
            self, event_type.as_str(), listener;
            "message" => WorkerContextMessageEventListener,
        );
        let id = id.ok_or_else(|| JsError::new(format!("unknown event_type:{}", event_type)))?;
        Ok(id)
    }

    #[js_func]
    pub fn post_message(&mut self, data: MessageData) -> Result<(), JsError> {
        (self.message_emitter)(data);
        Ok(())
    }

    fn receive_message(&mut self, data: MessageData) {
        let event = WorkerContextMessageEvent { data };
        let mut ctx = EventContext::new(self.as_weak());
        self.event_registration.emit(event, &mut ctx);
    }
}
