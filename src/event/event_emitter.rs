use crate::element::{Element, ViewEvent};
use crate::event::Event;
use crate::event_loop::{
    create_event_loop_fn_mut, EventLoopFnMutCallback,
};
use std::any::{Any, TypeId};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct EventEmitter {
    emitter: Arc<Mutex<EventLoopFnMutCallback<(TypeId, Box<dyn Any + Send + Sync>)>>>,
}

impl EventEmitter {
    pub fn new(element: &Element) -> EventEmitter {
        let weak = element.as_weak();
        let emitter = create_event_loop_fn_mut(move |p: (TypeId, Box<dyn Any + Send + Sync>)| {
            if let Ok(e) = weak.upgrade() {
                let ev = Event::new(p.1);
                e.emit_raw(p.0, ev);
            }
        });
        Self {
            emitter: Arc::new(Mutex::new(emitter)),
        }
    }

    pub fn emit<T: ViewEvent + Send + Sync + 'static>(&self, event: T) {
        let type_id = TypeId::of::<T>();
        let mut emitter = self.emitter.lock().unwrap();
        emitter.call((type_id, Box::new(event)));
    }
}
