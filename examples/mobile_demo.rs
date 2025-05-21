use deft::app::{App, IApp};
use deft::bootstrap;
use deft::js::js_engine::JsEngine;
use deft::loader::StaticModuleLoader;
use quick_js::loader::JsModuleLoader;

struct AppImpl {}

impl IApp for AppImpl {
    fn init_js_engine(&mut self, _js_engine: &mut JsEngine) {
        // js_engine.enable_localstorage(env::current_exe().unwrap().parent().unwrap().join("localstorage"));
    }
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static> {
        let mut ml = StaticModuleLoader::new();
        ml.add_module(
            "index.js".to_string(),
            include_str!("./demo-js/index.js").to_string(),
        );
        ml.add_module(
            "worker-index.js".to_string(),
            include_str!("./demo-js/worker-index.js").to_string(),
        );
        Box::new(ml)
    }
}

fn main() {
    let app = App::new(AppImpl {});
    bootstrap(app);
}

#[cfg(target_env = "ohos")]
#[openharmony_ability_derive::ability]
pub fn openharmony(openharmony_app: winit::platform::ohos::ability::OpenHarmonyApp) {
    let _ = deft_ohos_logger::init();
    let app = App::new(AppImpl {});
    deft::ohos_bootstrap(openharmony_app, app);
}
