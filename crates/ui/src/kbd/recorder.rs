use std::time::{Duration, Instant};

use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    KeybindingKeystroke, MouseButton, ParentElement as _, Render, SharedString, Styled as _,
    Subscription, Window, div, prelude::FluentBuilder as _, px,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
};

use super::Kbd;

const DEFAULT_MAX_KEYSTROKES: usize = 3;
const CANCEL_ESCAPE_COUNT: usize = 3;
const CANCEL_ESCAPE_WINDOW: Duration = Duration::from_millis(300);

/// Events emitted by [`KeystrokeRecorder`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeystrokeRecorderEvent {
    /// The currently recorded keystroke sequence changed.
    Changed(Vec<SharedString>),
    /// The user entered the recorder's cancel sequence.
    Cancel,
}

/// A focused shortcut recorder that captures and displays key sequences.
///
/// The recorder uses [`Kbd`] for each captured keystroke and component
/// [`Button`]s for its recording controls. By default it keeps up to three
/// keystrokes. Press Escape three times within 300ms to emit
/// [`KeystrokeRecorderEvent::Cancel`].
pub struct KeystrokeRecorder {
    focus_handle: FocusHandle,
    recording_focus_handle: FocusHandle,
    keystrokes: Vec<SharedString>,
    max_keystrokes: usize,
    escape_count: usize,
    last_escape_at: Option<Instant>,
    _subscription: Subscription,
}

impl KeystrokeRecorder {
    /// Create a shortcut recorder.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let listener = cx.listener(|this, event: &gpui::KeystrokeEvent, window, cx| {
            this.record_keystroke(event, window, cx);
        });
        let subscription = cx.intercept_keystrokes(listener);

        Self {
            focus_handle: cx.focus_handle(),
            recording_focus_handle: cx.focus_handle(),
            keystrokes: Vec::new(),
            max_keystrokes: DEFAULT_MAX_KEYSTROKES,
            escape_count: 0,
            last_escape_at: None,
            _subscription: subscription,
        }
    }

    /// Set the maximum number of keystrokes retained in one sequence.
    pub fn max_keystrokes(mut self, max_keystrokes: usize) -> Self {
        self.max_keystrokes = max_keystrokes.max(1);
        self
    }

    /// Return the canonical GPUI keybinding strings recorded so far.
    pub fn keystrokes(&self) -> &[SharedString] {
        &self.keystrokes
    }

    /// Start recording.
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.recording_focus_handle.focus(window, cx);
    }

    /// Stop recording while keeping the current sequence.
    pub fn stop(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle.focus(window, cx);
    }

    /// Clear the current sequence.
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.keystrokes.clear();
        self.reset_cancel_sequence();
        cx.emit(KeystrokeRecorderEvent::Changed(Vec::new()));
        cx.notify();
    }

    fn record_keystroke(
        &mut self,
        event: &gpui::KeystrokeEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.recording_focus_handle.is_focused(window) {
            return;
        }

        cx.stop_propagation();

        let keystroke: SharedString = KeybindingKeystroke::new_with_mapper(
            event.keystroke.clone(),
            false,
            cx.keyboard_mapper().as_ref(),
        )
        .unparse()
        .into();

        if self.cancel_sequence_detected(keystroke.as_ref(), Instant::now()) {
            cx.emit(KeystrokeRecorderEvent::Cancel);
            return;
        }

        if self.keystrokes.len() >= self.max_keystrokes {
            self.keystrokes.clear();
        }
        self.keystrokes.push(keystroke);
        cx.emit(KeystrokeRecorderEvent::Changed(self.keystrokes.clone()));
        cx.notify();
    }

    fn cancel_sequence_detected(&mut self, keystroke: &str, now: Instant) -> bool {
        if keystroke != "escape" {
            self.reset_cancel_sequence();
            return false;
        }

        self.escape_count = if self
            .last_escape_at
            .is_some_and(|then| now.duration_since(then) <= CANCEL_ESCAPE_WINDOW)
        {
            self.escape_count + 1
        } else {
            1
        };
        self.last_escape_at = Some(now);
        self.escape_count >= CANCEL_ESCAPE_COUNT
    }

    fn reset_cancel_sequence(&mut self) {
        self.escape_count = 0;
        self.last_escape_at = None;
    }
}

