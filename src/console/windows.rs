use std::env;
use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
pub fn attach_console() {
    let r = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
}
