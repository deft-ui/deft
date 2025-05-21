use crate as deft;
use quick_js::JsValue;
use serde::{Deserialize, Serialize};
use crate::js_deserialize;

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