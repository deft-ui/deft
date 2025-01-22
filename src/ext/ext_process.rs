use std::env;
use crate as lento;
use lento_macros::{js_func, js_methods};
use crate::app::exit_app;
use crate::is_mobile_platform;

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

    #[js_func]
    pub fn is_mobile_platform() -> bool {
        is_mobile_platform()
    }

}
