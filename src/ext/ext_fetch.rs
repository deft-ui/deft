use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::{anyhow, Error};
use quick_js::{JsValue, ResourceValue};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::js::js_value_util::{FromJsValue2, ToJsValue2};

#[derive(Clone)]
pub struct FetchResponse {
    response: Arc<Mutex<Response>>,
}

#[derive(Serialize, Deserialize)]
pub struct Header {
    pub name: String,
    pub value: String,
}

impl ToJsValue2 for FetchResponse {
    fn to_js_value(self) -> Result<JsValue, Error> {
        Ok(JsValue::Resource(ResourceValue {
            resource: Rc::new(RefCell::new(self)),
        }))
    }
}

impl FromJsValue2 for FetchResponse {
    fn from_js_value(value: JsValue) -> Result<Self, Error> {
        if let Some(r) = value.as_resource(|r: &mut FetchResponse| r.clone()) {
            Ok(r)
        } else {
            Err(anyhow!("invalid response"))
        }
    }
}

pub async fn fetch_create(url: String) -> Result<FetchResponse, Error> {
    let rsp = reqwest::get(url).await?;
    Ok(FetchResponse {
        response: Arc::new(Mutex::new(rsp)),
    })
}

pub async fn fetch_response_status(response: FetchResponse) -> Result<u16, Error> {
    let rsp = response.response.lock().await;
    Ok(rsp.status().as_u16())
}

pub async fn fetch_response_headers(response: FetchResponse) -> Result<Vec<Header>, Error> {
    let rsp = response.response.lock().await;
    let mut headers = Vec::new();
    rsp.headers().iter().for_each(|(k, v)| {
        if let Ok(v) = v.to_str() {
            let hd = Header {
                name: k.to_string(),
                value: v.to_string(),
            };
            headers.push(hd);
        }
    });
    Ok(headers)
}

pub async fn fetch_response_save(response: FetchResponse, path: String) -> Result<usize, Error> {
    let mut file = File::create_new(path).await?;
    let mut response = response.clone();
    let mut rsp = response.response.lock().await;
    let mut size = 0;
    while let Some(c) = rsp.chunk().await? {
        let data = c.to_vec();
        file.write_all(&data).await?;
        size += data.len();
    }
    Ok(size)
}
