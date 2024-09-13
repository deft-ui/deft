use std::path::PathBuf;
use anyhow::Error;

pub fn path_filename(path: String) -> Result<Option<String>, Error> {
    let p = PathBuf::from(path);
    Ok(p.file_name().map(|n| n.to_string_lossy().to_string()))
}

pub fn path_join(path: String, other: String) -> Result<String, Error> {
    let p = PathBuf::from(path);
    Ok(p.join(other).to_string_lossy().to_string())
}