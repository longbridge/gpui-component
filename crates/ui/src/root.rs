use crate::{
    dialog::Dialog,
    input::InputState,
    notification::{Notification, NotificationList},
    sheet::Sheet,
    window_border, ActiveTheme, Placement, StyledExt, TITLE_BAR_HEIGHT,
};
use gpui::{
    actions, canvas, div, prelude::FluentBuilder as _, AnyView, App, AppContext, Context,
    DefiniteLength, Entity, FocusHandle, InteractiveElement, IntoElement, KeyBinding,
    ParentElement as _, Render, StyleRefinement, Styled, Window,
};
use std::{any::TypeId, rc::Rc};

actions!(root, [Tab, TabPrev]);

const CONTEXT: &str = "Root";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
    ]);
}

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

    /// Return true, if there is an active Dialog.
    fn has_active_dialog(&mut self, cx: &mut App) -> bool;

    /// Closes the last active Dialog.
    fn close_dialog(&mut self, cx: &mut App);

    /// Closes all active Dialogs.
    fn close_all_dialogs(&mut self, cx: &mut App);

    /// Pushes a notification to the notification list.
    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App);

    /// Removes the notification with the given id.
    fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App);

    /// Clears all notifications.
    fn clear_notifications(&mut self, cx: &mut App);

    /// Returns number of notifications.
    fn notifications(&mut self, cx: &mut App) -> Rc<Vec<Entity<Notification>>>;

    /// Return current focused Input entity.
    fn focused_input(&mut self, cx: &mut App) -> Option<Entity<InputState>>;
    /// Returns true if there is a focused Input entity.
    fn has_focused_input(&mut self, cx: &mut App) -> bool;
}

impl WindowExt for Window {
    fn open_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        self.open_sheet_at(Placement::Right, cx, build)
    }

    fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        Root::update(self, cx, move |root, window, cx| {
            if root.active_sheet.is_none() {
                root.previous_focus_handle = window.focused(cx);
            }

            let focus_handle = cx.focus_handle();
            focus_handle.focus(window);

            root.active_sheet = Some(ActiveSheet {
                focus_handle,
                placement,
                builder: Rc::new(build),
            });
            cx.notify();
        })
    }

    fn has_active_sheet(&mut self, cx: &mut App) -> bool {
        Root::read(self, cx).active_sheet.is_some()
    }

    fn close_sheet(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
            root.focused_input = None;
            root.active_sheet = None;
            root.focus_back(window, cx);
            cx.notify();
        })
    }

    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        Root::update(self, cx, move |root, window, cx| {
            // Only save focus handle if there are no active dialogs.
            // This is used to restore focus when all dialogs are closed.
            if root.active_dialogs.len() == 0 {
                root.previous_focus_handle = window.focused(cx);
            }

            let focus_handle = cx.focus_handle();
            focus_handle.focus(window);

            root.active_dialogs.push(ActiveDialog {
                focus_handle,
                builder: Rc::new(build),
            });
            cx.notify();
        })
    }

    fn has_active_dialog(&mut self, cx: &mut App) -> bool {
        Root::read(self, cx).active_dialogs.len() > 0
    }

    fn close_dialog(&mut self, cx: &mut App) {
        Root::update(self, cx, move |root, window, cx| {
            root.focused_input = None;
            root.active_dialogs.pop();

            if let Some(top_dialog) = root.active_dialogs.last() {
                // Focus the next dialog.
                top_dialog.focus_handle.focus(window);
            } else {
                // Restore focus if there are no more dialogs.
                root.focus_back(window, cx);
            }
            cx.notify();
        })
    }

    fn close_all_dialogs(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
            root.focused_input = None;
            root.active_dialogs.clear();
            root.focus_back(window, cx);
            cx.notify();
        })
    }

    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App) {
        let note = note.into();
        Root::update(self, cx, move |root, window, cx| {
            root.notification
                .update(cx, |view, cx| view.push(note, window, cx));
            cx.notify();
        })
    }

    fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App) {
        Root::update(self, cx, move |root, window, cx| {
            root.notification.update(cx, |view, cx| {
                let id = TypeId::of::<T>();
                view.close(id, window, cx);
            });
            cx.notify();
        })
    }

    fn clear_notifications(&mut self, cx: &mut App) {
        Root::update(self, cx, move |root, window, cx| {
            root.notification
                .update(cx, |view, cx| view.clear(window, cx));
            cx.notify();
        })
    }

    fn notifications(&mut self, cx: &mut App) -> Rc<Vec<Entity<Notification>>> {
        let entity = Root::read(self, cx).notification.clone();
        Rc::new(entity.read(cx).notifications())
    }

    fn has_focused_input(&mut self, cx: &mut App) -> bool {
        Root::read(self, cx).focused_input.is_some()
    }

    fn focused_input(&mut self, cx: &mut App) -> Option<Entity<InputState>> {
        Root::read(self, cx).focused_input.clone()
    }
}

/// Root is a view for the App window for as the top level view (Must be the first view in the window).
///
/// It is used to manage the Sheet, Dialog, and Notification.
pub struct Root {
    style: StyleRefinement,
    /// Used to store the focus handle of the previous view.
    /// When the Dialog, Sheet closes, we will focus back to the previous view.
    previous_focus_handle: Option<FocusHandle>,
    active_sheet: Option<ActiveSheet>,
    pub(crate) active_dialogs: Vec<ActiveDialog>,
    pub(super) focused_input: Option<Entity<InputState>>,
    pub notification: Entity<NotificationList>,
    sheet_size: Option<DefiniteLength>,
    view: AnyView,
}

