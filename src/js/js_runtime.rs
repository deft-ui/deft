use std::future::Future;
use std::ops::{Deref, DerefMut};
use anyhow::Error;
use quick_js::{Context, ExecutionError, JsPromise, JsValue};
use skia_safe::textlayout::TextAlign;
use tokio::runtime::Runtime;
use winit::event_loop::EventLoopProxy;
use winit::window::CursorIcon;
use crate::app::{AppEvent};
use crate::base::UnsafeFnOnce;
use crate::cursor::parse_cursor;
use crate::js::js_value_util::JsValueHelper;
use crate::element::label::parse_align;
use crate::event_loop::{get_event_proxy,run_on_event_loop};
use crate::resource_table::ResourceTable;

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
        let resolver = PromiseResolver::new(promise, get_event_proxy());
        (result, resolver)
    }

    pub fn create_async_task<F>(&mut self, future: F) -> JsValue
    where
        F: Future<Output=Result<JsValue, Error>> + Send + 'static,
    {
        let (result, resolver) = self.create_promise();
        self.runtime.spawn(async move {
            let result = future.await;
            match result {
                Ok(res) => resolver.resolve(res),
                Err(e) => {
                    println!("promise error:{}", e);
                    resolver.reject(JsValue::String(format!("promise error:{}", e)));
                }
            }
            // resolver.resolve(result)
        });
        result
    }

    pub fn execute_main(&mut self) {
        self.context.execute_module("index.js").unwrap();
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
    event_loop_proxy: EventLoopProxy<AppEvent>,
}

impl PromiseResolver {
    pub fn new(promise: JsPromise, event_loop_proxy: EventLoopProxy<AppEvent>) -> Self {
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
            self.event_loop_proxy.send_event(AppEvent::Callback(callback.into_box())).unwrap();
        }
    }

    pub fn reject(mut self, value: JsValue) {
        unsafe {
            let p = self.promise.take().unwrap();
            let callback = UnsafeFnOnce::new(move || {
                let mut promise = Box::from_raw(p);
                promise.reject(value)
            });
            self.event_loop_proxy.send_event(AppEvent::Callback(callback.into_box())).unwrap();
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
            run_on_event_loop(|| callback.call())
        }
    }
}

pub trait JsValueView {
    fn as_bool(&self) -> Option<bool>;

    fn get_object_property(&self, key: &str) -> Option<JsValue>;
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

    fn get_object_property(&self, key: &str) -> Option<JsValue> {
        //TODO optimize
        if let Some(obj) = self.get_properties() {
            obj.get(key).cloned()
        } else {
            None
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

impl FromJsValue for CursorIcon {
    fn from_js_value(value: &JsValue) -> Option<Self> {
        match value {
            JsValue::String(str) => parse_cursor(str),
            _ => None,
        }
    }
}
