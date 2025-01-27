use quick_js::loader::{JsModuleLoader};
use deft::app::DeftApp;
use deft::bootstrap;
use deft::loader::StaticModuleLoader;

struct HelloDeftApp {}

impl DeftApp for HelloDeftApp {
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static> {
        let mut ml = StaticModuleLoader::new();
        ml.add_module("index.js".to_string(), include_str!("hello.js").to_string());
        Box::new(ml)
    }
}

fn main() {
    let app = HelloDeftApp {};
    bootstrap(Box::new(app));
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: winit::platform::android::activity::AndroidApp) {
    let app = HelloDeftApp {};
    deft::android_bootstrap(android_app, Box::new(app));
}