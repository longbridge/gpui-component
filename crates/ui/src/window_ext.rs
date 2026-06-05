use crate::{
    Placement, Root, Theme,
    dialog::{AlertDialog, Dialog},
    global_state::GlobalState,
    input::InputState,
    notification::Notification,
    sheet::Sheet,
};
use gpui::{App, ElementId, Entity, Window};
use std::rc::Rc;

/// Extension trait for [`Window`] to add dialog, sheet .. functionality.
pub trait WindowExt: Sized {
    /// Opens a Sheet at right placement.
    fn open_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    /// Opens a Sheet at the given placement.
    fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    /// Return true, if there is an active Sheet.
    fn has_active_sheet(&mut self, cx: &mut App) -> bool;

    /// Closes the active Sheet.
    fn close_sheet(&mut self, cx: &mut App);

    /// Opens a Dialog.
    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static;

    /// Opens an AlertDialog.
    ///
    /// This is a convenience method for opening an alert dialog with opinionated defaults.
    /// The footer buttons are center-aligned and include an icon based on the variant.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use gpui_component::{AlertDialog, alert::AlertVariant};
    ///
    /// window.open_alert_dialog(cx, |alert, _, _| {
    ///     alert.warning()
    ///         .title("Unsaved Changes")
    ///         .description("You have unsaved changes. Are you sure you want to leave?")
    ///         .show_cancel(true)
    /// });
    /// ```
    fn open_alert_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(AlertDialog, &mut Window, &mut App) -> AlertDialog + 'static;

    /// Return true, if there is an active Dialog.
    fn has_active_dialog(&mut self, cx: &mut App) -> bool;

    /// Closes the last active Dialog.
    fn close_dialog(&mut self, cx: &mut App);

    /// Closes all active Dialogs.
    fn close_all_dialogs(&mut self, cx: &mut App);

    /// Pushes a notification to the notification list.
    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App);

    /// Removes all notifications whose id matches `T`, including ones registered with
    /// either `Notification::id` or `Notification::id1` (any key).
    fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App);

    /// Removes a single notification matching the given type `T` and `key` (paired with `Notification::id1`).
    fn remove_notification1<T: Sized + 'static>(&mut self, key: impl Into<ElementId>, cx: &mut App);

    /// Clears all notifications.
    fn clear_notifications(&mut self, cx: &mut App);

    /// Returns number of notifications.
    fn notifications(&mut self, cx: &mut App) -> Rc<Vec<Entity<Notification>>>;

    /// Return current focused Input entity.
    fn focused_input(&mut self, cx: &mut App) -> Option<Entity<InputState>>;
    /// Returns true if there is a focused Input entity.
    fn has_focused_input(&mut self, cx: &mut App) -> bool;

    /// Returns the merged selected text across all selectable TextViews in
    /// this window, ordered top to bottom and joined with `\n`.
    ///
    /// Returns an empty string if the window root is not a [`Root`].
    fn selected_text(&mut self, cx: &mut App) -> String;

    /// Returns true if there is an active text selection in this window
    /// (either a window-level drag selection or a view-local selection such
    /// as select-all or a double-click word selection).
    fn has_text_selection(&mut self, cx: &mut App) -> bool;

    /// Clears the window-level text selection and all view-local selections.
    fn clear_text_selection(&mut self, cx: &mut App);

    /// Enables the system glass effect for the window background.
    ///
    /// - macOS 26 (Tahoe) or later: Liquid Glass, by embedding a native
    ///   `NSGlassEffectView` behind the window content.
    /// - Windows 11 22H2 (build 22621) or later: Mica backdrop.
    /// - Other platforms (older systems, Linux): no-op that returns `false`,
    ///   the window stays opaque.
    ///
    /// When enabled, the large surface colors of the theme (e.g. `background`,
    /// `title_bar`, `sidebar`) are automatically made semi-transparent to let
    /// the glass show through, this applies to all windows of the application.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let window = cx.open_window(options, |window, cx| {
    ///     let view = cx.new(|_| Example);
    ///     cx.new(|cx| Root::new(view, window, cx))
    /// })?;
    ///
    /// window.update(cx, |_, window, cx| {
    ///     window.enable_window_glass(cx);
    /// })?;
    /// ```
    fn enable_window_glass(&mut self, cx: &mut App) -> bool;

    /// Disables the system glass effect for the window background,
    /// restoring the opaque background.
    ///
    /// This is a no-op if the effect is not enabled.
    fn disable_window_glass(&mut self, cx: &mut App);

    /// Returns true if the system glass effect is enabled for the window.
    fn is_window_glass_enabled(&self, cx: &App) -> bool;
}

