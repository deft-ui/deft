use crate as lento;
use std::thread;
use native_dialog::FileDialog;
use serde::{Deserialize, Serialize};
use lento_macros::{js_func, js_methods};
use crate::ext::promise::Promise;
use crate::js_deserialize;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileDialogOptions {
    dialog_type: Option<String>,
}

js_deserialize!(FileDialogOptions);

#[allow(nonstandard_style)]
pub struct dialog;

#[js_methods]
impl dialog {

    #[js_func]
    pub async fn show_file_dialog(options: FileDialogOptions) -> Result<Vec<String>, String> {
        let promise = Promise::new();
        let p = promise.clone();
        thread::spawn(move || {
            let fd = FileDialog::new();
            let default_type = "single".to_string();
            let dialog_type = options.dialog_type.as_ref().unwrap_or(&default_type);
            let paths = match dialog_type.as_str() {
                "multiple" => {
                    fd.show_open_multiple_file().unwrap()
                }
                "single" => {
                    if let Some(f) = fd.show_open_single_file().unwrap() {
                        vec![f]
                    } else {
                        vec![]
                    }
                }
                "save" => {
                    if let Some(f) = fd.show_save_single_file().unwrap() {
                        vec![f]
                    } else {
                        vec![]
                    }
                }
                "dir" => {
                    if let Some(f) = fd.show_open_single_dir().unwrap() {
                        vec![f]
                    } else {
                        vec![]
                    }
                }
                _ => {
                    p.reject(format!("invalid dialog type:{}", dialog_type));
                    return;
                }
            };
            let path_str_list = paths.iter()
                .map(|it| it.to_string_lossy().to_string())
                .collect();
            p.resolve(path_str_list);
        });
        promise.await
    }
}

