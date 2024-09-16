use anyhow::Error;
use quick_js::{JsValue, ValueError};
use serde::{Deserialize, Serialize};
use crate::js::js_deserialze::JsDeserializer;
use crate::js::js_serde::JsValueSerializer;

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

pub trait ToJsValue {
    fn to_js_value(self) -> Result<JsValue, Error>;
}

pub trait SerializeResultToJsValue {
    fn to_js_value(self) -> Result<JsValue, Error>;
}

pub trait ResultToJsValue {
    fn to_js_value(self) -> Result<JsValue, Error>;
}


impl<F> SerializeToJsValue for F where F: Serialize {
    fn to_js_value(self) -> Result<JsValue, Error> {
        let serializer = JsValueSerializer {};
        let js_r = self.serialize(serializer)?;
        Ok(js_r)
    }
}

impl<F> SerializeResultToJsValue for Result<F, Error> where F: Serialize {
    fn to_js_value(self) -> Result<JsValue, Error> {
        match self {
            Ok(r) => {
                let serializer = JsValueSerializer {};
                let js_r = r.serialize(serializer)?;
                Ok(js_r)
            }
            Err(e) => Err(e)
        }

    }
}

impl ResultToJsValue for Result<JsValue, Error> {
    fn to_js_value(self) -> Result<JsValue, Error> {
        self
    }
}

impl ToJsValue for JsValue {
    fn to_js_value(self) -> Result<JsValue, Error> {
        Ok(self)
    }

}

pub trait DeserializeFromJsValue: Sized {
    fn from_js_value(value: JsValue) -> Result<Self, Error>;
}

pub trait FromJsValue: Sized {
    fn from_js_value(value: JsValue) -> Result<Self, Error>;
}

impl<F> DeserializeFromJsValue for F
where
    F: for <'a> Deserialize<'a>,
{
    fn from_js_value(value: JsValue) -> Result<Self, Error> {
        Ok(Self::deserialize(JsDeserializer { value })?)
    }
}

impl FromJsValue for JsValue {
    fn from_js_value(value: JsValue) -> Result<Self, Error> {
        Ok(value)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventResult {
    pub propagation_cancelled: bool,
    pub prevent_default: bool,
}

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