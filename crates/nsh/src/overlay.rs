use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use gpui_kit::{
    AnyElement, App, AsyncWindowContext, IntoElement, ParentElement, SharedString, Subscription,
    Window,
    component::{
        Placement, WindowExt,
        button::Button,
        dialog::{
            DialogAction, DialogButtonProps, DialogClose, DialogDescription, DialogFooter,
            DialogHandle,
        },
        sheet::SheetHandle,
    },
    prelude::FluentBuilder,
    px,
};
use nsh_core::ui::{Children, Event, PropEnum, PropError, Props, Slot};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use std::marker::PhantomData;

/// Native modal decisions and their accepted controlled state.
#[derive(Event)]
enum DialogEvent {
    OpenChange(bool),
    Confirm,
    Cancel,
    // The shared typed prop codec emits validation errors without constructing this enum.
    #[allow(dead_code)]
    Error(String),
}

#[derive(Event)]
enum SheetEvent {
    OpenChange(bool),
    // The shared typed prop codec emits validation errors without constructing this enum.
    #[allow(dead_code)]
    Error(String),
}

trait OverlayEvents: Event + Sized {
    fn open_change(open: bool) -> Self;
    fn decision(confirm: bool) -> Option<Self>;
}

impl OverlayEvents for DialogEvent {
    fn open_change(open: bool) -> Self {
        Self::OpenChange(open)
    }
    fn decision(confirm: bool) -> Option<Self> {
        Some(if confirm { Self::Confirm } else { Self::Cancel })
    }
}

impl OverlayEvents for SheetEvent {
    fn open_change(open: bool) -> Self {
        Self::OpenChange(open)
    }
    fn decision(_: bool) -> Option<Self> {
        None
    }
}

trait OverlayProps: Props {
    type Event: OverlayEvents;
    const SURFACE: Surface;
    fn open(&self) -> bool;
    fn options(&self) -> Options;
}

// Dialog and Alert share the decision contract, with different native defaults.
macro_rules! dialog_props {
    ($name:ident, $surface:ident, $close_button:literal, {$($extra:tt)*}, $closable:expr) => {
        #[derive(Clone, PartialEq, Props)]
        struct $name {
            #[prop(default)] open: bool,
            #[prop(default)] title: String,
            #[prop(default)] description: String,
            confirm_label: Option<String>,
            cancel_label: Option<String>,
            #[prop(default)] show_cancel: bool,
            #[prop(default = $close_button)] close_button: bool,
            #[prop(default = true)] close_on_confirm: bool,
            #[prop(default = true)] close_on_cancel: bool,
            #[prop(default = true)] keyboard: bool,
            size: Option<f32>,
            $($extra)*
            content: Option<Slot>,
            footer: Option<Slot>,
            children: Children,
        }
        impl OverlayProps for $name {
            type Event = DialogEvent;
            const SURFACE: Surface = Surface::$surface;
            fn open(&self) -> bool { self.open }
            fn options(&self) -> Options {
                Options {
                    title: self.title.clone().into(),
                    description: self.description.clone().into(),
                    confirm_label: self.confirm_label.clone().map(Into::into),
                    cancel_label: self.cancel_label.clone().map(Into::into),
                    show_cancel: self.show_cancel,
                    close_button: self.close_button,
                    close_on_confirm: self.close_on_confirm,
                    close_on_cancel: self.close_on_cancel,
                    keyboard: self.keyboard,
                    size: self.size,
                    overlay_closable: ($closable)(self),
                    ..Options::default()
                }
            }
        }
    };
}

dialog_props!(DialogProps, Dialog, true, {
    #[prop(default = true)] overlay_closable: bool,
    header: Option<Slot>,
}, |props: &DialogProps| props.overlay_closable);
dialog_props!(
    AlertDialogProps,
    Alert,
    false,
    {},
    |_: &AlertDialogProps| false
);

#[derive(Clone, Copy, Default, PartialEq, PropEnum)]
enum SheetPlacement {
    Top,
    #[default]
    Right,
    Bottom,
    Left,
}

impl From<SheetPlacement> for Placement {
    fn from(placement: SheetPlacement) -> Self {
        match placement {
            SheetPlacement::Top => Self::Top,
            SheetPlacement::Right => Self::Right,
            SheetPlacement::Bottom => Self::Bottom,
            SheetPlacement::Left => Self::Left,
        }
    }
}

#[derive(Clone, PartialEq, Props)]
struct SheetProps {
    #[prop(default)]
    open: bool,
    #[prop(default)]
    title: String,
    #[prop(default)]
    description: String,
    #[prop(default)]
    placement: SheetPlacement,
    size: Option<f32>,
    #[prop(default = true)]
    resizable: bool,
    #[prop(default = true)]
    overlay_closable: bool,
    content: Option<Slot>,
    footer: Option<Slot>,
    children: Children,
}

