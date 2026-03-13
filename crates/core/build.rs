fn main() {
    // 当环境变量发生变化时，cargo 会自动重新运行 build script 并重编译
    // 彻底解决 cargo cache 导致 option_env! 拿不到值的问题
    for key in [
        "SUPABASE_URL",
        "SUPABASE_ANON_KEY",
        "ONETCLI_UPDATE_URL",
        "ONETCLI_UPDATE_DOWNLOAD_URL",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
        if let Ok(val) = std::env::var(key) {
            if !val.is_empty() {
                println!("cargo:rustc-env={key}={val}");
            }
        }
    }
}
