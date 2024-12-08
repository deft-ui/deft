use crate as lento;
use anyhow::Error;
use quick_js::{JsValue, ValueError};
use serde::{Deserialize, Serialize};
use crate::js::js_deserialze::JsDeserializer;
use crate::js::js_serde::JsValueSerializer;
use crate::js_deserialize;

pub struct JsParam {
    pub value: JsValue
}

impl TryFrom<JsValue> for JsParam {
    type Error = ValueError;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        Ok(JsParam { value})
    }
}

pub trait SerializeToJsValue {
    fn to_js_value(self) -> Result<JsValue, Error>;
}


impl<F> SerializeToJsValue for F where F: Serialize {
    fn to_js_value(self) -> Result<JsValue, Error> {
        let serializer = JsValueSerializer {};
        let js_r = self.serialize(serializer)?;
        Ok(js_r)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventResult {
    pub propagation_cancelled: bool,
    pub prevent_default: bool,
}
js_deserialize!(EventResult);

pub trait JsValueHelper {
    fn as_number(&self) -> Option<f64>;
}

impl JsValueHelper for JsValue {
    fn as_number(&self) -> Option<f64> {
        match self {
            JsValue::Int(i) => Some(*i as f64),
            JsValue::Float(f) => Some(*f),
            _ => None
        }
    }
}