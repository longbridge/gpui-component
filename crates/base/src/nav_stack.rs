use std::rc::Rc;

use gpui::{
    AnyElement, AnyView, App, Context, Div, Entity, EventEmitter, IntoElement, ParentElement as _,
    RenderOnce, StyleRefinement, Styled, Window, div,
};

use crate::{
    StyledExt as _,
    motion::{Instant, MotionStatus, PresencePhase, Transition},
};

/// What a running transition is doing, in Qt's terms.
///
/// The operation decides paint order and lets a renderer move a pushed view
/// differently from a popped one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavOperation {
    /// A view was pushed over the previous top.
    Push,
    /// The top was popped, revealing the view below.
    Pop,
    /// The top was swapped for another view.
    Replace,
}

/// Whether one change runs the [`NavStack`]'s transition, as UIKit's
/// `animated:` and Qt's `StackView.Immediate` decide per call.
///
/// `Immediate` switches views on the spot even when the element has a
/// transition, which is what restoring a stack at launch or jumping to a
/// page from a command wants. A `NavStack` without a transition is always
/// immediate, whatever is passed here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavMotion {
    Animated,
    Immediate,
}

/// Emitted by [`NavStackState`] after the stack changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavStackEvent {
    Pushed,
    Popped,
    Replaced,
    Cleared,
}

/// The view leaving the stack, kept mounted until its transition finishes.
struct Transit {
    outgoing: AnyView,
    operation: NavOperation,
    started_at: Instant,
}

/// One frame of a running transition, sampled by the element.
struct TransitFrame {
    outgoing: AnyView,
    operation: NavOperation,
    progress: f32,
    status: MotionStatus,
}

/// A last-in-first-out stack of views, one visible at a time.
///
/// This is SwiftUI's `NavigationStack`, Qt's `StackView` and WinUI's
/// `Frame`: navigation between pages.
///
/// The stack owns which view is current and the lifecycle of a change: after
/// a push, pop or replace, the outgoing view stays mounted until the
/// [`NavStack`]'s transition finishes, so the application can animate it.
/// The views themselves, and what a transition looks like, belong to the
/// application.
///
/// `pop` keeps the root, as Qt's `StackView` and UIKit's navigation controller
/// do; `clear` is the way to empty the stack. A back button is shown when
/// `depth() > 1`.
pub struct NavStackState {
    views: Vec<AnyView>,
    transit: Option<Transit>,
}

impl EventEmitter<NavStackEvent> for NavStackState {}

impl Default for NavStackState {
    fn default() -> Self {
        Self::new()
    }
}

impl NavStackState {
    pub fn new() -> Self {
        Self {
            views: Vec::new(),
            transit: None,
        }
    }

    /// The number of views on the stack.
    pub fn depth(&self) -> usize {
        self.views.len()
    }

    pub fn is_empty(&self) -> bool {
        self.views.is_empty()
    }

    /// The view on top of the stack, which is the one shown once any
    /// transition has finished.
    pub fn current(&self) -> Option<&AnyView> {
        self.views.last()
    }

    /// Every view on the stack, root first.
    pub fn views(&self) -> &[AnyView] {
        &self.views
    }

    /// Pushes `view` on top of the stack.
    ///
    /// Into an empty stack this is immediate, like Qt's `initialItem`. Over
    /// an existing top it starts a [`NavOperation::Push`] transition, unless
    /// `motion` is [`NavMotion::Immediate`].
    pub fn push(&mut self, view: impl Into<AnyView>, motion: NavMotion, cx: &mut Context<Self>) {
        let outgoing = self.views.last().cloned();
        self.views.push(view.into());
        self.begin(outgoing, NavOperation::Push, motion, cx);
        cx.emit(NavStackEvent::Pushed);
        cx.notify();
    }

