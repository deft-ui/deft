use std::fmt::format;

#[macro_export]
macro_rules! js_value {
    ($ref_type: ty) => {
        impl lento::js::ToJsValue for $ref_type {
            fn to_js_value(self) -> Result<lento::js::JsValue, lento::js::ValueError> {
                Ok(lento::js::JsValue::Resource(lento::js::ResourceValue { resource: std::rc::Rc::new(std::cell::RefCell::new(self)) }))
            }
        }

        impl lento::js::FromJsValue for $ref_type {
            fn from_js_value(value: lento::js::JsValue) -> Result<Self, lento::js::ValueError> {
                if let Some(r) = value.as_resource(|r: &mut $ref_type| r.clone()) {
                    Ok(r)
                } else {
                    Err(lento::js::ValueError::UnexpectedType)
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
            fn to_js_value(self) -> Result<lento::js::JsValue, lento::js::ValueError> {
                let weak = self.as_weak();
                Ok(lento::js::JsValue::Resource(lento::js::ResourceValue { resource: std::rc::Rc::new(std::cell::RefCell::new(weak)) }))
            }
        }

        impl lento::js::FromJsValue for $ref_type {
            fn from_js_value(value: lento::js::JsValue) -> Result<Self, lento::js::ValueError> {
                if let Some(r) = value.as_resource(|r: &mut $weak_type| r.clone()) {
                    if let Ok(r) = r.upgrade() {
                        Ok(r)
                    } else {
                        Err(lento::js::ValueError::Internal("failed to upgrade weak reference".to_string()))
                    }
                } else {
                    Err(lento::js::ValueError::UnexpectedType)
                }
            }
        }
    };
}

/// Auto upgrade when convert to js value
#[macro_export]
macro_rules! js_auto_upgrade {
    ($weak_type: ty, $ref_type: ty) => {
        impl lento::js::ToJsValue for $weak_type {
            fn to_js_value(self) -> Result<lento::js::JsValue, lento::js::ValueError> {
                if let Ok(e) = self.upgrade() {
                    Ok(
                        lento::js::JsValue::Resource(
                            lento::js::ResourceValue { resource: std::rc::Rc::new(std::cell::RefCell::new(e)) }
                        )
                    )
                } else {
                    Err(lento::js::ValueError::Internal("failed to upgrade weak reference".to_string()))
                }
            }
        }
    };
}

#[macro_export]
macro_rules! js_serialize {
    ($ty: ty) => {
        impl lento::js::ToJsValue for $ty {
            fn to_js_value(self) -> Result<lento::js::JsValue, lento::js::ValueError> {
                let serializer = lento::js::js_serde::JsValueSerializer {};
                use serde::Serialize;
                let js_r = self.serialize(serializer).map_err(|e| lento::js::ValueError::Internal(e.to_string()))?;
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
             fn from_js_value(value: lento::js::JsValue) -> Result<Self, lento::js::ValueError> {
                 //TODO no unwrap
                 use serde::Deserialize;
                 Ok(Self::deserialize(lento::js::js_deserialze::JsDeserializer { value }).unwrap())
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
                return Err(JsError::new(format!("unknown event_type:{}", $actual_type)))
            }
        }
    };
}
