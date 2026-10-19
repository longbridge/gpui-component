//! Application-owned completion around a single-line [`Input`].
//!
//! Typing `$` opens a small menu of shell-style variables; typing filters it,
//! the keyboard drives it, and accepting replaces the typed `$prefix` with
//! the completed `$NAME `.
//!
//! The point of this example: **`InputState` carries no completion state.**
//! The candidate data, filtering, ranking, popup rendering, keyboard
//! ownership and the edit applied on accept all live in the application,
//! composed from the input's existing public API:
//!
//! - `InputEvent::Change` → re-read `value()` + `cursor()` and rescan the
//!   trigger (this owns trigger detection and dismissal).
//! - `capture_action` on the input's wrapper → own `MoveUp` / `MoveDown` /
//!   `Escape` / `IndentInline` before the input sees them while the menu is
//!   open (keyboard ownership).
//! - `InputEvent::PressEnter` → the app decides whether Enter accepts a
//!   candidate or means "submit".
//! - `set_selected_range` + `replace` → select the typed `$prefix` and swap
//!   it (one atomic undo step).
//!
//! The menu anchors to the input region, which keeps the example small;
//! applications that need caret anchoring can resolve it with
//! `cursor_layout()` / `range_to_bounds()` instead.

use gpui_kit::assets::Assets;
use gpui_kit::base::input::{Escape, IndentInline, MoveDown, MoveUp};
use gpui_kit::component::{
    input::{Input, InputEvent, InputState},
    *,
};
use gpui_kit::gpui::prelude::FluentBuilder as _;
use gpui_kit::*;

use std::ops::Range;

/// The static candidate table. A real application would fetch or compute
/// candidates per context — the input never knows they exist.
const VARIABLES: &[(&str, &str)] = &[
    ("EDITOR", "default editor"),
    ("HOME", "user home directory"),
    ("LANG", "locale identifier"),
    ("PATH", "executable search path"),
    ("PWD", "working directory"),
    ("SHELL", "login shell"),
    ("TERM", "terminal type"),
    ("USER", "current user name"),
];

/// Popup width: a narrow column, wide enough for the longest description.
const MENU_WIDTH: Pixels = px(280.);

/// An open menu. `token_start` is the byte offset of the `$` that opened it;
/// offsets are the units `cursor()` and `set_selected_range()` speak.
#[derive(Clone)]
struct Menu {
    token_start: usize,
    /// Filtered candidate names, in display order.
    items: Vec<&'static str>,
    selected: usize,
}

impl Menu {
    fn selected_name(&self) -> Option<&'static str> {
        self.items.get(self.selected).copied()
    }
}

pub struct Example {
    input_state: Entity<InputState>,
    /// `Some` while the menu is open; dropped on every dismissal path.
    menu: Option<Menu>,
    /// Last accepted completion or submission, echoed under the input.
    status: SharedString,

    /// Kept alive with the entity: dropping the `Example` drops its
    /// subscriptions.
    _subscription: Subscription,
}

/// Scan backwards from `cursor` for a `$`-triggered token.
///
/// Returns `(token_start, query)` when the caret sits inside a token — the
/// token starts at `$`, and `$` itself must sit at the start of the line or
/// after whitespace, so `a$b` and `mail$x` never trigger. `None` means the
/// caret is outside any token: the caller dismisses.
fn active_variable_query(text: &str, cursor: usize) -> Option<(usize, String)> {
    let bytes = text.as_bytes();
    let cursor = cursor.min(bytes.len());
    let mut start = cursor;
    while start > 0 {
        let b = bytes[start - 1];
        if b.is_ascii_alphanumeric() || b == b'_' {
            start -= 1;
        } else {
            break;
        }
    }
    // The character before the identifier run must be the `$` trigger,
    // and the `$` must open the token (start of text or after whitespace).
    if start == 0 || bytes[start - 1] != b'$' {
        return None;
    }
    if start >= 2 && !bytes[start - 2].is_ascii_whitespace() {
        return None;
    }
    Some((start - 1, text[start..cursor].to_string()))
}

/// Filter and rank the candidate table for `query`.
///
/// Ranking keeps the menu stable and predictable: prefix matches first,
/// then substring matches, then alphabetical. Returns an empty vec when
/// nothing matches — the caller treats that as "dismiss quietly".
fn ranked_variables(query: &str) -> Vec<&'static str> {
    let query = query.to_ascii_lowercase();
    let mut prefixed: Vec<&'static str> = Vec::new();
    let mut contained: Vec<&'static str> = Vec::new();
    for (name, _) in VARIABLES {
        let lower = name.to_ascii_lowercase();
        if lower.starts_with(&query) {
            prefixed.push(name);
        } else if lower.contains(&query) {
            contained.push(name);
        }
    }
    prefixed.sort_unstable();
    contained.sort_unstable();
    prefixed.extend(contained);
    prefixed
}

