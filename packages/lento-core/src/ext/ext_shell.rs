use std::process::Command;
use anyhow::Error;

pub fn shell_spawn(cmd: String, args: Option<Vec<String>>) -> Result<(), Error> {
    let mut cmd = Command::new(cmd);
    if let Some(args) = &args {
        cmd.args(args);
    }
    //TODO return child?
    cmd.spawn()?;
    Ok(())
}