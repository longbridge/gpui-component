use std::{
    any::TypeId,
    collections::{HashMap, VecDeque},
    rc::Rc,
    time::Duration,
};

use gpui::{
    Animation, AnimationExt, AnyElement, App, AppContext, ClickEvent, Context, DismissEvent,
    ElementId, Entity, EventEmitter, InteractiveElement as _, IntoElement, ParentElement as _,
    Pixels, Render, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    Subscription, Window, div, prelude::FluentBuilder, px,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smol::Timer;

use crate::{
    ActiveTheme as _, Anchor, Edges, Icon, IconName, Sizable as _, StyledExt, TITLE_BAR_HEIGHT,
    animation::cubic_bezier,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};

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
    closing: bool,
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

impl From<&'static str> for Notification {
    fn from(s: &'static str) -> Self {
        Self::new().message(s)
    }
}

impl From<(NotificationType, &'static str)> for Notification {
    fn from((type_, content): (NotificationType, &'static str)) -> Self {
        Self::new().message(content).with_type(type_)
    }
}

impl From<(NotificationType, SharedString)> for Notification {
    fn from((type_, content): (NotificationType, SharedString)) -> Self {
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
            closing: false,
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
    /// let notification = Notification::new("Hello").id::<MyNotificationKind>();
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

    /// Set the action button of the notification.
    pub fn action<F>(mut self, action: F) -> Self
    where
        F: Fn(&mut Self, &mut Window, &mut Context<Self>) -> Button + 'static,
    {
        self.action_builder = Some(Rc::new(action));
        self
    }

    /// Dismiss the notification.
    pub fn dismiss(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if self.closing {
            return;
        }
        self.closing = true;
        cx.notify();

        // Dismiss the notification after 0.15s to show the animation.
        cx.spawn(async move |view, cx| {
            Timer::after(Duration::from_secs_f32(0.15)).await;
            cx.update(|cx| {
                if let Some(view) = view.upgrade() {
                    view.update(cx, |view, cx| {
                        view.closing = false;
                        cx.emit(DismissEvent);
                    });
                }
            })
        })
        .detach()
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

        let closing = self.closing;
        let icon = match self.type_ {
            None => self.icon.clone(),
            Some(type_) => Some(type_.icon(cx)),
        };
        let has_icon = icon.is_some();
        let placement = cx.theme().notification.placement;

        h_flex()
            .id("notification")
            .group("")
            .occlude()
            .relative()
            .w_112()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .rounded(cx.theme().radius_lg)
            .shadow_md()
            .py_3p5()
            .px_4()
            .gap_3()
            .refine_style(&self.style)
            .when_some(icon, |this, icon| {
                this.child(div().absolute().py_3p5().left_4().child(icon))
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
            .when_some(self.on_click.clone(), |this, on_click| {
                this.on_click(cx.listener(move |view, event, window, cx| {
                    view.dismiss(window, cx);
                    on_click(event, window, cx);
                }))
            })
            .child(
                h_flex()
                    .absolute()
                    .top_3p5()
                    .right_3p5()
                    .invisible()
                    .group_hover("", |this| this.visible())
                    .child(
                        Button::new("close")
                            .icon(IconName::Close)
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _, window, cx| this.dismiss(window, cx))),
                    ),
            )
            .with_animation(
                ElementId::NamedInteger("slide-down".into(), closing as u64),
                Animation::new(Duration::from_secs_f64(0.25))
                    .with_easing(cubic_bezier(0.4, 0., 0.2, 1.)),
                move |this, delta| {
                    if closing {
                        let opacity = 1. - delta;
                        let that = this
                            .shadow_none()
                            .opacity(opacity)
                            .when(opacity < 0.85, |this| this.shadow_none());
                        match placement {
                            Anchor::TopRight | Anchor::BottomRight => {
                                let x_offset = px(0.) + delta * px(45.);
                                that.left(px(0.) + x_offset)
                            }
                            Anchor::TopLeft | Anchor::BottomLeft => {
                                let x_offset = px(0.) - delta * px(45.);
                                that.left(px(0.) + x_offset)
                            }
                            Anchor::TopCenter => {
                                let y_offset = px(0.) - delta * px(45.);
                                that.top(px(0.) + y_offset)
                            }
                            Anchor::BottomCenter => {
                                let y_offset = px(0.) + delta * px(45.);
                                that.top(px(0.) + y_offset)
                            }
                        }
                    } else {
                        let y_offset = match placement {
                            placement if placement.is_top() => px(-45.) + delta * px(45.),
                            placement if placement.is_bottom() => px(45.) - delta * px(45.),
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
    pub(crate) notifications: VecDeque<Entity<Notification>>,
    expanded: bool,
    _subscriptions: HashMap<NotificationId, Subscription>,
}

impl NotificationList {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            notifications: VecDeque::new(),
            expanded: false,
            _subscriptions: HashMap::new(),
        }
    }

    pub fn push(
        &mut self,
        notification: impl Into<Notification>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let notification = notification.into();
        let id = notification.id.clone();
        let autohide = notification.autohide;

        // Remove the notification by id, for keep unique.
        self.notifications.retain(|note| note.read(cx).id != id);

        let notification = cx.new(|_| notification);

        self._subscriptions.insert(
            id.clone(),
            cx.subscribe(&notification, move |view, _, _: &DismissEvent, cx| {
                view.notifications.retain(|note| id != note.read(cx).id);
                view._subscriptions.remove(&id);
            }),
        );

        self.notifications.push_back(notification.clone());
        if autohide {
            // Sleep for 5 seconds to autohide the notification
            cx.spawn_in(window, async move |_, cx| {
                Timer::after(Duration::from_secs(5)).await;

                if let Err(err) =
                    notification.update_in(cx, |note, window, cx| note.dismiss(window, cx))
                {
                    tracing::error!("failed to auto hide notification: {:?}", err);
                }
            })
            .detach();
        }
        cx.notify();
    }

    pub(crate) fn close(
        &mut self,
        id: impl Into<NotificationId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id: NotificationId = id.into();
        if let Some(n) = self.notifications.iter().find(|n| n.read(cx).id == id) {
            n.update(cx, |note, cx| note.dismiss(window, cx))
        }
        cx.notify();
    }

    pub fn clear(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.notifications.clear();
        cx.notify();
    }

    pub fn notifications(&self) -> Vec<Entity<Notification>> {
        self.notifications.iter().cloned().collect()
    }
}

impl Render for NotificationList {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let size = window.viewport_size();
        let items = self.notifications.iter().rev().take(10).rev().cloned();

        let placement = cx.theme().notification.placement;
        let margins = &cx.theme().notification.margins;

        v_flex()
            .id("notification-list")
            .max_h(size.height)
            .pt(margins.top)
            .pb(margins.bottom)
            .gap_3()
            .when(
                matches!(placement, Anchor::TopRight),
                |this| this.pr(margins.right), // ignore left
            )
            .when(
                matches!(placement, Anchor::TopLeft),
                |this| this.pl(margins.left), // ignore right
            )
            .when(
                matches!(placement, Anchor::BottomLeft),
                |this| this.flex_col_reverse().pl(margins.left), // ignore right
            )
            .when(
                matches!(placement, Anchor::BottomRight),
                |this| this.flex_col_reverse().pr(margins.right), // ignore left
            )
            .when(matches!(placement, Anchor::BottomCenter), |this| {
                this.flex_col_reverse()
            })
            .on_hover(cx.listener(|view, hovered, _, cx| {
                view.expanded = *hovered;
                cx.notify()
            }))
            .children(items)
    }
}
