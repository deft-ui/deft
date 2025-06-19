use crate::js::js_deserialze::JsDeserializer;
use crate::js::js_runtime::{JsContext, JsValueView};
use crate::js::js_serde::JsValueSerializer;
use crate::js::js_value_util::JsValueHelper;
use crate::mrc::Mrc;
use quick_js::{JsValue, ValueError};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
#[derive(Clone, Debug)]
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
    E: Error + 'static,
{
    #[cold]
    fn from(error: E) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

#[derive(Debug)]
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
    fn call(
        &self,
        js_context: &mut Mrc<JsContext>,
        args: Vec<JsValue>,
    ) -> Result<JsValue, JsCallError>;
}

pub trait FromJsValue: Sized {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError>;
}

pub trait BorrowFromJs {
    fn borrow_from_js<R, F: FnOnce(&mut Self) -> R>(
        value: JsValue,
        receiver: F,
    ) -> Result<R, ValueError>;
}

impl<T: FromJsValue> BorrowFromJs for T {
    fn borrow_from_js<R, F: FnOnce(&mut Self) -> R>(
        value: JsValue,
        receiver: F,
    ) -> Result<R, ValueError> {
        let mut value = Self::from_js_value(value).unwrap();
        Ok(receiver(&mut value))
    }
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

macro_rules! impl_number_from_js_value {
    ($ty: ty) => {
        impl FromJsValue for $ty {
            fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
                value
                    .as_number()
                    .map(|f| f as $ty)
                    .ok_or(ValueError::Internal(format!(
                        "Cannot convert js:{} to rust:{}",
                        value.value_type(),
                        stringify!($ty)
                    )))
            }
        }
    };
}

impl_number_from_js_value!(u8);
impl_number_from_js_value!(u16);
impl_number_from_js_value!(u32);
impl_number_from_js_value!(u64);
impl_number_from_js_value!(usize);
impl_number_from_js_value!(i8);
impl_number_from_js_value!(i16);
impl_number_from_js_value!(i32);
impl_number_from_js_value!(i64);
impl_number_from_js_value!(isize);
impl_number_from_js_value!(f32);
impl_number_from_js_value!(f64);

impl FromJsValue for bool {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
        value.as_bool().ok_or(ValueError::UnexpectedType)
    }
}

impl FromJsValue for JsValue {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
        Ok(value)
    }
}

impl<T: FromJsValue> FromJsValue for Option<T> {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
        match value {
            JsValue::Undefined | JsValue::Null => Ok(None),
            v => Ok(Some(T::from_js_value(v)?)),
        }
    }
}

impl<T: FromJsValue> FromJsValue for Vec<T> {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
        match value {
            JsValue::Array(items) => {
                let mut result = Vec::with_capacity(items.len());
                for item in items {
                    result.push(FromJsValue::from_js_value(item)?);
                }
                Ok(result)
            }
            _ => Err(ValueError::UnexpectedType),
        }
    }
}

macro_rules! impl_tuple_from_js_value {
    ($($id: ident,)*) => {
        impl<$( $id : FromJsValue,)*> FromJsValue for ($($id,)*) {
            fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
                if let JsValue::Array(items) = value {
                    let mut iter = items.into_iter();
                    let result = (
                        $(
                          $id::from_js_value(iter.next().ok_or(ValueError::UnexpectedType)?)?,
                        )*
                    );
                    Ok(result)
                } else {
                    Err(ValueError::UnexpectedType)
                }
            }
        }
    };
}

impl_tuple_from_js_value!(A, B,);
impl_tuple_from_js_value!(A, B, C,);
impl_tuple_from_js_value!(A, B, C, D,);
impl_tuple_from_js_value!(A, B, C, D, E,);
impl_tuple_from_js_value!(A, B, C, D, E, F,);
impl_tuple_from_js_value!(A, B, C, D, E, F, G,);

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

macro_rules! impl_int_to_js_value {
    ($ty: ty) => {
        impl ToJsValue for $ty {
            fn to_js_value(self) -> Result<JsValue, ValueError> {
                Ok(JsValue::Int(self as i32))
            }
        }
    };
}

impl_int_to_js_value!(u8);
impl_int_to_js_value!(u16);
impl_int_to_js_value!(i8);
impl_int_to_js_value!(i16);
impl_int_to_js_value!(i32);

macro_rules! impl_float_to_js_value {
    ($ty: ty) => {
        impl ToJsValue for $ty {
            fn to_js_value(self) -> Result<JsValue, ValueError> {
                Ok(JsValue::Float(self as f64))
            }
        }
    };
}

