use std::io;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::data_dir::get_data_path;

pub fn appfs_data_path(name: Option<String>) -> io::Result<String> {
    let p = if let Some(name) = name {
        get_data_path(&name)
    } else {
        get_data_path("")
    };
    Ok(p.to_string_lossy().to_string())
}

pub async fn appfs_exists(path: String) -> io::Result<bool> {
    let path = get_data_path(&path);
    Ok(path.exists())
}

pub async fn appfs_readdir(path: String) -> io::Result<Vec<String>> {
    let root = get_data_path(path.as_str());
    let mut dirs = fs::read_dir(&root).await?;
    let mut result = Vec::new();
    while let Some(d) = dirs.next_entry().await? {
        let p = d.file_name().to_string_lossy().to_string();
        result.push(p);
    }
    Ok(result)
}

pub async fn appfs_write(path: String, content: String) -> io::Result<()> {
    let path = get_data_path(&path);
    let mut file = File::create(&path).await?;
    file.write_all(content.as_bytes()).await
}

pub async fn appfs_write_new(path: String, content: String) -> io::Result<()> {
    let path = get_data_path(&path);
    let mut file = File::create_new(&path).await?;
    file.write_all(content.as_bytes()).await
}

pub async fn appfs_read(path: String) -> io::Result<String> {
    let path = get_data_path(&path);
    let mut file = File::open(&path).await?;
    let mut result = String::new();
    file.read_to_string(&mut result).await?;
    Ok(result)
}

pub async fn appfs_delete_file(path: String) -> io::Result<()> {
    let path = get_data_path(&path);
    fs::remove_file(&path).await
}

pub async fn appfs_create_dir(path: String) -> io::Result<()> {
    let path = get_data_path(&path);
    fs::create_dir(&path).await
}

pub async fn appfs_create_dir_all(path: String) -> io::Result<()> {
    let path = get_data_path(&path);
    fs::create_dir_all(&path).await
}

pub async fn appfs_remove_dir(path: String) -> io::Result<()> {
    let path = get_data_path(&path);
    fs::remove_dir(&path).await
}

pub async fn appfs_remove_dir_all(path: String) -> io::Result<()> {
    let path = get_data_path(&path);
    fs::remove_dir_all(&path).await
}