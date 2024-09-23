use std::fmt::Debug;

use anyhow::anyhow;
use quick_js::{Context, JsValue};
use quick_js::loader::JsModuleLoader;
use serde::Serialize;
use tokio::runtime::Builder;
use winit::event::WindowEvent;
use winit::window::WindowId;

use crate::{export_js_api, export_js_async_api, export_js_object_api};
use crate::app::exit_app;
use crate::console::Console;
use crate::element::{element_create, ElementRef};
use crate::ext::ext_appfs::{appfs_create_dir, appfs_create_dir_all, appfs_data_path, appfs_delete_file, appfs_exists, appfs_read, appfs_readdir, appfs_remove_dir, appfs_remove_dir_all, appfs_write, appfs_write_new};
use crate::ext::ext_audio::{audio_add_event_listener, audio_create, audio_stop, audio_remove_event_listener, AudioResource, audio_play, audio_pause, AudioOptions};
use crate::ext::ext_base64::base64_encode_str;
use crate::ext::ext_env::{env_exe_dir, env_exe_path};
use crate::ext::ext_fetch::{fetch_create, fetch_response_body_string, fetch_response_headers, fetch_response_save, fetch_response_status, FetchResponse};
use crate::ext::ext_frame::{create_frame, frame_close, frame_set_modal, FrameAttrs, handle_window_event};
use crate::ext::ext_fs::{fs_create_dir, fs_create_dir_all, fs_delete_file, fs_exists, fs_read_dir, fs_remove_dir, fs_remove_dir_all, fs_rename, fs_stat};
use crate::ext::ext_http::{http_request, http_upload, UploadOptions};
use crate::ext::ext_localstorage::{localstorage_get, localstorage_set};
use crate::ext::ext_path::{path_filename, path_join};
use crate::ext::ext_shell::shell_spawn;
use crate::ext::ext_timer::{timer_clear_interval, timer_clear_timeout, timer_set_interval, timer_set_timeout};
#[cfg(feature = "tray")]
use crate::ext::ext_tray::{SystemTrayResource, tray_create, TrayMenu};
use crate::ext::ext_websocket::{WsConnectionResource, ws_connect, ws_read};
use crate::frame::FrameWeak;
use crate::js::js_runtime::JsContext;
use crate::js::js_serde::JsValueSerializer;
use crate::js::js_value_util::DeserializeFromJsValue;
use crate::mrc::Mrc;

pub struct JsEngine {
    js_context: Mrc<JsContext>,
}

impl JsEngine {

