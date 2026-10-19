use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use gpui_kit::{
    AnyElement, App, AsyncWindowContext, IntoElement, ParentElement, SharedString, Window,
    component::{
        WindowExt,
        button::Button,
        notification::{Notification, NotificationDelivery, NotificationType},
    },
    div,
    prelude::FluentBuilder,
};
use nsh_core::ui::{Children, Event, PropEnum, Props, Slot};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};

#[derive(Clone, Copy, Default, PartialEq, PropEnum)]
enum NotificationVariant {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}
impl From<NotificationVariant> for NotificationType {
    fn from(value: NotificationVariant) -> Self {
        match value {
            NotificationVariant::Info => Self::Info,
            NotificationVariant::Success => Self::Success,
            NotificationVariant::Warning => Self::Warning,
            NotificationVariant::Error => Self::Error,
        }
    }
}
#[derive(Clone, PartialEq, Props)]
struct NotificationProps {
    #[prop(default)]
    open: bool,
    title: Option<String>,
    #[prop(default)]
    message: String,
    #[prop(default)]
    variant: NotificationVariant,
    #[prop(default = true)]
    autohide: bool,
    action_label: Option<String>,
    content: Option<Slot>,
    /// Content inside the native action button; requires actionLabel.
    action: Option<Slot>,
    children: Children,
}
#[derive(Event)]
enum NotificationEvent {
    OpenChange(bool),
    Action,
    // The shared typed prop codec emits this event without constructing the adapter enum.
    #[allow(dead_code)]
    Error(String),
}

pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeNotification>(
        "gpui-component.Notification",
        "Notification",
        NativeNotification::new,
    );
    registry
        .metadata()
        .native_controlled("gpui-component.Notification", &[("openChange", &["open"])]);
}

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

struct Session {
    alive: bool,
    open: bool,
    generation: u64,
    revision: u64,
    requested_open: Option<bool>,
}

struct NativeNotification {
    id: usize,
    context: TypedComponentContext<NotificationEvent>,
    session: Rc<RefCell<Session>>,
    dirty: bool,
    window: Option<AsyncWindowContext>,
}

impl NativeNotification {
    fn new(context: TypedComponentContext<NotificationEvent>) -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            context,
            session: Rc::new(RefCell::new(Session {
                alive: true,
                open: false,
                generation: 0,
                revision: 0,
                requested_open: None,
            })),
            dirty: false,
            window: None,
        }
    }
}

impl TypedNativeComponent for NativeNotification {
    type Props = NotificationProps;
    type Event = NotificationEvent;
    fn render(
        &mut self,
        props: &NotificationProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        self.window.get_or_insert_with(|| window.to_async(cx));
        if !std::mem::take(&mut self.dirty) {
            return None;
        }
        let revision = {
            let mut session = self.session.borrow_mut();
            session.revision += 1;
            session.revision
        };
        let (id, session, context) = (self.id, self.session.clone(), self.context.clone());
        let (title, message, variant, autohide) = (
            props.title.clone().map(SharedString::from),
            SharedString::from(props.message.clone()),
            NotificationType::from(props.variant),
            props.autohide,
        );
        let action_label = props.action_label.clone().map(SharedString::from);
        window.defer(cx, move |window, cx| {
            let generation = {
                let mut session = session.borrow_mut();
                if !session.alive || session.revision != revision {
                    return;
                }
                match session.requested_open.take() {
                    Some(false) => {
                        session.open = false;
                        session.generation += 1;
                        drop(session);
                        window.remove_notification1::<NativeNotification>(id, cx);
                        return;
                    }
                    Some(true) => session.open = true,
                    None if !session.open => return,
                    None => {}
                }
                session.generation += 1;
                session.generation
            };
            let weak = Rc::downgrade(&session);
            let content_context = context.clone();
            let action_context = context.clone();
            let action_session = weak.clone();
            window.push_notification(
                Notification::new()
                    .id1::<NativeNotification>(id)
                    .message(message)
                    .when_some(title, |note, title| note.title(title))
                    .with_type(variant)
                    .autohide(autohide)
                    .delivery(NotificationDelivery::InApp)
                    .content(move |_, _, _| {
                        div()
                            .children(crate::bridge::children(&content_context))
                            .children(crate::bridge::slot(&content_context, "content"))
                            .into_any_element()
                    })
                    .when_some(action_label, |note, label| {
                        note.action(move |_, _, _| {
                            let context = action_context.clone();
                            let session = action_session.clone();
                            Button::new("notification-action")
                                .label(label.clone())
                                .children(crate::bridge::slot(&context, "action"))
                                .on_click(move |_, _, _| {
                                    if session.upgrade().is_some_and(|session| {
                                        let session = session.borrow();
                                        session.alive
                                            && session.open
                                            && session.generation == generation
                                    }) {
                                        context.emit(NotificationEvent::Action);
                                    }
                                })
                        })
                    })
                    .on_close(move |_, _| {
                        let Some(session) = weak.upgrade() else {
                            return;
                        };
                        {
                            let mut session = session.borrow_mut();
                            if !session.alive || !session.open || session.generation != generation {
                                return;
                            }
                            session.open = false;
                        }
                        context.emit(NotificationEvent::OpenChange(false));
                    }),
                cx,
            );
        });
        None
    }

    fn props_changed(
        &mut self,
        props: &NotificationProps,
        changed: &[String],
        _: &mut Window,
        _: &mut App,
    ) {
        if changed.iter().any(|name| name == "open") {
            self.session.borrow_mut().requested_open = Some(props.open);
        }
        self.dirty = true;
    }
}

impl Drop for NativeNotification {
    fn drop(&mut self) {
        self.session.borrow_mut().alive = false;
        if let Some(window) = self.window.as_ref() {
            let id = self.id;
            window
                .spawn(async move |cx| {
                    let _ = cx.update(|window, cx| {
                        window.remove_notification1::<NativeNotification>(id, cx)
                    });
                })
                .detach();
        }
    }
}
