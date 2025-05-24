use std::ffi::CString;

/// Browser process settings.
#[derive(Debug, Default)]
pub struct Settings {
    pub(crate) locale: Option<CString>,
    pub(crate) cache_path: Option<CString>,
    pub(crate) root_cache_path: Option<CString>,
    pub(crate) browser_subprocess_path: Option<CString>,
}

impl Settings {
    /// Creates a new [`Settings`] instance with default values.
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// The locale string that will be passed to CEF.
    ///
    /// If `None` the default locale of "en-US" will be used.
    pub fn locale(mut self, locale: impl Into<Vec<u8>>) -> Self {
        self.locale = Some(CString::new(locale).expect("invalid locale string"));
        self
    }

    /// The directory where data for the global browser cache will be stored on
    /// disk.
    ///
    /// If this value is non-empty then it must be an absolute path that is
    /// either equal to or a child directory of `root_cache_path`. If
    /// this value is empty then browsers will be created in "incognito mode"
    /// where in-memory caches are used for storage and no profile-specific data
    /// is persisted to disk (installation-specific data will still be persisted
    /// in root_cache_path). HTML5 databases such as localStorage will only
    /// persist across sessions if a cache path is specified.
    pub fn cache_path(mut self, path: impl Into<Vec<u8>>) -> Self {
        self.cache_path = Some(CString::new(path).expect("invalid cache path"));
        self
    }

    /// The root directory for installation-specific data and the parent
    /// directory for profile-specific data.
    ///
    ///  If this value is `None` then the default platform-specific directory
    /// will  be used ("~/.config/cef_user_data" directory on Linux,
    ///  "~/Library/Application Support/CEF/User Data" directory on MacOS,
    ///  "AppData\Local\CEF\User Data" directory under the user profile
    /// directory  on Windows). Use of the default directory is not
    /// recommended in  production applications (see below).
    ///
    /// NOTE: Multiple application instances writing to the same root_cache_path
    /// directory could result in data corruption.
    pub fn root_cache_path(mut self, path: impl Into<Vec<u8>>) -> Self {
        self.root_cache_path = Some(CString::new(path).expect("invalid root cache path"));
        self
    }

    /// The path to a separate executable that will be launched for
    /// sub-processes.
    ///
    /// If this value is not set on Windows or Linux then the
    /// main process executable will be used. If this value is not set on
    /// macOS then a helper executable must exist at
    /// `Contents/Frameworks/<app> Helper.app/Contents/MacOS/<app> Helper`
    /// in the top-level app bundle. See the comments on CefExecuteProcess()
    /// for details.
    ///
    /// If this value is set then it must be an absolute path.
    pub fn browser_subprocess_path(mut self, path: impl Into<Vec<u8>>) -> Self {
        self.browser_subprocess_path =
            Some(CString::new(path).expect("invalid browser subprocess path"));
        self
    }
}
