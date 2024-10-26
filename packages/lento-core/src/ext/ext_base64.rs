use anyhow::Error;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;

pub fn base64_encode_str(value: String) -> Result<String, Error> {
    Ok(BASE64_STANDARD.encode(value.as_bytes()))
}