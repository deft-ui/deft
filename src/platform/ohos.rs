use std::sync::{Arc, LazyLock, Mutex};
use log::debug;
use ohos_ime_binding::{AttachOptions, IME};
use winit::event_loop::EventLoop;
use napi_derive_ohos::napi;
use ohos_hilog_binding::hilog_info;
use winit::platform::ohos::EventLoopExtOpenHarmony;
use crate::app::{AppEvent, AppEventPayload, WinitApp};
use crate::send_app_event;

static IME_INST: LazyLock<Arc<Mutex<IME>>> = LazyLock::new(|| {
    let mut ime = IME::new(AttachOptions::default());
    ime.insert_text(|input| {
        hilog_info!("ime input: {}", input);
        //TODO optimize window_id
        let window_id = 1;
        if let Err(e) = send_app_event(AppEvent::CommitInput(window_id, input)) {
            debug!("send app event error: {:?}", e);
        }
    });
    Arc::new(Mutex::new(ime))
});

pub fn show_soft_keyboard() {
    let ime = IME_INST.lock().unwrap();
    ime.show_keyboard();
}

pub fn hide_soft_keyboard() {
    let ime = IME_INST.lock().unwrap();
    ime.hide_keyboard();
}

pub fn run_app(event_loop: EventLoop<AppEventPayload>, app: WinitApp) {
    event_loop.spawn_app(app);
}

#[napi]
pub fn send_input(window_id: u32, input: String)  {
    ohos_hilog_binding::hilog_info!("js input: {}", input);
    if let Err(e) = send_app_event(AppEvent::CommitInput(window_id as i32, input)) {
        debug!("send app event error: {:?}", e);
    }
}