use std::{any::TypeId, borrow::Cow, collections::HashMap, rc::Rc, time::Duration};

use gpui::{
    Anchor, Animation, AnimationExt, AnyElement, App, AppContext, ClickEvent, Context,
    DismissEvent, ElementId, Entity, EventEmitter, FocusHandle, InteractiveElement as _,
    IntoElement, ParentElement as _, Pixels, Render, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Subscription, Window, div, prelude::FluentBuilder, px,
};
use gpui_base::{
    Toast as BaseToast, ToastManager, ToastMotion, ToastOptions, ToastStack, ToastStackState,
    ToastTransitionStatus,
};

use crate::{
    ActiveTheme as _, Edges, Icon, IconName, Sizable as _, StyledExt, TITLE_BAR_HEIGHT,
    animation::cubic_bezier,
    button::{Button, ButtonVariants as _},
    v_flex,
};

const NOTIFICATION_TRANSITION_DURATION: Duration = Duration::from_millis(400);
const NOTIFICATION_EXIT_DURATION: Duration = Duration::from_millis(200);
const NOTIFICATION_TRANSITION_OFFSET: Pixels = px(96.);
const DEFAULT_NOTIFICATION_WIDTH: Pixels = px(356.);
struct DismissRequest;

#[derive(Debug, Clone, Copy, Default)]
pub enum NotificationType {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl NotificationType {
    fn icon(&self, cx: &App) -> Icon {
        match self {
            Self::Info => Icon::new(IconName::Info).text_color(cx.theme().info),
            Self::Success => Icon::new(IconName::CircleCheck).text_color(cx.theme().success),
            Self::Warning => Icon::new(IconName::TriangleAlert).text_color(cx.theme().warning),
            Self::Error => Icon::new(IconName::CircleX).text_color(cx.theme().danger),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub(crate) enum NotificationId {
    Id(TypeId),
    IdAndElementId(TypeId, ElementId),
}

impl From<TypeId> for NotificationId {
    fn from(type_id: TypeId) -> Self {
        Self::Id(type_id)
    }
}

impl From<(TypeId, ElementId)> for NotificationId {
    fn from((type_id, id): (TypeId, ElementId)) -> Self {
        Self::IdAndElementId(type_id, id)
    }
}

/// A notification element.
pub struct Notification {
    /// The id is used make the notification unique.
    /// Then you push a notification with the same id, the previous notification will be replaced.
    ///
    /// None means the notification will be added to the end of the list.
    id: NotificationId,
    style: StyleRefinement,
    type_: Option<NotificationType>,
    title: Option<SharedString>,
    message: Option<SharedString>,
    icon: Option<Icon>,
    autohide: bool,
    action_builder: Option<Rc<dyn Fn(&mut Self, &mut Window, &mut Context<Self>) -> Button>>,
    content_builder: Option<Rc<dyn Fn(&mut Self, &mut Window, &mut Context<Self>) -> AnyElement>>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    transition_status: ToastTransitionStatus,
}

impl From<String> for Notification {
    fn from(s: String) -> Self {
        Self::new().message(s)
    }
}

impl From<SharedString> for Notification {
    fn from(s: SharedString) -> Self {
        Self::new().message(s)
    }
}

impl From<&str> for Notification {
    fn from(s: &str) -> Self {
        Self::new().message(s)
    }
}

impl<'a> From<Cow<'a, str>> for Notification {
    fn from(s: Cow<'a, str>) -> Self {
        Self::new().message(s)
    }
}

impl<T> From<(NotificationType, T)> for Notification
where
    T: Into<SharedString>,
{
    fn from((type_, content): (NotificationType, T)) -> Self {
        Self::new().message(content).with_type(type_)
    }
}

struct DefaultIdType;

impl Notification {
    /// Create a new notification.
    ///
    /// The default id is a random UUID.
    pub fn new() -> Self {
        let id: SharedString = uuid::Uuid::new_v4().to_string().into();
        let id = (TypeId::of::<DefaultIdType>(), id.into());

        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            title: None,
            message: None,
            type_: None,
            icon: None,
            autohide: true,
            action_builder: None,
            content_builder: None,
            on_click: None,
            on_close: None,
            transition_status: ToastTransitionStatus::Starting,
        }
    }

    /// Set the message of the notification, default is None.
    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Create an info notification with the given message.
    pub fn info(message: impl Into<SharedString>) -> Self {
        Self::new()
            .message(message)
            .with_type(NotificationType::Info)
    }

    /// Create a success notification with the given message.
    pub fn success(message: impl Into<SharedString>) -> Self {
        Self::new()
            .message(message)
            .with_type(NotificationType::Success)
    }

    /// Create a warning notification with the given message.
    pub fn warning(message: impl Into<SharedString>) -> Self {
        Self::new()
            .message(message)
            .with_type(NotificationType::Warning)
    }

    /// Create an error notification with the given message.
    pub fn error(message: impl Into<SharedString>) -> Self {
        Self::new()
            .message(message)
            .with_type(NotificationType::Error)
    }

    /// Set the type for unique identification of the notification.
    ///
    /// ```rs
    /// struct MyNotificationKind;
    /// let notification = Notification::new().message("Hello").id::<MyNotificationKind>();
    /// ```
    pub fn id<T: Sized + 'static>(mut self) -> Self {
        self.id = TypeId::of::<T>().into();
        self
    }

    /// Set the type and id of the notification, used to uniquely identify the notification.
    pub fn id1<T: Sized + 'static>(mut self, key: impl Into<ElementId>) -> Self {
        self.id = (TypeId::of::<T>(), key.into()).into();
        self
    }

