fn main() {
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
