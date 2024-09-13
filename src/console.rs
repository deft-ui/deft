use quick_js::console::{ConsoleBackend, Level};
use quick_js::JsValue;

pub struct Console {

}

impl Console {
    pub fn new() -> Self {
        Self {}
    }
}

impl ConsoleBackend for Console {
    fn log(&self, level: Level, values: Vec<JsValue>) {
        println!("{}:{:?}", level, values);
    }

}