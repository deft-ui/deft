use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
pub fn attach_console() {
    let _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
}
