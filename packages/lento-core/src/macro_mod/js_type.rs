#[macro_export]
macro_rules! define_ref_and_resource {
    ($ty: ident, $target_ty: ty) => {
        crate::define_ref!($ty, $target_ty);
        crate::define_resource!($ty);
    };
}

#[macro_export]
macro_rules! define_resource {
    ($ty: ident) => {
        impl crate::js::js_value_util::ToJsValue for $ty {
            fn to_js_value(self) -> Result<JsValue, Error> {
                Ok(JsValue::Resource(quick_js::ResourceValue { resource: std::rc::Rc::new(std::cell::RefCell::new(self)) }))
            }
        }

        impl crate::js::js_value_util::FromJsValue for $ty {
            fn from_js_value(value: JsValue) -> Result<Self, Error> {
                if let Some(r) = value.as_resource(|r: &mut $ty| r.clone()) {
                    Ok(r)
                } else {
                    use anyhow::anyhow;
                    Err(anyhow!("invalid value"))
                }
            }
        }
    };
}

#[macro_export]
macro_rules! js_value {
    ($ref_type: ty) => {
        impl lento::js::ToJsValue for $ref_type {
            fn to_js_value(self) -> Result<lento::js::JsValue, quick_js::ValueError> {
                Ok(lento::js::JsValue::Resource(quick_js::ResourceValue { resource: std::rc::Rc::new(std::cell::RefCell::new(self)) }))
            }
        }

        impl lento::js::FromJsValue for $ref_type {
            fn from_js_value(value: lento::js::JsValue) -> Result<Self, quick_js::ValueError> {
                if let Some(r) = value.as_resource(|r: &mut $ref_type| r.clone()) {
                    Ok(r)
                } else {
                    Err(quick_js::ValueError::UnexpectedType)
                }
            }
        }
    };
}

#[macro_export]
macro_rules! js_weak_value {
    ($ref_type: ty, $weak_type: ty) => {
        $crate::js_value!($weak_type);
        impl lento::js::ToJsValue for $ref_type {
            fn to_js_value(self) -> Result<lento::js::JsValue, quick_js::ValueError> {
                let weak = self.as_weak();
                Ok(lento::js::JsValue::Resource(quick_js::ResourceValue { resource: std::rc::Rc::new(std::cell::RefCell::new(weak)) }))
            }
        }

        impl lento::js::FromJsValue for $ref_type {
            fn from_js_value(value: lento::js::JsValue) -> Result<Self, quick_js::ValueError> {
                if let Some(r) = value.as_resource(|r: &mut $weak_type| r.clone()) {
                    if let Ok(r) = r.upgrade() {
                        Ok(r)
                    } else {
                        Err(quick_js::ValueError::Internal("failed to upgrade weak reference".to_string()))
                    }
                } else {
                    Err(quick_js::ValueError::UnexpectedType)
                }
            }
        }
    };
}

#[macro_export]
macro_rules! js_serialize {
    ($ty: ty) => {
        impl lento::js::ToJsValue for $ty {
            fn to_js_value(self) -> Result<lento::js::JsValue, quick_js::ValueError> {
                let serializer = lento::js::js_serde::JsValueSerializer {};
                let js_r = self.serialize(serializer).map_err(|e| quick_js::ValueError::Internal(e.to_string()))?;
                Ok(js_r)
            }
        }
    };
}

#[macro_export]
macro_rules! js_deserialize {
    ($ty: ty) => {

        impl lento::js::FromJsValue for $ty
        {
             fn from_js_value(value: quick_js::JsValue) -> Result<Self, quick_js::ValueError> {
                 //TODO no unwrap
                 Ok(Self::deserialize(lento::js::js_deserialze::JsDeserializer { value }).unwrap())
             }

        }
    };
}

#[macro_export]
macro_rules! js_element_event {
    ($event_type: expr, $ty: ty) => {
        impl $ty {
             pub fn emit(self, element: lento::element::Element) {
                let event = lento::base::Event::<ElementWeak>::new($event_type, self, element.as_weak());
                element.emit_event($event_type, event);
            }
            pub fn emit_weak(self, element: lento::element::ElementWeak) -> bool {
                if let Ok(el) = element.upgrade() {
                    self.emit(el);
                    true
                } else {
                    false
                }
            }
        }
    };
}

#[macro_export]
macro_rules! bind_js_event_listener {
    ($target: expr, $actual_type: expr, $listener: expr; $($event_type: expr => $listener_type: ty, )* ) => {
        match $actual_type {
            $(
                $event_type => {
                    use lento::js::FromJsValue;
                    $target.register_event_listener(<$listener_type>::from_js_value($listener)?)
                }
            )*
            _ => {
                return Err(JsError::from_str("unknown event_type"))
            }
        }
    };
}
