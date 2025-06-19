use cfg_aliases::cfg_aliases;
fn main() {
    println!("cargo:rustc-env=EMCC_CFLAGS=-s MAX_WEBGL_VERSION=2 -s MODULARIZE=1 -s EXPORT_NAME=loadDeftApp -s EXPORTED_RUNTIME_METHODS=GL,cwrap");
    println!("cargo:rerun-if-changed=build.rs");

    cfg_aliases! {
        // Systems.
        web_platform: { all(target_family = "wasm", target_os = "unknown") },
        emscripten_platform: { all(target_family = "wasm", target_os = "emscripten") },
        macos_platform: { target_os = "macos" },
        windows_platform: { target_os = "windows" },
        linux_platform: { all(target_os = "linux", not(ohos)) },
        desktop_platform: { any(windows_platform, linux_platform, macos_platform) },
        ohos: { target_env = "ohos" },

    }
}