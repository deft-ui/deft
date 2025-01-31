use deft::app::{App, IApp};
use deft::bootstrap;
use quick_js::loader::{FsJsModuleLoader, JsModuleLoader};

struct AppImpl {}

impl IApp for AppImpl {
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static> {
        let ml = FsJsModuleLoader::new(".");
        Box::new(ml)
    }
}

fn main() {
    let app = App::new(AppImpl {});
    bootstrap(app);
}
