use cfg_aliases::cfg_aliases;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    cfg_aliases! {
        // Systems
        mobile_platform: { any(target_os = "ios", target_os = "android") },
    }
}