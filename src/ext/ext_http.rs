use crate as deft;
use std::collections::HashMap;
use std::str::FromStr;
use anyhow::Error;
use reqwest::{Body, multipart};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use deft_macros::{js_func, js_methods};
use crate::{js_deserialize, js_serialize};

pub struct HttpOptions {}

#[derive(Serialize, Deserialize)]
pub struct HttpResponse {
    status: u16,
    body: String,
}

#[derive(Serialize, Deserialize)]
pub struct UploadOptions {
    file: String,
    field: String,
    data: HashMap<String, String>,
    headers: HashMap<String, String>,
}

#[allow(nonstandard_style)]
pub struct http;

js_serialize!(HttpResponse);
js_deserialize!(UploadOptions);
#[js_methods]
impl http {

    #[js_func]
    pub async fn request(url: String) -> Result<HttpResponse, Error> {
        let rsp = reqwest::get(url).await?;
        let status = rsp.status().as_u16();
        let body = rsp.text().await?;
        Ok(HttpResponse {
            status,
            body,
        })
    }

    #[js_func]
    pub async fn upload(url: String, options: UploadOptions) -> Result<HttpResponse, Error> {
        let mut form = reqwest::multipart::Form::new();
        let file = File::open(options.file).await?;
        let stream = FramedRead::new(file, BytesCodec::new());
        let file_body = Body::wrap_stream(stream);
        let stream = multipart::Part::stream(file_body).file_name("test");
        let mut headers: HeaderMap = HeaderMap::new();
        for (k, v) in &options.headers {
            headers.insert(HeaderName::from_str(k)?, HeaderValue::from_str(v)?);
        }

        for (k, v) in options.data {
            form = form.text(k, v);
        }
        form = form.part(options.field.clone(), stream);

        let client = reqwest::Client::new();
        let rsp = client
            .post(url)
            .headers(headers)
            .multipart(form)
            .send().await?;
        let status = rsp.status().as_u16();
        let body = rsp.text().await?;
        Ok(HttpResponse {
            status,
            body,
        })
    }
}

