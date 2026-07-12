use crate as deft;
use deft_macros::js_methods;
use crate::js_module;

pub struct Console;

js_module!(Console, include_str!("./console.js"));

#[js_methods]
impl Console {
    #[js_func]
    pub fn print(text: String) {
        print!("{}", text);
        #[cfg(target_env = "ohos")]
        ohos_hilog_binding::hilog_info!("{}", text);
    }
}
