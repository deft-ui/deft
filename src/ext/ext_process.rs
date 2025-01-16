use std::env;
use crate as lento;
use lento_macros::{js_func, js_methods};
use crate::app::exit_app;

#[allow(nonstandard_style)]
pub struct process;

#[js_methods]
impl process {

    #[js_func]
    pub fn exit(code: i32) {
        exit_app(code);
    }

    #[js_func]
    pub fn argv() -> Vec<String> {
        env::args().collect()
    }

}
