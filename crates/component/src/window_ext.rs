use crate::root::{OverlayClosed, WindowState};
use crate::{
    Placement,
    dialog::{AlertDialog, Dialog},
    input::AnyInputState,
    notification::Notification,
    sheet::Sheet,
};
use gpui::{App, ElementId, Entity, FocusHandle, Subscription, Window};
use std::rc::Rc;

/// Identifies one window dialog without owning its lifetime.
///
/// Dropping a handle does not close the dialog. Keep the subscription returned
/// by [`Self::on_closed`] for as long as close notifications are needed.
#[derive(Clone, Debug)]
pub struct DialogHandle {
    focus: FocusHandle,
}

impl DialogHandle {
    /// Whether this dialog is still in this window's dialog stack.
    pub fn is_open(&self, window: &Window, cx: &App) -> bool {
        WindowState::try_read(window, cx).is_some_and(|root| root.dialog_is_open(&self.focus))
    }

    /// Closes only this dialog, preserving dialogs above it and their focus.
    /// Already closed handles do nothing. This does not call `Dialog::on_close`.
    pub fn close(&self, window: &mut Window, cx: &mut App) {
        WindowState::try_update(window, cx, |root, window, cx| {
            root.close_dialog_for(&self.focus, window, cx)
        });
    }

    /// Rebuilds the window's overlays after captured presentation state changes.
    pub fn refresh(&self, window: &mut Window, cx: &mut App) {
        WindowState::try_update(window, cx, |root, _, cx| {
            if root.dialog_is_open(&self.focus) {
                cx.notify();
            }
        });
    }

    /// Observes removal from the live window's dialog stack, including close-all
    /// and scoped removal. It does not replay a past removal or observe window destruction.
    pub fn on_closed(
        &self,
        window: &mut Window,
        cx: &mut App,
        callback: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Subscription {
        on_overlay_closed(&self.focus, window, cx, callback)
    }
}

/// Identifies one window sheet; replacing it makes this handle inactive.
#[derive(Clone, Debug)]
pub struct SheetHandle {
    focus: FocusHandle,
}

impl SheetHandle {
    /// Whether this is the window's current sheet.
    pub fn is_open(&self, window: &Window, cx: &App) -> bool {
        WindowState::try_read(window, cx).is_some_and(|root| root.sheet_is_open(&self.focus))
    }

    /// Closes this sheet if still current. A replaced handle never closes its
    /// replacement. This does not call `Sheet::on_close`.
    pub fn close(&self, window: &mut Window, cx: &mut App) {
        WindowState::try_update(window, cx, |root, window, cx| {
            root.close_sheet_for(&self.focus, window, cx)
        });
    }

    /// Rebuilds the window's overlays after captured presentation state changes.
    pub fn refresh(&self, window: &mut Window, cx: &mut App) {
        WindowState::try_update(window, cx, |root, _, cx| {
            if root.sheet_is_open(&self.focus) {
                cx.notify();
            }
        });
    }

    /// Moves this sheet to another edge while retaining focus and content state.
    pub fn set_placement(&self, placement: Placement, window: &mut Window, cx: &mut App) {
        WindowState::try_update(window, cx, |root, _, cx| {
            root.set_sheet_placement_for(&self.focus, placement, cx)
        });
    }

    /// Observes removal from the live window, including replacement by another
    /// sheet. Keep the subscription; past removal and window destruction are not reported.
    pub fn on_closed(
        &self,
        window: &mut Window,
        cx: &mut App,
        callback: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Subscription {
        on_overlay_closed(&self.focus, window, cx, callback)
    }
}

fn on_overlay_closed(
    focus: &FocusHandle,
    window: &mut Window,
    cx: &mut App,
    callback: impl Fn(&mut Window, &mut App) + 'static,
) -> Subscription {
    let root = WindowState::entity(window, cx).expect("component window state is not initialized");
    let focus = focus.clone();
    window.subscribe(&root, cx, move |_, event: &OverlayClosed, window, cx| {
        if event.0 == focus {
            callback(window, cx);
        }
    })
}

/// Extension trait for [`Window`] to add dialog, sheet .. functionality.
pub trait WindowExt: Sized {
    /// Opens a Sheet at right placement.
    ///
    /// The builder follows the same rendering contract as
    /// [`WindowExt::open_dialog`]: it may run again when the sheet layer is
    /// rebuilt. Keep it cheap and create persistent entities before opening,
    /// not inside the builder.
    fn open_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    /// Opens a Sheet at the given placement.
    ///
    /// The builder follows the same rendering contract as
    /// [`WindowExt::open_dialog`]: it may run again when the sheet layer is
    /// rebuilt. Keep it cheap and create persistent entities before opening,
    /// not inside the builder.
    fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    /// Return true, if there is an active Sheet.
    fn has_active_sheet(&mut self, cx: &mut App) -> bool;

