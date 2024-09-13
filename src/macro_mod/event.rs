use crate::base::ElementEvent;
#[macro_export]
macro_rules! define_event {
    ($name: ident, $bind_trait: ident, $key: expr, $bind_func: ident, $emit_func: ident,$event_trait: ident, $as_func: ident, $ty: ty) => {
        pub struct $name {

        }

        impl $name {
            pub fn key() -> &'static str {
                $key
            }

            pub fn try_match<F: FnMut(&$ty)>(key: &str, event: &crate::base::ElementEvent, mut callback: F) -> bool {
                if key == Self::key() {
                   if let Some(detail) = event.detail.downcast_ref::<$ty>() {
                       callback(detail);
                       return true;
                   }
                }
                false
            }
        }

        pub trait $bind_trait {
            fn $bind_func<F: FnMut(&mut crate::base::ElementEventContext, &mut $ty) + 'static>(&mut self, handler: F) -> u32;
            fn $emit_func(&mut self, detail: $ty);
        }

        pub trait $event_trait {
            fn $as_func<F: FnMut(&$ty)>(&mut self, callback: F) -> bool;
        }

        impl $bind_trait for crate::element::ElementRef {
            fn $bind_func<F: FnMut(&mut crate::base::ElementEventContext, &mut $ty) + 'static>(&mut self, mut handler: F) -> u32 {
                self.add_event_listener($key, Box::new(move |e| {
                    if let Some(me) = e.detail.downcast_mut::<$ty>() {
                        handler(&mut e.context, me);
                    }
                }))
            }
            fn $emit_func(&mut self, detail: $ty) {
                use crate::base::ElementEvent;
                let mut event = ElementEvent::new($key, detail, self.clone());
                self.emit_event($key, &mut event);
            }
        }

        impl $event_trait for crate::base::Event<crate::element::ElementRef> {
            fn $as_func<F: FnMut(&$ty)>(&mut self, mut callback: F) -> bool {
                if self.event_type == $key {
                    if let Some(detail) = self.detail.downcast_ref::<$ty>() {
                        callback(detail);
                        return true;
                    }
                }
                return false;
            }
        }
    };
}

#[macro_export]
macro_rules! js_event_bind {
    ($target: expr, $event_name: expr, $event_type: ty, $param_event_name: expr, $param_handler: expr) => {
        if $param_event_name == $event_name {
            use crate::js::js_serde::JsValueSerializer;
            use serde::{Deserialize, Serialize};
            let ret = $target.add_event_listener($param_event_name, Box::new(move |e| {
                if let Some(me) = e.detail.downcast_mut::<$event_type>() {
                    //TODO remove target_id
                    let target_id = 0;//e.context.target.get_id();
                    $param_handler(&mut e.context, target_id as i32,  me.serialize(JsValueSerializer {}).unwrap());
                }
            }));
            return Ok(ret as i32)
        }
    };
}

#[macro_export]
macro_rules! js_event_bind2 {
    ($target: expr, $event_name: expr, $event_type: ty, $param_event_name: expr, $param_handler: expr) => {
        if $param_event_name == $event_name {
            use crate::js::js_serde::JsValueSerializer;
            let ret = $target.add_event_listener($param_event_name, Box::new(move |e| {
                if let Some(me) = e.detail.downcast_mut::<$event_type>() {
                    let target_id = e.context.target.get_id();
                    $param_handler(&mut e.context, target_id as i32,  me.serialize(JsValueSerializer {}).unwrap());
                }
            }));
            return ret
        }
    };
}