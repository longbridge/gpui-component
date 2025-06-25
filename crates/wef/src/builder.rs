use std::ffi::{CString, c_void};

use raw_window_handle::RawWindowHandle;

use crate::{Browser, BrowserHandler, FuncRegistry, ffi::*};

/// A builder for creating a browser instance.
pub struct BrowserBuilder<T> {
    parent: Option<RawWindowHandle>,
    width: u32,
    height: u32,
    device_scale_factor: f32,
    frame_rate: u32,
    url: String,
    handler: T,
    func_registry: FuncRegistry,
}

impl BrowserBuilder<()> {
    pub(crate) fn new() -> BrowserBuilder<()> {
        BrowserBuilder {
            parent: None,
            width: 100,
            height: 100,
            device_scale_factor: 1.0,
            frame_rate: 60,
            url: "about:blank".to_string(),
            handler: (),
            func_registry: Default::default(),
        }
    }
}

pub(crate) struct BrowserState<T> {
    pub(crate) handler: T,
    pub(crate) func_registry: FuncRegistry,
}

impl<T> BrowserBuilder<T>
where
    T: BrowserHandler,
{
    /// Sets the parent window handle.
    ///
    /// Default is `None`.
    pub fn parent(self, parent: impl Into<Option<RawWindowHandle>>) -> Self {
        Self {
            parent: parent.into(),
            ..self
        }
    }

    /// Sets the size of the render target.
    pub fn size(self, width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            ..self
        }
    }

    /// Sets the device scale factor.
    ///
    /// Default is `1.0`.
    #[inline]
    pub fn device_scale_factor(self, device_scale_factor: f32) -> Self {
        Self {
            device_scale_factor,
            ..self
        }
    }

    /// Sets the frame rate.
    ///
    /// Default is `60`.
    pub fn frame_rate(self, frame_rate: u32) -> Self {
        Self { frame_rate, ..self }
    }

    /// Sets the URL to load.
    ///
    /// Default is `about:blank`.
    pub fn url(self, url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..self
        }
    }

    /// Sets the event handler.
    #[inline]
    pub fn handler<Q>(self, handler: Q) -> BrowserBuilder<Q>
    where
        Q: BrowserHandler,
    {
        BrowserBuilder {
            parent: self.parent,
            width: self.width,
            height: self.height,
            device_scale_factor: self.device_scale_factor,
            frame_rate: self.frame_rate,
            url: self.url,
            handler,
            func_registry: self.func_registry,
        }
    }

    /// Sets the function registry.
    ///
    /// See also [`FuncRegistry`] for more details.
    pub fn func_registry(self, func_registry: FuncRegistry) -> Self {
        Self {
            func_registry,
            ..self
        }
    }

    /// Cosumes the builder and creates a [`Browser`] instance.
    ///
    /// The creation of the browser is asynchronous, and the
    /// [`BrowserHandler::on_created`] method will be called upon completion.
    pub fn build(self) -> Browser {
        let callbacks = CBrowserCallbacks {
            on_created: crate::browser_handler::on_created::<T>,
            on_closed: crate::browser_handler::on_closed::<T>,
            on_popup_show: crate::browser_handler::on_popup_show::<T>,
            on_popup_position: crate::browser_handler::on_popup_position::<T>,
            on_paint: crate::browser_handler::on_paint::<T>,
            on_address_changed: crate::browser_handler::on_address_changed::<T>,
            on_title_changed: crate::browser_handler::on_title_changed::<T>,
            on_favicon_url_changed: crate::browser_handler::on_favicon_url_changed::<T>,
            on_tooltip: crate::browser_handler::on_tooltip::<T>,
            on_status_message: crate::browser_handler::on_status_message::<T>,
            on_console_message: crate::browser_handler::on_console_message::<T>,
            on_cursor_changed: crate::browser_handler::on_cursor_changed::<T>,
            on_before_popup: crate::browser_handler::on_before_popup::<T>,
            on_loading_progress_changed: crate::browser_handler::on_loading_progress_changed::<T>,
            on_loading_state_changed: crate::browser_handler::on_loading_state_changed::<T>,
            on_load_start: crate::browser_handler::on_load_start::<T>,
            on_load_end: crate::browser_handler::on_load_end::<T>,
            on_load_error: crate::browser_handler::on_load_error::<T>,
            on_ime_composition_range_changed:
                crate::browser_handler::on_ime_composition_range_changed::<T>,
            on_file_dialog: crate::browser_handler::on_file_dialog::<T>,
            on_context_menu: crate::browser_handler::on_context_menu::<T>,
            on_find_result: crate::browser_handler::on_find_result::<T>,
            on_js_dialog: crate::browser_handler::on_js_dialog::<T>,
            on_query: crate::browser_handler::on_query::<T>,
        };
        let handler = Box::into_raw(Box::new(BrowserState {
            handler: self.handler,
            func_registry: self.func_registry.clone(),
        }));
        let parent_window_handle: *const c_void = match self.parent {
            Some(RawWindowHandle::Win32(handle)) => handle.hwnd.get() as *const c_void,
            Some(RawWindowHandle::AppKit(handle)) => handle.ns_view.as_ptr(),
            Some(RawWindowHandle::Xcb(handle)) => handle.window.get() as *const c_void,
            _ => std::ptr::null(),
        };

        extern "C" fn destroy_handler<T>(user_data: *mut c_void) {
            unsafe { _ = Box::from_raw(user_data as *mut T) }
        }

        let url_cstr = CString::new(self.url).unwrap();
        let inject_javascript = CString::new(self.func_registry.javascript()).unwrap();
        let settings = CBrowserSettings {
            parent: parent_window_handle,
            device_scale_factor: self.device_scale_factor,
            width: self.width as i32,
            height: self.height as i32,
            frame_rate: self.frame_rate as i32,
            url: url_cstr.as_ptr(),
            inject_javascript: inject_javascript.as_ptr(),
            callbacks,
            userdata: handler as *mut c_void,
            destroy_userdata: destroy_handler::<T>,
        };

        unsafe {
            Browser {
                wef_browser: wef_browser_create(&settings),
            }
        }
    }
}
