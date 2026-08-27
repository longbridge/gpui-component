#[cfg(not(target_family = "wasm"))]
use std::time::Instant;
use std::{rc::Rc, time::Duration};
#[cfg(target_family = "wasm")]
use web_time::Instant;

use gpui::{App, ElementId, SharedString, SpringConfig, SpringState, SpringTarget, Window};

use crate::animation::{Lerp, ease_out_cubic};

/// Matches GPUI's own default spring settling tolerance.
const DEFAULT_SPRING_EPSILON: f32 = 0.001;

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

/// A physical spring policy for [`spring`].
///
/// A spring is the counterpart to [`Transition`] for values that can be
/// retargeted while they are still moving. A duration-based transition restarts
/// its easing from the value sampled at that instant, which is continuous in
/// position but not in velocity. A spring carries velocity across the retarget,
/// so a value reversed mid-flight decelerates and turns around instead of
/// snapping to a new curve's initial speed.
#[derive(Clone, Copy)]
pub struct Spring {
    response: Duration,
    damping: f32,
    epsilon: f32,
    travel: bool,
}

impl Spring {
    /// Builds a spring that reaches its target in about `response` without
    /// overshooting it.
    ///
    /// `response` is not a duration in the sense [`Transition::new`] means one.
    /// A spring has no end to schedule: this is the period one full oscillation
    /// would take without damping, which is the scale the motion is felt at
    /// rather than the moment it stops. The remaining fraction of a percent
    /// keeps settling past it, until it is within the tolerance
    /// [`Self::with_epsilon`] sets.
    ///
    /// A zero response adopts the target on the spot, as a zero duration does
    /// for a transition. Say that with [`Self::with_travel`] where it is what
    /// you mean; a zero here is the degenerate case, defined so an infinitely
    /// stiff spring resolves rather than dividing by its own period.
    pub const fn new(response: Duration) -> Self {
        Self {
            response,
            damping: 1.0,
            epsilon: DEFAULT_SPRING_EPSILON,
            travel: true,
        }
    }

    /// Sets the damping ratio, which is `1.0` — no overshoot — by default.
    ///
    /// Below `1.0` the spring passes its target and comes back; above `1.0` it
    /// approaches slowly. Overshoot suits a value with room to pass its target
    /// and nothing to collide with. A height, an opacity, or anything bounded by
    /// the geometry around it should stay at the default.
    ///
    /// This is $\zeta$, not GPUI's `SpringConfig::damping`, which is the
    /// coefficient $c = 2 \zeta \omega_0$.
    pub const fn with_damping(mut self, ratio: f32) -> Self {
        self.damping = ratio;
        self
    }

    /// Sets whether the spring travels to its target or adopts it on the spot.
    ///
    /// A value the pointer is already moving — a panel being dragged by its
    /// resize handle — must not lag behind the pointer, so the spring stops
    /// travelling for as long as the drag lasts. Retained state stays pinned to
    /// the target meanwhile, so travel resumes from the value the drag released
    /// rather than from wherever the spring was when it began.
    ///
    /// This says at the call that the motion is suspended, and it says it
    /// without disturbing the response, damping or tolerance the spring is
    /// configured with — which a policy swapped out for the length of the drag
    /// would have to restate or discard.
    pub const fn with_travel(mut self, travel: bool) -> Self {
        self.travel = travel;
        self
    }

    /// Sets the settling tolerance, expressed in the target's own units.
    ///
    /// The default suits targets that move within a normalized `0..1` range. A
    /// spring over pixels settles perceptibly sooner with a coarser tolerance,
    /// which also ends the animation frames that the remaining sub-pixel motion
    /// would otherwise request.
    pub const fn with_epsilon(mut self, epsilon: f32) -> Self {
        self.epsilon = epsilon;
        self
    }

    /// The physical parameters GPUI integrates. The response must be non-zero;
    /// [`spring`] adopts the target before reaching here when it is not.
    ///
    /// Derived on use rather than stored, so the builders stay `const`: neither
    /// `Duration::as_secs_f32` nor the square root that recovers a damping ratio
    /// from a built config can be called from a `const fn`.
    fn config(&self) -> SpringConfig {
        let frequency = std::f32::consts::TAU / self.response.as_secs_f32();
        SpringConfig::new(frequency * frequency, 2.0 * self.damping * frequency, 1.0)
    }
}

#[derive(Clone, Copy)]
struct SpringTransition {
    state: SpringState,
    target: f32,
    updated_at: Instant,
}

