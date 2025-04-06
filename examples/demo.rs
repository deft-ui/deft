use std::env;
use deft::app::{App, IApp};
use deft::bootstrap;
use quick_js::loader::{FsJsModuleLoader, JsModuleLoader};
use deft::js::js_engine::JsEngine;

struct AppImpl {}

impl IApp for AppImpl {
    fn init_js_engine(&mut self, js_engine: &mut JsEngine) {
        js_engine.enable_localstorage(env::current_exe().unwrap().parent().unwrap().join("localstorage"));
    }
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static> {
        let ml = FsJsModuleLoader::new(".");
        Box::new(ml)
    }
}

fn main() {
    env_logger::init();
    let app = App::new(AppImpl {});
    bootstrap(app);
}
