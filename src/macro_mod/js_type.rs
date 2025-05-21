
#[macro_export]
macro_rules! js_value {
    ($ref_type: ty) => {
        impl deft::js::ToJsValue for $ref_type {
            fn to_js_value(self) -> Result<deft::js::JsValue, deft::js::ValueError> {
                Ok(deft::js::JsValue::Resource(deft::js::ResourceValue { resource: std::rc::Rc::new(std::cell::RefCell::new(self)) }))
            }
        }

        impl deft::js::FromJsValue for $ref_type {
            fn from_js_value(value: deft::js::JsValue) -> Result<Self, deft::js::ValueError> {
                if let Some(r) = value.as_resource(|r: &mut $ref_type| r.clone()) {
                    Ok(r)
                } else {
                    Err(deft::js::ValueError::UnexpectedType)
                }
            }
        }
    };
}

#[macro_export]
macro_rules! js_weak_value {
    ($ref_type: ty, $weak_type: ty) => {
        $crate::js_value!($weak_type);
        impl deft::js::ToJsValue for $ref_type {
            fn to_js_value(self) -> Result<deft::js::JsValue, deft::js::ValueError> {
                let weak = self.as_weak();
                Ok(deft::js::JsValue::Resource(deft::js::ResourceValue { resource: std::rc::Rc::new(std::cell::RefCell::new(weak)) }))
            }
        }

        impl deft::js::FromJsValue for $ref_type {
            fn from_js_value(value: deft::js::JsValue) -> Result<Self, deft::js::ValueError> {
                if let Some(r) = value.as_resource(|r: &mut $weak_type| r.clone()) {
                    if let Ok(r) = r.upgrade() {
                        Ok(r)
                    } else {
                        Err(deft::js::ValueError::Internal("failed to upgrade weak reference".to_string()))
                    }
                } else {
                    Err(deft::js::ValueError::UnexpectedType)
                }
            }
        }
    };
}

/// Auto upgrade when convert to js value
#[macro_export]
macro_rules! js_auto_upgrade {
    ($weak_type: ty, $ref_type: ty) => {
        impl deft::js::ToJsValue for $weak_type {
            fn to_js_value(self) -> Result<deft::js::JsValue, deft::js::ValueError> {
                if let Ok(e) = self.upgrade() {
                    Ok(
                        deft::js::JsValue::Resource(
                            deft::js::ResourceValue { resource: std::rc::Rc::new(std::cell::RefCell::new(e)) }
                        )
                    )
                } else {
                    Err(deft::js::ValueError::Internal("failed to upgrade weak reference".to_string()))
                }
            }
        }
    };
}

#[macro_export]
macro_rules! js_serialize {
    ($ty: ty) => {
        impl deft::js::ToJsValue for $ty {
            fn to_js_value(self) -> Result<deft::js::JsValue, deft::js::ValueError> {
                let serializer = deft::js::js_serde::JsValueSerializer {};
                use serde::Serialize;
                let js_r = self.serialize(serializer).map_err(|e| deft::js::ValueError::Internal(e.to_string()))?;
                Ok(js_r)
            }
        }
    };
}

#[macro_export]
macro_rules! js_deserialize {
    ($ty: ty) => {

        impl deft::js::FromJsValue for $ty
        {
             fn from_js_value(value: deft::js::JsValue) -> Result<Self, deft::js::ValueError> {
                 use serde::Deserialize;
                 let v = Self::deserialize(deft::js::js_deserialze::JsDeserializer { value })
                    .map_err(|e| deft::js::ValueError::Internal(format!("Failed to deserialize js valued: {:?}", e)))?;
                 Ok(v)
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
                    use deft::js::FromJsValue;
                    Some($target.register_event_listener(<$listener_type>::from_js_value($listener)?))
                }
            )*
            _ => {
                None
            }
        }
    };
}
