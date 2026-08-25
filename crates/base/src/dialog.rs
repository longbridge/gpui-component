use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::{
    AnyElement, App, ClickEvent, FocusHandle, Global, InteractiveElement as _, IntoElement,
    KeyBinding, MouseButton, ParentElement, Pixels, RenderOnce, Role,
    StatefulInteractiveElement as _, StyleRefinement, Styled, WeakFocusHandle, Window, anchored,
    deferred, div, point, prelude::FluentBuilder as _, px,
};
use smallvec::SmallVec;

use crate::actions::{Cancel, Confirm};
use crate::{FocusTrapElement as _, StyledExt as _};

const CONTEXT: &str = "Dialog";
type Decision = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool>;
type Closed = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type CloseRequest = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type OpenRequest = Rc<dyn Fn(&mut Window, &mut App)>;
type OpenChange = Rc<dyn Fn(bool, DialogChangeReason, &mut Window, &mut App)>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DialogChangeReason {
    TriggerPress,
    BackdropPress,
    Cancel,
    Confirm,
    Imperative,
}

#[derive(Clone)]
pub struct DialogHandle {
    open: Rc<Cell<bool>>,
    on_open_change: Rc<RefCell<Option<OpenChange>>>,
}

impl DialogHandle {
    pub fn new(open: bool) -> Self {
        Self {
            open: Rc::new(Cell::new(open)),
            on_open_change: Rc::new(RefCell::new(None)),
        }
    }
    pub fn is_open(&self) -> bool {
        self.open.get()
    }
    pub fn open(&self, window: &mut Window, cx: &mut App) {
        self.set_open(true, DialogChangeReason::Imperative, window, cx);
    }
    pub fn close(&self, window: &mut Window, cx: &mut App) {
        self.set_open(false, DialogChangeReason::Imperative, window, cx);
    }
    pub(crate) fn set_open(
        &self,
        open: bool,
        reason: DialogChangeReason,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.open.replace(open) == open {
            return;
        }
        let callback = self.on_open_change.borrow().clone();
        if let Some(callback) = callback {
            callback(open, reason, window, cx);
        }
        window.refresh();
    }
}

fn request_open_change(
    handle: &Option<DialogHandle>,
    callback: &Option<OpenChange>,
    open: bool,
    reason: DialogChangeReason,
    window: &mut Window,
    cx: &mut App,
) {
    if let Some(handle) = handle {
        handle.set_open(open, reason, window, cx);
    } else if let Some(callback) = callback {
        callback(open, reason, window, cx);
    }
}

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
    handle: Option<DialogHandle>,
    open: bool,
    on_open_change: Option<OpenChange>,
}

/// Unstyled trigger that owns pointer activation for opening a dialog.
#[derive(IntoElement)]
pub struct DialogTrigger {
    trigger: AnyElement,
    open: OpenRequest,
    handle: Option<DialogHandle>,
}