impl OverlayProps for SheetProps {
    type Event = SheetEvent;
    const SURFACE: Surface = Surface::Sheet;
    fn open(&self) -> bool {
        self.open
    }
    fn options(&self) -> Options {
        Options {
            title: self.title.clone().into(),
            description: self.description.clone().into(),
            placement: self.placement.into(),
            size: self.size,
            resizable: self.resizable,
            overlay_closable: self.overlay_closable,
            ..Options::default()
        }
    }
}

pub(super) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeOverlay<DialogProps>>(
        "gpui-component.Dialog",
        "Dialog",
        NativeOverlay::new,
    );
    registry.register::<NativeOverlay<AlertDialogProps>>(
        "gpui-component.AlertDialog",
        "AlertDialog",
        NativeOverlay::new,
    );
    registry.register::<NativeOverlay<SheetProps>>(
        "gpui-component.Sheet",
        "Sheet",
        NativeOverlay::new,
    );
    for name in [
        "gpui-component.Dialog",
        "gpui-component.AlertDialog",
        "gpui-component.Sheet",
    ] {
        registry
            .metadata()
            .native_controlled(name, &[("openChange", &["open"])]);
    }
}

#[derive(Clone, Copy)]
enum Surface {
    Dialog,
    Alert,
    Sheet,
}

#[derive(Clone)]
struct Options {
    title: SharedString,
    description: SharedString,
    confirm_label: Option<SharedString>,
    cancel_label: Option<SharedString>,
    show_cancel: bool,
    close_button: bool,
    close_on_confirm: bool,
    close_on_cancel: bool,
    overlay_closable: bool,
    keyboard: bool,
    size: Option<f32>,
    placement: Placement,
    resizable: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            title: SharedString::default(),
            description: SharedString::default(),
            confirm_label: None,
            cancel_label: None,
            show_cancel: false,
            close_button: true,
            close_on_confirm: true,
            close_on_cancel: true,
            overlay_closable: true,
            keyboard: true,
            size: None,
            placement: Placement::Right,
            resizable: true,
        }
    }
}

impl Options {
    fn footer(&self) -> DialogFooter {
        DialogFooter::new()
            .when(self.show_cancel, |footer| {
                footer.child(DialogClose::new().trigger(|button| {
                    button.label(self.cancel_label.clone().unwrap_or_else(|| "Cancel".into()))
                }))
            })
            .child(DialogAction::new().child(
                Button::new("ok").label(self.confirm_label.clone().unwrap_or_else(|| "OK".into())),
            ))
    }

    fn button_props(&self) -> DialogButtonProps {
        let props = DialogButtonProps::default().show_cancel(self.show_cancel);
        let props = match self.confirm_label.clone() {
            Some(label) => props.ok_text(label),
            None => props,
        };
        match self.cancel_label.clone() {
            Some(label) => props.cancel_text(label),
            None => props,
        }
    }
}

#[derive(Clone)]
enum Handle {
    Dialog(DialogHandle),
    Sheet(SheetHandle),
}

impl Handle {
    fn is_open(&self, window: &Window, cx: &App) -> bool {
        match self {
            Self::Dialog(handle) => handle.is_open(window, cx),
            Self::Sheet(handle) => handle.is_open(window, cx),
        }
    }

    fn close(&self, window: &mut Window, cx: &mut App) {
        match self {
            Self::Dialog(handle) => handle.close(window, cx),
            Self::Sheet(handle) => handle.close(window, cx),
        }
    }

    fn refresh(&self, options: &Options, window: &mut Window, cx: &mut App) {
        match self {
            Self::Dialog(handle) => handle.refresh(window, cx),
            Self::Sheet(handle) => {
                handle.set_placement(options.placement, window, cx);
                handle.refresh(window, cx);
            }
        }
    }

