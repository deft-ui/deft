use crate::element::{Element, ViewEvent};
use crate::event::Event;
use crate::event_loop::{create_event_loop_fn_mut, EventLoopFnMutCallback};
use std::any::TypeId;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct EventEmitter {
    emitter: Arc<Mutex<EventLoopFnMutCallback<(TypeId, EventWrapper)>>>,
}

struct EventWrapper {
    event: Event,
}

unsafe impl Send for EventWrapper {}
unsafe impl Sync for EventWrapper {}

impl EventEmitter {
    pub fn new(element: &Element) -> EventEmitter {
        let weak = element.as_weak();
        let emitter = create_event_loop_fn_mut(move |p: (TypeId, EventWrapper)| {
            if let Ok(e) = weak.upgrade() {
                e.emit_raw(p.0, p.1.event);
            }
        });
        Self {
            emitter: Arc::new(Mutex::new(emitter)),
        }
    }

    pub fn emit<T: ViewEvent + Send + Sync + 'static>(&self, event: T) {
        let type_id = TypeId::of::<T>();
        let mut emitter = self.emitter.lock().unwrap();
        let event_wrapper = EventWrapper { event: Event::new(event) };
        emitter.call((type_id, event_wrapper));
    }
}
