use crate as deft;
use std::thread;
use native_dialog::FileDialog;
use serde::{Deserialize, Serialize};
use deft_macros::{js_func, js_methods};
use quick_js::JsValue;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use crate::event_loop::create_event_loop_callback;
use crate::ext::promise::Promise;
use crate::frame::Frame;
use crate::js::js_event_loop::js_create_event_loop_fn_mut;
use crate::js_deserialize;

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
    pub fn show_file_dialog(options: FileDialogOptions, frame: Option<Frame>, callback: JsValue) {
        let mut owner = None;
        if let Some(frame) = frame {
            owner = Some(DialogHandle(frame.window.raw_window_handle()));
        }

        let mut success = {
            let callback = callback.clone();
            js_create_event_loop_fn_mut(move |path_str_list: Vec<String>| {
                let path_str_list = path_str_list.into_iter().map(|it| JsValue::String(it)).collect::<Vec<_>>();
                callback.call_as_function(vec![JsValue::Bool(true), JsValue::Array(path_str_list)]);
            })
        };
        let mut fail = js_create_event_loop_fn_mut(move |error: String| {
            callback.call_as_function(vec![JsValue::Bool(false), JsValue::String(error)]);
        });

        thread::spawn(move || {
            let mut fd = FileDialog::new();
            if let Some(owner) = owner {
                unsafe {
                    fd = fd.set_owner_handle(owner.0);
                }
            }
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
                    let msg = format!("invalid dialog type:{}", dialog_type);
                    fail.call(msg);
                    return;
                }
            };
            let path_str_list = paths.iter()
                .map(|it| it.to_string_lossy().to_string())
                .collect();
            success.call(path_str_list);
        });
    }
}

