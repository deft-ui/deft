use cfg_aliases::cfg_aliases;
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    cfg_aliases! {
        // Systems.
        android_platform: { target_os = "android" },
        web_platform: { all(target_family = "wasm", target_os = "unknown") },
        emscripten_platform: { all(target_family = "wasm", target_os = "emscripten") },
        macos_platform: { target_os = "macos" },
        ios_platform: { target_os = "ios" },
        windows_platform: { target_os = "windows" },
        linux_platform: { all(target_os = "linux", not(ohos)) },
        desktop_platform: { any(windows_platform, linux_platform, macos_platform) },
        apple: { any(target_os = "ios", target_os = "macos") },
        free_unix: { all(unix, not(apple), not(android_platform), not(target_os = "emscripten")) },
        redox: { target_os = "redox" },
        ohos: { target_env = "ohos" },

        // Native displays.
        x11_platform: { all(free_unix, not(redox), not(ohos)) },
        wayland_platform: { all(free_unix, not(redox), not(ohos)) },
        orbital_platform: { redox },
        // Systems
        mobile_platform: { any(target_os = "ios", target_os = "android") },

        // Available
        fs_enabled: { not(target_family = "wasm") }

    }

    #[cfg(target_env = "ohos")]
    napi_build_ohos::setup();
}
