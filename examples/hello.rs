use quick_js::loader::{JsModuleLoader};
use deft::app::{App, IApp};
use deft::bootstrap;
use deft::loader::StaticModuleLoader;

struct HelloAppImpl {}

impl IApp for HelloAppImpl {
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static> {
        let mut ml = StaticModuleLoader::new();
        ml.add_module("index.js".to_string(), include_str!("hello.js").to_string());
        Box::new(ml)
    }
}

fn main() {
    let app = App::new(HelloAppImpl {});
    bootstrap(app);
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: winit::platform::android::activity::AndroidApp) {
    let app = App::new(HelloAppImpl {});
    deft::android_bootstrap(android_app, app);
}