use crate as lento;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Error};
use quick_js::{JsValue, ValueError};
use reqwest::{Method, Response};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use lento_macros::{js_methods};
use crate::{js_serialize, js_value};
use crate::js::js_serde::JsValueSerializer;
use crate::js::JsPo;

#[derive(Clone)]
pub struct FetchResponse {
    response: Arc<Mutex<Response>>,
}

js_value!(FetchResponse);

#[derive(Serialize, Deserialize)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct FetchOptions {
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
    pub proxy: Option<String>,
}

#[allow(nonstandard_style)]
pub struct fetch;

js_serialize!(Header);


#[js_methods]
impl fetch {

    #[js_func]
    pub async fn create(url: String, options: Option<JsPo<FetchOptions>>) -> Result<FetchResponse, Error> {
        let mut client_builder = reqwest::Client::builder();
        let mut method = Method::GET;
        let mut headers = HeaderMap::new();
        let mut body = None;
        if let Some(options) = &options {
            if let Some(proxy) = &options.proxy {
                if proxy.is_empty() {
                    client_builder = client_builder.no_proxy();
                } else {
                    client_builder = client_builder.proxy(reqwest::Proxy::all(proxy)?);
                }
            }
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
            body = options.body.clone();
        }
        let mut client = client_builder.build()?;
        let mut req_builder = client
            .request(method, url)
            .headers(headers);
        if let Some(body) = body {
            req_builder = req_builder.body(body);
        }
        let rsp = req_builder.send().await?;
        Ok(FetchResponse {
            response: Arc::new(Mutex::new(rsp)),
        })
    }

    #[js_func]
    pub async fn response_status(response: FetchResponse) -> Result<u16, Error> {
        let rsp = response.response.lock().await;
        Ok(rsp.status().as_u16())
    }

    #[js_func]
    pub async fn response_headers(response: FetchResponse) -> Result<Vec<Header>, Error> {
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

    #[js_func]
    pub async fn response_body_string(response: FetchResponse) -> Result<String, Error> {
        let mut rsp = response.response.lock().await;
        let mut result = Vec::new();
        while let Some(c) = rsp.chunk().await? {
            let mut data = c.to_vec();
            result.append(&mut data);
        }
        Ok(String::from_utf8(result)?)
    }

    #[js_func]
    pub async fn response_save(response: FetchResponse, path: String) -> Result<usize, Error> {
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
}