impl Example {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder("echo $HO... — type $ to complete a variable")
        });

        // The subscription is the whole trigger channel: every edit
        // re-runs trigger detection, which owns both "menu opens" and
        // "menu dismisses" (caret left the token, whitespace closed it,
        // nothing matches).
        let _subscription = cx.subscribe_in(&input_state, window, {
            move |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::Change => this.sync_menu(window, cx),
                // The app owns Enter: with the menu open it accepts;
                // without it, Enter is this example's "submit".
                InputEvent::PressEnter {
                    secondary: false, ..
                } => {
                    if this.menu.is_some() {
                        this.accept(window, cx);
                    } else {
                        this.submit(cx);
                    }
                }
                InputEvent::Blur => this.menu = None,
                _ => {}
            }
        });

        Self {
            input_state,
            menu: None,
            status: "Type $ to complete a variable; Enter submits.".into(),
            _subscription,
        }
    }

    /// Re-scan the trigger and open, update or dismiss the menu.
    fn sync_menu(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let input = self.input_state.read(cx);
        let Some((token_start, query)) = active_variable_query(&input.value(), input.cursor())
        else {
            self.menu = None;
            cx.notify();
            return;
        };

        let items = ranked_variables(&query);
        if items.is_empty() {
            // Quiet dismissal: no "no results" ghost row.
            self.menu = None;
        } else {
            // Keep the row selection stable while the filter narrows.
            let selected = self
                .menu
                .as_ref()
                .map_or(0, |menu| menu.selected.min(items.len() - 1));
            self.menu = Some(Menu {
                token_start,
                items,
                selected,
            });
        }
        cx.notify();
    }

    /// Accept the selected candidate: replace the typed `$prefix` with
    /// `$NAME ` in one atomic undo step.
    fn accept(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.clone() else {
            return;
        };
        let Some(name) = menu.selected_name() else {
            return;
        };
        let inserted = format!("${name} ");

        // Re-read the latest text and cursor: the buffer may have changed
        // since the menu opened, so the token range is revalidated against
        // what is on screen right now, not what opened the menu.
        let (text, cursor) = {
            let input = self.input_state.read(cx);
            (input.value().to_string(), input.cursor())
        };
        let range: Range<usize> = match active_variable_query(&text, cursor) {
            Some((token_start, _)) if token_start == menu.token_start => token_start..cursor,
            // Stale token: dismiss instead of guessing where to edit.
            _ => {
                self.menu = None;
                cx.notify();
                return;
            }
        };

        self.input_state.update(cx, |input, cx| {
            input.set_selected_range(range, cx);
            input.replace(inserted, window, cx);
            input.focus_handle(cx).focus(window, cx);
        });
        self.status = format!("Completed: ${name}").into();
        self.menu = None;
        cx.notify();
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        self.status = format!("Submitted: {}", self.input_state.read(cx).value()).into();
        cx.notify();
    }

    fn render_menu(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let Some(menu) = self.menu.clone() else {
            return div().id("completion-menu");
        };
        let theme = cx.theme();

        v_flex()
            .id("completion-menu")
            .w(MENU_WIDTH)
            .max_h(px(260.))
            .overflow_y_scroll()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .shadow_lg()
            .p_1()
            .gap_y_0p5()
            .children(menu.items.iter().enumerate().map(|(ix, name)| {
                let selected = ix == menu.selected;
                h_flex()
                    .id(ix)
                    .px_2()
                    .py_1()
                    .gap_2()
                    .rounded_sm()
                    .when(selected, |this| this.bg(theme.selection))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if let Some(menu) = this.menu.as_mut() {
                            menu.selected = ix;
                        }
                        this.accept(window, cx);
                    }))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_size(px(13.))
                            .text_color(theme.foreground)
                            .child(*name),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(
                                VARIABLES
                                    .iter()
                                    .find(|(var, _)| var == name)
                                    .map(|(_, desc)| *desc)
                                    .unwrap_or_default(),
                            ),
                    )
            }))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The wrapper owns the keyboard while the menu is open: actions are
        // captured before the input handles them, and every handler is
        // gated on `menu.is_some()` so the input behaves normally when the
        // menu is closed. Tab accepts (`IndentInline` is what the input
        // context binds tab to).
        let input_frame = v_flex()
            .w_full()
            .child(Input::new(&self.input_state).small())
            .capture_action(cx.listener(|this, _: &MoveUp, _, cx| {
                if let Some(menu) = this.menu.as_mut() {
                    menu.selected = menu.selected.saturating_sub(1);
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &MoveDown, _, cx| {
                if let Some(menu) = this.menu.as_mut() {
                    menu.selected = (menu.selected + 1).min(menu.items.len() - 1);
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &Escape, _, cx| {
                if this.menu.take().is_some() {
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &IndentInline, window, cx| {
                if this.menu.is_some() {
                    this.accept(window, cx);
                    cx.stop_propagation();
                }
            }));

        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .p_5()
            .gap_2()
            // The menu mounts as a deferred overlay above the input region.
            // Applications that need caret anchoring resolve it with
            // `cursor_layout()` / `range_to_bounds()` instead.
            .child(
                div()
                    .relative()
                    .w(px(360.))
                    .when(self.menu.is_some(), |this| {
                        this.child(deferred(
                            h_flex()
                                .absolute()
                                .left_0()
                                .right_0()
                                .bottom(relative(1.))
                                .pb_1()
                                .items_end()
                                .child(div().min_w_0().child(self.render_menu(cx))),
                        ))
                    })
                    .child(input_frame),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(cx.theme().muted_foreground)
                    .child(self.status.clone()),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(800.), px(600.)), cx)),
            ..Default::default()
        };

        gpui_kit::open_window(window_options, cx, |window, cx| {
            cx.new(|cx| Example::new(window, cx))
        })
        .expect("Failed to open window");
    });
}