    fn on_closed(
        &self,
        window: &mut Window,
        cx: &mut App,
        callback: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Subscription {
        match self {
            Self::Dialog(handle) => handle.on_closed(window, cx, callback),
            Self::Sheet(handle) => handle.on_closed(window, cx, callback),
        }
    }
}

struct Session {
    alive: bool,
    revision: u64,
    generation: u64,
    closing: bool,
    handle: Option<Handle>,
    closed: Option<Subscription>,
    requested_open: Option<bool>,
}

fn handle_decision<E: OverlayEvents>(
    session: &Weak<RefCell<Session>>,
    generation: u64,
    context: &TypedComponentContext<E>,
    confirm: bool,
    close: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(session) = session.upgrade() else {
        return;
    };
    let handle = {
        let session = session.borrow();
        if !session.alive || session.closing || session.generation != generation {
            return;
        }
        session.handle.clone()
    };
    let Some(handle) = handle.filter(|handle| handle.is_open(window, cx)) else {
        return;
    };
    if close {
        session.borrow_mut().closing = true;
    }
    if let Some(event) = E::decision(confirm) {
        context.emit(event);
    }
    if close {
        // Closing an owned resource must never pop a newly opened host dialog
        // through a stale callback from the previous painted frame.
        handle.close(window, cx);
    }
}

struct NativeOverlay<P: OverlayProps> {
    surface: Surface,
    context: TypedComponentContext<P::Event>,
    options: Rc<RefCell<Options>>,
    session: Rc<RefCell<Session>>,
    window: Option<AsyncWindowContext>,
    dirty: bool,
    props: PhantomData<P>,
}

impl<P: OverlayProps> NativeOverlay<P> {
    fn new(context: TypedComponentContext<P::Event>) -> Self {
        Self {
            surface: P::SURFACE,
            context,
            options: Rc::new(RefCell::new(P::initial().options())),
            session: Rc::new(RefCell::new(Session {
                alive: true,
                revision: 0,
                generation: 0,
                closing: false,
                handle: None,
                closed: None,
                requested_open: None,
            })),
            window: None,
            dirty: false,
            props: PhantomData,
        }
    }

