use std::path::PathBuf;

pub(crate) fn find_cef_root() -> PathBuf {
    if let Ok(cef_root) = std::env::var("CEF_ROOT") {
        cef_root.into()
    } else {
        dirs::home_dir().expect("get home directory").join(".cef")
    }
}
