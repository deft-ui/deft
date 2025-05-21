use crate as deft;
use anyhow::Error;
use deft_macros::js_methods;
use std::process::Command;

#[allow(nonstandard_style)]
pub struct shell;

#[js_methods]
impl shell {
    #[js_func]
    pub fn spawn(cmd: String, args: Option<Vec<String>>) -> Result<(), Error> {
        let mut cmd = Command::new(cmd);
        if let Some(args) = &args {
            cmd.args(args);
        }
        //TODO return child?
        cmd.spawn()?;
        Ok(())
    }
}
