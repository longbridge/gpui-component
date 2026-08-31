//! `Radio` — one controlled option in a group.
//!
//! Like `Checkbox`, it holds nothing: the script says which option is checked
//! and re-reads its own state every render. Unlike `Checkbox`, the two
//! directions are not symmetric. Base drops `on_click` entirely once the radio
//! is checked or disabled, because a radio cannot deselect itself, so the
//! callback only ever fires for a *newly* chosen option and only ever carries
//! `true`. A script that expects a `false` to clear the selection would wait
//! forever; clearing is the group's business, and the group is the script's.
//!
//! `set_position` carries "option 2 of 5" to a screen reader. It draws nothing,
//! so a group that omits it looks identical and announces nothing about where
//! the reader is in the set.

use std::rc::Rc;

use gpui::{AnyElement, SharedString, StyleRefinement};
use gpui_base::Radio;

use crate::{
    engine::ShellRuntime,
    materialize::{
        Behavior, Children, StateStyles, dispatch_change, finish, tracked_focus, warn_ignored_key,
        warn_unhonoured_a11y, with_active_and_focus, with_hover, with_input_handlers,
    },
};

/// One radio. Controlled: `checked(...)` in, `on_change(...)` out.
pub(in crate::materialize) fn radio(
    runtime: &Rc<ShellRuntime>,
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "Radio");
    // `Radio::render` announces `Role::RadioButton` and mirrors `checked` into
    // `aria_selected`, so those two would be overwritten rather than honoured.
    warn_unhonoured_a11y(
        &behavior,
        "Radio",
        &["track_focus", "tab_index", "tab_stop"],
    );
    let mut radio = Radio::new(SharedString::from(id))
        .checked(behavior.checked)
        .disabled(behavior.disabled);

    if let Some(focus) = tracked_focus(runtime, &behavior, "Radio") {
        radio = radio.track_focus(&focus);
    }
    if let Some(index) = behavior.tab_index {
        radio = radio.tab_index(index);
    }
    if let Some(stop) = behavior.tab_stop {
        radio = radio.tab_stop(stop);
    }

    if let Some(label) = behavior.accessibility_label.clone() {
        radio = radio.accessibility_label(label);
    }

    if let Some((position, size)) = behavior.position_in_set {
        radio = radio.set_position(position, size);
    }

    if let Some(callback) = behavior.on_change {
        let runtime = Rc::downgrade(runtime);
        radio = radio.on_change(move |checked, _, window, cx| {
            dispatch_change(&runtime, callback, checked, window, cx);
        });
    }

    let radio = with_hover(radio, &states);
    let radio = with_active_and_focus(radio, &states);
    let radio = with_input_handlers(radio, &behavior, runtime);
    finish(radio, refinement, children)
}
