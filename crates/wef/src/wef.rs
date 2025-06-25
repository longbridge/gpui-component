use std::ffi::{CString, c_char, c_void};

use crate::{ApplicationHandler, Error, app_handler::ApplicationState, ffi::*, settings::Settings};

/// Initialize the CEF browser process.
///
/// This function should be called on the main application thread to
/// initialize the CEF browser process.
pub fn init<T>(settings: Settings<T>) -> Result<(), Error>
where
    T: ApplicationHandler,
{
    unsafe {
        let callbacks = CAppCallbacks {
            on_schedule_message_pump_work: crate::app_handler::on_schedule_message_pump_work::<T>,
        };

        let handler = Box::into_raw(Box::new(ApplicationState {
            handler: settings.handler,
        }));

        extern "C" fn destroy_handler<T>(user_data: *mut c_void) {
            unsafe { _ = Box::from_raw(user_data as *mut T) }
        }

        let c_settings = CSettings {
            locale: to_cstr_ptr_opt(settings.locale.as_deref()),
            cache_path: to_cstr_ptr_opt(settings.cache_path.as_deref()),
            root_cache_path: to_cstr_ptr_opt(settings.root_cache_path.as_deref()),
            browser_subprocess_path: to_cstr_ptr_opt(settings.browser_subprocess_path.as_deref()),
            callbacks,
            userdata: handler as *mut c_void,
            destroy_userdata: destroy_handler::<T>,
        };

        if !wef_init(&c_settings) {
            return Err(Error::InitializeBrowserProcess);
        }
    }

    Ok(())
}

/// Executes the CEF subprocess.
///
/// This function should be called from the application entry point function
/// to execute a secondary process. It can be used to run secondary
/// processes from the browser client executable.
///
/// If called for the browser process (identified by no "type" command-line
/// value) it will return immediately with a value of `false`.
///
/// If called for a recognized secondary process it will block until the
/// process should exit and then return `true`.
///
/// # Examples
///
/// ```rust, no_run
/// use wef::Settings;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     if wef::exec_process()? {
///         return Ok(());
///     }
///
///     wef::init(Settings::new());
///     // ... event loop
///     wef::shutdown();
///     Ok(())
/// }
/// ```
pub fn exec_process() -> Result<bool, Error> {
    let args: Vec<CString> = std::env::args()
        .filter_map(|arg| CString::new(arg).ok())
        .collect();
    let c_args: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
    Ok(unsafe { wef_exec_process(c_args.as_ptr(), args.len() as i32) })
}

/// Perform a single iteration of CEF message loop processing.
///
/// This function is  provided for cases where the CEF message loop must be
/// integrated into an existing application message loop.
pub fn do_message_work() {
    unsafe { wef_do_message_work() };
}

/// Shuts down the CEF library.
///
/// # Panics
///
/// This function **MUST NOT** be called while any `CefBrowser` instances are
/// still alive. If there are any `CefBrowser` objects that have not been
/// dropped properly at the time of calling this function, it will likely lead
/// to a crash or undefined behavior.
pub fn shutdown() {
    unsafe { wef_shutdown() };
}

/// Launch the Wef application.
///
/// This function initializes the CEF library and runs the main process.
/// It is a convenience function that combines the [`init`] and [`shutdown`]
/// functions.
///
/// On macOS, it also loads the CEF framework using the
/// `crate::FrameworkLoader`.
///
/// # Panics
///
/// This function panics if the CEF library fails to initialize or if the
/// CEF framework fails to load on macOS.
///
/// # Examples
///
/// ```rust, no_run
/// use wef::Settings;
///
/// fn main() {
///     let settings = Settings::new();
///     wef::launch(settings, || {
///         // do something in the main process
///     });
/// }
/// ```
pub fn launch<T, F, R>(settings: Settings<T>, f: F) -> R
where
    T: ApplicationHandler,
    F: FnOnce() -> R,
{
    if cfg!(not(target_os = "macos")) {
        if exec_process().expect("failed to execute process") {
            // Is helper process, exit immediately
            std::process::exit(0);
        }
    }

    #[cfg(target_os = "macos")]
    let _ = crate::FrameworkLoader::load_in_main().expect("failed to load CEF framework");

    // Run the main process
    init(settings).expect("failed to initialize CEF");
    let res = f();
    shutdown();
    res
}
