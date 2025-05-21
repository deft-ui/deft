use crate as deft;
use std::path::PathBuf;
use anyhow::Error;
use deft_macros::js_methods;

#[allow(nonstandard_style)]
pub struct path;

#[js_methods]
impl path {

    #[js_func]
    pub fn filename(p: String) -> Result<Option<String>, Error> {
        let p = PathBuf::from(p);
        Ok(p.file_name().map(|n| n.to_string_lossy().to_string()))
    }

    #[js_func]
    pub fn join(p: String, other: String) -> Result<String, Error> {
        let p = PathBuf::from(p);
        Ok(p.join(other).to_string_lossy().to_string())
    }
}