    /// Closes the active Sheet.
    fn close_sheet(&mut self, cx: &mut App);

    /// Opens a Dialog.
    ///
    /// The builder is a render closure, not a one-shot constructor. It may
    /// run again when the dialog layer is rebuilt while the dialog is open.
    /// View caching means a displayed frame need not rebuild the layer; the
    /// builder is not a periodic update callback. Keep it cheap and idempotent.
    ///
    /// Create persistent entities, such as an [`InputState`](crate::input::InputState)
    /// or a child view, before opening and capture their handles in the builder.
    /// Creating them with `cx.new(...)` inside the builder replaces them on
    /// each rebuild and loses their editing state.
    ///
    /// `open_dialog` initially focuses the dialog shell, replacing any focus
    /// set before the call. To choose initial content focus, set it only on
    /// the builder's first invocation, with a guard captured separately for
    /// each opening. Never focus unconditionally on subsequent rebuilds: that
    /// would steal focus from another control or a nested dialog.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::cell::Cell;
    /// use gpui_kit::{AppContext as _, Focusable as _};
    /// use gpui_kit::component::{WindowExt as _, input::{Input, InputState}};
    ///
    /// // Both the input and the one-time focus guard belong to this opening.
    /// let input = cx.new(|cx| InputState::new(window, cx).placeholder("URL"));
    /// let initial_focus = Cell::new(true);
    /// window.open_dialog(cx, move |dialog, window, cx| {
    ///     if initial_focus.replace(false) {
    ///         input.focus_handle(cx).focus(window, cx);
    ///     }
    ///     dialog.title("Import").child(Input::new(&input))
    /// });
    /// ```
    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static;

    /// Opens a dialog and returns its stable instance handle. The rendering
    /// contract is the same as [`Self::open_dialog`].
    fn open_dialog_with_handle<F>(&mut self, cx: &mut App, build: F) -> DialogHandle
    where
        Self: std::borrow::BorrowMut<Window>,
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        let focus = WindowState::update(self.borrow_mut(), cx, |root, window, cx| {
            root.open_dialog(build, window, cx)
        });
        DialogHandle { focus }
    }

    /// Opens an alert dialog with a handle for scoped removal and close observation.
    fn open_alert_dialog_with_handle<F>(&mut self, cx: &mut App, build: F) -> DialogHandle
    where
        Self: std::borrow::BorrowMut<Window>,
        F: Fn(AlertDialog, &mut Window, &mut App) -> AlertDialog + 'static,
    {
        let focus = WindowState::update(self.borrow_mut(), cx, |root, window, cx| {
            root.open_dialog(
                move |_, window, cx| {
                    build(AlertDialog::new(cx), window, cx).build_surface(window, cx)
                },
                window,
                cx,
            )
        });
        DialogHandle { focus }
    }

    /// Opens a sheet at an edge and returns a handle that remains safe after
    /// replacement. The rendering contract is the same as [`Self::open_sheet_at`].
    fn open_sheet_at_with_handle<F>(
        &mut self,
        placement: Placement,
        cx: &mut App,
        build: F,
    ) -> SheetHandle
    where
        Self: std::borrow::BorrowMut<Window>,
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        let focus = WindowState::update(self.borrow_mut(), cx, |root, window, cx| {
            root.open_sheet_at(placement, build, window, cx)
        });
        SheetHandle { focus }
    }

