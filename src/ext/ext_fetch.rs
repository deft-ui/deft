use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Error};
use quick_js::{JsValue, ResourceValue};
use reqwest::{Method, Response};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use crate::define_resource;

use crate::js::js_value_util::{FromJsValue, ToJsValue};

#[derive(Clone)]
pub struct FetchResponse {
    response: Arc<Mutex<Response>>,
}

define_resource!(FetchResponse);

#[derive(Serialize, Deserialize)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct FetchOptions {
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}

pub async fn fetch_create(url: String, options: Option<FetchOptions>) -> Result<FetchResponse, Error> {
    let mut client = reqwest::Client::new();
    let mut method = Method::GET;
    let mut headers = HeaderMap::new();
    if let Some(options) = &options {
        if let Some(m) = &options.method {
            method = match m.to_lowercase().as_str() {
                "get" => Method::GET,
                "post" => Method::POST,
                "put" => Method::PUT,
                "delete" => Method::DELETE,
                "head" => Method::HEAD,
                "options" => Method::OPTIONS,
                m => return Err(anyhow!("invalid method: {}", m)),
            };
        }
        if let Some(hds) = &options.headers {
            for (k, v) in hds {
                headers.insert(HeaderName::from_str(k)?, HeaderValue::from_str(v)?);
            }
        }
    }
    let rsp = client
        .request(method, url)
        .headers(headers)
        .send()
        .await?;
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

pub async fn fetch_response_body_string(response: FetchResponse) -> Result<String, Error> {
    let mut rsp = response.response.lock().await;
    let mut result = Vec::new();
    while let Some(c) = rsp.chunk().await? {
        let mut data = c.to_vec();
        result.append(&mut data);
    }
    Ok(String::from_utf8(result)?)
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
