use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, FocusHandle, InteractiveElement as _, IntoElement, KeyBinding,
    MouseButton, ParentElement, Pixels, RenderOnce, Role, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};
use smallvec::SmallVec;

use crate::actions::{Cancel, Confirm};
use crate::{FocusTrapElement as _, StyledExt as _};

const CONTEXT: &str = "Dialog";
type Decision = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool>;
type Closed = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type CloseRequest = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type OpenRequest = Rc<dyn Fn(&mut Window, &mut App)>;

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
    ]);
}

impl Dialog {
    pub fn on_ok(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.on_ok = Rc::new(handler);
        self
    }

    pub fn on_cancel(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.on_cancel = Rc::new(handler);
        self
    }

    pub fn on_close(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Rc::new(handler);
        self
    }
}

/// Unstyled modal host owning focus, keyboard actions, dismissal, and callback ordering.
#[derive(IntoElement)]
pub struct Dialog {
    style: StyleRefinement,
    focus: FocusHandle,
    role: Role,
    layer: usize,
    keyboard: bool,
    overlay_closable: bool,
    topmost: bool,
    dismiss_below_y: Pixels,
    backdrop: Option<AnyElement>,
    popup: Option<AnyElement>,
    children: SmallVec<[AnyElement; 2]>,
    on_ok: Decision,
    on_cancel: Decision,
    on_close: Closed,
    request_close: CloseRequest,
}

/// Unstyled trigger that owns pointer activation for opening a dialog.
#[derive(IntoElement)]
pub struct DialogTrigger {
    trigger: AnyElement,
    open: OpenRequest,
}

impl DialogTrigger {
    pub fn new(trigger: impl IntoElement) -> Self {
        Self {
            trigger: trigger.into_any_element(),
            open: Rc::new(|_, _| {}),
        }
    }

    pub fn on_open(mut self, open: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.open = Rc::new(open);
        self
    }
}

impl RenderOnce for DialogTrigger {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                (self.open)(window, cx);
                cx.stop_propagation();
            })
            .child(self.trigger)
    }
}

macro_rules! dialog_part {
    ($(#[$meta:meta])* $name:ident, $id:literal) => {
        $(#[$meta])*
        #[derive(IntoElement)]
        pub struct $name {
            style: StyleRefinement,
            children: SmallVec<[AnyElement; 2]>,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    style: StyleRefinement::default(),
                    children: SmallVec::new(),
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.children.extend(elements);
            }
        }

        impl RenderOnce for $name {
            fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
                div()
                    .id($id)
                    .children(self.children)
                    .refine_style(&self.style)
            }
        }
    };
}

dialog_part!(
    /// Unstyled backdrop part rendered behind a dialog popup.
    DialogBackdrop,
    "dialog-backdrop"
);

dialog_part!(
    /// Unstyled popup part containing dialog content.
    DialogPopup,
    "dialog-popup"
);

/// Unstyled title slot for a dialog surface.
#[derive(IntoElement)]
pub struct DialogTitle {
    base: gpui::Div,
    style: StyleRefinement,
    children: SmallVec<[AnyElement; 2]>,
}

impl DialogTitle {
    pub fn new() -> Self {
        Self {
            base: div(),
            style: StyleRefinement::default(),
            children: SmallVec::new(),
        }
    }
}

impl Default for DialogTitle {
    fn default() -> Self {
        Self::new()
    }
}
impl Styled for DialogTitle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl ParentElement for DialogTitle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}
impl RenderOnce for DialogTitle {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .id("dialog-title")
            .children(self.children)
            .refine_style(&self.style)
    }
}

/// Unstyled descriptive-content slot for a dialog surface.
#[derive(IntoElement)]
pub struct DialogDescription {
    base: gpui::Div,
    style: StyleRefinement,
    children: SmallVec<[AnyElement; 2]>,
}

impl DialogDescription {
    pub fn new() -> Self {
        Self {
            base: div(),
            style: StyleRefinement::default(),
            children: SmallVec::new(),
        }
    }
}

impl Default for DialogDescription {
    fn default() -> Self {
        Self::new()
    }
}
impl Styled for DialogDescription {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl ParentElement for DialogDescription {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}
impl RenderOnce for DialogDescription {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .id("dialog-description")
            .children(self.children)
            .refine_style(&self.style)
    }
}