    /// Pops the top view and returns it, starting a [`NavOperation::Pop`]
    /// transition to the view below.
    ///
    /// The root is never popped: this returns `None` at a depth of one or
    /// less.
    pub fn pop(&mut self, motion: NavMotion, cx: &mut Context<Self>) -> Option<AnyView> {
        if self.views.len() <= 1 {
            return None;
        }
        let popped = self.views.pop()?;
        self.begin(Some(popped.clone()), NavOperation::Pop, motion, cx);
        cx.emit(NavStackEvent::Popped);
        cx.notify();
        Some(popped)
    }

    /// Pops every view above the root in one [`NavOperation::Pop`]
    /// transition from the previous top, and returns them root-side first.
    pub fn pop_to_root(&mut self, motion: NavMotion, cx: &mut Context<Self>) -> Vec<AnyView> {
        if self.views.len() <= 1 {
            return Vec::new();
        }
        let popped: Vec<AnyView> = self.views.drain(1..).collect();
        self.begin(popped.last().cloned(), NavOperation::Pop, motion, cx);
        cx.emit(NavStackEvent::Popped);
        cx.notify();
        popped
    }

    /// Swaps the top view for `view` and returns the one replaced, starting a
    /// [`NavOperation::Replace`] transition. On an empty stack this is a
    /// push.
    pub fn replace(
        &mut self,
        view: impl Into<AnyView>,
        motion: NavMotion,
        cx: &mut Context<Self>,
    ) -> Option<AnyView> {
        let Some(replaced) = self.views.pop() else {
            self.push(view, motion, cx);
            return None;
        };
        self.views.push(view.into());
        self.begin(Some(replaced.clone()), NavOperation::Replace, motion, cx);
        cx.emit(NavStackEvent::Replaced);
        cx.notify();
        Some(replaced)
    }

    /// Empties the stack immediately, abandoning any running transition.
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.views.clear();
        self.transit = None;
        cx.emit(NavStackEvent::Cleared);
        cx.notify();
    }

    /// Starts a transition from `outgoing`, or none for an immediate change.
    /// A transition already running is finished on the spot either way, so
    /// at most one outgoing view is ever mounted.
    fn begin(
        &mut self,
        outgoing: Option<AnyView>,
        operation: NavOperation,
        motion: NavMotion,
        cx: &Context<Self>,
    ) {
        if motion == NavMotion::Immediate {
            self.transit = None;
            return;
        }
        self.transit = outgoing.map(|outgoing| Transit {
            outgoing,
            operation,
            started_at: cx.background_executor().now(),
        });
    }

    /// Samples the running transition at `now`, dropping the outgoing view
    /// once the transition has finished. `None` means there is nothing in
    /// transit: no `transition` finishes immediately.
    fn advance(&mut self, now: Instant, transition: Option<&Transition>) -> Option<TransitFrame> {
        let transit = self.transit.as_ref()?;
        let (progress, status) = match transition {
            Some(transition) => {
                let elapsed = now.saturating_duration_since(transit.started_at);
                let (progress, status) = transition.progress(elapsed, transition.duration());
                (transition.sample(progress), status)
            }
            None => (1.0, MotionStatus::Finished),
        };
        if status == MotionStatus::Finished {
            self.transit = None;
            return None;
        }
        Some(TransitFrame {
            outgoing: transit.outgoing.clone(),
            operation: transit.operation,
            progress,
            status,
        })
    }
}

type ItemRenderer = Rc<dyn Fn(NavPage, &mut Window, &mut App) -> AnyElement>;

/// An unstyled host for a [`NavStackState`].
///
/// The container is positioned so that the two views of a transition can
/// overlap; each mounted view is handed to the `item` renderer as a
/// [`NavPage`] that already fills the container. Everything else — size,
/// clipping, background, and how a transition moves — is the application's.
///
/// Without a `transition` the stack switches views immediately, as it also
/// does under reduced motion.
#[derive(IntoElement)]
pub struct NavStack {
    base: Div,
    style: StyleRefinement,
    state: Entity<NavStackState>,
    transition: Option<Transition>,
    render_item: Option<ItemRenderer>,
}

impl NavStack {
    pub fn new(state: &Entity<NavStackState>) -> Self {
        Self {
            base: div(),
            style: StyleRefinement::default(),
            state: state.clone(),
            transition: None,
            render_item: None,
        }
    }