    /// Set the title of the notification, default is None.
    ///
    /// If title is None, the notification will not have a title.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the icon of the notification.
    ///
    /// If icon is None, the notification will use the default icon of the type.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the type of the notification, default is NotificationType::Info.
    pub fn with_type(mut self, type_: NotificationType) -> Self {
        self.type_ = Some(type_);
        self
    }

    /// Set the auto hide of the notification, default is true.
    pub fn autohide(mut self, autohide: bool) -> Self {
        self.autohide = autohide;
        self
    }

    /// Set the click callback of the notification.
    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// Set the close callback of the notification.
    ///
    /// Triggered when the notification is closed by any means
    /// (close button, middle-click, autohide, click handler, or programmatic close).
    pub fn on_close(mut self, on_close: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(on_close));
        self
    }

    /// Set the action button of the notification.
    ///
    /// When an action is set, the notification will not autohide.
    pub fn action<F>(mut self, action: F) -> Self
    where
        F: Fn(&mut Self, &mut Window, &mut Context<Self>) -> Button + 'static,
    {
        self.action_builder = Some(Rc::new(action));
        self.autohide = false;
        self
    }

    /// Dismiss the notification.
    pub fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = window;
        cx.emit(DismissRequest);
    }

    fn begin_close(&mut self, cx: &mut Context<Self>) {
        if self.transition_status != ToastTransitionStatus::Ending {
            self.transition_status = ToastTransitionStatus::Ending;
            cx.notify();
        }
    }

    fn complete_enter(&mut self, cx: &mut Context<Self>) {
        if self.transition_status == ToastTransitionStatus::Starting {
            self.transition_status = ToastTransitionStatus::Present;
            cx.notify();
        }
    }

    fn complete_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
        if let Some(on_close) = self.on_close.clone() {
            on_close(window, cx);
        }
    }

    /// Set the content of the notification.
    pub fn content(
        mut self,
        content: impl Fn(&mut Self, &mut Window, &mut Context<Self>) -> AnyElement + 'static,
    ) -> Self {
        self.content_builder = Some(Rc::new(content));
        self
    }
}

impl EventEmitter<DismissEvent> for Notification {}
impl EventEmitter<DismissRequest> for Notification {}
impl FluentBuilder for Notification {}
impl Styled for Notification {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for Notification {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self
            .content_builder
            .clone()
            .map(|builder| builder(self, window, cx));
        let action = self
            .action_builder
            .clone()
            .map(|builder| builder(self, window, cx).small().mr_3p5());

        let transition_status = self.transition_status;
        let closing = transition_status == ToastTransitionStatus::Ending;
        let icon = match self.type_ {
            None => self.icon.clone(),
            Some(type_) => Some(type_.icon(cx)),
        };
        let has_icon = icon.is_some();
        let placement = cx.theme().notification.placement;

