use anyhow::Error;
use serde::{Deserialize, Serialize};

pub struct HttpOptions {}

#[derive(Serialize, Deserialize)]
pub struct HttpResponse {
    status: u16,
    body: String,
}

pub async fn http_request(url: String) -> Result<HttpResponse, Error> {
    let rsp = reqwest::get(url).await?;
    let status = rsp.status().as_u16();
    let body = rsp.text().await?;
    Ok(HttpResponse {
        status,
        body,
    })
}