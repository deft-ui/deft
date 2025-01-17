use quick_js::loader::{JsModuleLoader};
use lento::app::LentoApp;
use lento::bootstrap;
use lento::loader::StaticModuleLoader;

struct HelloLentoApp {}

impl LentoApp for HelloLentoApp {
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static> {
        let mut ml = StaticModuleLoader::new();
        ml.add_module("index.js".to_string(), include_str!("hello.js").to_string());
        Box::new(ml)
    }
}

fn main() {
    let app = HelloLentoApp {};
    bootstrap(Box::new(app));
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: winit::platform::android::activity::AndroidApp) {
    let app = HelloLentoApp {};
    lento::android_bootstrap(android_app, Box::new(app));
}