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
                   if let Some(detail) = event.detail.raw().downcast_ref::<$ty>() {
                       callback(detail);
                       return true;
                   }
                }
                false
            }

            pub fn try_match_mut<F: FnMut(&mut crate::base::EventContext<crate::element::ElementRef>, &mut $ty)>(key: &str, event: &mut crate::base::ElementEvent, mut callback: F) -> bool {
                if key == Self::key() {
                   if let Some(detail) = event.detail.raw_mut().downcast_mut::<$ty>() {
                       callback(&mut event.context, detail);
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
                    if let Some(me) = e.detail.raw_mut().downcast_mut::<$ty>() {
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
                    if let Some(detail) = self.detail.raw().downcast_ref::<$ty>() {
                        callback(detail);
                        return true;
                    }
                }
                return false;
            }
        }
    };
}