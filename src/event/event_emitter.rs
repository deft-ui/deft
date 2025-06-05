use crate::element::{Element, ViewEvent};
use crate::event_loop::{create_event_loop_fn_mut, EventLoopFnMutCallback};
use std::any::{Any, TypeId};

#[derive(Clone)]
pub struct EventEmitter {
    emitter: EventLoopFnMutCallback<(TypeId, Box<dyn Any + Send + Sync>)>,
}

impl EventEmitter {
    pub fn new(element: &Element) -> EventEmitter {
        let weak = element.as_weak();
        let emitter = create_event_loop_fn_mut(move |p: (TypeId, Box<dyn Any + Send + Sync>)| {
            if let Ok(mut e) = weak.upgrade() {
                let ev: Box<dyn Any> = p.1;
                e.emit_raw(p.0, ev);
            }
        });
        Self { emitter }
    }

    pub fn emit<T: ViewEvent + Send + Sync + 'static>(&mut self, event: T) {
        let type_id = TypeId::of::<T>();
        self.emitter.call((type_id, Box::new(event)));
    }
}