        BaseToast::new("notification")
            .transition_status(transition_status)
            .h_flex()
            .group("")
            .occlude()
            .relative()
            .w_full()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tokens.popover)
            .rounded(cx.theme().radius_lg)
            .shadow_md()
            .py_3p5()
            .px_4()
            .gap_3()
            .refine_style(&self.style)
            .when_some(icon, |this, icon| {
                this.child(div().absolute().top(px(18.)).left_4().child(icon))
            })
            .child(
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .when(has_icon, |this| this.pl_6())
                    .when_some(self.title.clone(), |this, title| {
                        this.child(div().text_sm().font_semibold().child(title))
                    })
                    .when_some(self.message.clone(), |this, message| {
                        this.child(div().text_sm().child(message))
                    })
                    .when_some(content, |this, content| this.child(content)),
            )
            .when_some(action, |this, action| this.child(action))
            .child(
                div()
                    .absolute()
                    .top_1()
                    .right_1()
                    .invisible()
                    .group_hover("", |this| this.visible())
                    .child(
                        Button::new("close")
                            .icon(IconName::Close)
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.dismiss(window, cx);
                            })),
                    ),
            )
            .when_some(self.on_click.clone(), |this, on_click| {
                this.on_click(cx.listener(move |view, event, window, cx| {
                    view.dismiss(window, cx);
                    on_click(event, window, cx);
                }))
            })
            .on_aux_click(cx.listener(move |view, event: &ClickEvent, window, cx| {
                if event.is_middle_click() {
                    view.dismiss(window, cx);
                }
            }))
            .with_animation(
                ElementId::NamedInteger("slide-down".into(), closing as u64),
                Animation::new(if closing {
                    NOTIFICATION_EXIT_DURATION
                } else {
                    NOTIFICATION_TRANSITION_DURATION
                })
                .with_easing(cubic_bezier(0.25, 0.1, 0.25, 1.)),
                move |this, delta| {
                    if closing {
                        let opacity = 1. - delta;
                        let that = this.opacity(opacity);
                        let y_offset = match placement {
                            Anchor::TopLeft | Anchor::TopRight | Anchor::TopCenter => {
                                -delta * NOTIFICATION_TRANSITION_OFFSET
                            }
                            Anchor::BottomLeft | Anchor::BottomRight | Anchor::BottomCenter => {
                                delta * NOTIFICATION_TRANSITION_OFFSET
                            }
                            _ => px(0.),
                        };
                        that.top(y_offset)
                    } else {
                        let y_offset = match placement {
                            Anchor::TopLeft | Anchor::TopRight | Anchor::TopCenter => {
                                -NOTIFICATION_TRANSITION_OFFSET
                                    + delta * NOTIFICATION_TRANSITION_OFFSET
                            }
                            Anchor::BottomLeft | Anchor::BottomRight | Anchor::BottomCenter => {
                                NOTIFICATION_TRANSITION_OFFSET
                                    - delta * NOTIFICATION_TRANSITION_OFFSET
                            }
                            _ => px(0.),
                        };
                        let opacity = delta;
                        this.top(px(0.) + y_offset)
                            .opacity(opacity)
                            .when(opacity < 0.85, |this| this.shadow_none())
                    }
                },
            )
    }
}

/// The settings for notifications.
#[derive(Debug, Clone)]
pub struct NotificationSettings {
    /// The placement of the notification, default: [`Anchor::TopRight`]
    pub placement: Anchor,
    /// The margins of the notification with respect to the window edges.
    pub margins: Edges<Pixels>,
    /// The maximum number of notifications to show at once, default: 10
    pub max_items: usize,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        let offset = px(16.);
        Self {
            placement: Anchor::TopRight,
            margins: Edges {
                top: TITLE_BAR_HEIGHT + offset, // avoid overlap with title bar
                right: offset,
                bottom: offset,
                left: offset,
            },
            max_items: 10,
        }
    }
}

/// A list of notifications.
pub struct NotificationList {
    /// Notifications that will be auto hidden.
    pub(crate) notifications: ToastManager<NotificationId, Entity<Notification>>,
    stack_state: ToastStackState,
    focus_handle: FocusHandle,
    _subscriptions: HashMap<NotificationId, Subscription>,
}

