fn main() {
    // 当环境变量发生变化时，cargo 会自动重新运行 build script 并重编译
    for key in [
        "ONETCLI_WECHAT_QR_URL",
        "ONETCLI_ALIPAY_QR_URL",
        "ONETCLI_PAYPAL_QR_URL",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
        if let Ok(val) = std::env::var(key) {
            if !val.is_empty() {
                println!("cargo:rustc-env={key}={val}");
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../resources/windows/onetcli.ico");
        res.set("ProductName", "OnetCli");
        res.set("FileDescription", "OnetCli - One Net Client");
        res.set("LegalCopyright", "Copyright (c) 2025 OnetCli");
        res.compile().expect("Failed to compile Windows resources");
    }
}
