use quick_js::loader::{FsJsModuleLoader, JsModuleLoader};
use deft::app::DeftApp;
use deft::bootstrap;

struct DefaultDeftApp {}

impl DeftApp for DefaultDeftApp {
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static> {
        let ml = FsJsModuleLoader::new(".");
        Box::new(ml)
    }
}

fn main() {
    let app = DefaultDeftApp {};
    bootstrap(Box::new(app));
}