use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, FocusHandle, InteractiveElement as _, IntoElement, KeyBinding,
    MouseButton, ParentElement, RenderOnce, Role, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, actions, div, prelude::FluentBuilder as _,
};
use smallvec::SmallVec;

use crate::{FocusTrapElement as _, StyledExt as _};

const CONTEXT: &str = "Dialog";
actions!(dialog, [CancelDialog, ConfirmDialog]);

type Decision = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool>;
type Closed = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type CloseRequest = Rc<dyn Fn(bool, &mut Window, &mut App)>;

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", CancelDialog, Some(CONTEXT)),
        KeyBinding::new("enter", ConfirmDialog, Some(CONTEXT)),
    ]);
}

#[derive(Clone)]
pub struct DialogCallbacks {
    confirm: Decision,
    cancel: Decision,
    closed: Closed,
}

impl Default for DialogCallbacks {
    fn default() -> Self {
        Self {
            confirm: Rc::new(|_, _, _| true),
            cancel: Rc::new(|_, _, _| true),
            closed: Rc::new(|_, _, _| {}),
        }
    }
}

impl DialogCallbacks {
    pub fn on_confirm(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.confirm = Rc::new(handler);
        self
    }

    pub fn on_cancel(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.cancel = Rc::new(handler);
        self
    }

    pub fn on_close(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.closed = Rc::new(handler);
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
    overlay: Option<AnyElement>,
    surface: Option<AnyElement>,
    children: SmallVec<[AnyElement; 2]>,
    callbacks: DialogCallbacks,
    request_close: CloseRequest,
}

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
    children: SmallVec<[AnyElement; 1]>,
}

impl DialogClose {
    pub fn new() -> Self {
        Self {
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
impl RenderOnce for DialogClose {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .id("dialog-close")
            .on_click(|_, window, cx| window.dispatch_action(Box::new(CancelDialog), cx))
            .children(self.children)
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
            overlay: None,
            surface: None,
            children: SmallVec::new(),
            callbacks: DialogCallbacks::default(),
            request_close: Rc::new(|_, _, _| {}),
        }
    }

    pub fn overlay(mut self, element: impl IntoElement) -> Self {
        self.overlay = Some(element.into_any_element());
        self
    }
    pub fn surface(mut self, element: impl IntoElement) -> Self {
        self.surface = Some(element.into_any_element());
        self
    }
    pub fn keyboard(mut self, value: bool) -> Self {
        self.keyboard = value;
        self
    }
    pub fn overlay_closable(mut self, value: bool) -> Self {
        self.overlay_closable = value;
        self
    }
    pub fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }
    pub fn callbacks(mut self, value: DialogCallbacks) -> Self {
        self.callbacks = value;
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
        let callbacks = self.callbacks;
        let request_close = self.request_close;
        let cancel = callbacks.cancel.clone();
        let confirm = callbacks.confirm.clone();
        let closed = callbacks.closed.clone();
        let overlay_closable = self.overlay_closable && self.topmost;

        div()
            .id(("dialog-host", self.layer))
            .role(self.role)
            .track_focus(&self.focus)
            .focus_trap(format!("dialog-{}", self.layer), &self.focus)
            .key_context(CONTEXT)
            .when(self.keyboard, |this| {
                let request_cancel = request_close.clone();
                let request_confirm = request_close.clone();
                let closed_cancel = closed.clone();
                this.on_action(move |_: &CancelDialog, window, cx| {
                    let event = ClickEvent::default();
                    if cancel(&event, window, cx) {
                        request_cancel(false, window, cx);
                        closed_cancel(&event, window, cx);
                    }
                })
                .on_action(move |_: &ConfirmDialog, window, cx| {
                    let event = ClickEvent::default();
                    if confirm(&event, window, cx) {
                        request_confirm(true, window, cx);
                        closed(&event, window, cx);
                    }
                })
            })
            .when_some(self.overlay, |this, overlay| {
                let cancel = callbacks.cancel.clone();
                let closed = callbacks.closed.clone();
                let request_close = request_close.clone();
                this.child(
                    div()
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            cx.stop_propagation();
                            let event = ClickEvent::default();
                            if overlay_closable && cancel(&event, window, cx) {
                                request_close(false, window, cx);
                                closed(&event, window, cx);
                            }
                        })
                        .child(overlay),
                )
            })
            .children(self.surface)
            .children(self.children)
            .refine_style(&self.style)
    }
}
