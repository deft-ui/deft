#![windows_subsystem = "windows"]
use deft::app::{App, IApp};
use deft::bootstrap;
use deft::js::js_engine::JsEngine;
use quick_js::loader::JsModuleLoader;

struct AppImpl {}

impl IApp for AppImpl {
    fn init_js_engine(&mut self, _js_engine: &mut JsEngine) {
        // js_engine.enable_localstorage(env::current_exe().unwrap().parent().unwrap().join("localstorage"));
    }
    #[cfg(desktop_platform)]
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static> {
        use quick_js::loader::FsJsModuleLoader;
        let ml = FsJsModuleLoader::new("examples/demo-js");
        Box::new(ml)
    }

    #[cfg(not(desktop_platform))]
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static> {
        use deft::loader::StaticModuleLoader;
        let mut ml = StaticModuleLoader::new();
        ml.add_module(
            "index.js".to_string(),
            include_str!("./demo-js/index.js").to_owned(),
        );
        Box::new(ml)
    }
}

fn bootstrap_app() {
    let app = App::new(AppImpl {});
    bootstrap(app);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();
    bootstrap_app();
}

#[cfg(target_os = "emscripten")]
pub fn main() {
    // Do nothing
}

#[cfg(target_os = "emscripten")]
#[no_mangle]
pub extern "C" fn asm_main() {
    use deft::log::SimpleLogger;
    SimpleLogger::init_with_max_level(log::LevelFilter::Info);
    bootstrap_app();
}
