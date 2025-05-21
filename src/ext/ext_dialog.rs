use crate as deft;
use crate::js::js_event_loop::js_create_event_loop_fn_mut;
use crate::js::JsError;
use crate::js_deserialize;
use crate::window::Window;
use deft_macros::js_methods;
use native_dialog::FileDialog;
use quick_js::JsValue;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use serde::{Deserialize, Serialize};
use std::thread;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileDialogOptions {
    dialog_type: Option<String>,
}

js_deserialize!(FileDialogOptions);

struct DialogHandle(pub RawWindowHandle);

unsafe impl Send for DialogHandle {}

#[allow(nonstandard_style)]
pub struct dialog;

#[js_methods]
impl dialog {
    #[js_func]
    pub fn show_file_dialog(
        options: FileDialogOptions,
        window: Option<Window>,
        callback: JsValue,
    ) -> Result<(), JsError> {
        let mut owner = None;
        if let Some(window) = window {
            owner = Some(DialogHandle(window.window.raw_window_handle()?));
        }

        let mut success = {
            let callback = callback.clone();
            js_create_event_loop_fn_mut(move |path_str_list: Vec<String>| {
                let path_str_list = path_str_list
                    .into_iter()
                    .map(|it| JsValue::String(it))
                    .collect::<Vec<_>>();
                let _ = callback
                    .call_as_function(vec![JsValue::Bool(true), JsValue::Array(path_str_list)]);
            })
        };
        let mut fail = js_create_event_loop_fn_mut(move |error: String| {
            let _ = callback.call_as_function(vec![JsValue::Bool(false), JsValue::String(error)]);
        });

        thread::spawn(move || {
            let fd = FileDialog::new();
            if let Some(_owner) = owner {
                //TODO fix owner handle
                // unsafe {
                // fd = fd.set_owner_handle(owner.0);
                // }
            }
            let default_type = "single".to_string();
            let dialog_type = options.dialog_type.as_ref().unwrap_or(&default_type);
            let paths = match dialog_type.as_str() {
                "multiple" => fd.show_open_multiple_file().unwrap(),
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
                    let msg = format!("invalid dialog type:{}", dialog_type);
                    fail.call(msg);
                    return;
                }
            };
            let path_str_list = paths
                .iter()
                .map(|it| it.to_string_lossy().to_string())
                .collect();
            success.call(path_str_list);
        });
        Ok(())
    }
}