    /// The timing every push, pop and replace runs under.
    pub fn transition(mut self, transition: Transition) -> Self {
        self.transition = Some(transition);
        self
    }

    /// Renders each mounted view. The item is already positioned to fill the
    /// container; refine it to move or fade the view by its phase and
    /// progress, then return it.
    pub fn item(
        mut self,
        render: impl Fn(NavPage, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.render_item = Some(Rc::new(render));
        self
    }
}

impl Styled for NavStack {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for NavStack {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let now = cx.background_executor().now();
        let transition = if cx.reduce_motion() {
            None
        } else {
            self.transition.as_ref()
        };
        let frame = self
            .state
            .update(cx, |state, _| state.advance(now, transition));
        let views = self.state.read(cx).views.clone();

        let mut items = Vec::with_capacity(2);
        if let Some(current) = views.last() {
            let index = views.len() - 1;
            match frame {
                Some(frame) => {
                    if matches!(frame.status, MotionStatus::Delayed | MotionStatus::Running) {
                        window.request_animation_frame();
                    }
                    let current = NavPage::new(
                        current.clone(),
                        index,
                        PresencePhase::Entering,
                        Some(frame.operation),
                        frame.progress,
                    );
                    let outgoing_index = match frame.operation {
                        NavOperation::Push => index.saturating_sub(1),
                        NavOperation::Pop | NavOperation::Replace => index + 1,
                    };
                    let outgoing = NavPage::new(
                        frame.outgoing,
                        outgoing_index,
                        PresencePhase::Exiting,
                        Some(frame.operation),
                        frame.progress,
                    );
                    // A pushed or replacing view paints over what it covers; a
                    // popped view paints over what it reveals.
                    match frame.operation {
                        NavOperation::Push | NavOperation::Replace => {
                            items.push(outgoing);
                            items.push(current);
                        }
                        NavOperation::Pop => {
                            items.push(current);
                            items.push(outgoing);
                        }
                    }
                }
                None => items.push(NavPage::new(
                    current.clone(),
                    index,
                    PresencePhase::Present,
                    None,
                    1.0,
                )),
            }
        }

        let render_item = self.render_item;
        self.base
            .relative()
            .refine_style(&self.style)
            .children(items.into_iter().map(|item| match &render_item {
                Some(render) => render(item, window, cx),
                None => item.into_any_element(),
            }))
    }
}

/// One mounted view of a [`NavStack`], handed to the item renderer.
///
/// The item fills its container. Its readers describe where the view is in
/// the change that is running, so the renderer can move it: `phase` says
/// whether it is arriving, settled, or leaving; `operation` says which
/// change; `progress` runs from `0.0` to `1.0` over the transition, already
/// eased, and is shared by both views of one change.
#[derive(IntoElement)]
pub struct NavPage {
    base: Div,
    style: StyleRefinement,
    view: AnyView,
    index: usize,
    phase: PresencePhase,
    operation: Option<NavOperation>,
    progress: f32,
}

impl NavPage {
    fn new(
        view: AnyView,
        index: usize,
        phase: PresencePhase,
        operation: Option<NavOperation>,
        progress: f32,
    ) -> Self {
        Self {
            base: div(),
            style: StyleRefinement::default(),
            view,
            index,
            phase,
            operation,
            progress,
        }
    }

    pub fn view(&self) -> &AnyView {
        &self.view
    }

    /// The view's position on the stack, root first. A view on its way out
    /// keeps the position it had.
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn phase(&self) -> PresencePhase {
        self.phase
    }

    /// The change in progress, or `None` once the stack has settled.
    pub fn operation(&self) -> Option<NavOperation> {
        self.operation
    }

    pub fn progress(&self) -> f32 {
        self.progress
    }
}

impl Styled for NavPage {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for NavPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .absolute()
            .inset_0()
            .child(self.view)
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, time::Duration};

    use gpui::{AppContext as _, Render, TestAppContext};

    use super::*;

    struct Page;