impl NotificationList {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let view = cx.entity().downgrade();
        cx.spawn_in(window, async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                if view
                    .update_in(cx, |view, window, cx| view.advance(window, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self {
            notifications: ToastManager::new(ToastMotion::sonner()),
            stack_state: ToastStackState::default(),
            focus_handle: cx.focus_handle().tab_stop(true),
            _subscriptions: HashMap::new(),
        }
    }

    pub fn push(
        &mut self,
        notification: impl Into<Notification>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let notification = notification.into();
        let id = notification.id.clone();
        let autohide = notification.autohide;

        let notification = cx.new(|_| notification);

        let dismiss_id = id.clone();
        self._subscriptions.insert(
            id.clone(),
            cx.subscribe(&notification, move |view, _, _: &DismissRequest, cx| {
                if view
                    .notifications
                    .dismiss(&dismiss_id, cx.background_executor().now())
                {
                    if let Some(note) = view.notifications.get(&dismiss_id) {
                        note.update(cx, |note, cx| note.begin_close(cx));
                    }
                }
            }),
        );

        self.notifications.push(
            id,
            notification,
            ToastOptions {
                timeout: autohide.then_some(Duration::from_secs(5)),
            },
            cx.background_executor().now(),
        );
        cx.notify();
    }

    fn advance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let changes = self.notifications.advance(
            cx.background_executor().now(),
            self.stack_state.is_expanded() || !window.is_window_active(),
        );
        for id in changes.presented {
            if let Some(note) = self.notifications.get(&id) {
                note.update(cx, |note, cx| note.complete_enter(cx));
            }
        }
        for id in changes.ending {
            if let Some(note) = self.notifications.get(&id) {
                note.update(cx, |note, cx| note.begin_close(cx));
            }
        }
        for (id, note) in changes.removed {
            self._subscriptions.remove(&id);
            note.update(cx, |note, cx| note.complete_close(window, cx));
        }
        if changes.changed {
            cx.notify();
        }
    }

    pub(crate) fn close(
        &mut self,
        id: impl Into<NotificationId>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id: NotificationId = id.into();
        if self
            .notifications
            .dismiss(&id, cx.background_executor().now())
        {
            if let Some(n) = self.notifications.get(&id) {
                n.update(cx, |note, cx| note.begin_close(cx))
            }
        }
        cx.notify();
    }

    /// Close all notifications whose id matches the given [`TypeId`], regardless of
    /// whether they were registered via [`Notification::id`] or [`Notification::id1`].
    pub(crate) fn close_by_type(
        &mut self,
        type_id: TypeId,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let matched: Vec<_> = self
            .notifications
            .iter()
            .filter(|(id, _, _)| match id {
                NotificationId::Id(t) | NotificationId::IdAndElementId(t, _) => *t == type_id,
            })
            .map(|(_, notification, _)| notification)
            .cloned()
            .collect();
        for n in matched {
            let id = n.read(cx).id.clone();
            if self
                .notifications
                .dismiss(&id, cx.background_executor().now())
            {
                n.update(cx, |note, cx| note.begin_close(cx));
            }
        }
        cx.notify();
    }

    pub fn clear(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        for id in self
            .notifications
            .dismiss_all(cx.background_executor().now())
        {
            if let Some(note) = self.notifications.get(&id) {
                note.update(cx, |note, cx| note.begin_close(cx));
            }
        }
        cx.notify();
    }

    pub fn notifications(&self) -> Vec<Entity<Notification>> {
        self.notifications
            .iter()
            .map(|(_, value, _)| value.clone())
            .collect()
    }
}

impl Render for NotificationList {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let size = window.viewport_size();
        let max_items = cx.theme().notification.max_items;
        let items = self
            .notifications
            .visible(max_items)
            .map(|(id, value, _)| (id.clone(), value.clone()))
            .collect::<Vec<_>>();

        let placement = cx.theme().notification.placement;
        let stack = items.into_iter().fold(
            ToastStack::new("notification-list", self.stack_state.clone()),
            |stack, (id, item)| stack.item(format!("{id:?}"), item),
        );

        stack
            .placement(placement)
            .focus_handle(self.focus_handle.clone())
            .v_flex()
            .w(DEFAULT_NOTIFICATION_WIDTH)
            .max_h(size.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use gpui::{TestAppContext, VisualTestContext};

    struct FooKind;
    struct BarKind;

    struct TestRoot {
        list: Entity<NotificationList>,
        other_focus: FocusHandle,
    }

    impl Render for TestRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .child(self.list.clone())
                .child(div().track_focus(&self.other_focus))
        }
    }