    fn schedule(&mut self, window: &mut Window, cx: &mut App) {
        self.window.get_or_insert_with(|| window.to_async(cx));
        if !std::mem::take(&mut self.dirty) {
            return;
        }
        let revision = {
            let mut session = self.session.borrow_mut();
            session.revision += 1;
            session.revision
        };
        let session = self.session.clone();
        let options = self.options.clone();
        let context = self.context.clone();
        let surface = self.surface;
        // Native props arrive during prepaint. Root ownership changes must run
        // after that phase, once the complete batch of props is available.
        window.defer(cx, move |window, cx| {
            let requested_open = {
                let mut session = session.borrow_mut();
                if !session.alive || session.revision != revision {
                    return;
                }
                session.requested_open.take()
            };
            if requested_open == Some(false) {
                let handle = {
                    let mut session = session.borrow_mut();
                    session.closed.take();
                    session.handle.take()
                };
                if let Some(handle) = handle {
                    handle.close(window, cx);
                }
                return;
            }
            let handle = session.borrow().handle.clone();
            if let Some(handle) = handle.filter(|handle| handle.is_open(window, cx)) {
                handle.refresh(&options.borrow(), window, cx);
                return;
            }
            // Changing title/content after a user dismissal must not reopen it.
            if requested_open != Some(true) {
                return;
            }
            let generation = {
                let mut session = session.borrow_mut();
                session.closed.take();
                session.generation += 1;
                session.closing = false;
                session.generation
            };
            let close_context = context.clone();
            let handle = match surface {
                Surface::Dialog => {
                    let weak = Rc::downgrade(&session);
                    Handle::Dialog(window.open_dialog_with_handle(cx, move |dialog, _, _| {
                        let options = options.borrow();
                        let confirm_context = context.clone();
                        let cancel_context = context.clone();
                        let confirm_session = weak.clone();
                        let cancel_session = weak.clone();
                        let close_on_confirm = options.close_on_confirm;
                        let close_on_cancel = options.close_on_cancel;
                        dialog
                            .when_some(crate::bridge::slot(&context, "header"), |dialog, header| {
                                dialog.header(header)
                            })
                            .title(
                                crate::bridge::slot(&context, "title")
                                    .unwrap_or_else(|| options.title.clone().into_any_element()),
                            )
                            .children(crate::bridge::children(&context))
                            .children(crate::bridge::slot(&context, "content"))
                            .footer(
                                crate::bridge::slot(&context, "footer")
                                    .unwrap_or_else(|| options.footer().into_any_element()),
                            )
                            .when(!options.description.is_empty(), |dialog| {
                                dialog.child(
                                    DialogDescription::new().child(options.description.clone()),
                                )
                            })
                            .button_props(options.button_props())
                            .close_button(options.close_button)
                            .overlay_closable(options.overlay_closable)
                            .keyboard(options.keyboard)
                            .when_some(options.size, |dialog, width| dialog.width(px(width)))
                            .on_ok(move |_, window, cx| {
                                handle_decision(
                                    &confirm_session,
                                    generation,
                                    &confirm_context,
                                    true,
                                    close_on_confirm,
                                    window,
                                    cx,
                                );
                                false
                            })
                            .on_cancel(move |_, window, cx| {
                                handle_decision(
                                    &cancel_session,
                                    generation,
                                    &cancel_context,
                                    false,
                                    close_on_cancel,
                                    window,
                                    cx,
                                );
                                false
                            })
                    }))
                }
                Surface::Alert => {
                    let weak = Rc::downgrade(&session);
                    Handle::Dialog(
                        window.open_alert_dialog_with_handle(cx, move |dialog, _, _| {
                            let options = options.borrow();
                            let confirm_context = context.clone();
                            let cancel_context = context.clone();
                            let confirm_session = weak.clone();
                            let cancel_session = weak.clone();
                            let close_on_confirm = options.close_on_confirm;
                            let close_on_cancel = options.close_on_cancel;
                            dialog
                                .title(
                                    crate::bridge::slot(&context, "title").unwrap_or_else(|| {
                                        options.title.clone().into_any_element()
                                    }),
                                )
                                .children(crate::bridge::children(&context))
                                .children(crate::bridge::slot(&context, "content"))
                                .when_some(
                                    crate::bridge::slot(&context, "footer"),
                                    |dialog, footer| dialog.footer(footer),
                                )
                                .when(!options.description.is_empty(), |dialog| {
                                    dialog.description(options.description.clone())
                                })
                                .button_props(options.button_props())
                                .close_button(options.close_button)
                                .keyboard(options.keyboard)
                                .when_some(options.size, |dialog, width| dialog.width(px(width)))
                                .on_ok(move |_, window, cx| {
                                    handle_decision(
                                        &confirm_session,
                                        generation,
                                        &confirm_context,
                                        true,
                                        close_on_confirm,
                                        window,
                                        cx,
                                    );
                                    false
                                })
                                .on_cancel(move |_, window, cx| {
                                    handle_decision(
                                        &cancel_session,
                                        generation,
                                        &cancel_context,
                                        false,
                                        close_on_cancel,
                                        window,
                                        cx,
                                    );
                                    false
                                })
                        }),
                    )
                }
                Surface::Sheet => {
                    let placement = options.borrow().placement;
                    Handle::Sheet(window.open_sheet_at_with_handle(
                        placement,
                        cx,
                        move |sheet, _, _| {
                            let options = options.borrow();
                            sheet
                                .title(
                                    crate::bridge::slot(&context, "title").unwrap_or_else(|| {
                                        options.title.clone().into_any_element()
                                    }),
                                )
                                .children(crate::bridge::children(&context))
                                .children(crate::bridge::slot(&context, "content"))
                                .when_some(
                                    crate::bridge::slot(&context, "footer"),
                                    |sheet, footer| sheet.footer(footer),
                                )
                                .child(options.description.clone())
                                .overlay_closable(options.overlay_closable)
                                .resizable(options.resizable)
                                .when_some(options.size, |sheet, size| sheet.size(px(size)))
                        },
                    ))
                }
            };
            let weak = Rc::downgrade(&session);
            let closed = handle.on_closed(window, cx, move |_, _| {
                let Some(session) = weak.upgrade() else {
                    return;
                };
                {
                    let mut session = session.borrow_mut();
                    if !session.alive || session.generation != generation {
                        return;
                    }
                    session.handle = None;
                }
                close_context.emit(P::Event::open_change(false));
            });
            let mut session = session.borrow_mut();
            session.handle = Some(handle);
            session.closed = Some(closed);
        });
    }
}

impl<P: OverlayProps> TypedNativeComponent for NativeOverlay<P> {
    type Props = P;
    type Event = P::Event;

    fn render(&mut self, _: &P, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        self.schedule(window, cx);
        None
    }

    fn validate_props(&self, props: &P) -> Result<(), PropError> {
        if props
            .options()
            .size
            .is_some_and(|size| !size.is_finite() || size <= 0.)
        {
            return Err(PropError(
                "Overlay size must be a positive finite number".into(),
            ));
        }
        Ok(())
    }

    fn props_changed(&mut self, props: &P, changed: &[String], _: &mut Window, _: &mut App) {
        *self.options.borrow_mut() = props.options();
        // An explicit equal-valued controlled acknowledgement is meaningful.
        // Unrelated title/content updates must not reopen a dismissed overlay.
        if changed.iter().any(|name| name == "open") {
            self.session.borrow_mut().requested_open = Some(props.open());
        }
        self.dirty = true;
    }
}

impl<P: OverlayProps> Drop for NativeOverlay<P> {
    fn drop(&mut self) {
        let mut session = self.session.borrow_mut();
        session.alive = false;
        session.closed.take();
        let handle = session.handle.take();
        if let (Some(handle), Some(window)) = (handle, self.window.as_ref()) {
            window
                .spawn(async move |cx| {
                    let _ = cx.update(|window, cx| handle.close(window, cx));
                })
                .detach();
        }
    }
}
