use std::env;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use anyhow::Error;
use log::error;
use quick_js::{Context, ExecutionError, JsPromise, JsValue, ValueError};
use skia_safe::wrapper::NativeTransmutableWrapper;
use tokio::runtime::Runtime;
use winit::event_loop::EventLoopProxy;
use winit::window::CursorIcon;
use crate::app::{AppEvent};
use crate::base::UnsafeFnOnce;
use crate::js::js_value_util::JsValueHelper;
use crate::element::label::parse_align;
use crate::js::{JsError, ToJsCallResult};
use crate::js::js_event_loop::{js_create_event_loop_proxy, JsEvent, JsEventLoopProxy};
use crate::resource_table::ResourceTable;
use crate::text::TextAlign;

pub struct JsContext {
    context: Context,
    runtime: Runtime,
}

impl JsContext {
    pub fn new(context: Context, runtime: Runtime) -> Self {
        Self {
            context,
            runtime,
        }
    }

    pub fn create_promise(&mut self) -> (JsValue, PromiseResolver) {
        let promise = JsPromise::new(&mut self.context);
        let result = promise.js_value();
        let elp = js_create_event_loop_proxy();
        let resolver = PromiseResolver::new(promise, elp);
        (result, resolver)
    }

    pub fn create_async_task2<F, O>(&mut self, future: F) -> JsValue
    where
        F: Future<Output=O> + Send + 'static,
        O: ToJsCallResult,
    {
        let (result, resolver) = self.create_promise();
        self.runtime.spawn(async move {
            let res = future.await;
            match res.to_js_call_result() {
                Ok(r) => {resolver.resolve(r)}
                Err(e) => {resolver.reject(JsValue::String(format!("js call error:{:?}", e)))}
            }
        });
        result
    }

    pub fn execute_main(&mut self) {
        let module_name = env::var("DEFT_ENTRY").unwrap_or("index.js".to_string());
        self.context.execute_module(&module_name).unwrap();
    }

    pub fn execute_module(&mut self, module_name: &str) -> Result<(), ExecutionError> {
        self.context.execute_module(&module_name)
    }

    pub fn execute_pending_job(&self) -> Result<bool, ExecutionError> {
        self.context.execute_pending_job()
    }

}

impl Deref for JsContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl DerefMut for JsContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}


pub struct PromiseResolver {
    promise: Option<*mut JsPromise>,
    event_loop_proxy: JsEventLoopProxy,
}

impl PromiseResolver {
    pub fn new(promise: JsPromise, event_loop_proxy: JsEventLoopProxy) -> Self {
        Self {
            promise: Some(Box::into_raw(Box::new(promise))),
            event_loop_proxy,
        }
    }
    pub fn resolve(mut self, value: JsValue) {
        unsafe {
            let p = self.promise.take().unwrap();
            let callback = UnsafeFnOnce::new(move || {
                let mut promise = Box::from_raw(p);
                promise.resolve(value)
            });
            self.event_loop_proxy.schedule_macro_task(callback.into_box()).unwrap();
        }
    }

    pub fn settle(mut self, result: Result<JsValue, String>) {
        match result {
            Ok(v) => {
                self.resolve(v)
            }
            Err(e) => self.reject(JsValue::String(e)),
        }
    }

    pub fn reject(mut self, value: JsValue) {
        unsafe {
            let p = self.promise.take().unwrap();
            let callback = UnsafeFnOnce::new(move || {
                let mut promise = Box::from_raw(p);
                promise.reject(value)
            });
            self.event_loop_proxy.schedule_macro_task(callback.into_box()).unwrap();
        }
    }

}

unsafe impl Send for PromiseResolver {}

unsafe impl Sync for PromiseResolver {}

impl Drop for PromiseResolver {
    fn drop(&mut self) {
        if let Some(p) = self.promise {
            let mut callback = unsafe {
                UnsafeFnOnce::new(move || {
                    let _ = Box::from_raw(p);
                })
            };
            self.event_loop_proxy.schedule_macro_task(callback.into_box()).unwrap();
        }
    }
}

pub trait JsValueView {
    fn as_bool(&self) -> Option<bool>;

    fn as_number_array(&self) -> Option<Vec<f32>>;

}

impl JsValueView for JsValue {
    fn as_bool(&self) -> Option<bool> {
        match &self {
            JsValue::Bool(v) => {
                Some(*v)
            }
            _ => {
                None
            }
        }
    }

    fn as_number_array(&self) -> Option<Vec<f32>> {
        if let JsValue::Array(a) = self {
            let mut result = Vec::with_capacity(a.len());
            for e in a {
                result.push(e.as_number()? as f32);
            }
            Some(result)
        } else {
            None
        }
    }
}


pub trait FromJsValue: Sized {
    fn from_js_value(value: &JsValue) -> Option<Self>;
}

impl FromJsValue for f32 {
    fn from_js_value(value: &JsValue) -> Option<Self> {
        match value {
            JsValue::Int(i) => Some(*i as f32),
            JsValue::Float(f) => Some(*f as f32),
            _ => None
        }
    }
}

impl FromJsValue for String {
    fn from_js_value(value: &JsValue) -> Option<Self> {
        Some(value.as_str()?.to_string())
    }
}

impl FromJsValue for bool {
    fn from_js_value(value: &JsValue) -> Option<Self> {
        value.as_bool()
    }
}

impl FromJsValue for TextAlign {
    fn from_js_value(value: &JsValue) -> Option<Self> {
        if let JsValue::String(str) = value {
            Some(parse_align(str))
        } else {
            None
        }
    }
}

impl FromJsValue for usize {
    fn from_js_value(value: &JsValue) -> Option<Self> {
        if let JsValue::Int(i) = value {
            Some(*i as usize)
        } else {
            None
        }
    }
}

impl FromJsValue for Vec<usize> {
    fn from_js_value(value: &JsValue) -> Option<Self> {
        match value {
            JsValue::Array(a) => {
                let mut result = Vec::with_capacity(a.len());
                for e in a {
                    result.push(usize::from_js_value(e)?);
                }
                Some(result)
            },
            _ => {
                None
            }
        }

        //let arr = Vec::<usize>::from_js_value(value)?;

    }
}

impl FromJsValue for (usize, usize) {
    fn from_js_value(value: &JsValue) -> Option<Self> {
        let arr = Vec::<usize>::from_js_value(value)?;
        if arr.len() == 2 {
            let v = (
                *arr.get(0).unwrap(),
                *arr.get(1).unwrap()
            );
            Some(v)
        } else {
            None
        }
    }
}

impl crate::js::js_binding::FromJsValue for CursorIcon {
    fn from_js_value(value: JsValue) -> Result<Self, ValueError> {
        match value {
            JsValue::String(str) => CursorIcon::from_str(&str).map_err(|_e| ValueError::UnexpectedType),
            _ => Err(ValueError::UnexpectedType),
        }
    }
}

impl crate::js::js_binding::ToJsValue for CursorIcon {
    fn to_js_value(self) -> Result<JsValue, ValueError> {
        Ok(JsValue::String(self.name().to_string()))
    }
}