/// Wrapper that dispatches the dialog cancel action when activated.
#[derive(IntoElement)]
pub struct DialogClose {
    style: StyleRefinement,
    children: SmallVec<[AnyElement; 1]>,
}

impl DialogClose {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: SmallVec::new(),
        }
    }
}
impl Default for DialogClose {
    fn default() -> Self {
        Self::new()
    }
}
impl ParentElement for DialogClose {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}
impl Styled for DialogClose {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for DialogClose {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .id("dialog-close")
            .on_click(|_, window, cx| window.dispatch_action(Box::new(Cancel), cx))
            .children(self.children)
            .refine_style(&self.style)
    }
}

impl Dialog {
    pub fn new(cx: &mut App) -> Self {
        Self {
            style: StyleRefinement::default(),
            focus: cx.focus_handle(),
            role: Role::Dialog,
            layer: 0,
            keyboard: true,
            overlay_closable: true,
            topmost: true,
            dismiss_below_y: px(0.),
            backdrop: None,
            popup: None,
            children: SmallVec::new(),
            on_ok: Rc::new(|_, _, _| true),
            on_cancel: Rc::new(|_, _, _| true),
            on_close: Rc::new(|_, _, _| {}),
            request_close: Rc::new(|_, _, _| {}),
        }
    }

    pub fn backdrop(mut self, element: impl IntoElement) -> Self {
        self.backdrop = Some(element.into_any_element());
        self
    }
    pub fn popup(mut self, element: impl IntoElement) -> Self {
        self.popup = Some(element.into_any_element());
        self
    }
    pub fn close_on_escape(mut self, value: bool) -> Self {
        self.keyboard = value;
        self
    }
    pub fn close_on_backdrop_press(mut self, value: bool) -> Self {
        self.overlay_closable = value;
        self
    }
    pub fn dismiss_below_y(mut self, value: Pixels) -> Self {
        self.dismiss_below_y = value;
        self
    }
    pub(crate) fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }
    #[doc(hidden)]
    pub fn layer(mut self, index: usize, topmost: bool) -> Self {
        self.layer = index;
        self.topmost = topmost;
        self
    }
    #[doc(hidden)]
    pub fn focus_handle(mut self, value: FocusHandle) -> Self {
        self.focus = value;
        self
    }
    #[doc(hidden)]
    pub fn request_close(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.request_close = Rc::new(handler);
        self
    }
}

impl Styled for Dialog {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl ParentElement for Dialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Dialog {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let request_close = self.request_close;
        let cancel = self.on_cancel.clone();
        let confirm = self.on_ok.clone();
        let closed = self.on_close.clone();
        let overlay_closable = self.overlay_closable && self.topmost;
        let dismiss_below_y = self.dismiss_below_y;

        div()
            .id(("dialog-host", self.layer))
            .role(self.role)
            .track_focus(&self.focus)
            .focus_trap(format!("dialog-{}", self.layer), &self.focus)
            .when(self.keyboard, |this| this.key_context(CONTEXT))
            .map(|this| {
                let request_cancel = request_close.clone();
                let request_confirm = request_close.clone();
                let closed_cancel = closed.clone();
                this.on_action(move |_: &Cancel, window, cx| {
                    let event = ClickEvent::default();
                    if cancel(&event, window, cx) {
                        request_cancel(false, window, cx);
                        closed_cancel(&event, window, cx);
                    }
                })
                .on_action(move |_: &Confirm, window, cx| {
                    let event = ClickEvent::default();
                    if confirm(&event, window, cx) {
                        request_confirm(true, window, cx);
                        closed(&event, window, cx);
                    }
                })
            })
            .when_some(self.backdrop, |this, backdrop| {
                let cancel = self.on_cancel.clone();
                let closed = self.on_close.clone();
                let request_close = request_close.clone();
                this.child(
                    div()
                        .on_any_mouse_down(move |event, window, cx| {
                            if event.position.y < dismiss_below_y {
                                return;
                            }
                            let button = event.button;
                            cx.stop_propagation();
                            let event = ClickEvent::default();
                            if button == MouseButton::Left
                                && overlay_closable
                                && cancel(&event, window, cx)
                            {
                                closed(&event, window, cx);
                                request_close(false, window, cx);
                            }
                        })
                        .child(backdrop),
                )
            })
            .children(self.popup)
            .children(self.children)
            .refine_style(&self.style)
    }
}