impl WindowExt for Window {
    #[inline]
    fn open_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        self.open_sheet_at(Placement::Right, cx, build)
    }

    #[inline]
    fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        Root::update(self, cx, move |root, window, cx| {
            root.open_sheet_at(placement, build, window, cx);
        })
    }

    #[inline]
    fn has_active_sheet(&mut self, cx: &mut App) -> bool {
        Root::read(self, cx).active_sheet.is_some()
    }

    #[inline]
    fn close_sheet(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
            root.close_sheet(window, cx);
        })
    }

    #[inline]
    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        Root::update(self, cx, move |root, window, cx| {
            root.open_dialog(build, window, cx);
        })
    }

    #[inline]
    fn open_alert_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(AlertDialog, &mut Window, &mut App) -> AlertDialog + 'static,
    {
        self.open_dialog(cx, move |_, window, cx| {
            build(AlertDialog::new(cx), window, cx).into_dialog(window, cx)
        })
    }

    #[inline]
    fn has_active_dialog(&mut self, cx: &mut App) -> bool {
        Root::read(self, cx).active_dialogs.len() > 0
    }

    #[inline]
    fn close_dialog(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
            root.close_dialog(window, cx);
        })
    }

    #[inline]
    fn close_all_dialogs(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
            root.close_all_dialogs(window, cx);
        })
    }

    #[inline]
    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App) {
        let note = note.into();
        Root::update(self, cx, |root, window, cx| {
            root.push_notification(note, window, cx);
        })
    }

    #[inline]
    fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
            root.remove_notification::<T>(window, cx);
        })
    }

    #[inline]
    fn remove_notification1<T: Sized + 'static>(
        &mut self,
        key: impl Into<ElementId>,
        cx: &mut App,
    ) {
        let key = key.into();
        Root::update(self, cx, |root, window, cx| {
            root.remove_notification1::<T>(key, window, cx);
        })
    }

    #[inline]
    fn clear_notifications(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
            root.clear_notifications(window, cx);
        })
    }

    #[inline]
    fn notifications(&mut self, cx: &mut App) -> Rc<Vec<Entity<Notification>>> {
        Rc::new(Root::read(self, cx).notification.read(cx).notifications())
    }

    #[inline]
    fn has_focused_input(&mut self, cx: &mut App) -> bool {
        Root::read(self, cx).focused_input.is_some()
    }

    #[inline]
    fn focused_input(&mut self, cx: &mut App) -> Option<Entity<InputState>> {
        Root::read(self, cx).focused_input.clone()
    }

    #[inline]
    fn selected_text(&mut self, cx: &mut App) -> String {
        let Some(root) = self.root::<Root>().flatten() else {
            return String::new();
        };
        root.read(cx).window_selected_text(cx)
    }

    #[inline]
    fn has_text_selection(&mut self, cx: &mut App) -> bool {
        let Some(root) = self.root::<Root>().flatten() else {
            return false;
        };
        root.read(cx).has_text_selection(cx)
    }

    #[inline]
    fn clear_text_selection(&mut self, cx: &mut App) {
        let Some(root) = self.root::<Root>().flatten() else {
            return;
        };
        root.update(cx, |root, cx| root.clear_text_selection(cx));
    }

    fn enable_window_glass(&mut self, cx: &mut App) -> bool {
        let window_id = self.window_handle().window_id();
        if GlobalState::global(cx).glass_windows.contains(&window_id) {
            return true;
        }
        if !window_glass::enable(self) {
            return false;
        }

        GlobalState::global_mut(cx).glass_windows.insert(window_id);
        // Reapply the theme to make the surface colors semi-transparent.
        Theme::change(Theme::global(cx).mode, Some(self), cx);
        true
    }

    fn disable_window_glass(&mut self, cx: &mut App) {
        let window_id = self.window_handle().window_id();
        if !GlobalState::global_mut(cx).glass_windows.remove(&window_id) {
            return;
        }

        window_glass::disable(self);
        // Reapply the theme to restore the original surface colors.
        Theme::change(Theme::global(cx).mode, Some(self), cx);
    }

    #[inline]
    fn is_window_glass_enabled(&self, cx: &App) -> bool {
        GlobalState::global(cx).is_window_glass_enabled(self)
    }
}