impl DialogTrigger {
    pub fn new(trigger: impl IntoElement) -> Self {
        Self {
            trigger: trigger.into_any_element(),
            open: Rc::new(|_, _| {}),
            handle: None,
        }
    }
    pub fn handle(mut self, handle: DialogHandle) -> Self {
        self.handle = Some(handle);
        self
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
                if let Some(handle) = self.handle.as_ref() {
                    handle.set_open(true, DialogChangeReason::TriggerPress, window, cx);
                }
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

/// Topmost dialog hosts that did not contain the window focus on the previous frame.
///
/// `contains_focused` can only see the dispatch tree of the last rendered frame, so
/// anything focused during this frame is not in it yet and reads as "outside" — a
/// dialog on the frame it opens, or an input that just appeared inside one. Waiting
/// for a second consecutive frame lets the tree catch up, so only focus that is
/// genuinely elsewhere is reclaimed.
#[derive(Default)]
struct EscapedHosts(Vec<WeakFocusHandle>);

impl Global for EscapedHosts {}

impl EscapedHosts {
    /// Records `focus` as looking escaped, returning whether it already did last frame.
    fn mark(focus: &FocusHandle, cx: &mut App) -> bool {
        let hosts = &mut cx.default_global::<Self>().0;
        hosts.retain(|host| host.upgrade().is_some());
        let host = focus.downgrade();
        if hosts.contains(&host) {
            return true;
        }
        hosts.push(host);
        false
    }

    /// Forgets `focus`, which holds the window focus again (or no longer owns it).
    fn clear(focus: &FocusHandle, cx: &mut App) {
        let host = focus.downgrade();
        cx.default_global::<Self>()
            .0
            .retain(|other| *other != host && other.upgrade().is_some());
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
            handle: None,
            open: true,
            on_open_change: None,
        }
    }
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
    pub fn handle(mut self, handle: DialogHandle) -> Self {
        if let Some(callback) = self.on_open_change.as_ref() {
            *handle.on_open_change.borrow_mut() = Some(callback.clone());
        }
        self.handle = Some(handle);
        self
    }
    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, DialogChangeReason, &mut Window, &mut App) + 'static,
    ) -> Self {
        let handler: OpenChange = Rc::new(handler);
        if let Some(handle) = self.handle.as_ref() {
            *handle.on_open_change.borrow_mut() = Some(handler.clone());
        }
        self.on_open_change = Some(handler);
        self
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = self
            .handle
            .as_ref()
            .map_or(self.open, DialogHandle::is_open);
        if !open {
            return div().into_any_element();
        }
        // A modal owns the window focus: reclaim it when focus escapes, or
        // Confirm/Cancel (dispatched along the focus path) silently miss this host.
        // Escaping is only believed once it survives a frame, see `EscapedHosts`.
        if !self.topmost || self.focus.contains_focused(window, cx) {
            EscapedHosts::clear(&self.focus, cx);
        } else if EscapedHosts::mark(&self.focus, cx) {
            window.focus(&self.focus, cx);
        } else {
            window.request_animation_frame();
        }
        let request_close = self.request_close;
        let cancel = self.on_cancel.clone();
        let confirm = self.on_ok.clone();
        let closed = self.on_close.clone();
        let overlay_closable = self.overlay_closable && self.topmost;
        let dismiss_below_y = self.dismiss_below_y;
        let escape_handle = self.handle.clone();
        let confirm_handle = self.handle.clone();
        let backdrop_handle = self.handle.clone();
        let escape_change = self.on_open_change.clone();
        let confirm_change = self.on_open_change.clone();
        let backdrop_change = self.on_open_change.clone();
        let viewport = window.viewport_size();

        deferred(
            anchored().position(point(px(0.), px(0.))).child(
                div()
                    .id(("dialog-host", self.layer))
                    .absolute()
                    .top_0()
                    .left_0()
                    .w(viewport.width)
                    .h(viewport.height)
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
                                request_open_change(
                                    &escape_handle,
                                    &escape_change,
                                    false,
                                    DialogChangeReason::Cancel,
                                    window,
                                    cx,
                                );
                                request_cancel(false, window, cx);
                                closed_cancel(&event, window, cx);
                            }
                        })
                        .on_action(move |_: &Confirm, window, cx| {
                            let event = ClickEvent::default();
                            if confirm(&event, window, cx) {
                                request_open_change(
                                    &confirm_handle,
                                    &confirm_change,
                                    false,
                                    DialogChangeReason::Confirm,
                                    window,
                                    cx,
                                );
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
                                        request_open_change(
                                            &backdrop_handle,
                                            &backdrop_change,
                                            false,
                                            DialogChangeReason::BackdropPress,
                                            window,
                                            cx,
                                        );
                                        request_close(false, window, cx);
                                        closed(&event, window, cx);
                                    }
                                })
                                .child(backdrop),
                        )
                    })
                    .children(self.popup)
                    .children(self.children)
                    .refine_style(&self.style),
            ),
        )
        .with_priority(10 + self.layer)
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Context, Render, point};
    use std::{cell::RefCell, rc::Rc};

    struct TriggerHarness {
        handle: DialogHandle,
    }
    impl Render for TriggerHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            DialogTrigger::new(div().size(px(100.))).handle(self.handle.clone())
        }
    }

    #[gpui::test]
    fn trigger_opens_shared_handle_and_reports_reason(cx: &mut gpui::TestAppContext) {
        let handle = DialogHandle::new(false);
        let changes = Rc::new(RefCell::new(Vec::new()));
        *handle.on_open_change.borrow_mut() = Some({
            let changes = changes.clone();
            Rc::new(move |open, reason, _, _| changes.borrow_mut().push((open, reason)))
        });
        let (_, cx) = cx.add_window_view({
            let handle = handle.clone();
            move |_, _| TriggerHarness { handle }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_click(point(px(20.), px(20.)), Default::default());

        assert!(handle.is_open());
        assert_eq!(
            &*changes.borrow(),
            &[(true, DialogChangeReason::TriggerPress)]
        );
    }

    struct FocusHarness {
        handle: DialogHandle,
        dialog_focus: FocusHandle,
        inner_focus: FocusHandle,
        swapped_focus: FocusHandle,
        outside_focus: FocusHandle,
        swapped: bool,
    }
    impl Render for FocusHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let content = if self.swapped {
                div().track_focus(&self.swapped_focus)
            } else {
                div().track_focus(&self.inner_focus)
            };
            div()
                .child(div().track_focus(&self.outside_focus).size(px(10.)))
                .child(
                    Dialog::new(cx)
                        .handle(self.handle.clone())
                        .focus_handle(self.dialog_focus.clone())
                        .child(content.size(px(50.))),
                )
        }
    }

    fn focus_harness(
        cx: &mut gpui::TestAppContext,
        handle: DialogHandle,
    ) -> (&mut gpui::VisualTestContext, gpui::Entity<FocusHarness>) {
        cx.update(|cx| crate::focus_trap::init(cx));
        let (view, cx) = cx.add_window_view(move |_, cx| FocusHarness {
            handle,
            dialog_focus: cx.focus_handle(),
            inner_focus: cx.focus_handle(),
            swapped_focus: cx.focus_handle(),
            outside_focus: cx.focus_handle(),
            swapped: false,
        });
        (cx, view)
    }

    fn draw(cx: &mut gpui::VisualTestContext) {
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
    }

    /// Callers open a dialog and immediately hand focus to an input inside it.
    /// Neither the host nor that input has been in a dispatch tree yet, so the
    /// modal must not read that as "focus escaped" on the frame it first renders.
    #[gpui::test]
    fn content_focused_before_first_frame_keeps_focus(cx: &mut gpui::TestAppContext) {
        let handle = DialogHandle::new(false);
        let (cx, view) = focus_harness(cx, handle.clone());
        let (dialog_focus, inner_focus) = cx.update(|_, cx| {
            let this = view.read(cx);
            (this.dialog_focus.clone(), this.inner_focus.clone())
        });

        // A frame while the dialog is still closed: neither handle is in the tree.
        draw(cx);

        // `Root::open_dialog` focuses the host, then the caller focuses the input.
        cx.update(|window, cx| {
            handle.open(window, cx);
            window.focus(&dialog_focus, cx);
            window.focus(&inner_focus, cx);
            window.draw(cx).clear(cx);
        });
        cx.update(|window, _| {
            assert!(
                inner_focus.is_focused(window),
                "dialog stole focus from its own content on the frame it opened"
            );
        });

        // And it must stay there once the tree knows about both.
        draw(cx);
        cx.update(|window, _| {
            assert!(
                inner_focus.is_focused(window),
                "dialog stole focus from its own content on a later frame"
            );
        });
    }

    /// An input that appears inside an already open dialog is just as new to the
    /// dispatch tree — a second-factor field replacing a password field.
    #[gpui::test]
    fn content_focused_as_it_appears_keeps_focus(cx: &mut gpui::TestAppContext) {
        let handle = DialogHandle::new(true);
        let (cx, view) = focus_harness(cx, handle);
        let swapped_focus = cx.update(|_, cx| view.read(cx).swapped_focus.clone());

        draw(cx);
        draw(cx);

        // Swap the field in and focus it in one go, as the caller does.
        cx.update(|window, cx| {
            view.update(cx, |this, cx| {
                this.swapped = true;
                cx.notify();
            });
            window.focus(&swapped_focus, cx);
            window.draw(cx).clear(cx);
        });
        cx.update(|window, _| {
            assert!(
                swapped_focus.is_focused(window),
                "dialog stole focus from an input that had just appeared inside it"
            );
        });
    }

    /// Focus that genuinely leaves the modal is still reclaimed, or the footer's
    /// `Confirm`/`Cancel` never reach this host.
    #[gpui::test]
    fn focus_escaping_to_another_element_is_reclaimed(cx: &mut gpui::TestAppContext) {
        let handle = DialogHandle::new(true);
        let (cx, view) = focus_harness(cx, handle);
        let (dialog_focus, outside_focus) = cx.update(|_, cx| {
            let this = view.read(cx);
            (this.dialog_focus.clone(), this.outside_focus.clone())
        });

        cx.update(|window, cx| {
            window.focus(&dialog_focus, cx);
            window.draw(cx).clear(cx);
        });
        draw(cx);

        // Something outside the modal takes focus, the way a deferred
        // focus scheduled elsewhere does after the dialog is already open.
        cx.update(|window, cx| {
            window.focus(&outside_focus, cx);
            window.draw(cx).clear(cx);
            window.draw(cx).clear(cx);
        });
        cx.update(|window, _| {
            assert!(
                dialog_focus.is_focused(window),
                "dialog did not reclaim focus that escaped it"
            );
        });
    }
}
