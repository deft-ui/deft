use crate as deft;
use deft_macros::js_methods;

pub struct Console;

#[js_methods]
impl Console {

    #[js_func]
    pub fn print(text: String) {
        print!("{}", text);
        #[cfg(target_env = "ohos")]
        ohos_hilog_binding::hilog_info!("{}", text);
    }
}