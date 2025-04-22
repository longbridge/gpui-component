use crate::{
    drawer::Drawer,
    input::TextInput,
    modal::Modal,
    notification::{Notification, NotificationList},
    window_border, ActiveTheme, Placement,
};
use gpui::{
    canvas, div, prelude::FluentBuilder as _, AnyView, App, AppContext, Context, DefiniteLength,
    Entity, FocusHandle, InteractiveElement, IntoElement, ParentElement as _, Render, Styled,
    Window,
};
use std::rc::Rc;

/// Extension trait for [`WindowContext`] and [`ViewContext`] to add drawer functionality.
pub trait ContextModal: Sized {
    /// Opens a Drawer at right placement.
    fn open_drawer<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Drawer, &mut Window, &mut App) -> Drawer + 'static;

    /// Opens a Drawer at the given placement.
    fn open_drawer_at<F>(&mut self, placement: Placement, cx: &mut App, build: F)
    where
        F: Fn(Drawer, &mut Window, &mut App) -> Drawer + 'static;

    /// Return true, if there is an active Drawer.
    fn has_active_drawer(&mut self, cx: &mut App) -> bool;

    /// Closes the active Drawer.
    fn close_drawer(&mut self, cx: &mut App);

    /// Opens a Modal.
    fn open_modal<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Modal, &mut Window, &mut App) -> Modal + 'static;

    /// Return true, if there is an active Modal.
    fn has_active_modal(&mut self, cx: &mut App) -> bool;

    /// Closes the last active Modal.
    fn close_modal(&mut self, cx: &mut App);

    /// Closes all active Modals.
    fn close_all_modals(&mut self, cx: &mut App);

    /// Pushes a notification to the notification list.
    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App);
    fn clear_notifications(&mut self, cx: &mut App);
    /// Returns number of notifications.
    fn notifications(&mut self, cx: &mut App) -> Rc<Vec<Entity<Notification>>>;

    /// Return current focused Input entity.
    fn focused_input(&mut self, cx: &mut App) -> Option<Entity<TextInput>>;
    /// Returns true if there is a focused Input entity.
    fn has_focused_input(&mut self, cx: &mut App) -> bool;
}

