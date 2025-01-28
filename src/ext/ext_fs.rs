use std::io;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::PathBuf;
use deft_macros::js_func;
use serde::{Deserialize, Serialize};
use tokio::fs;
use crate as deft;
use crate::js::JsPo;

#[derive(Serialize, Deserialize, Debug)]
pub struct Stat {
    size: u64,
    is_dir: bool,
    is_file: bool,
}

#[js_func]
pub async fn fs_read_dir(path: String) -> io::Result<Vec<String>> {
    let mut dirs = fs::read_dir(&path).await?;
    let mut result = Vec::new();
    while let Some(entry) = dirs.next_entry().await? {
        result.push(entry.file_name().to_string_lossy().to_string());
    }
    Ok(result)
}

#[js_func]
pub async fn fs_exists(path: String) -> io::Result<bool> {
    let path = PathBuf::from(path);
    Ok(path.exists())
}

#[js_func]
pub async fn fs_rename(path: String, dest: String) -> io::Result<()> {
    fs::rename(path, dest).await
}

#[js_func]
pub async fn fs_delete_file(path: String) -> io::Result<()> {
    let path = PathBuf::from(path);
    fs::remove_file(&path).await
}

#[js_func]
pub async fn fs_stat(path: String) -> io::Result<JsPo<Stat>> {
    let meta = fs::metadata(&path).await?;
    Ok(JsPo::new(Stat {
        #[cfg(unix)]
        size: meta.size(),
        #[cfg(windows)]
        size: meta.file_size(),
        is_dir: meta.is_dir(),
        is_file: meta.is_file(),
    }))
}

#[js_func]
pub async fn fs_create_dir(path: String) -> io::Result<()> {
    fs::create_dir(&path).await
}

#[js_func]
pub async fn fs_create_dir_all(path: String) -> io::Result<()> {
    fs::create_dir_all(&path).await
}

#[js_func]
pub async fn fs_remove_dir(path: String) -> io::Result<()> {
    fs::remove_dir(&path).await
}

#[js_func]
pub async fn fs_remove_dir_all(path: String) -> io::Result<()> {
    fs::remove_dir_all(&path).await
}