/// Returns the current value for a spring travelling toward `target`.
///
/// State is keyed by `id` exactly as [`transition`] keys its own. The first
/// value is adopted immediately; later target changes preserve both the current
/// position and the current velocity, so an interrupted spring is redirected
/// rather than restarted.
///
/// Call this while rendering an element, where GPUI keyed element state is
/// available. A channel id must identify one value within that element.
pub fn spring<T>(
    id: impl Into<TransitionId>,
    target: T,
    policy: Spring,
    window: &mut Window,
    cx: &mut App,
) -> T::Output
where
    T: SpringTarget,
{
    let id: ElementId = id.into().into();
    let now = cx.background_executor().now();
    let target_position = target.target();
    let state = window.use_keyed_state(id, cx, |_, _| SpringTransition {
        state: SpringState {
            position: target_position,
            velocity: 0.0,
        },
        target: target_position,
        updated_at: now,
    });

    let snapshot = *state.read(cx);
    let at_rest_on_target =
        snapshot.state.position == target_position && snapshot.state.velocity == 0.0;

    // The overwhelmingly common case: a spring nothing is currently moving. It
    // has no state to advance and no frame to ask for, so it never builds a
    // config or steps one — a settled spring costs a read and two comparisons.
    // Every branch below would return this same value and write nothing.
    //
    // Resting writes nothing, so `updated_at` goes stale for as long as the rest
    // lasts. The next retarget then steps a zero displacement at zero velocity
    // over that whole gap, which any elapsed time leaves where it is, so the
    // stale clock cannot move the value — it only has to not produce a NaN, and
    // every term the propagator scales is finite.
    if at_rest_on_target {
        return target.resolve(target_position);
    }

    let settle = |state: &mut SpringTransition| {
        state.state = SpringState {
            position: target_position,
            velocity: 0.0,
        };
        state.target = target_position;
        state.updated_at = now;
    };

    if cx.reduce_motion() || !policy.travel || policy.response.is_zero() {
        state.update(cx, |state, _| settle(state));
        return target.resolve(target_position);
    }

    // Advance over the frame that just elapsed, which the previous target
    // governed, before adopting the new one for the frame to come.
    let elapsed = now
        .saturating_duration_since(snapshot.updated_at)
        .as_secs_f32();
    let config = policy.config();
    let stepped = config.step(snapshot.state, snapshot.target, elapsed);

    if config.is_settled(stepped, target_position, policy.epsilon) {
        state.update(cx, |state, _| settle(state));
        return target.resolve(target_position);
    }

    state.update(cx, |state, _| {
        state.state = stepped;
        state.target = target_position;
        state.updated_at = now;
    });
    window.request_animation_frame();
    target.resolve(stepped.position)
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

    struct SpringView {
        target: Rc<Cell<f32>>,
        policy: Rc<Cell<Spring>>,
        samples: Rc<RefCell<Vec<f32>>>,
    }

    impl Render for SpringView {
        fn render(
            &mut self,
            window: &mut Window,
            cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            self.samples.borrow_mut().push(spring(
                ("spring-test", "value"),
                self.target.get(),
                self.policy.get(),
                window,
                cx,
            ));
            Empty
        }
    }

    struct SpringFixture {
        window: WindowHandle<SpringView>,
        target: Rc<Cell<f32>>,
        policy: Rc<Cell<Spring>>,
        samples: Rc<RefCell<Vec<f32>>>,
    }

    impl SpringFixture {
        fn open(cx: &mut TestAppContext, policy: Spring) -> Self {
            let target = Rc::new(Cell::new(0.0));
            let policy = Rc::new(Cell::new(policy));
            let samples = Rc::new(RefCell::new(Vec::new()));
            let window = cx.open_window(size(px(100.), px(100.)), {
                let target = target.clone();
                let policy = policy.clone();
                let samples = samples.clone();
                move |_, _| SpringView {
                    target,
                    policy,
                    samples,
                }
            });
            cx.run_until_parked();
            Self {
                window,
                target,
                policy,
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

        fn advance(&self, cx: &mut TestAppContext, millis: u64, target: f32) -> f32 {
            cx.executor().advance_clock(Duration::from_millis(millis));
            self.render(cx, target)
        }

        fn pending_frame(&self, cx: &mut TestAppContext) -> usize {
            self.window
                .update(cx, |_, window, cx| window.simulate_next_frame(cx))
                .unwrap()
        }
    }

    #[gpui::test]
    fn a_spring_adopts_its_first_target_immediately(cx: &mut TestAppContext) {
        let fixture = SpringFixture::open(cx, Spring::new(Duration::from_millis(300)));
        assert_eq!(*fixture.samples.borrow().first().unwrap(), 0.0);
    }

    #[gpui::test]
    fn a_spring_travels_toward_its_target_over_time(cx: &mut TestAppContext) {
        let fixture = SpringFixture::open(cx, Spring::new(Duration::from_millis(300)));
        assert_eq!(fixture.render(cx, 1.0), 0.0);

        let early = fixture.advance(cx, 50, 1.0);
        let late = fixture.advance(cx, 50, 1.0);
        assert!(
            0.0 < early && early < late && late < 1.0,
            "expected monotonic approach, got {early} then {late}"
        );
    }

    #[gpui::test]
    fn a_reversed_spring_keeps_its_momentum_before_turning_around(cx: &mut TestAppContext) {
        let fixture = SpringFixture::open(cx, Spring::new(Duration::from_millis(300)));
        fixture.render(cx, 1.0);
        let reversed_at = fixture.advance(cx, 100, 1.0);

        // Retarget mid-flight. A duration-based transition restarts its easing
        // here and moves away from 1.0 on the very next frame.
        assert_eq!(fixture.render(cx, 0.0), reversed_at);

        let next = fixture.advance(cx, 16, 0.0);
        assert!(
            next > reversed_at,
            "expected the spring to carry its velocity past {reversed_at}, got {next}"
        );

        assert_eq!(fixture.advance(cx, 1_000, 0.0), 0.0);
    }

    #[gpui::test]
    fn a_bouncy_spring_overshoots_its_target(cx: &mut TestAppContext) {
        let fixture = SpringFixture::open(
            cx,
            Spring::new(Duration::from_millis(350)).with_damping(0.7),
        );
        fixture.render(cx, 1.0);
        for _ in 0..30 {
            fixture.advance(cx, 16, 1.0);
        }

        let peak = fixture
            .samples
            .borrow()
            .iter()
            .copied()
            .fold(f32::MIN, f32::max);
        assert!(peak > 1.0, "expected an overshoot past 1.0, got {peak}");
    }

    #[gpui::test]
    fn a_settled_spring_stops_requesting_frames(cx: &mut TestAppContext) {
        let fixture = SpringFixture::open(cx, Spring::new(Duration::from_millis(300)));
        fixture.render(cx, 1.0);
        assert_eq!(fixture.pending_frame(cx), 1);

        assert_eq!(fixture.advance(cx, 2_000, 1.0), 1.0);
        fixture.pending_frame(cx);
        cx.run_until_parked();
        assert_eq!(fixture.pending_frame(cx), 0);
    }

    #[gpui::test]
    fn a_spring_that_is_not_travelling_adopts_its_target_on_the_spot(cx: &mut TestAppContext) {
        let travelling = Spring::new(Duration::from_millis(300));
        let fixture = SpringFixture::open(cx, travelling.with_travel(false));

        assert_eq!(fixture.render(cx, 1.0), 1.0);
        assert_eq!(fixture.pending_frame(cx), 0);
        assert_eq!(fixture.advance(cx, 100, 5.0), 5.0);

        // Travel resumes from the value the suspension left behind. A spring
        // that had kept the state it held beforehand would jump back to it here.
        fixture.policy.set(travelling);
        assert_eq!(fixture.render(cx, 6.0), 5.0);
        let next = fixture.advance(cx, 50, 6.0);
        assert!(
            5.0 < next && next < 6.0,
            "expected travel to resume from 5.0, got {next}"
        );
    }

    #[gpui::test]
    fn a_zero_response_spring_resolves_instead_of_dividing_by_its_period(cx: &mut TestAppContext) {
        let fixture = SpringFixture::open(cx, Spring::new(Duration::ZERO));
        assert_eq!(fixture.render(cx, 1.0), 1.0);
        assert_eq!(fixture.pending_frame(cx), 0);
    }

    #[gpui::test]
    fn reduced_motion_adopts_the_spring_target_without_requesting_a_frame(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_reduce_motion(true));
        let fixture = SpringFixture::open(
            cx,
            Spring::new(Duration::from_millis(350)).with_damping(0.7),
        );
        assert_eq!(fixture.render(cx, 1.0), 1.0);
        assert_eq!(fixture.pending_frame(cx), 0);
    }
}
