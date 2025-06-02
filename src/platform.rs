#[cfg(ohos)]
mod ohos;
#[cfg(ohos)]
pub use ohos::*;

#[cfg(android_platform)]
mod android;
#[cfg(android_platform)]
pub use android::*;

#[cfg(linux_platform)]
mod linux;
#[cfg(linux_platform)]
pub use linux::*;

#[cfg(windows_platform)]
mod windows;
#[cfg(windows_platform)]
pub use windows::*;

#[cfg(macos_platform)]
mod macos;
#[cfg(macos_platform)]
pub use macos::*;

#[cfg(ios_platform)]
mod ios;
#[cfg(ios_platform)]
pub use ios::*;
