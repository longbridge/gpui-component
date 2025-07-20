

#[cfg(target_os = "windows")]
fn main_win() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("../../assets/logo2.ico");
    res.compile().unwrap();
}


fn main() {
    // This is a no-op for non-Windows platforms.
    #[cfg(target_os = "windows")]
    main_win();
}