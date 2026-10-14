//! Boundary displacement shared by inputs and scroll viewports.
//!
//! Viewport owners keep their logical scroll position clamped and apply this
//! displacement only to content geometry. Inputs drive it directly, while
//! `ScrollBounce` can own a list or scroll area's vertical wheel handling.

use crate::scroll_bounce::Physics;
use gpui::TouchPhase;
use std::time::Duration;
#[cfg(not(target_family = "wasm"))]
use std::time::Instant;
#[cfg(target_family = "wasm")]
use web_time::Instant;

/// A pause without packets that ends a gesture.
///
/// Momentum arrives once per frame with no phase of its own, and some precise
/// scrolling devices send no phases, so a pause marks the end of their stream.
/// `ScrollBounce` waits the same time for the same reason.
const GESTURE_GAP: Duration = Duration::from_millis(250);

/// The fraction of the text's speed the edge carries when the text runs into
/// it or a finger lets go of it. Measured on `NSScrollView`, and the same as
/// WebKit's rubber-band amplitude: a fling arriving at 5000 px/s throws the
/// edge about 45 px out, 80 ms before it turns back.
const CARRY: f32 = 0.31;

/// A finger that sends nothing for this long before lifting has stopped:
/// it lets go of the edge with no speed.
const STILL: Duration = Duration::from_millis(50);

/// The interval assumed for the first packet of a stream, and the shortest
/// one trusted, so that coalesced packets do not read as a huge speed.
const FRAME: f32 = 1. / 60.;
const MIN_INTERVAL: f32 = 1. / 240.;

#[derive(Default)]
pub(crate) struct Overscroll {
    physics: Physics,
    sampled_at: Option<Instant>,
    last_packet_at: Option<Instant>,
    /// When the stream last moved, and how fast, in pixels per second.
    moved_at: Option<Instant>,
    velocity: f32,
    /// The edge already answered the rest of this gesture: a fling ran into
    /// it, or a finger let go of it stretched. The momentum that follows
    /// still scrolls the text back from the edge, but no longer moves the
    /// edge.
    spent: bool,
}

impl Overscroll {
    pub(crate) fn set_motion(&mut self, motion: crate::ScrollBounceMotion) {
        self.physics.set_motion(motion);
    }
    /// How far the text is drawn from where it scrolled to.
    pub(crate) fn offset(&self) -> f32 {
        self.physics.offset()
    }

    /// A new gesture starts: it may stretch the edge again.
    fn start_gesture(&mut self) {
        self.spent = false;
        self.velocity = 0.;
    }

    /// Track the stream's speed from packets that moved.
    fn sample(&mut self, dy: f32, gap: Option<Duration>, now: Instant) {
        if dy == 0. {
            return;
        }
        let seconds = gap.map_or(FRAME, |gap| gap.as_secs_f32().max(MIN_INTERVAL));
        let velocity = dy / seconds;
        // Finger packets are uneven; average the last few.
        self.velocity = if self.moved_at.is_some() && self.velocity * velocity > 0. {
            (self.velocity + velocity) / 2.
        } else {
            velocity
        };
        self.moved_at = Some(now);
    }