    fn ids(list: &Entity<NotificationList>, cx: &mut VisualTestContext) -> Vec<NotificationId> {
        list.read_with(cx, |l, cx| {
            l.notifications
                .iter()
                .map(|(_, n, _)| n.read(cx).id.clone())
                .collect()
        })
    }

    /// Drive the dismiss animation timer + propagate the resulting `DismissEvent`
    /// so that closed notifications are removed from the list.
    fn flush_dismiss(cx: &mut VisualTestContext) {
        cx.background_executor
            .advance_clock(NOTIFICATION_EXIT_DURATION + Duration::from_millis(50));
        cx.run_until_parked();
    }

    #[gpui::test]
    fn closing_toast_stays_mounted_until_its_transition_finishes(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(Theme::default()));
        let (root, cx) = cx.add_window_view(|window, cx| TestRoot {
            list: cx.new(|cx| NotificationList::new(window, cx)),
            other_focus: cx.focus_handle(),
        });
        cx.update(|window, _| window.activate_window());
        let list = root.read_with(cx, |root, _| root.list.clone());

        list.update_in(cx, |list, window, cx| {
            list.push(
                Notification::info("closing")
                    .id::<FooKind>()
                    .autohide(false),
                window,
                cx,
            );
            list.close(TypeId::of::<FooKind>(), window, cx);
        });

        cx.background_executor
            .advance_clock(NOTIFICATION_EXIT_DURATION - Duration::from_millis(1));
        cx.run_until_parked();
        assert_eq!(ids(&list, cx).len(), 1);