impl ContextModal for Window {
    fn open_drawer<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Drawer, &mut Window, &mut App) -> Drawer + 'static,
    {
        self.open_drawer_at(Placement::Right, cx, build)
    }

    fn open_drawer_at<F>(&mut self, placement: Placement, cx: &mut App, build: F)
    where
        F: Fn(Drawer, &mut Window, &mut App) -> Drawer + 'static,
    {
        Root::update(self, cx, move |root, window, cx| {
            if root.active_drawer.is_none() {
                root.previous_focus_handle = window.focused(cx);
            }

            let focus_handle = cx.focus_handle();
            focus_handle.focus(window);

            root.active_drawer = Some(ActiveDrawer {
                focus_handle,
                placement,
                builder: Rc::new(build),
            });
            cx.notify();
        })
    }

    fn has_active_drawer(&mut self, cx: &mut App) -> bool {
        Root::read(self, cx).active_drawer.is_some()
    }

    fn close_drawer(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
            root.focused_input = None;
            root.active_drawer = None;
            root.focus_back(window, cx);
            cx.notify();
        })
    }

    fn open_modal<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Modal, &mut Window, &mut App) -> Modal + 'static,
    {
        Root::update(self, cx, move |root, window, cx| {
            // Only save focus handle if there are no active modals.
            // This is used to restore focus when all modals are closed.
            if root.active_modals.len() == 0 {
                root.previous_focus_handle = window.focused(cx);
            }

            let focus_handle = cx.focus_handle();
            focus_handle.focus(window);

            root.active_modals.push(ActiveModal {
                focus_handle,
                builder: Rc::new(build),
            });
            cx.notify();
        })
    }

    fn has_active_modal(&mut self, cx: &mut App) -> bool {
        Root::read(self, cx).active_modals.len() > 0
    }

    fn close_modal(&mut self, cx: &mut App) {
        Root::update(self, cx, move |root, window, cx| {
            root.focused_input = None;
            root.active_modals.pop();

            if let Some(top_modal) = root.active_modals.last() {
                // Focus the next modal.
                top_modal.focus_handle.focus(window);
            } else {
                // Restore focus if there are no more modals.
                root.focus_back(window, cx);
            }
            cx.notify();
        })
    }

    fn close_all_modals(&mut self, cx: &mut App) {
        Root::update(self, cx, |root, window, cx| {
            root.focused_input = None;
            root.active_modals.clear();
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

    fn focused_input(&mut self, cx: &mut App) -> Option<Entity<TextInput>> {
        Root::read(self, cx).focused_input.clone()
    }
}

// impl<V> ContextModal for Context<'_, V> {
//     fn open_drawer<F>(&mut self, cx: &mut App, build: F)
//     where
//         F: Fn(Drawer, &mut Window, &mut App) -> Drawer + 'static,
//     {
//         self.deref_mut().open_drawer(cx, build)
//     }

//     fn open_drawer_at<F>(&mut self, cx: &mut App, placement: Placement, build: F)
//     where
//         F: Fn(Drawer, &mut Window, &mut App) -> Drawer + 'static,
//     {
//         self.deref_mut().open_drawer_at(cx, placement, build)
//     }

//     fn has_active_modal(&self, cx: &mut App) -> bool {
//         self.deref().has_active_modal(cx)
//     }

//     fn close_drawer(&mut self, cx: &mut App) {
//         self.deref_mut().close_drawer(cx)
//     }

//     fn open_modal<F>(&mut self, cx: &mut App, build: F)
//     where
//         F: Fn(Modal, &mut Window, &mut App) -> Modal + 'static,
//     {
//         self.deref_mut().open_modal(cx, build)
//     }

//     fn has_active_drawer(&self, cx: &mut App) -> bool {
//         self.deref().has_active_drawer(cx)
//     }

//     /// Close the last active modal.
//     fn close_modal(&mut self, cx: &mut App) {
//         self.deref_mut().close_modal(cx)
//     }

//     /// Close all modals.
//     fn close_all_modals(&mut self, cx: &mut App) {
//         self.deref_mut().close_all_modals(cx)
//     }

//     fn push_notification(&mut self, cx: &mut App, note: impl Into<Notification>) {
//         self.deref_mut().push_notification(cx, note)
//     }

//     fn clear_notifications(&mut self, cx: &mut App) {
//         self.deref_mut().clear_notifications(cx)
//     }

//     fn notifications(&self, cx: &mut App) -> Rc<Vec<Entity<Notification>>> {
//         self.deref().notifications(cx)
//     }
// }

/// Root is a view for the App window for as the top level view (Must be the first view in the window).
///
/// It is used to manage the Drawer, Modal, and Notification.
pub struct Root {
    /// Used to store the focus handle of the previous view.
    /// When the Modal, Drawer closes, we will focus back to the previous view.
    previous_focus_handle: Option<FocusHandle>,
    active_drawer: Option<ActiveDrawer>,
    pub(crate) active_modals: Vec<ActiveModal>,
    pub(super) focused_input: Option<Entity<TextInput>>,
    pub notification: Entity<NotificationList>,
    drawer_size: Option<DefiniteLength>,
    view: AnyView,
}

#[derive(Clone)]
struct ActiveDrawer {
    focus_handle: FocusHandle,
    placement: Placement,
    builder: Rc<dyn Fn(Drawer, &mut Window, &mut App) -> Drawer + 'static>,
}

#[derive(Clone)]
pub(crate) struct ActiveModal {
    focus_handle: FocusHandle,
    builder: Rc<dyn Fn(Modal, &mut Window, &mut App) -> Modal + 'static>,
}

impl Root {
    pub fn new(view: AnyView, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            previous_focus_handle: None,
            active_drawer: None,
            active_modals: Vec::new(),
            focused_input: None,
            notification: cx.new(|cx| NotificationList::new(window, cx)),
            drawer_size: None,
            view,
        }
    }

    pub fn update<F>(window: &mut Window, cx: &mut App, f: F)
    where
        F: FnOnce(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    {
        if let Some(Some(root)) = window.root::<Root>() {
            root.update(cx, |root, cx| f(root, window, cx));
        }
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
    pub fn render_notification_layer(
        window: &mut Window,
        cx: &mut App,
    ) -> Option<impl IntoElement> {
        let root = window.root::<Root>()??;

        let active_drawer_placement = root.read(cx).active_drawer.clone().map(|d| d.placement);

        let (mt, mr) = match active_drawer_placement {
            Some(Placement::Right) => (None, root.read(cx).drawer_size),
            Some(Placement::Top) => (root.read(cx).drawer_size, None),
            _ => (None, None),
        };

        Some(
            div()
                .when_some(mt, |this, offset| this.mt(offset))
                .when_some(mr, |this, offset| this.mr(offset))
                .child(root.read(cx).notification.clone()),
        )
    }

    /// Render the Drawer layer.
    pub fn render_drawer_layer(window: &mut Window, cx: &mut App) -> Option<impl IntoElement> {
        let root = window.root::<Root>()??;

        if let Some(active_drawer) = root.read(cx).active_drawer.clone() {
            let mut drawer = Drawer::new(window, cx);
            drawer = (active_drawer.builder)(drawer, window, cx);
            drawer.focus_handle = active_drawer.focus_handle.clone();
            drawer.placement = active_drawer.placement;

            let drawer_size = drawer.size;

            return Some(
                div().relative().child(drawer).child(
                    canvas(
                        move |_, _, cx| root.update(cx, |r, _| r.drawer_size = Some(drawer_size)),
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                ),
            );
        }

        None
    }

    /// Render the Modal layer.
    pub fn render_modal_layer(window: &mut Window, cx: &mut App) -> Option<impl IntoElement> {
        let root = window.root::<Root>()??;

        let active_modals = root.read(cx).active_modals.clone();
        let mut has_overlay = false;

        if active_modals.is_empty() {
            return None;
        }

        Some(
            div().children(active_modals.iter().enumerate().map(|(i, active_modal)| {
                let mut modal = Modal::new(window, cx);

                modal = (active_modal.builder)(modal, window, cx);
                modal.layer_ix = i;
                // Give the modal the focus handle, because `modal` is a temporary value, is not possible to
                // keep the focus handle in the modal.
                //
                // So we keep the focus handle in the `active_modal`, this is owned by the `Root`.
                modal.focus_handle = active_modal.focus_handle.clone();

                // Keep only have one overlay, we only render the first modal with overlay.
                if has_overlay {
                    modal.overlay_visible = false;
                }
                if modal.has_overlay() {
                    has_overlay = true;
                }

                modal
            })),
        )
    }

    /// Return the root view of the Root.
    pub fn view(&self) -> &AnyView {
        &self.view
    }
}

impl Render for Root {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let base_font_size = cx.theme().font_size;
        window.set_rem_size(base_font_size);

        window_border().child(
            div()
                .id("root")
                .relative()
                .size_full()
                .font_family(".SystemUIFont")
                .bg(cx.theme().background)
                .text_color(cx.theme().foreground)
                .child(self.view.clone()),
        )
    }
}
