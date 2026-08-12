#[cfg(not(target_family = "wasm"))]
use std::time::Instant;
use std::{rc::Rc, time::Duration};
#[cfg(target_family = "wasm")]
use web_time::Instant;

use gpui::{App, ElementId, SharedString, Window};

use crate::animation::{Lerp, ease_out_cubic};

/// A value that can be interpolated between two application-owned targets.
pub trait Interpolate: Clone {
    fn interpolate(&self, target: &Self, progress: f32) -> Self;
}

impl<T: Lerp> Interpolate for T {
    fn interpolate(&self, target: &Self, progress: f32) -> Self {
        self.lerp(target, progress)
    }
}

/// CSS-like timing policy for a target-value transition.
///
/// This type is intentionally separate from [`crate::animation::Transition`],
/// whose legacy interface applies concrete fade, slide, and size effects to an
/// element. A value transition never chooses a visual property for the caller.
#[derive(Clone)]
pub struct Transition {
    duration: Duration,
    delay: Duration,
    easing: Rc<dyn Fn(f32) -> f32>,
}

impl Transition {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            delay: Duration::ZERO,
            easing: Rc::new(ease_out_cubic),
        }
    }

    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn ease(mut self, easing: impl Fn(f32) -> f32 + 'static) -> Self {
        self.easing = Rc::new(easing);
        self
    }

    fn sample(&self, progress: f32) -> f32 {
        (self.easing)(progress.clamp(0.0, 1.0))
    }

    fn progress(&self, elapsed: Duration) -> f32 {
        if elapsed <= self.delay {
            return 0.0;
        }
        if self.duration.is_zero() {
            return 1.0;
        }
        (elapsed.saturating_sub(self.delay).as_secs_f32() / self.duration.as_secs_f32()).min(1.0)
    }
}

/// Identifies one independently transitioning value.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TransitionId(ElementId);

impl From<ElementId> for TransitionId {
    fn from(id: ElementId) -> Self {
        Self(id)
    }
}

impl From<&'static str> for TransitionId {
    fn from(id: &'static str) -> Self {
        Self(id.into())
    }
}

impl From<String> for TransitionId {
    fn from(id: String) -> Self {
        Self(id.into())
    }
}

impl From<SharedString> for TransitionId {
    fn from(id: SharedString) -> Self {
        Self(id.into())
    }
}

impl From<usize> for TransitionId {
    fn from(id: usize) -> Self {
        Self(id.into())
    }
}

impl From<i32> for TransitionId {
    fn from(id: i32) -> Self {
        Self(id.into())
    }
}

impl From<TransitionId> for ElementId {
    fn from(id: TransitionId) -> Self {
        ElementId::NamedChild(id.0.into(), "__base-transition-state".into())
    }
}

impl<I, C> From<(I, C)> for TransitionId
where
    I: Into<ElementId>,
    C: Into<SharedString>,
{
    fn from((id, channel): (I, C)) -> Self {
        Self(ElementId::NamedChild(id.into().into(), channel.into()))
    }
}

#[derive(Clone)]
struct ValueTransition<T> {
    from: T,
    target: T,
    started_at: Instant,
}