impl EventEmitter<KeystrokeRecorderEvent> for KeystrokeRecorder {}

impl Focusable for KeystrokeRecorder {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for KeystrokeRecorder {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let recording = self.recording_focus_handle.is_focused(window);
        let entity_id = cx.entity().entity_id();
        let center = if self.keystrokes.is_empty() {
            div()
                .text_color(cx.theme().muted_foreground.opacity(0.7))
                .text_sm()
                .child(if recording {
                    t!("KeystrokeRecorder.recording")
                } else {
                    t!("KeystrokeRecorder.placeholder")
                })
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .justify_center()
                .gap_2()
                .children(self.keystrokes.iter().map(|keystroke| {
                    gpui::Keystroke::parse(keystroke.as_ref())
                        .map(|keystroke| Kbd::new(keystroke).into_any_element())
                        .unwrap_or_else(|_| div().child(keystroke.clone()).into_any_element())
                }))
                .into_any_element()
        };

        let left = div()
            .w(px(64.))
            .flex()
            .items_center()
            .gap_1()
            .when(recording, |this| {
                this.child(div().size_2().rounded_full().bg(cx.theme().danger))
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().danger)
                            .child(t!("KeystrokeRecorder.rec")),
                    )
            });

        let right = div()
            .w(px(112.))
            .flex()
            .items_center()
            .justify_end()
            .gap_1()
            .when(recording, |this| {
                this.child(
                    Button::new(("keystroke-recorder-stop", entity_id))
                        .small()
                        .ghost()
                        .label(t!("KeystrokeRecorder.stop"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.stop(window, cx);
                            cx.stop_propagation();
                        })),
                )
                .child(
                    Button::new(("keystroke-recorder-clear", entity_id))
                        .small()
                        .ghost()
                        .label(t!("KeystrokeRecorder.clear"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.clear(cx);
                            this.focus(window, cx);
                            cx.stop_propagation();
                        })),
                )
            })
            .when(!recording, |this| {
                this.child(
                    Button::new(("keystroke-recorder-record", entity_id))
                        .small()
                        .ghost()
                        .label(t!("KeystrokeRecorder.record"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.focus(window, cx);
                            cx.stop_propagation();
                        })),
                )
            });

        div()
            .key_context("KeystrokeRecorder")
            .track_focus(&self.focus_handle)
            .child(
                div()
                    .track_focus(&self.recording_focus_handle)
                    .min_h(px(40.))
                    .w_full()
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .rounded(cx.theme().radius)
                    .border_1()
                    .border_color(if recording {
                        cx.theme().ring
                    } else {
                        cx.theme().input
                    })
                    .bg(if recording {
                        cx.theme().tokens.list_active.background
                    } else {
                        cx.theme().tokens.popover.background
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, window, cx| {
                            this.focus(window, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .child(left)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(center),
                    )
                    .child(right),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, TestAppContext};

    #[gpui::test]
    fn three_quick_escape_keystrokes_cancel(cx: &mut TestAppContext) {
        let recorder = cx.new(KeystrokeRecorder::new);
        let now = Instant::now();

        recorder.update(cx, |recorder, _| {
            assert!(!recorder.cancel_sequence_detected("escape", now));
            assert!(!recorder.cancel_sequence_detected("escape", now + Duration::from_millis(100)));
            assert!(recorder.cancel_sequence_detected("escape", now + Duration::from_millis(200)));
        });
    }

    #[gpui::test]
    fn another_key_resets_the_cancel_sequence(cx: &mut TestAppContext) {
        let recorder = cx.new(KeystrokeRecorder::new);
        let now = Instant::now();

        recorder.update(cx, |recorder, _| {
            assert!(!recorder.cancel_sequence_detected("escape", now));
            assert!(!recorder.cancel_sequence_detected("escape", now + Duration::from_millis(50)));
            assert!(!recorder.cancel_sequence_detected("enter", now + Duration::from_millis(100)));
            assert!(!recorder.cancel_sequence_detected("escape", now + Duration::from_millis(150)));
        });
    }
}