/// Liquid Glass, embeds a native `NSGlassEffectView` (macOS 26+) behind the
/// window content.
///
/// This mirrors how GPUI injects an `NSVisualEffectView` for
/// [`WindowBackgroundAppearance::Blurred`](gpui::WindowBackgroundAppearance):
/// the glass view is added to the window's `contentView`, positioned below
/// the GPUI rendered view, with an autoresizing mask to follow window resize.
#[cfg(target_os = "macos")]
mod window_glass {
    use gpui::{Window, WindowBackgroundAppearance};
    use objc2::ffi::{NSInteger, NSUInteger};
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_foundation::NSRect;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    /// `NSViewWidthSizable | NSViewHeightSizable`
    const AUTORESIZE_WIDTH_HEIGHT: NSUInteger = (1 << 1) | (1 << 4);
    /// `NSWindowBelow`
    const NS_WINDOW_BELOW: NSInteger = -1;

    /// Returns the `contentView` of the window's native `NSWindow`.
    fn content_view(window: &Window) -> Option<*mut AnyObject> {
        let handle = HasWindowHandle::window_handle(window).ok()?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return None;
        };

        unsafe {
            let ns_view: *mut AnyObject = handle.ns_view.as_ptr().cast();
            let ns_window: *mut AnyObject = msg_send![ns_view, window];
            if ns_window.is_null() {
                return None;
            }
            let content_view: *mut AnyObject = msg_send![ns_window, contentView];
            if content_view.is_null() {
                return None;
            }

            Some(content_view)
        }
    }

    pub(super) fn enable(window: &mut Window) -> bool {
        // The class only exists on macOS 26 (Tahoe) and later, so a runtime
        // lookup doubles as the OS version check.
        let Some(glass_class) = AnyClass::get(c"NSGlassEffectView") else {
            return false;
        };
        let Some(content_view) = content_view(window) else {
            return false;
        };

        // Let GPUI make the window non-opaque and its Metal layer
        // transparent, so the glass below shows through.
        window.set_background_appearance(WindowBackgroundAppearance::Transparent);

        unsafe {
            let bounds: NSRect = msg_send![content_view, bounds];
            let glass: Retained<AnyObject> = msg_send![glass_class, new];
            let _: () = msg_send![&*glass, setFrame: bounds];
            let _: () = msg_send![&*glass, setAutoresizingMask: AUTORESIZE_WIDTH_HEIGHT];
            let _: () = msg_send![
                content_view,
                addSubview: &*glass,
                positioned: NS_WINDOW_BELOW,
                relativeTo: std::ptr::null_mut::<AnyObject>()
            ];
        }

        true
    }

    /// Removes the injected glass views and restores the opaque background.
    pub(super) fn disable(window: &mut Window) {
        let Some(glass_class) = AnyClass::get(c"NSGlassEffectView") else {
            return;
        };
        let Some(content_view) = content_view(window) else {
            return;
        };

        unsafe {
            // The `subviews` getter returns a copy of the array, so it is
            // safe to remove views while iterating over it.
            let subviews: Retained<AnyObject> = msg_send![content_view, subviews];
            let count: usize = msg_send![&*subviews, count];
            for i in 0..count {
                let view: *mut AnyObject = msg_send![&*subviews, objectAtIndex: i];
                let is_glass: bool = msg_send![view, isKindOfClass: glass_class];
                if is_glass {
                    let _: () = msg_send![view, removeFromSuperview];
                }
            }
        }

        window.set_background_appearance(WindowBackgroundAppearance::Opaque);
    }
}

/// Mica backdrop (Windows 11 22H2+), via GPUI's
/// [`WindowBackgroundAppearance::MicaBackdrop`](gpui::WindowBackgroundAppearance).
#[cfg(target_os = "windows")]
mod window_glass {
    use gpui::{Window, WindowBackgroundAppearance};

    /// The Mica backdrop requires Windows 11 22H2 (build 22621) or later,
    /// GPUI silently ignores it on older builds.
    const MIN_BUILD_NUMBER: u32 = 22621;

    pub(super) fn enable(window: &mut Window) -> bool {
        if windows_version::OsVersion::current().build < MIN_BUILD_NUMBER {
            return false;
        }

        window.set_background_appearance(WindowBackgroundAppearance::MicaBackdrop);
        true
    }

    pub(super) fn disable(window: &mut Window) {
        window.set_background_appearance(WindowBackgroundAppearance::Opaque);
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod window_glass {
    pub(super) fn enable(_window: &mut gpui::Window) -> bool {
        false
    }

    pub(super) fn disable(_window: &mut gpui::Window) {}
}
