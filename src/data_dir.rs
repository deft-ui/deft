use std::env;
use std::path::PathBuf;
use std::str::FromStr;

pub const ENV_KEY: &str = "APP_DATA_PATH";

pub fn get_data_path(name: &str) -> PathBuf {
    let data_root = if let Ok(v) = env::var(ENV_KEY) {
        PathBuf::from_str(&v).unwrap()
    } else {
        let my_path = env::current_exe().unwrap();
        my_path.parent().unwrap().parent().unwrap().join("data")
    };
    if name.is_empty() {
        data_root
    } else {
        data_root.join(name)
    }
}