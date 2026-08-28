//! `Tabs` and `Tab` — a tab list, and the tabs in it.
//!
//! Neither holds a selection. `Tabs` supplies the list role and nothing else;
//! each `Tab` is *told* whether it is selected and reports activation back
//! through `on_click`. That is base's own arrangement — the root deliberately
//! has no builder at all — and it is why a script keeps the selected index in
//! its own state and re-reads it every render, exactly as it does for a
//! `Checkbox`.
//!
//! `set_position` is the one method here with no counterpart on an ordinary
//! element: it carries "tab 2 of 5" to a screen reader. It changes nothing on
//! screen, so a tab list that omits it looks identical and announces nothing
//! about where the reader is in the set.

use std::rc::Rc;

use gpui::{AnyElement, SharedString, StyleRefinement};
use gpui_base::{Tab, Tabs};

use crate::{
    engine::ShellRuntime,
    materialize::{
        Behavior, Children, StateStyles, dispatch_click, finish, warn_ignored_key,
        warn_unhonoured_a11y, with_active_and_focus, with_hover, with_input_handlers,
    },
};

/// The tab list. Its identity comes from `new(id)`, so `id()` is ignored.
pub(in crate::materialize) fn tab_list(
    runtime: &Rc<ShellRuntime>,
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "Tabs");
    warn_unhonoured_a11y(&behavior, "Tabs", &[]);
    let tabs = Tabs::new(SharedString::from(id));
    let tabs = with_hover(tabs, &states);
    let tabs = with_active_and_focus(tabs, &states);
    let tabs = with_input_handlers(tabs, &behavior, runtime);
    finish(tabs, refinement, children)
}

/// One tab. Controlled: `selected(...)` in, `on_click(...)` out.
pub(in crate::materialize) fn tab(
    runtime: &Rc<ShellRuntime>,
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "Tab");
    warn_unhonoured_a11y(&behavior, "Tab", &[]);
    let mut tab = Tab::new(SharedString::from(id))
        .selected(behavior.selected)
        .disabled(behavior.disabled);

    if let Some(label) = behavior.accessibility_label.clone() {
        tab = tab.accessibility_label(label);
    }

    if let Some((position, size)) = behavior.position_in_set {
        tab = tab.set_position(position, size);
    }

    if let Some(callback) = behavior.on_click {
        let runtime = Rc::downgrade(runtime);
        tab = tab.on_click(move |event, window, cx| {
            dispatch_click(&runtime, callback, event, window, cx);
        });
    }

    let tab = with_hover(tab, &states);
    let tab = with_active_and_focus(tab, &states);
    let tab = with_input_handlers(tab, &behavior, runtime);
    finish(tab, refinement, children)
}
