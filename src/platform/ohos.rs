use crate::app::{AppEvent, AppEventPayload, WinitApp};
use crate::{send_app_event, some_or_return};
use log::debug;
use napi_derive_ohos::napi;
use ohos_hilog_binding::hilog_info;
use ohos_ime_binding::{AttachOptions, IME};
use std::sync::{Arc, LazyLock, Mutex};
use winit::event_loop::EventLoop;
use winit::platform::ohos::EventLoopExtOpenHarmony;

static LAST_INPUT_WIN_ID: LazyLock<Arc<Mutex<Option<i32>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

fn set_last_focused_window_id(id: i32) {
    debug!("set_last_focused_window_id: {}", id);
    let mut locked = LAST_INPUT_WIN_ID.lock().unwrap();
    locked.replace(id);
}
fn get_last_focused_window_id() -> Option<i32> {
    let locked = LAST_INPUT_WIN_ID.lock().unwrap();
    locked.clone()
}

fn create_ime_instance() -> IME {
    let ime = IME::new(AttachOptions::default());
    ime.insert_text(|input| {
        let window_id = some_or_return!(get_last_focused_window_id());
        hilog_info!("ime input: {}, {}", window_id, input);
        if let Err(e) = send_app_event(AppEvent::CommitInput(window_id, input)) {
            debug!("send app event error: {:?}", e);
        }
    });
    ime.on_delete(|len| {
        let window_id = some_or_return!(get_last_focused_window_id());
        hilog_info!("ime delete: {}, {}", window_id, len);
        for _ in 0..len {
            if let Err(e) = send_app_event(AppEvent::NamedKeyInput(
                window_id,
                "Backspace".to_string(),
                true,
            )) {
                debug!("send app event error: {:?}", e);
            }
        }
    });
    ime
}

static IME_INST: LazyLock<Arc<Mutex<Option<IME>>>> = LazyLock::new(|| Arc::new(Mutex::new(None)));

pub fn resume_ime() {
    let mut ime = IME_INST.lock().unwrap();
    *ime = Some(create_ime_instance());
}

pub fn show_soft_keyboard(window_id: i32) {
    set_last_focused_window_id(window_id);
    let ime = IME_INST.lock().unwrap();
    if let Some(ime) = &*ime {
        ime.show_keyboard();
    }
}

pub fn hide_soft_keyboard(_window_id: i32) {
    let ime = IME_INST.lock().unwrap();
    if let Some(ime) = &*ime {
        ime.hide_keyboard();
    }
}

pub fn run_app(event_loop: EventLoop<AppEventPayload>, app: WinitApp) {
    event_loop.spawn_app(app);
}

#[napi]
pub fn send_input(window_id: u32, input: String) {
    ohos_hilog_binding::hilog_info!("js input: {}", input);
    if let Err(e) = send_app_event(AppEvent::CommitInput(window_id as i32, input)) {
        debug!("send app event error: {:?}", e);
    }
}
