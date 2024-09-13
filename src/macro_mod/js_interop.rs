#[macro_export]
macro_rules! js_get_prop {
    ($key:expr, $self:ident, $method: ident, $name: expr) => {
        if $name == $key {
            use crate::js::js_serde::JsValueSerializer;
            let serializer = JsValueSerializer {};
            return Ok($self.$method().serialize(serializer)?);
        }
    };
}

#[macro_export]
macro_rules! js_call {
    ($key:expr, $ty: ident, $self:ident, $method: ident, $name: expr, $param: expr) => {
        {
            use crate::js::js_runtime::FromJsValue;
            if $key == $name {
                if let Some(v) = $ty::from_js_value(&$param) {
                    $self.$method(v);
                }
                return;
            }
        }

    };
    ($key:expr, $ty: ty, $self:ident, $method: ident, $name: expr, $param: expr) => {
        {
            use crate::js::js_runtime::FromJsValue;
            if $key == $name {
                if let Some(v) = <$ty>::from_js_value(&$param) {
                    $self.$method(v);
                }
                return;
            }
        }

    };
        ($key:expr, $ty: ident, $self:ident, $method: ident, $name: expr, $param: expr, $ret: expr) => {
        {
            use crate::js::js_runtime::FromJsValue;
            if $key == $name {
                if let Some(v) = $ty::from_js_value(&$param) {
                    $self.$method(v);
                }
                return $ret;
            }
        }

    };
}

#[macro_export]
macro_rules! js_call_rust {
    ($key:expr, $ty: ident, $self:ident, $method: ident, $name: expr, $param: expr) => {
        {
            use crate::JsDeserializer;
            use serde::Deserialize;
            let deserializer = JsDeserializer {value: $param.clone()};
            if $key == $name {
                if let Ok(v) = $ty::deserialize(deserializer) {
                    $self.$method(v);
                }
                return;
            }
        }

    };
    ($key:expr, $ty: ty, $self:ident, $method: ident, $name: expr, $param: expr) => {
        {
            use crate::JsDeserializer;
            use serde::Deserialize;
            let deserializer = JsDeserializer {value: $param.clone()};
            if $key == $name {
                if let Ok(v) = $ty::deserialize(deserializer) {
                    $self.$method(v);
                }
                return;
            }
        }

    };
        ($key:expr, $ty: ident, $self:ident, $method: ident, $name: expr, $param: expr, $ret: expr) => {
        {
            use crate::JsDeserializer;
            use serde::Deserialize;
            let deserializer = JsDeserializer {value: $param.clone()};
            if $key == $name {
                if let Ok(v) = $ty::deserialize(deserializer) {
                    $self.$method(v);
                }
                return $ret;
            }
        }

    };
}