    pub fn new<L: JsModuleLoader>(loader: L) -> Self {
        let runtime = Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();
        let js_context = Context::builder()
            .console(Console::new())
            .module_loader(loader)
            .build().unwrap();
        let js_context = Mrc::new(JsContext::new(js_context, runtime));

        js_context.add_callback("console_print", move |str: String| {
            print!("{}", str);
            0
        }).unwrap();

        // frame
        export_js_api!(js_context, "frame_create", create_frame, FrameAttrs);
        export_js_api!(js_context, "frame_set_modal", frame_set_modal, FrameWeak, FrameWeak);
        export_js_api!(js_context, "frame_close", frame_close, FrameWeak);
        export_js_object_api!(js_context, "frame_set_body", FrameWeak, set_body, ElementRef);
        export_js_object_api!(js_context, "frame_set_title", FrameWeak, set_title, String);
        export_js_object_api!(js_context, "frame_bind_event", FrameWeak, bind_event, String, JsValue);
        export_js_object_api!(js_context, "frame_set_visible", FrameWeak, set_visible, bool);
        export_js_object_api!(js_context, "frame_remove_event_listener", FrameWeak, remove_event_listener, String, u32);

        // view
        export_js_api!(js_context, "view_create", element_create, i32);
        export_js_object_api!(js_context, "view_set_property",  ElementRef, set_property, String, JsValue);
        export_js_object_api!(js_context, "view_get_property", ElementRef, get_property, String);
        export_js_object_api!(js_context, "view_add_child", ElementRef, add_child, ElementRef, i32);
        export_js_object_api!(js_context, "view_remove_child", ElementRef, remove_child, u32);
        export_js_object_api!(js_context, "view_set_style", ElementRef, set_style, JsValue);
        export_js_object_api!(js_context, "view_set_hover_style", ElementRef, set_hover_style, JsValue);
        export_js_object_api!(js_context, "view_bind_event",ElementRef, bind_event, String, JsValue);
        export_js_object_api!(js_context, "view_remove_event_listener",ElementRef, remove_event_listener, String, u32);

        //timer
        export_js_api!(js_context, "setTimeout", timer_set_timeout, JsValue, Option<i32>);
        export_js_api!(js_context, "clearTimeout", timer_clear_timeout, i32);
        export_js_api!(js_context, "setInterval", timer_set_interval, JsValue, i32);
        export_js_api!(js_context, "clearInterval", timer_clear_interval, i32);

        // fs
        export_js_async_api!(js_context, "fs_read_dir", fs_read_dir, String);
        export_js_async_api!(js_context, "fs_stat", fs_stat, String);
        export_js_async_api!(js_context, "fs_exists", fs_exists, String);
        export_js_async_api!(js_context, "fs_rename", fs_rename, String, String);
        export_js_async_api!(js_context, "fs_delete_file", fs_delete_file, String);
        export_js_async_api!(js_context, "fs_create_dir", fs_create_dir, String);
        export_js_async_api!(js_context, "fs_create_dir_all", fs_create_dir_all, String);
        export_js_async_api!(js_context, "fs_remove_dir", fs_remove_dir, String);
        export_js_async_api!(js_context, "fs_remove_dir_all", fs_remove_dir_all, String);

        // http
        export_js_async_api!(js_context, "http_request", http_request, String);
        export_js_async_api!(js_context, "http_upload", http_upload, String, UploadOptions);
        export_js_async_api!(js_context, "fetch_create", fetch_create, String);
        export_js_async_api!(js_context, "fetch_response_status", fetch_response_status, FetchResponse);
        export_js_async_api!(js_context, "fetch_response_headers", fetch_response_headers, FetchResponse);
        export_js_async_api!(js_context, "fetch_response_body_string", fetch_response_body_string, FetchResponse);
        export_js_async_api!(js_context, "fetch_response_save", fetch_response_save, FetchResponse, String);


        // websocket
        export_js_async_api!(js_context, "ws_connect", ws_connect, String);
        export_js_async_api!(js_context, "ws_read", ws_read, WsConnectionResource);

        // tray
        #[cfg(feature = "tray")]
        {
            export_js_api!(js_context, "tray_create",     tray_create, String);
            export_js_object_api!(js_context, "tray_set_menus",  SystemTrayResource, set_menus, Vec::<TrayMenu>);
            export_js_object_api!(js_context, "tray_set_icon",   SystemTrayResource, set_icon, String);
            export_js_object_api!(js_context, "tray_set_title",  SystemTrayResource, set_title, String);
            export_js_object_api!(js_context, "tray_remove_event_listener", SystemTrayResource, remove_event_listener, String, i32);
            export_js_object_api!(js_context, "tray_bind_event", SystemTrayResource, bind_event, String, JsValue);
        }

        // env
        export_js_api!(js_context, "env_exe_dir", env_exe_dir);
        export_js_api!(js_context, "env_exe_path", env_exe_path);

        // appfs
        export_js_api!(js_context, "appfs_data_path", appfs_data_path, Option<String>);
        export_js_async_api!(js_context, "appfs_exists", appfs_exists, String);
        export_js_async_api!(js_context, "appfs_readdir", appfs_readdir, String);
        export_js_async_api!(js_context, "appfs_read", appfs_read, String);
        export_js_async_api!(js_context, "appfs_write", appfs_write, String, String);
        export_js_async_api!(js_context, "appfs_write_new", appfs_write_new, String, String);
        export_js_async_api!(js_context, "appfs_delete_file", appfs_delete_file, String);
        export_js_async_api!(js_context, "appfs_create_dir", appfs_create_dir, String);
        export_js_async_api!(js_context, "appfs_create_dir_all", appfs_create_dir_all, String);
        export_js_async_api!(js_context, "appfs_remove_dir", appfs_remove_dir, String);
        export_js_async_api!(js_context, "appfs_remove_dir_all", appfs_remove_dir_all, String);

        // localstorage
        export_js_api!(js_context, "localstorage_get", localstorage_get, String);
        export_js_api!(js_context, "localstorage_set", localstorage_set, String, String);

        // path
        export_js_api!(js_context, "path_filename", path_filename, String);
        export_js_api!(js_context, "path_join", path_join, String, String);

        //audio
        export_js_api!(js_context, "audio_create", audio_create, AudioOptions);
        export_js_api!(js_context, "audio_play", audio_play, AudioResource);
        export_js_api!(js_context, "audio_pause", audio_pause, AudioResource);
        export_js_api!(js_context, "audio_stop", audio_stop, AudioResource);
        export_js_api!(js_context, "audio_add_event_listener", audio_add_event_listener, AudioResource, String, JsValue);
        export_js_api!(js_context, "audio_remove_event_listener", audio_remove_event_listener, AudioResource, String, u32);

        // base64
        export_js_api!(js_context, "base64_encode_str", base64_encode_str, String);

        // shell
        export_js_api!(js_context, "shell_spawn", shell_spawn, String, Option::<Vec<String>>);

        //process
        export_js_api!(js_context, "process_exit", exit_app, i32);

        let libjs = String::from_utf8_lossy(include_bytes!("../../lib.js"));
        js_context.eval_module(&libjs, "lib.js").unwrap();

        Self {
            js_context,
        }
    }

    pub fn execute_main(&mut self) {
        self.js_context.execute_main();
    }

    pub fn handle_window_event(&mut self, window_id: WindowId, event: WindowEvent) {
        handle_window_event(window_id, event);
    }

    pub fn execute_pending_jobs(&self) {
        let jc = self.js_context.clone();
        loop {
            let job_res = jc.execute_pending_job();
            match job_res {
                Ok(res) => {
                    if !res {
                        break;
                    }
                }
                Err(e) => {
                    eprint!("job error:{:?}", e);
                    break;
                }
            }
        }
    }

}