use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[allow(non_camel_case_types)]
pub enum CefBuildsPlatform {
    Auto,
    Windows_x64,
    Macos_x64,
    Macos_arm64,
    Linux_x64,
}

pub(crate) const DEFAULT_CEF_VERSION: &str = "137.0.10+g7e14fe1+chromium-137.0.7151.69";

impl CefBuildsPlatform {
    fn arch(&self) -> Option<&'static str> {
        match self {
            CefBuildsPlatform::Auto => match (std::env::consts::OS, std::env::consts::ARCH) {
                ("windows", "x86_64") => CefBuildsPlatform::Windows_x64.arch(),
                ("macos", "x86_64") => CefBuildsPlatform::Macos_x64.arch(),
                ("macos", "aarch64") => CefBuildsPlatform::Macos_arm64.arch(),
                ("linux", "x86_64") => CefBuildsPlatform::Linux_x64.arch(),
                _ => None,
            },
            CefBuildsPlatform::Windows_x64 => Some("windows64"),
            CefBuildsPlatform::Macos_x64 => Some("macosx64"),
            CefBuildsPlatform::Macos_arm64 => Some("macosarm64"),
            CefBuildsPlatform::Linux_x64 => Some("linux64"),
        }
    }

    pub(crate) fn download_url(&self, version: &str) -> Option<String> {
        Some(format!(
            "https://cef-builds.spotifycdn.com/cef_binary_{version}_{arch}.tar.bz2",
            version = version,
            arch = self.arch()?
        ))
    }

    pub(crate) fn root_dir_name(&self, version: &str) -> Option<String> {
        Some(format!(
            "cef_binary_{version}_{arch}",
            version = version,
            arch = self.arch()?
        ))
    }
}