        cx.background_executor
            .advance_clock(Duration::from_millis(1));
        cx.run_until_parked();
        assert!(ids(&list, cx).is_empty());
    }

    #[gpui::test]
    fn focus_and_inactive_window_pause_autohide_and_present_phase_is_projected(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| cx.set_global(Theme::default()));
        let (root, cx) = cx.add_window_view(|window, cx| TestRoot {
            list: cx.new(|cx| NotificationList::new(window, cx)),
            other_focus: cx.focus_handle(),
        });
        let list = root.read_with(cx, |root, _| root.list.clone());
        list.update_in(cx, |list, window, cx| {
            list.push(Notification::info("paused").id::<FooKind>(), window, cx);
        });

        cx.background_executor
            .advance_clock(NOTIFICATION_TRANSITION_DURATION);
        cx.run_until_parked();
        let note = list.read_with(cx, |list, _| {
            list.notifications
                .get(&NotificationId::from(TypeId::of::<FooKind>()))
                .unwrap()
                .clone()
        });
        assert_eq!(
            note.read_with(cx, |note, _| note.transition_status),
            ToastTransitionStatus::Present
        );

        cx.background_executor.advance_clock(Duration::from_secs(5));
        cx.run_until_parked();
        assert_eq!(
            note.read_with(cx, |note, _| note.transition_status),
            ToastTransitionStatus::Present
        );

        let list_focus = list.read_with(cx, |list, _| list.focus_handle.clone());
        cx.update(|window, cx| {
            window.activate_window();
            list_focus.focus(window, cx);
            window.draw(cx).clear(cx);
        });
        cx.background_executor.advance_clock(Duration::from_secs(5));
        cx.run_until_parked();
        assert_eq!(
            note.read_with(cx, |note, _| note.transition_status),
            ToastTransitionStatus::Present
        );

        let other_focus = root.read_with(cx, |root, _| root.other_focus.clone());
        cx.update(|window, cx| {
            other_focus.focus(window, cx);
            window.draw(cx).clear(cx);
        });
        assert!(!list.read_with(cx, |list, _| list.stack_state.is_expanded()));
        cx.background_executor
            .advance_clock(Duration::from_secs(5) + Duration::from_millis(50));
        cx.run_until_parked();
        assert_eq!(
            note.read_with(cx, |note, _| note.transition_status),
            ToastTransitionStatus::Ending
        );
    }

    #[gpui::test]
    fn close_by_type_removes_id_and_all_id1_of_same_type(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(Theme::default()));
        let (root, cx) = cx.add_window_view(|window, cx| TestRoot {
            list: cx.new(|cx| NotificationList::new(window, cx)),
            other_focus: cx.focus_handle(),
        });
        let list = root.read_with(cx, |r, _| r.list.clone());

        list.update_in(cx, |list, window, cx| {
            list.push(
                Notification::info("plain").id::<FooKind>().autohide(false),
                window,
                cx,
            );
            list.push(
                Notification::info("a").id1::<FooKind>(1).autohide(false),
                window,
                cx,
            );
            list.push(
                Notification::info("b").id1::<FooKind>(2).autohide(false),
                window,
                cx,
            );
            list.push(
                Notification::info("bar").id::<BarKind>().autohide(false),
                window,
                cx,
            );
        });
        cx.run_until_parked();
        assert_eq!(ids(&list, cx).len(), 4);

        list.update_in(cx, |list, window, cx| {
            list.close_by_type(TypeId::of::<FooKind>(), window, cx);
        });
        flush_dismiss(cx);

        let remaining = ids(&list, cx);
        assert_eq!(
            remaining,
            vec![NotificationId::Id(TypeId::of::<BarKind>())],
            "only the BarKind notification should survive"
        );
    }

    #[gpui::test]
    fn close_with_id_and_element_id_removes_only_matching_key(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(Theme::default()));
        let (root, cx) = cx.add_window_view(|window, cx| TestRoot {
            list: cx.new(|cx| NotificationList::new(window, cx)),
            other_focus: cx.focus_handle(),
        });
        let list = root.read_with(cx, |r, _| r.list.clone());

        list.update_in(cx, |list, window, cx| {
            list.push(
                Notification::info("a").id1::<FooKind>(1).autohide(false),
                window,
                cx,
            );
            list.push(
                Notification::info("b").id1::<FooKind>(2).autohide(false),
                window,
                cx,
            );
            list.push(
                Notification::info("plain").id::<FooKind>().autohide(false),
                window,
                cx,
            );
        });

        list.update_in(cx, |list, window, cx| {
            list.close(
                (TypeId::of::<FooKind>(), ElementId::from(1usize)),
                window,
                cx,
            );
        });
        flush_dismiss(cx);

        let remaining = ids(&list, cx);
        assert_eq!(remaining.len(), 2);
        assert!(remaining.contains(&NotificationId::IdAndElementId(
            TypeId::of::<FooKind>(),
            ElementId::from(2usize),
        )));
        assert!(remaining.contains(&NotificationId::Id(TypeId::of::<FooKind>())));
    }

    #[gpui::test]
    fn close_with_only_type_id_does_not_match_id1_entries(cx: &mut TestAppContext) {
        // The plain `close(TypeId)` form (used by the legacy code path) must keep
        // its narrow semantics: it only matches `NotificationId::Id`, not
        // `NotificationId::IdAndElementId`. The new `close_by_type` is the broad form.
        cx.update(|cx| cx.set_global(Theme::default()));
        let (root, cx) = cx.add_window_view(|window, cx| TestRoot {
            list: cx.new(|cx| NotificationList::new(window, cx)),
            other_focus: cx.focus_handle(),
        });
        let list = root.read_with(cx, |r, _| r.list.clone());

        list.update_in(cx, |list, window, cx| {
            list.push(
                Notification::info("a").id1::<FooKind>(1).autohide(false),
                window,
                cx,
            );
        });

        list.update_in(cx, |list, window, cx| {
            list.close(TypeId::of::<FooKind>(), window, cx);
        });
        flush_dismiss(cx);

        assert_eq!(ids(&list, cx).len(), 1, "id1 entry should remain untouched");
    }

    #[gpui::test]
    fn close_by_type_with_no_match_is_noop(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(Theme::default()));
        let (root, cx) = cx.add_window_view(|window, cx| TestRoot {
            list: cx.new(|cx| NotificationList::new(window, cx)),
            other_focus: cx.focus_handle(),
        });
        let list = root.read_with(cx, |r, _| r.list.clone());

        list.update_in(cx, |list, window, cx| {
            list.push(
                Notification::info("bar").id::<BarKind>().autohide(false),
                window,
                cx,
            );
        });

        list.update_in(cx, |list, window, cx| {
            list.close_by_type(TypeId::of::<FooKind>(), window, cx);
        });
        flush_dismiss(cx);

        assert_eq!(ids(&list, cx).len(), 1);
    }
}
