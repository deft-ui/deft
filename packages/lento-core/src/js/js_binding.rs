use std::error::Error;
use std::fmt::Display;
use quick_js::{JsValue, ValueError};
use crate::js::js_runtime::JsContext;
use crate::mrc::Mrc;

#[derive(Clone)]
pub struct JsError {
    message: String,
}

impl JsError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
    pub fn from_str(message: &str) -> Self {
        Self::new(message.to_string())
    }
}

impl<E> From<E> for JsError
where
    E: Error + Send + Sync + 'static,
{
    #[cold]
    fn from(error: E) -> Self {
        Self {
            message: error.to_string()
        }
    }
}

pub enum JsCallError {
    ConversionError(ValueError),
    ExecutionError(JsError),
}

impl From<ValueError> for JsCallError {
    fn from(value: ValueError) -> Self {
        Self::ConversionError(value)
    }
}

impl Display for JsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message.clone())
    }
}

pub trait JsFunc {
    fn name(&self) -> &str;
    fn args_count(&self) -> usize;
    fn call(&self, js_context: &mut Mrc<JsContext>, args: Vec<JsValue>) -> Result<JsValue, JsCallError>;
}

pub trait FromJsValue: Sized {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError>;
}


pub trait ToJsValue {
    fn to_js_value(self) -> Result<JsValue, ValueError>;
}

pub trait ToJsCallResult {
    fn to_js_call_result(self) -> Result<JsValue, JsCallError>;
}

impl FromJsValue for String {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
        match value {
            JsValue::String(s) => Ok(s),
            _ => Err(ValueError::UnexpectedType),
        }
    }
}

impl FromJsValue for JsValue {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
        Ok(value)
    }
}

impl ToJsValue for () {
    fn to_js_value(self) -> Result<JsValue, ValueError> {
        Ok(JsValue::Undefined)
    }
}

impl ToJsValue for String {
    fn to_js_value(self) -> Result<JsValue, ValueError> {
        Ok(JsValue::String(self))
    }
}

impl ToJsValue for JsValue {
    fn to_js_value(self) -> Result<JsValue, ValueError> {
        Ok(self)
    }
}

impl<T: ToJsValue> ToJsCallResult for T {
    fn to_js_call_result(self) -> Result<JsValue, JsCallError> {
        match self.to_js_value() {
            Ok(v) => { Ok(v) }
            Err(e) => { Err(JsCallError::ConversionError(e)) }
        }
    }
}

impl<T: ToJsValue> ToJsCallResult for Result<T, JsError> {
    fn to_js_call_result(self) -> Result<JsValue, JsCallError> {
        match self {
            Ok(v) => {
                v.to_js_call_result()
            }
            Err(e) => {
                Err(JsCallError::ExecutionError(e))
            }
        }
    }
}