#[derive(Clone)]
struct ActiveSheet {
    focus_handle: FocusHandle,
    placement: Placement,
    builder: Rc<dyn Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static>,
}

#[derive(Clone)]
pub(crate) struct ActiveDialog {
    focus_handle: FocusHandle,
    builder: Rc<dyn Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static>,
}

impl Root {
    pub fn new(view: AnyView, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            style: StyleRefinement::default(),
            previous_focus_handle: None,
            active_sheet: None,
            active_dialogs: Vec::new(),
            focused_input: None,
            notification: cx.new(|cx| NotificationList::new(window, cx)),
            sheet_size: None,
            view,
        }
    }

    pub fn update<F, R>(window: &mut Window, cx: &mut App, f: F) -> R
    where
        F: FnOnce(&mut Self, &mut Window, &mut Context<Self>) -> R,
    {
        let root = window
            .root::<Root>()
            .flatten()
            .expect("BUG: window first layer should be a gpui_component::Root.");

        root.update(cx, |root, cx| f(root, window, cx))
    }

    pub fn read<'a>(window: &'a Window, cx: &'a App) -> &'a Self {
        &window
            .root::<Root>()
            .expect("The window root view should be of type `ui::Root`.")
            .unwrap()
            .read(cx)
    }

    fn focus_back(&mut self, window: &mut Window, _: &mut App) {
        if let Some(handle) = self.previous_focus_handle.clone() {
            window.focus(&handle);
        }
    }

    // Render Notification layer.
    fn render_notification_layer(
        &mut self,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        let active_sheet_placement = self.active_sheet.clone().map(|d| d.placement);

        let (mt, mr) = match active_sheet_placement {
            Some(Placement::Right) => (None, self.sheet_size),
            Some(Placement::Top) => (self.sheet_size, None),
            _ => (None, None),
        };

        Some(
            div()
                .absolute()
                .top(TITLE_BAR_HEIGHT)
                .right_0()
                .when_some(mt, |this, offset| this.mt(offset))
                .when_some(mr, |this, offset| this.mr(offset))
                .child(self.notification.clone()),
        )
    }

    /// Render the Sheet layer.
    fn render_sheet_layer(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        let active_sheet = self.active_sheet.clone()?;
        let root = cx.entity();
        let mut sheet = Sheet::new(window, cx);
        sheet = (active_sheet.builder)(sheet, window, cx);
        sheet.focus_handle = active_sheet.focus_handle.clone();
        sheet.placement = active_sheet.placement;

        let size = sheet.size;

        Some(
            div().relative().child(sheet).child(
                canvas(
                    move |_, _, cx| root.update(cx, |r, _| r.sheet_size = Some(size)),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            ),
        )
    }

    /// Render the Dialog layer.
    fn render_dialog_layer(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        let active_dialogs = self.active_dialogs.clone();
        if active_dialogs.is_empty() {
            return None;
        }

        let mut show_overlay_ix = None;

        let mut dialogs = active_dialogs
            .iter()
            .enumerate()
            .map(|(i, active_dialog)| {
                let mut dialog = Dialog::new(window, cx);

                dialog = (active_dialog.builder)(dialog, window, cx);

                // Give the dialog the focus handle, because `dialog` is a temporary value, is not possible to
                // keep the focus handle in the dialog.
                //
                // So we keep the focus handle in the `active_dialog`, this is owned by the `Root`.
                dialog.focus_handle = active_dialog.focus_handle.clone();

                dialog.layer_ix = i;
                // Find the dialog which one needs to show overlay.
                if dialog.has_overlay() {
                    show_overlay_ix = Some(i);
                }

                dialog
            })
            .collect::<Vec<_>>();

        if let Some(ix) = show_overlay_ix {
            if let Some(dialog) = dialogs.get_mut(ix) {
                dialog.overlay_visible = true;
            }
        }

        Some(div().children(dialogs))
    }

    /// Return the root view of the Root.
    pub fn view(&self) -> &AnyView {
        &self.view
    }

    fn on_action_tab(&mut self, _: &Tab, window: &mut Window, _: &mut Context<Self>) {
        window.focus_next();
    }

    fn on_action_tab_prev(&mut self, _: &TabPrev, window: &mut Window, _: &mut Context<Self>) {
        window.focus_prev();
    }
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let base_font_size = cx.theme().font_size;
        window.set_rem_size(base_font_size);

        window_border().child(
            div()
                .id("root")
                .key_context(CONTEXT)
                .on_action(cx.listener(Self::on_action_tab))
                .on_action(cx.listener(Self::on_action_tab_prev))
                .font_family(".SystemUIFont")
                .bg(cx.theme().background)
                .text_color(cx.theme().foreground)
                .refine_style(&self.style)
                .relative()
                .size_full()
                .child(self.view.clone())
                .children(self.render_sheet_layer(window, cx))
                .children(self.render_dialog_layer(window, cx))
                .children(self.render_notification_layer(window, cx)),
        )
    }
}
