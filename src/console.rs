#[cfg(windows)]
mod windows;

use quick_js::console::{ConsoleBackend, Level};
use quick_js::JsValue;

pub fn init_console() {
    #[cfg(windows)]
    windows::attach_console();
}

pub struct Console {}

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
