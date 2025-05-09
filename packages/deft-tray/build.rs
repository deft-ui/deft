use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        linux_pc: { all(target_os = "linux", not(target_env = "ohos"))},
    }
}