impl_float_to_js_value!(u32);
impl_float_to_js_value!(u64);
impl_float_to_js_value!(usize);
impl_float_to_js_value!(i64);
impl_float_to_js_value!(f32);
impl_float_to_js_value!(f64);

impl ToJsValue for bool {
    fn to_js_value(self) -> Result<JsValue, ValueError> {
        Ok(JsValue::Bool(self))
    }
}

impl ToJsValue for JsValue {
    fn to_js_value(self) -> Result<JsValue, ValueError> {
        Ok(self)
    }
}

impl<T: ToJsValue> ToJsValue for Vec<T> {
    fn to_js_value(self) -> Result<JsValue, ValueError> {
        let mut values = Vec::new();
        for v in self {
            values.push(v.to_js_value()?);
        }
        Ok(JsValue::Array(values))
    }
}

impl<T: ToJsValue> ToJsValue for Option<T> {
    fn to_js_value(self) -> Result<JsValue, ValueError> {
        match self {
            None => Ok(JsValue::Undefined),
            Some(e) => Ok(e.to_js_value()?),
        }
    }
}

macro_rules! impl_tuple_to_js_value {
    ($($idx: tt => $id: ident,)*) => {
        impl<$( $id : ToJsValue,)*> ToJsValue for ($($id,)*) {
            fn to_js_value(self) -> Result<JsValue, ValueError> {
                let mut result = Vec::new();
                $(
                    result.push(self.$idx.to_js_value()?);
                )*
                Ok(JsValue::Array(result))
            }
        }
    };
}

impl_tuple_to_js_value!(
    0 => A,
    1 => B,
);
impl_tuple_to_js_value!(
    0 => A,
    1 => B,
    2 => C,
);
impl_tuple_to_js_value!(
    0 => A,
    1 => B,
    2 => C,
    3 => D,
);
impl_tuple_to_js_value!(
    0 => A,
    1 => B,
    2 => C,
    3 => D,
    4 => E,
);
impl_tuple_to_js_value!(
    0 => A,
    1 => B,
    2 => C,
    3 => D,
    4 => E,
    5 => F,
);
impl_tuple_to_js_value!(
    0 => A,
    1 => B,
    2 => C,
    3 => D,
    4 => E,
    5 => F,
    6 => G,
);

impl<T: ToJsValue> ToJsCallResult for T {
    fn to_js_call_result(self) -> Result<JsValue, JsCallError> {
        match self.to_js_value() {
            Ok(v) => Ok(v),
            Err(e) => Err(JsCallError::ConversionError(e)),
        }
    }
}

impl<T: ToJsValue, E: ToString> ToJsCallResult for Result<T, E> {
    fn to_js_call_result(self) -> Result<JsValue, JsCallError> {
        match self {
            Ok(v) => v.to_js_call_result(),
            Err(e) => {
                let e = JsError::from_str(&e.to_string());
                Err(JsCallError::ExecutionError(e))
            }
        }
    }
}

pub struct JsPo<T> {
    value: T,
}

impl<T> JsPo<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
    pub fn take(self) -> T {
        self.value
    }
}

impl<T> Deref for JsPo<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for JsPo<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> FromJsValue for JsPo<T>
where
    T: for<'a> Deserialize<'a>,
{
    fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
        match T::deserialize(JsDeserializer { value }) {
            Ok(v) => Ok(JsPo { value: v }),
            Err(e) => Err(ValueError::Internal(e.to_string())),
        }
    }
}

impl<T> ToJsValue for JsPo<T>
where
    T: Serialize,
{
    fn to_js_value(self) -> Result<JsValue, ValueError> {
        T::serialize(&self.value, JsValueSerializer {})
            .map_err(|e| ValueError::Internal(format!("Failed to serialize value: {:?}", e)))
    }
}

pub struct JsResource<T> {
    value: T,
}

impl<T: 'static> ToJsValue for JsResource<T> {
    fn to_js_value(self) -> Result<JsValue, ValueError> {
        Ok(JsValue::Resource(quick_js::ResourceValue {
            resource: std::rc::Rc::new(std::cell::RefCell::new(self.value)),
        }))
    }
}

impl<T: Clone + 'static> FromJsValue for JsResource<T> {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
        if let Some(value) = value.as_resource(|r: &mut T| r.clone()) {
            Ok(JsResource { value })
        } else {
            Err(ValueError::UnexpectedType)
        }
    }
}