/// Returns the current value for a CSS-like transition toward `target`.
///
/// State is keyed by `id`. The first value is adopted immediately; later target
/// changes transition from the value sampled at that instant. Components opt
/// into this function explicitly—base components do not install default motion.
///
/// Call this while rendering an element, where GPUI keyed element state is
/// available. A channel id must identify one value type within that element.
pub fn transition<T>(
    id: impl Into<TransitionId>,
    target: T,
    policy: Transition,
    window: &mut Window,
    cx: &mut App,
) -> T
where
    T: Interpolate + PartialEq + 'static,
{
    let id: ElementId = id.into().into();
    let now = cx.background_executor().now();
    let state = window.use_keyed_state(id, cx, |_, _| ValueTransition {
        from: target.clone(),
        target: target.clone(),
        started_at: now,
    });

    let snapshot = state.read(cx).clone();

    if cx.reduce_motion() || policy.duration.is_zero() {
        if snapshot.from != target || snapshot.target != target {
            state.update(cx, |state, _| {
                state.from = target.clone();
                state.target = target.clone();
                state.started_at = now;
            });
        }
        return target;
    }

    let elapsed = now.saturating_duration_since(snapshot.started_at);
    let progress = policy.progress(elapsed);
    let sampled = snapshot
        .from
        .interpolate(&snapshot.target, policy.sample(progress));

    let (value, running) = if snapshot.target != target {
        state.update(cx, |state, _| {
            state.from = sampled.clone();
            state.target = target.clone();
            state.started_at = now;
        });
        (sampled, true)
    } else {
        (sampled, progress < 1.0 && snapshot.from != snapshot.target)
    };
    if running {
        window.request_animation_frame();
    }
    value
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        time::Duration,
    };

    use gpui::{Empty, IntoElement, Render, TestAppContext, WindowHandle, px, size};

    use super::*;

    #[test]
    fn transition_ids_accept_element_like_scalars_and_named_channels() {
        assert_eq!(
            TransitionId::from("opacity"),
            TransitionId::from(ElementId::from("opacity"))
        );
        assert_ne!(
            TransitionId::from(("terms", "fill")),
            TransitionId::from(("terms", "mark-opacity"))
        );
        let _: TransitionId = 7usize.into();
        let _: TransitionId = 7i32.into();
    }

    struct TestView {
        target: Rc<Cell<f32>>,
        duration: Duration,
        samples: Rc<RefCell<Vec<f32>>>,
    }

    impl Render for TestView {
        fn render(
            &mut self,
            window: &mut Window,
            cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            self.samples.borrow_mut().push(transition(
                ("test", "value"),
                self.target.get(),
                Transition::new(self.duration).ease(|t| t),
                window,
                cx,
            ));
            Empty
        }
    }

    struct DelayedView {
        target: Rc<Cell<f32>>,
        samples: Rc<RefCell<Vec<f32>>>,
    }

    impl Render for DelayedView {
        fn render(
            &mut self,
            window: &mut Window,
            cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            self.samples.borrow_mut().push(transition(
                ("delayed-test", "value"),
                self.target.get(),
                Transition::new(Duration::from_millis(100))
                    .delay(Duration::from_millis(50))
                    .ease(|t| t),
                window,
                cx,
            ));
            Empty
        }
    }

    struct Fixture {
        window: WindowHandle<TestView>,
        target: Rc<Cell<f32>>,
        samples: Rc<RefCell<Vec<f32>>>,
    }

    impl Fixture {
        fn open(cx: &mut TestAppContext, duration: Duration) -> Self {
            let target = Rc::new(Cell::new(0.0));
            let samples = Rc::new(RefCell::new(Vec::new()));
            let window = cx.open_window(size(px(100.), px(100.)), {
                let target = target.clone();
                let samples = samples.clone();
                move |_, _| TestView {
                    target,
                    duration,
                    samples,
                }
            });
            cx.run_until_parked();
            Self {
                window,
                target,
                samples,
            }
        }

        fn render(&self, cx: &mut TestAppContext, target: f32) -> f32 {
            self.target.set(target);
            self.window
                .update(cx, |_, window, _| window.refresh())
                .unwrap();
            cx.run_until_parked();
            *self.samples.borrow().last().unwrap()
        }

        fn pending_frame(&self, cx: &mut TestAppContext) -> usize {
            self.window
                .update(cx, |_, window, cx| window.simulate_next_frame(cx))
                .unwrap()
        }
    }

    #[gpui::test]
    fn a_zero_duration_target_change_is_immediate(cx: &mut TestAppContext) {
        let fixture = Fixture::open(cx, Duration::ZERO);
        assert_eq!(fixture.render(cx, 1.0), 1.0);
    }

    #[gpui::test]
    fn a_changed_target_transitions_over_time(cx: &mut TestAppContext) {
        let duration = Duration::from_millis(100);
        let fixture = Fixture::open(cx, duration);
        assert_eq!(fixture.render(cx, 10.0), 0.0);

        cx.executor().advance_clock(Duration::from_millis(50));
        assert_eq!(fixture.render(cx, 10.0), 5.0);
    }

    #[gpui::test]
    fn requested_animation_frames_resample_without_manual_refresh(cx: &mut TestAppContext) {
        let duration = Duration::from_millis(100);
        let fixture = Fixture::open(cx, duration);
        assert_eq!(fixture.render(cx, 10.0), 0.0);

        cx.executor().advance_clock(Duration::from_millis(50));
        assert_eq!(fixture.pending_frame(cx), 1);
        cx.run_until_parked();

        assert_eq!(*fixture.samples.borrow().last().unwrap(), 5.0);
    }

    #[gpui::test]
    fn reversing_uses_the_current_sample_without_jumping(cx: &mut TestAppContext) {
        let duration = Duration::from_millis(100);
        let fixture = Fixture::open(cx, duration);
        assert_eq!(fixture.render(cx, 10.0), 0.0);

        cx.executor().advance_clock(Duration::from_millis(50));
        assert_eq!(fixture.render(cx, 0.0), 5.0);
        cx.executor().advance_clock(Duration::from_millis(25));
        assert_eq!(fixture.render(cx, 0.0), 3.75);
    }

    #[gpui::test]
    fn delay_holds_the_previous_value_before_interpolation(cx: &mut TestAppContext) {
        let target = Rc::new(Cell::new(0.0));
        let samples = Rc::new(RefCell::new(Vec::new()));
        let window = cx.open_window(size(px(100.), px(100.)), {
            let target = target.clone();
            let samples = samples.clone();
            move |_, _| DelayedView { target, samples }
        });
        cx.run_until_parked();

        target.set(10.0);
        window.update(cx, |_, window, _| window.refresh()).unwrap();
        cx.run_until_parked();
        assert_eq!(*samples.borrow().last().unwrap(), 0.0);

        cx.executor().advance_clock(Duration::from_millis(50));
        window.update(cx, |_, window, _| window.refresh()).unwrap();
        cx.run_until_parked();
        assert_eq!(*samples.borrow().last().unwrap(), 0.0);

        cx.executor().advance_clock(Duration::from_millis(50));
        window.update(cx, |_, window, _| window.refresh()).unwrap();
        cx.run_until_parked();
        assert_eq!(*samples.borrow().last().unwrap(), 5.0);
    }

    #[gpui::test]
    fn a_completed_transition_stops_requesting_frames(cx: &mut TestAppContext) {
        let duration = Duration::from_millis(100);
        let fixture = Fixture::open(cx, duration);
        fixture.render(cx, 1.0);
        assert_eq!(fixture.pending_frame(cx), 1);

        cx.executor().advance_clock(duration);
        assert_eq!(fixture.render(cx, 1.0), 1.0);
        fixture.pending_frame(cx);
        cx.run_until_parked();
        assert_eq!(fixture.pending_frame(cx), 0);
    }

    #[gpui::test]
    fn reduced_motion_adopts_the_target_without_requesting_a_frame(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_reduce_motion(true));
        let duration = Duration::from_millis(100);
        let fixture = Fixture::open(cx, duration);
        assert_eq!(fixture.render(cx, 1.0), 1.0);
        assert_eq!(fixture.pending_frame(cx), 0);
    }
}
