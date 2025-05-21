use crate as deft;
use anyhow::Error;
use deft_macros::js_methods;
use std::env as std_env;

#[allow(nonstandard_style)]
pub struct env;

#[js_methods]
impl env {
    #[js_func]
    pub fn exe_dir() -> Result<String, Error> {
        let exe = std_env::current_exe()?;
        Ok(exe.parent().unwrap().to_string_lossy().to_string())
    }

    #[js_func]
    pub fn exe_path() -> Result<String, Error> {
        let exe = std_env::current_exe()?;
        Ok(exe.to_string_lossy().to_string())
    }
}