    /// Handle one vertical packet of `dy` over a viewport `extent` tall.
    /// `scroll` moves the text by a delta and returns how much of it the text
    /// used. Returns whether the stretch changed.
    pub(crate) fn packet(
        &mut self,
        dy: f32,
        phase: TouchPhase,
        extent: f32,
        now: Instant,
        mut scroll: impl FnMut(f32) -> f32,
    ) -> bool {
        let before = self.offset();
        let mut thrown = false;
        let gap = self
            .last_packet_at
            .replace(now)
            .map(|at| now.saturating_duration_since(at))
            .filter(|gap| *gap < GESTURE_GAP);
        // A finger resting on the trackpad sends nothing, however long it
        // holds the edge; only its own `Ended` lets go. A pause ends the
        // other streams.
        if gap.is_none() && !self.physics.is_dragging() {
            self.start_gesture();
            self.moved_at = None;
        }
        self.sample(dy, gap, now);
        if phase == TouchPhase::Started {
            // Grab the edge, a returning one included, where it is.
            self.physics.begin(extent);
            self.spent = false;
        }
        if self.physics.is_dragging() {
            // A stretched edge takes the packet first; only what crosses back
            // over the edge scrolls the text.
            let rest = if self.offset() != 0. {
                self.physics.pull(dy)
            } else {
                dy
            };
            if rest != 0. {
                let unused = rest - scroll(rest);
                if unused != 0. {
                    self.physics.pull(unused);
                }
            }
            if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                let moving = self
                    .moved_at
                    .is_some_and(|at| now.saturating_duration_since(at) < STILL);
                let velocity = if moving { self.velocity } else { 0. };
                self.physics.let_go(velocity * CARRY);
                thrown = true;
                self.spent = self.offset() != 0.;
            }
        } else if self.spent {
            // Momentum after the edge answered: back into the text it
            // scrolls; against the edge it has nothing left to move.
            scroll(dy);
        } else {
            // Momentum or phaseless precise scrolling: it scrolls the text,
            // and when the text runs into the edge it throws the edge as
            // fast as it arrived, once.
            let unused = dy - scroll(dy);
            if unused != 0. {
                self.physics.let_go(self.velocity * CARRY);
                thrown = true;
                self.spent = true;
            }
        }
        let changed = (thrown && self.physics.is_moving()) || self.offset() != before;
        // Start the clock when the edge starts moving, and again when it is
        // let go, however long it was held; a packet that arrives while it
        // moves must not lose the time since the last frame.
        if thrown || (changed && self.sampled_at.is_none()) {
            self.sampled_at = Some(now);
        }
        changed
    }

    /// Advance the return to `now`. Returns whether it is still moving.
    pub(crate) fn step(&mut self, now: Instant) -> bool {
        let elapsed = self
            .sampled_at
            .replace(now)
            .map_or(0., |at| now.saturating_duration_since(at).as_secs_f32());
        let moving = self.physics.step(elapsed);
        if !moving {
            self.sampled_at = None;
        }
        moving
    }

    /// Drop any stretch at once, as when bouncing is turned off.
    pub(crate) fn reset(&mut self) {
        let mut physics = std::mem::take(&mut self.physics);
        physics.reset();
        *self = Self {
            physics,
            ..Self::default()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Text that can scroll `room` further toward its end.
    fn text(room: &mut f32) -> impl FnMut(f32) -> f32 + '_ {
        move |dy: f32| {
            let used = dy.max(-*room);
            *room += used;
            used
        }
    }

    #[test]
    fn a_swipe_past_the_end_stretches_and_springs_back() {
        let now = Instant::now();
        let mut overscroll = Overscroll::default();
        let mut room = 30.;
        overscroll.packet(-10., TouchPhase::Started, 400., now, text(&mut room));
        overscroll.packet(-100., TouchPhase::Moved, 400., now, text(&mut room));
        // The text used the 30px it had left; the rest stretches the edge.
        assert_eq!(room, 0.);
        let stretched = overscroll.offset();
        assert!(stretched < 0. && stretched > -80., "{stretched}");

        overscroll.packet(0., TouchPhase::Ended, 400., now, text(&mut room));
        // Momentum after the release is answered by the spring alone.
        assert!(!overscroll.packet(-40., TouchPhase::Moved, 400., now, text(&mut room)));
        let mut later = now;
        while overscroll.step(later) {
            later += Duration::from_millis(16);
        }
        assert_eq!(overscroll.offset(), 0.);
    }

    #[test]
    fn pulling_back_unstretches_before_scrolling_the_text() {
        let now = Instant::now();
        let mut overscroll = Overscroll::default();
        let mut room = 0.;
        overscroll.packet(0., TouchPhase::Started, 400., now, text(&mut room));
        overscroll.packet(-50., TouchPhase::Moved, 400., now, text(&mut room));
        assert!(overscroll.offset() < 0.);
        // Back the other way: the first 50px return the edge, the next 20px
        // scroll the text back from its end.
        let mut scrolled = 0.;
        overscroll.packet(70., TouchPhase::Moved, 400., now, |dy| {
            scrolled += dy;
            dy
        });
        assert_eq!(overscroll.offset(), 0.);
        assert_eq!(scrolled, 20.);
    }

    /// A momentum stream of `speed` px/s decaying as AppKit's does, against
    /// text already at its end. Returns the edge's offset at each frame.
    fn fling(speed: f32) -> Vec<f32> {
        let start = Instant::now();
        let frame = Duration::from_micros(16_667);
        let mut overscroll = Overscroll::default();
        let mut room = 0.;
        let mut dy = -speed / 60.;
        let mut path = Vec::new();
        for n in 0..120 {
            let at = start + frame * n;
            // Momentum keeps arriving well after the edge has settled.
            if n < 100 {
                overscroll.packet(dy, TouchPhase::Moved, 300., at, text(&mut room));
                dy *= 0.95;
            }
            overscroll.step(at);
            path.push(overscroll.offset());
        }
        path
    }

    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    #[test]
    fn a_fling_throws_the_edge_as_far_as_nsscrollview_does() {
        // Measured on NSScrollView: a fling arriving at 5000 px/s peaks about
        // 45 px out, roughly 80 ms after impact, and settles within 0.65 s.
        let path = fling(5000.);
        let peak = path.iter().copied().fold(0., f32::min);
        assert!((-49. ..=-40.).contains(&peak), "peak {peak}");
        let turned = path.iter().position(|&offset| offset == peak).unwrap();
        assert!((4..=6).contains(&turned), "turned at frame {turned}");
        assert_eq!(path[50], 0., "the edge settles");
        // The momentum still arriving after impact never throws it again.
        assert!(path[turned..].windows(2).all(|pair| pair[1] >= pair[0]));
        assert!(path[50..].iter().all(|&offset| offset == 0.));
        // And twice the speed throws it about twice as far.
        let faster = fling(10_000.).iter().copied().fold(0., f32::min);
        assert!((faster / peak - 2.).abs() < 0.15, "{faster} vs {peak}");
    }

    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    #[test]
    fn a_stretch_follows_the_travel_as_nsscrollview_does() {
        // Measured on a 300 px NSScrollView: 764 px of travel past the end
        // stretches it 46 px, 2400 px stretches it 112 px.
        let stretch = |travel: f32| {
            let start = Instant::now();
            let mut overscroll = Overscroll::default();
            let mut room = 0.;
            overscroll.packet(0., TouchPhase::Started, 300., start, text(&mut room));
            overscroll.packet(-travel, TouchPhase::Moved, 300., start, text(&mut room));
            -overscroll.offset()
        };
        assert!((43. ..=50.).contains(&stretch(764.)), "{}", stretch(764.));
        assert!(
            (106. ..=118.).contains(&stretch(2400.)),
            "{}",
            stretch(2400.)
        );
    }

    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    #[test]
    fn a_still_release_returns_at_full_speed() {
        // NSScrollView lets go of a stretch held still and halves it in
        // 55 ms, starting at once rather than easing out.
        let start = Instant::now();
        let mut overscroll = Overscroll::default();
        let mut room = 0.;
        overscroll.packet(0., TouchPhase::Started, 300., start, text(&mut room));
        overscroll.packet(-800., TouchPhase::Moved, 300., start, text(&mut room));
        let held = overscroll.offset();
        let lifted = start + Duration::from_millis(200);
        overscroll.packet(0., TouchPhase::Ended, 300., lifted, text(&mut room));
        overscroll.step(lifted + Duration::from_millis(8));
        assert!(overscroll.offset() / held < 0.92, "starts at once");
        overscroll.step(lifted + Duration::from_millis(55));
        let half = overscroll.offset() / held;
        assert!((0.45..=0.55).contains(&half), "{half}");
    }

    #[test]
    fn after_a_pause_the_next_flick_throws_the_edge_again() {
        let start = Instant::now();
        let mut overscroll = Overscroll::default();
        let mut room = 0.;
        assert!(overscroll.packet(-40., TouchPhase::Moved, 400., start, text(&mut room)));
        let mut at = start;
        while overscroll.step(at) {
            at += Duration::from_millis(16);
        }
        let later = at + GESTURE_GAP;
        assert!(overscroll.packet(-40., TouchPhase::Moved, 400., later, text(&mut room)));
    }

    #[test]
    fn momentum_back_into_the_text_scrolls_while_the_edge_returns() {
        let start = Instant::now();
        let mut overscroll = Overscroll::default();
        let mut room = 0.;
        overscroll.packet(0., TouchPhase::Started, 400., start, text(&mut room));
        overscroll.packet(-800., TouchPhase::Moved, 400., start, text(&mut room));
        let lifted = start + Duration::from_millis(100);
        overscroll.packet(0., TouchPhase::Ended, 400., lifted, text(&mut room));
        let later = lifted + Duration::from_millis(16);
        overscroll.step(later);
        let before = overscroll.offset();
        assert!(before < 0.);
        overscroll.packet(20., TouchPhase::Moved, 400., later, text(&mut room));
        assert_eq!(room, 20.);
        assert_eq!(overscroll.offset(), before, "the edge does not jump");
        overscroll.step(later + Duration::from_millis(16));
        assert!(overscroll.offset() > before && overscroll.offset() < 0.);
    }

    #[test]
    fn a_new_precise_gesture_takes_over_without_jumping() {
        let start = Instant::now();
        let mut overscroll = Overscroll::default();
        let mut room = 0.;
        overscroll.packet(-80., TouchPhase::Moved, 400., start, text(&mut room));
        overscroll.step(start + Duration::from_millis(16));
        let later = start + Duration::from_millis(200);
        overscroll.step(later);
        assert!(overscroll.offset() < 0.);
        let before = overscroll.offset();
        overscroll.packet(0., TouchPhase::Started, 400., later, text(&mut room));
        assert!(
            (overscroll.offset() - before).abs() < 0.001,
            "interruption does not jump"
        );
        assert!(overscroll.packet(-40., TouchPhase::Moved, 400., later, text(&mut room)));
        overscroll.step(later + Duration::from_millis(16));
        assert!(
            overscroll.offset() < before,
            "the new input pushes outward again"
        );
    }
}
