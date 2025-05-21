use crate as deft;
use anyhow::Error;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use deft_macros::js_methods;

#[allow(nonstandard_style)]
pub struct Base64;

#[js_methods]
impl Base64 {
    #[js_func]
    pub fn encode_str(value: String) -> Result<String, Error> {
        Ok(BASE64_STANDARD.encode(value.as_bytes()))
    }
}