    /// Opens an AlertDialog.
    ///
    /// This is a convenience method for opening an alert dialog with opinionated defaults.
    /// The footer buttons are center-aligned and include an icon based on the variant.
    ///
    /// The builder follows the same rendering contract as
    /// [`WindowExt::open_dialog`]: it may run again when the dialog layer is
    /// rebuilt. Keep it cheap and create persistent entities before opening,
    /// not inside the builder.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use gpui_kit::component::{AlertDialog, alert::AlertVariant};
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

    /// Return the currently focused input state.
    ///
    /// Covers `Input`, `Textarea`, `Editor` and `OtpInput`, use
    /// [`AnyInputState::as_input`] and friends to get the concrete state.
    /// A registration whose focus handle is no longer focused (e.g. the input
    /// was removed from the tree while focused) is treated as `None`.
    fn focused_input(&mut self, cx: &mut App) -> Option<AnyInputState>;
    /// Returns true if there is a focused Input entity.
    fn has_focused_input(&mut self, cx: &mut App) -> bool;
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
        WindowState::update(self, cx, move |root, window, cx| {
            root.open_sheet_at(placement, build, window, cx);
        })
    }

    #[inline]
    fn has_active_sheet(&mut self, cx: &mut App) -> bool {
        WindowState::read(self, cx).active_sheet.is_some()
    }

    #[inline]
    fn close_sheet(&mut self, cx: &mut App) {
        WindowState::update(self, cx, |root, window, cx| {
            root.close_sheet(window, cx);
        })
    }

    #[inline]
    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        WindowState::update(self, cx, move |root, window, cx| {
            root.open_dialog(build, window, cx);
        })
    }

    #[inline]
    fn open_alert_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(AlertDialog, &mut Window, &mut App) -> AlertDialog + 'static,
    {
        self.open_dialog(cx, move |_, window, cx| {
            build(AlertDialog::new(cx), window, cx).build_surface(window, cx)
        })
    }

    #[inline]
    fn has_active_dialog(&mut self, cx: &mut App) -> bool {
        !WindowState::read(self, cx).active_dialogs.is_empty()
    }

    #[inline]
    fn close_dialog(&mut self, cx: &mut App) {
        WindowState::update(self, cx, |root, window, cx| {
            root.close_dialog(window, cx);
        })
    }

    #[inline]
    fn close_all_dialogs(&mut self, cx: &mut App) {
        WindowState::update(self, cx, |root, window, cx| {
            root.close_all_dialogs(window, cx);
        })
    }

    #[inline]
    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App) {
        let note = note.into();
        WindowState::update(self, cx, |root, window, cx| {
            root.push_notification(note, window, cx);
        })
    }

    #[inline]
    fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App) {
        WindowState::update(self, cx, |root, window, cx| {
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
        WindowState::update(self, cx, |root, window, cx| {
            root.remove_notification1::<T>(key, window, cx);
        })
    }

    #[inline]
    fn clear_notifications(&mut self, cx: &mut App) {
        WindowState::update(self, cx, |root, window, cx| {
            root.clear_notifications(window, cx);
        })
    }

    #[inline]
    fn notifications(&mut self, cx: &mut App) -> Rc<Vec<Entity<Notification>>> {
        Rc::new(
            WindowState::read(self, cx)
                .notification
                .read(cx)
                .notifications(),
        )
    }

    #[inline]
    fn has_focused_input(&mut self, cx: &mut App) -> bool {
        self.focused_input(cx).is_some()
    }

    fn focused_input(&mut self, cx: &mut App) -> Option<AnyInputState> {
        let state = WindowState::read(self, cx).focused_input.clone()?;
        if state.focus_handle(cx).is_focused(self) {
            return Some(state);
        }

        // An input removed from the tree while focused never re-renders to
        // unregister itself; drop the stale registration lazily.
        WindowState::try_update(self, cx, |root, _, cx| {
            if root.focused_input.as_ref() == Some(&state) {
                root.focused_input = None;
                cx.notify();
            }
        });
        None
    }
}
