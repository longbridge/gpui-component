use crate::{
    ActiveTheme, Anchor, ElementExt, Placement,
    dialog::Dialog,
    input::InputState,
    notification::{Notification, NotificationList},
    sheet::Sheet,
    window_border,
};
use gpui::{
    AnyView, App, AppContext, Context, DefiniteLength, Entity, FocusHandle, InteractiveElement,
    IntoElement, KeyBinding, ParentElement as _, Render, Styled, WeakFocusHandle, Window, actions,
    div, prelude::FluentBuilder as _,
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

/// Root is a view for the App window for as the top level view (Must be the first view in the window).
///
/// It is used to manage the Sheet, Dialog, and Notification.
pub struct Root {
    pub(crate) active_sheet: Option<ActiveSheet>,
    pub(crate) active_dialogs: Vec<ActiveDialog>,
    pub(super) focused_input: Option<Entity<InputState>>,
    pub notification: Entity<NotificationList>,
    sheet_size: Option<DefiniteLength>,
    view: AnyView,
}

#[derive(Clone)]
pub(crate) struct ActiveSheet {
    focus_handle: FocusHandle,
    /// The previous focused handle before opening the Sheet.
    previous_focused_handle: Option<WeakFocusHandle>,
    placement: Placement,
    builder: Rc<dyn Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static>,
}

#[derive(Clone)]
pub(crate) struct ActiveDialog {
    focus_handle: FocusHandle,
    /// The previous focused handle before opening the Dialog.
    previous_focused_handle: Option<WeakFocusHandle>,
    builder: Rc<dyn Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static>,
}

impl ActiveDialog {
    pub(crate) fn new(
        focus_handle: FocusHandle,
        previous_focused_handle: Option<WeakFocusHandle>,
        builder: impl Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    ) -> Self {
        Self {
            focus_handle,
            previous_focused_handle,
            builder: Rc::new(builder),
        }
    }
}

impl Root {
    /// Create a new Root view.
    pub fn new(view: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            active_sheet: None,
            active_dialogs: Vec::new(),
            focused_input: None,
            notification: cx.new(|cx| NotificationList::new(window, cx)),
            sheet_size: None,
            view: view.into(),
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

    // Render Notification layer.
    pub fn render_notification_layer(
        window: &mut Window,
        cx: &mut App,
    ) -> Option<impl IntoElement + use<>> {
        let root = window.root::<Root>()??;

        let active_sheet_placement = root.read(cx).active_sheet.clone().map(|d| d.placement);

        let sheet_size = root.read(cx).sheet_size;
        let (mt, mr, mb, ml) = match active_sheet_placement {
            Some(Placement::Top) => (sheet_size, None, None, None),
            Some(Placement::Right) => (None, sheet_size, None, None),
            Some(Placement::Bottom) => (None, None, sheet_size, None),
            Some(Placement::Left) => (None, None, None, sheet_size),
            _ => (None, None, None, None),
        };

        let placement = cx.theme().notification.placement;

        Some(
            div()
                .absolute()
                .when(matches!(placement, Anchor::TopRight), |this| {
                    this.top_0().right_0()
                })
                .when(matches!(placement, Anchor::TopLeft), |this| {
                    this.top_0().left_0()
                })
                .when(matches!(placement, Anchor::TopCenter), |this| {
                    this.top_0().mx_auto()
                })
                .when(matches!(placement, Anchor::BottomRight), |this| {
                    this.bottom_0().right_0()
                })
                .when(matches!(placement, Anchor::BottomLeft), |this| {
                    this.bottom_0().left_0()
                })
                .when(matches!(placement, Anchor::BottomCenter), |this| {
                    this.bottom_0().mx_auto()
                })
                .when_some(mt, |this, offset| this.mt(offset))
                .when_some(mr, |this, offset| this.mr(offset))
                .when_some(mb, |this, offset| this.mb(offset))
                .when_some(ml, |this, offset| this.ml(offset))
                .child(root.read(cx).notification.clone()),
        )
    }

    /// Render the Sheet layer.
    pub fn render_sheet_layer(
        window: &mut Window,
        cx: &mut App,
    ) -> Option<impl IntoElement + use<>> {
        let root = window.root::<Root>()??;

        if let Some(active_sheet) = root.read(cx).active_sheet.clone() {
            let mut sheet = Sheet::new(window, cx);
            sheet = (active_sheet.builder)(sheet, window, cx);
            sheet.focus_handle = active_sheet.focus_handle.clone();
            sheet.placement = active_sheet.placement;

            let size = sheet.size;

            return Some(
                div()
                    .relative()
                    .child(sheet)
                    .on_prepaint(move |_, _, cx| root.update(cx, |r, _| r.sheet_size = Some(size))),
            );
        }

        None
    }

    /// Render the Dialog layer.
    pub fn render_dialog_layer(
        window: &mut Window,
        cx: &mut App,
    ) -> Option<impl IntoElement + use<>> {
        let root = window.root::<Root>()??;

        let active_dialogs = root.read(cx).active_dialogs.clone();

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

    pub fn open_dialog<F>(&mut self, build: F, window: &mut Window, cx: &mut Context<'_, Root>)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        let previous_focused_handle = window.focused(cx).map(|h| h.downgrade());
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        self.active_dialogs.push(ActiveDialog::new(
            focus_handle,
            previous_focused_handle,
            build,
        ));
        cx.notify();
    }

    pub fn close_dialog(&mut self, window: &mut Window, cx: &mut Context<'_, Root>) {
        self.focused_input = None;
        if let Some(handle) = self
            .active_dialogs
            .pop()
            .and_then(|d| d.previous_focused_handle)
            .and_then(|h| h.upgrade())
        {
            window.focus(&handle, cx);
        }
        cx.notify();
    }

    pub fn close_all_dialogs(&mut self, window: &mut Window, cx: &mut Context<'_, Root>) {
        self.focused_input = None;
        let previous_focused_handle = self
            .active_dialogs
            .first()
            .and_then(|d| d.previous_focused_handle.clone());
        self.active_dialogs.clear();
        if let Some(handle) = previous_focused_handle.and_then(|h| h.upgrade()) {
            window.focus(&handle, cx);
        }
        cx.notify();
    }

    pub fn open_sheet_at<F>(
        &mut self,
        placement: Placement,
        build: F,
        window: &mut Window,
        cx: &mut Context<'_, Root>,
    ) where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        let previous_focused_handle = window.focused(cx).map(|h| h.downgrade());

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);
        self.active_sheet = Some(ActiveSheet {
            focus_handle,
            previous_focused_handle,
            placement,
            builder: Rc::new(build),
        });
        cx.notify();
    }

    pub fn close_sheet(&mut self, window: &mut Window, cx: &mut Context<'_, Root>) {
        self.focused_input = None;
        if let Some(previous_handle) = self
            .active_sheet
            .as_ref()
            .and_then(|s| s.previous_focused_handle.as_ref())
            .and_then(|h| h.upgrade())
        {
            window.focus(&previous_handle, cx);
        }
        self.active_sheet = None;
        cx.notify();
    }

    pub fn push_notification(
        &mut self,
        note: impl Into<Notification>,
        window: &mut Window,
        cx: &mut Context<'_, Root>,
    ) {
        self.notification
            .update(cx, |view, cx| view.push(note, window, cx));
        cx.notify();
    }

    pub fn remove_notification<T: Sized + 'static>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Root>,
    ) {
        self.notification.update(cx, |view, cx| {
            let id = TypeId::of::<T>();
            view.close(id, window, cx);
        });
        cx.notify();
    }

    pub fn clear_notifications(&mut self, window: &mut Window, cx: &mut Context<'_, Root>) {
        self.notification
            .update(cx, |view, cx| view.clear(window, cx));
        cx.notify();
    }

    /// Return the root view of the Root.
    pub fn view(&self) -> &AnyView {
        &self.view
    }

    fn on_action_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_next(cx);
    }

    fn on_action_tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_prev(cx);
    }
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(cx.theme().font_size);

        window_border().child(
            div()
                .id("root")
                .key_context(CONTEXT)
                .on_action(cx.listener(Self::on_action_tab))
                .on_action(cx.listener(Self::on_action_tab_prev))
                .relative()
                .size_full()
                .font_family(cx.theme().font_family.clone())
                .bg(cx.theme().background)
                .text_color(cx.theme().foreground)
                .child(self.view.clone()),
        )
    }
}
