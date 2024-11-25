pub use lento_core::*;
pub use lento_macros::*;

use crate::app::{App, AppEvent, LentoApp};
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;


fn run_event_loop(event_loop: EventLoop<AppEvent>, lento_app: Box<dyn LentoApp>) {
    let el_proxy = event_loop.create_proxy();
    let mut app = App::new(lento_app, el_proxy);
    event_loop.run_app(&mut app).unwrap();
}

pub fn bootstrap(lento_app: Box<dyn LentoApp>) {
    let event_loop: EventLoop<AppEvent> = EventLoop::with_user_event().build().unwrap();
    run_event_loop(event_loop, lento_app);
}

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_bootstrap(app: AndroidApp, lento_app: Box<dyn LentoApp>) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    // android_logger::init_once(android_logger::Config::default().with_min_level(log::Level::Debug));

    // info!("starting");
    if let Some(p) = app.internal_data_path() {
        let data_path = p.into_os_string().to_string_lossy().to_string();
        println!("internal data_path:{}", data_path);
        unsafe {
            env::set_var(data_dir::ENV_KEY, data_path);
        }
    }
    println!("data path: {:?}", data_dir::get_data_path(""));
    let event_loop = EventLoop::with_user_event().with_android_app(app).build().unwrap();
    run_event_loop(event_loop, lento_app);
}
