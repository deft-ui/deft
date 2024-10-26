use std::env;
use anyhow::Error;

pub fn env_exe_dir() -> Result<String, Error> {
    let exe = env::current_exe()?;
    Ok(exe.parent().unwrap().to_string_lossy().to_string())
}

pub fn env_exe_path() -> Result<String, Error> {
    let exe = env::current_exe()?;
    Ok(exe.to_string_lossy().to_string())
}