use quick_js::loader::{FsJsModuleLoader, JsModuleLoader};
use lento::app::LentoApp;
use lento::bootstrap;

struct DefaultLentoApp {}

impl LentoApp for DefaultLentoApp {
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static> {
        let ml = FsJsModuleLoader::new(".");
        Box::new(ml)
    }
}

fn main() {
    let app = DefaultLentoApp {};
    bootstrap(Box::new(app));
}