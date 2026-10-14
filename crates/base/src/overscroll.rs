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

/// How far a fling or a precise flick that runs into the edge throws it, per
/// pixel it had left to scroll, and how far at most.
///
/// Those packets measure speed rather than a finger's travel: a fast fling
/// steps hundreds of pixels at once. So they throw the edge, and the spring
/// carries it out and back, instead of moving it there in one frame.
const KICK_PER_PIXEL: f32 = 0.3;
const MAX_KICK: f32 = 30.;

#[derive(Default)]
pub(crate) struct Overscroll {
    physics: Physics,
    sampled_at: Option<Instant>,
    last_packet_at: Option<Instant>,
    /// The edge already answered the rest of this gesture: the momentum after
    /// a stretched release. The spring plays; the text stays where it is.
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
        if self.physics.is_dragging() {
            self.physics.release();
        }
        self.spent = false;
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
        // Phaseless devices can reverse without sending Started. A deliberate
        // reversal takes over from the returning edge immediately.
        if dy * before < 0. && !self.physics.is_dragging() {
            self.reset();
        }
        let mut kicked = false;
        let paused = self
            .last_packet_at
            .replace(now)
            .is_none_or(|at| now.saturating_duration_since(at) >= GESTURE_GAP);
        if paused {
            self.start_gesture();
        }
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
                self.physics.release();
                self.spent = self.offset() != 0.;
            }
        } else if !self.spent {
            // Momentum or phaseless precise scrolling: it scrolls the
            // text, and a fling or a flick that runs into the edge throws it,
            // harder the faster it arrives, until the edge turns back.
            let unused = dy - scroll(dy);
            kicked = self.physics.kick(unused * KICK_PER_PIXEL, MAX_KICK);
        }
        let changed = kicked || self.offset() != before;
        // Start the clock when the edge starts moving; a packet that arrives
        // while it moves must not lose the time since the last frame.
        if changed && self.sampled_at.is_none() {
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

    #[test]
    fn a_flick_into_the_edge_throws_it_out_and_back_smoothly() {
        let start = Instant::now();
        let frame = Duration::from_millis(16);
        let mut overscroll = Overscroll::default();
        let mut room = 0.;
        // A precise flick at the end: a small packet, then larger ones, one a
        // frame; and one more after the edge has turned back.
        let packets = [
            (0, -2.),
            (1, -20.),
            (2, -80.),
            (3, -110.),
            (4, -130.),
            (12, -100.),
        ];
        let mut path = Vec::new();
        for n in 0..60 {
            let at = start + frame * n;
            if let Some((_, dy)) = packets.iter().find(|(at_frame, _)| *at_frame == n) {
                overscroll.packet(*dy, TouchPhase::Moved, 400., at, text(&mut room));
            }
            overscroll.step(at);
            path.push(overscroll.offset());
        }
        assert_eq!(path[0], 0., "the edge starts where it is");
        let peak = path.iter().copied().fold(0., f32::min);
        assert!(peak < -10. && peak >= -MAX_KICK, "peak {peak}");
        let largest_step = path
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0., f32::max);
        // It glides: no frame covers half of the throw.
        assert!(
            largest_step < MAX_KICK / 2.,
            "a frame moved the edge {largest_step}px"
        );
        let turned = path.iter().position(|&offset| offset == peak).unwrap();
        assert!(turned < 12, "the edge turned back before the late packet");
        assert!(
            path[turned..].windows(2).all(|pair| pair[1] >= pair[0]),
            "a packet after the edge turned back does not throw it again"
        );
        assert_eq!(*path.last().unwrap(), 0., "the edge settles");
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
    fn a_reverse_packet_takes_over_the_return_immediately() {
        let start = Instant::now();
        let mut overscroll = Overscroll::default();
        let mut room = 0.;
        overscroll.packet(0., TouchPhase::Started, 400., start, text(&mut room));
        overscroll.packet(-80., TouchPhase::Moved, 400., start, text(&mut room));
        overscroll.packet(0., TouchPhase::Ended, 400., start, text(&mut room));
        let later = start + Duration::from_millis(16);
        overscroll.step(later);
        assert!(overscroll.offset() < 0.);
        overscroll.packet(20., TouchPhase::Moved, 400., later, text(&mut room));
        assert_eq!(room, 20.);
        assert_eq!(overscroll.offset(), 0.);
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