    impl Render for Page {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    fn page(cx: &mut TestAppContext) -> AnyView {
        cx.new(|_| Page).into()
    }

    fn stack(cx: &mut TestAppContext) -> (Entity<NavStackState>, Rc<RefCell<Vec<NavStackEvent>>>) {
        let stack = cx.new(|_| NavStackState::new());
        let events = Rc::new(RefCell::new(Vec::new()));
        cx.update({
            let events = events.clone();
            let stack = stack.clone();
            move |cx| {
                cx.subscribe(&stack, move |_, event: &NavStackEvent, _| {
                    events.borrow_mut().push(*event);
                })
                .detach();
            }
        });
        (stack, events)
    }

    #[gpui::test]
    fn push_and_pop_keep_the_root(cx: &mut TestAppContext) {
        let (stack, events) = stack(cx);
        let (root, second) = (page(cx), page(cx));

        stack.update(cx, |stack, cx| {
            stack.push(root.clone(), NavMotion::Animated, cx)
        });
        assert!(stack.read_with(cx, |stack, _| stack.transit.is_none()));

        stack.update(cx, |stack, cx| {
            stack.push(second.clone(), NavMotion::Animated, cx)
        });
        stack.read_with(cx, |stack, _| {
            assert_eq!(stack.depth(), 2);
            assert_eq!(stack.current(), Some(&second));
            let transit = stack
                .transit
                .as_ref()
                .expect("push over a view transitions");
            assert_eq!(transit.operation, NavOperation::Push);
            assert_eq!(transit.outgoing, root);
        });

        let popped = stack.update(cx, |stack, cx| stack.pop(NavMotion::Animated, cx));
        assert_eq!(popped, Some(second.clone()));
        stack.read_with(cx, |stack, _| {
            assert_eq!(stack.views(), std::slice::from_ref(&root));
            let transit = stack.transit.as_ref().expect("pop transitions");
            assert_eq!(transit.operation, NavOperation::Pop);
            assert_eq!(transit.outgoing, second);
        });

        assert_eq!(
            stack.update(cx, |stack, cx| stack.pop(NavMotion::Animated, cx)),
            None
        );
        assert_eq!(stack.read_with(cx, |stack, _| stack.depth()), 1);
        assert_eq!(
            &*events.borrow(),
            &[
                NavStackEvent::Pushed,
                NavStackEvent::Pushed,
                NavStackEvent::Popped
            ]
        );
    }

    #[gpui::test]
    fn pop_to_root_returns_everything_above_it(cx: &mut TestAppContext) {
        let (stack, _) = stack(cx);
        let pages: Vec<AnyView> = (0..3).map(|_| page(cx)).collect();
        for view in &pages {
            stack.update(cx, |stack, cx| {
                stack.push(view.clone(), NavMotion::Animated, cx)
            });
        }

        assert_eq!(
            stack.update(cx, |stack, cx| stack.pop_to_root(NavMotion::Animated, cx)),
            pages[1..]
        );
        stack.read_with(cx, |stack, _| {
            assert_eq!(stack.views(), &pages[..1]);
            assert_eq!(stack.transit.as_ref().map(|t| &t.outgoing), Some(&pages[2]));
        });
        assert!(
            stack
                .update(cx, |stack, cx| stack.pop_to_root(NavMotion::Animated, cx))
                .is_empty()
        );
    }

    #[gpui::test]
    fn replace_swaps_the_top_and_pushes_into_an_empty_stack(cx: &mut TestAppContext) {
        let (stack, events) = stack(cx);
        let (first, second) = (page(cx), page(cx));

        assert_eq!(
            stack.update(cx, |stack, cx| stack.replace(
                first.clone(),
                NavMotion::Animated,
                cx
            )),
            None
        );
        assert_eq!(
            stack.update(cx, |stack, cx| stack.replace(
                second.clone(),
                NavMotion::Animated,
                cx
            )),
            Some(first.clone())
        );
        stack.read_with(cx, |stack, _| {
            assert_eq!(stack.views(), std::slice::from_ref(&second));
            let transit = stack.transit.as_ref().expect("replace transitions");
            assert_eq!(transit.operation, NavOperation::Replace);
            assert_eq!(transit.outgoing, first);
        });

        stack.update(cx, |stack, cx| stack.clear(cx));
        stack.read_with(cx, |stack, _| {
            assert!(stack.is_empty());
            assert!(stack.transit.is_none());
        });
        assert_eq!(
            &*events.borrow(),
            &[
                NavStackEvent::Pushed,
                NavStackEvent::Replaced,
                NavStackEvent::Cleared
            ]
        );
    }

