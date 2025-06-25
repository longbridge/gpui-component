use std::ffi::CString;

use crate::ApplicationHandler;

/// Application settings.
#[derive(Debug)]
pub struct Settings<T> {
    pub(crate) locale: Option<CString>,
    pub(crate) cache_path: Option<CString>,
    pub(crate) root_cache_path: Option<CString>,
    pub(crate) browser_subprocess_path: Option<CString>,
    pub(crate) external_message_pump: bool,
    pub(crate) handler: T,
}

impl Settings<()> {
    /// Creates a new [`Settings`] instance with default values.
    #[inline]
    pub fn new() -> Self {
        Self {
            locale: None,
            cache_path: None,
            root_cache_path: None,
            browser_subprocess_path: None,
            external_message_pump: false,
            handler: (),
        }
    }
}

impl<T> Settings<T> {
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

    /// Enable to control browser process main (UI) thread message pump
    /// scheduling via the
    /// [`crate::ApplicationHandler::on_schedule_message_pump_work`]
    /// callback.
    ///
    /// This option is recommended for use in combination with the
    /// [`crate::do_message_work`] function in cases where the CEF message loop
    /// must be integrated into an existing application message.
    pub fn external_message_pump(mut self, enable: bool) -> Self {
        self.external_message_pump = enable;
        self
    }

    /// Sets the event handler.
    #[inline]
    pub fn handler<Q>(self, handler: Q) -> Settings<Q>
    where
        Q: ApplicationHandler,
    {
        Settings {
            locale: self.locale,
            cache_path: self.cache_path,
            root_cache_path: self.root_cache_path,
            browser_subprocess_path: self.browser_subprocess_path,
            external_message_pump: self.external_message_pump,
            handler,
        }
    }
}
