//! `RadioGroup` and `ToggleGroup` — the two grouping containers.
//!
//! Neither knows what is selected. A `RadioGroup` does not pick the checked
//! radio and a `ToggleGroup` does not pick the pressed toggle; the state lives
//! on each child, told in through `checked(...)` / `selected(...)` and reported
//! back through its own handler. The container contributes the grouping
//! semantics and nothing else, which is why a script keeps the chosen value in
//! its own state exactly as it does for a lone `Checkbox`.
//!
//! `axis` is semantic only. It sets what a screen reader announces about the
//! group's orientation and does *not* lay the children out: a group is still a
//! plain block until the script says `.flex().flex_row()` or `.flex_col()`. The
//! two are independent on purpose — a wrapped toolbar is announced horizontal
//! while laying out as a wrapping row — but a script that sets one and expects
//! the other gets a group that reads correctly and looks wrong.
//!
//! The only other difference between the two is the default each carries when
//! the script says nothing: a radio group is announced vertical, a toggle group
//! horizontal. That is base's choice, so `axis` is applied only when the script
//! actually asked for one.

use gpui::{AnyElement, SharedString, StyleRefinement};
use gpui_base::{RadioGroup, ToggleGroup};

use crate::materialize::{
    Behavior, Children, StateStyles, finish, warn_ignored_key, warn_unhonoured_a11y,
    with_active_and_focus, with_hover,
};

/// A set of radios. Its identity comes from `new(id)`, so `id()` is ignored.
pub(in crate::materialize) fn radio_group(
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "RadioGroup");
    warn_unhonoured_a11y(&behavior, "RadioGroup", &[]);
    let mut group = RadioGroup::new(SharedString::from(id));
    if let Some(axis) = behavior.axis {
        group = group.axis(axis);
    }

    let group = with_hover(group, &states);
    let group = with_active_and_focus(group, &states);
    finish(group, refinement, children)
}

/// A set of toggles, announced as a toolbar. Identified by `new(id)`.
pub(in crate::materialize) fn toggle_group(
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "ToggleGroup");
    warn_unhonoured_a11y(&behavior, "ToggleGroup", &[]);
    let mut group = ToggleGroup::new(SharedString::from(id));
    if let Some(axis) = behavior.axis {
        group = group.axis(axis);
    }

    let group = with_hover(group, &states);
    let group = with_active_and_focus(group, &states);
    finish(group, refinement, children)
}