    #[gpui::test]
    fn advance_drops_the_outgoing_view_when_the_transition_finishes(cx: &mut TestAppContext) {
        let (stack, _) = stack(cx);
        let (root, second) = (page(cx), page(cx));
        stack.update(cx, |stack, cx| {
            stack.push(root, NavMotion::Animated, cx);
            stack.push(second, NavMotion::Animated, cx);
        });
        let started_at = stack.read_with(cx, |stack, _| stack.transit.as_ref().unwrap().started_at);
        let transition = Transition::new(Duration::from_millis(200));

        stack.update(cx, |stack, _| {
            let frame = stack
                .advance(started_at + Duration::from_millis(100), Some(&transition))
                .expect("halfway through, still in transit");
            assert_eq!(frame.operation, NavOperation::Push);
            assert_eq!(frame.status, MotionStatus::Running);
            assert!(frame.progress > 0.0 && frame.progress < 1.0);
            assert!(stack.transit.is_some());

            assert!(
                stack
                    .advance(started_at + Duration::from_millis(200), Some(&transition))
                    .is_none()
            );
            assert!(stack.transit.is_none());
        });
    }

    #[gpui::test]
    fn without_a_transition_the_change_is_immediate(cx: &mut TestAppContext) {
        let (stack, _) = stack(cx);
        let (root, second) = (page(cx), page(cx));
        stack.update(cx, |stack, cx| {
            stack.push(root, NavMotion::Animated, cx);
            stack.push(second, NavMotion::Animated, cx);
        });
        let started_at = stack.read_with(cx, |stack, _| stack.transit.as_ref().unwrap().started_at);
        stack.update(cx, |stack, _| {
            assert!(stack.advance(started_at, None).is_none());
            assert!(stack.transit.is_none());
        });
    }

    #[gpui::test]
    fn an_immediate_change_mounts_no_outgoing_view(cx: &mut TestAppContext) {
        let (stack, events) = stack(cx);
        let (root, second, third) = (page(cx), page(cx), page(cx));
        stack.update(cx, |stack, cx| {
            stack.push(root, NavMotion::Animated, cx);
            stack.push(second, NavMotion::Animated, cx);
            stack.push(third.clone(), NavMotion::Immediate, cx);
        });
        stack.read_with(cx, |stack, _| {
            assert_eq!(stack.current(), Some(&third));
            assert!(
                stack.transit.is_none(),
                "immediate also ends the running push"
            );
        });
        assert_eq!(
            stack.update(cx, |stack, cx| stack.pop(NavMotion::Immediate, cx)),
            Some(third)
        );
        assert!(stack.read_with(cx, |stack, _| stack.transit.is_none()));
        assert_eq!(events.borrow().len(), 4);
    }

    #[gpui::test]
    fn a_new_operation_replaces_the_running_transition(cx: &mut TestAppContext) {
        let (stack, _) = stack(cx);
        let pages: Vec<AnyView> = (0..3).map(|_| page(cx)).collect();
        for view in &pages {
            stack.update(cx, |stack, cx| {
                stack.push(view.clone(), NavMotion::Animated, cx)
            });
        }
        stack.update(cx, |stack, cx| {
            stack.pop(NavMotion::Animated, cx);
        });
        stack.read_with(cx, |stack, _| {
            let transit = stack.transit.as_ref().unwrap();
            assert_eq!(transit.operation, NavOperation::Pop);
            assert_eq!(transit.outgoing, pages[2]);
        });
    }
}
