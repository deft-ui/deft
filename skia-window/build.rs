use cfg_aliases::cfg_aliases;

fn main() {
    // XXX this is taken from glutin/build.rs.

    // Setup alias to reduce `cfg` boilerplate.
    cfg_aliases! {
        // Systems.
        android: { target_os = "android" },
        wasm: { target_arch = "wasm32" },
        macos: { target_os = "macos" },
        ios: { target_os = "ios" },
        apple: { any(target_os = "ios", target_os = "macos") },
        free_unix: { all(unix, not(apple), not(android)) },
        ohos: { target_env = "ohos" },

        // Native displays.
        x11_platform: { all(feature = "x11", free_unix, not(wasm), not(ohos)) },
        wayland_platform: { all(feature = "wayland", free_unix, not(wasm), not(ohos)) },

        // Backends.
        egl_backend: { all(feature = "egl", any(windows, unix), not(apple), not(wasm)) },
        glx_backend: { all(feature = "glx", x11_platform, not(wasm)) },
        wgl_backend: { all(feature = "wgl", windows, not(wasm)) },
        cgl_backend: { all(macos, not(wasm)) },
    }

}
