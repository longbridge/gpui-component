//! `Popup`, and the two anchored surfaces built over it.
//!
//! `Popup` is the machinery: it measures its trigger, works out which corner of
//! the surface to pin where, paints the content in a deferred layer above the
//! rest of the window, and keeps it clear of the window edges. `Popover` and
//! `HoverCard` add an open state and a way of changing it — a press for one,
//! the pointer resting for the other — and nothing else.
//!
//! A script reaches for the bare `Popup` when the open state already belongs to
//! something else: a `Select` owns one, and a popover of its own underneath it
//! would be a second control fighting the first for the same Escape key.
//!
//! `Popover` and `HoverCard` are behavior and nothing else. Base gives neither
//! a `Styled` nor a `ParentElement` impl, because everything either of them
//! puts on screen is the element in its `trigger` slot or the element in its
//! `content` slot, and the script owns both outright. A style call on the root
//! would vanish rather than land somewhere unexpected, so it is reported
//! instead. The `Popup` under them is the opposite: it is a real box, and its
//! bounds are what the surface is anchored to.
//!
//! # Why the slots take elements, when `window.open_dialog` takes a function
//!
//! This is the one place the script API deliberately disagrees with itself, so
//! the reason is written down here before somebody comes along to make it
//! consistent.
//!
//! A dialog is a view. It is opened from an event, it outlives the render that
//! opened it, and the window owns it until it is closed — so the only thing
//! that can describe it is a function, because there is no render pass in which
//! to build it yet.
//!
//! A popover's content is not a view. It belongs to the same description tree
//! as the trigger beside it and is rebuilt by the same `render`, which is
//! exactly what makes the script's `cx.notify()` reach inside it. Handing it a
//! function would turn it into a script view of its own, invalidated
//! separately, and the symptom is specific enough to recognise: pick an item in
//! an open menu, watch a count outside the menu change, and watch the same
//! count inside the menu stay where it was. One render pass, one description
//! tree — the popover's content has to stay in it.
//!
//! # Both slots are materialized whether or not the surface is open
//!
//! [`materialize_node`](crate::materialize) builds every slot before the
//! component sees it, so a closed popover still pays to build its content.
//! `AnyElement` is `'static`, so the built element moves into base's `FnOnce`
//! and is dropped unused when the surface is shut — correct, but not free.
//!
//! Making it lazy means giving the closure a way to materialize a subtree of
//! its own, which needs `RenderSnapshot`'s arena to become an `Rc<SpecArena>`
//! that outlives the call. That is a change to the snapshot contract rather
//! than to this file, it is P5 of the binding plan, and it is deliberately not
//! done here.

use std::rc::Rc;

use gpui::{
    AnyElement, InteractiveElement as _, IntoElement as _, ParentElement as _, Refineable as _,
    SharedString, StyleRefinement, Styled as _, div, prelude::FluentBuilder as _,
};
use gpui_base::{HoverCard, Popover, Popup};

use crate::{
    engine::ShellRuntime,
    materialize::{
        Behavior, Children, Slots, StateStyles, dispatch_change, take_slot, tracked_focus,
        warn_ignored_key, warn_unhonoured_a11y, warn_unread_slots, warn_unsupported,
        warn_without_surface, with_active_and_focus, with_aria, with_gpui_focus, with_hover,
    },
};

/// The bare anchored surface: a trigger, and a content layer above the window.
///
/// It holds no open state of its own and takes none. What is on screen is
/// whatever filled the `content` slot, so a script opens the surface by filling
/// it and closes it by leaving it empty — usually
/// `.when(this.open, el => el.content(...))`. That is not a shortcut around a
/// missing builder: base's `Popup` genuinely has no open state, because the
/// components that use it each hold their own.
pub(in crate::materialize) fn popup(
    runtime: &Rc<ShellRuntime>,
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    mut slots: Slots,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "Popup");
    warn_unsupported(
        "Popup",
        &[
            // Named first because it is the one a script arrives expecting,
            // having met it on a `Popover` a page earlier.
            ("open", behavior.open.is_some()),
            ("default_open", behavior.default_open),
            ("overlay_closable", behavior.overlay_closable.is_some()),
            ("mouse_button", behavior.mouse_button.is_some()),
            ("open_delay", behavior.open_delay.is_some()),
            ("close_delay", behavior.close_delay.is_some()),
            ("on_open_change", behavior.on_open_change.is_some()),
            ("disabled", behavior.disabled),
        ],
    );

    let trigger = take_slot(&mut slots, "trigger");
    let content = take_slot(&mut slots, "content");
    warn_unread_slots(&slots, "Popup");
    if !children.is_empty() {
        tracing::warn!(
            "a Popup renders its trigger and its `content` slot and nothing else, so the {} \
             ordinary children given to it are dropped",
            children.len()
        );
    }

    // Unlike the two surfaces above, a `Popup` *is* a box: the trigger goes
    // inside it, and its bounds are what the content is anchored to. So its
    // styles land, its state styles land, and GPUI's own focus and
    // accessibility builders reach it — it keeps the interactivity it was
    // given rather than rebuilding one in `render`.
    let mut popup = Popup::new(
        SharedString::from(id),
        trigger.unwrap_or_else(|| div().into_any_element()),
    )
    .when_some(behavior.anchor, |popup, anchor| popup.anchor(anchor))
    .when_some(content, |popup, content| popup.content(content));
    popup.style().refine(&refinement);

    let focus = tracked_focus(runtime, &behavior, "Popup");
    let popup = with_gpui_focus(popup, &behavior, focus.as_ref());
    let popup = with_aria(popup, &behavior);
    let popup = with_hover(popup, &states);
    with_active_and_focus(popup, &states).into_any_element()
}

/// A click-driven anchored surface. Controlled through `open` and
/// `on_open_change`, exactly as a `Checkbox` is controlled through `checked`
/// and `on_change`.
pub(in crate::materialize) fn popover(
    runtime: &Rc<ShellRuntime>,
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    mut slots: Slots,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "Popover");
    // `track_focus` is the one it honours: base focuses the tracked handle when
    // the surface opens, which is how the first field of a form inside a
    // popover takes the keyboard.
    warn_unhonoured_a11y(&behavior, "Popover", &["track_focus"]);
    warn_without_surface("Popover", &refinement, &states, &children);
    warn_unsupported(
        "Popover",
        &[
            ("open_delay", behavior.open_delay.is_some()),
            ("close_delay", behavior.close_delay.is_some()),
        ],
    );

    let trigger = take_slot(&mut slots, "trigger");
    let content = take_slot(&mut slots, "content");
    warn_unread_slots(&slots, "Popover");
    if trigger.is_none() {
        tracing::warn!(
            "a Popover with no `trigger` renders nothing: the trigger is the whole of what is \
             on screen while the surface is closed"
        );
    }

    Popover::new(SharedString::from(id))
        .default_open(behavior.default_open)
        .when_some(trigger, |popover, trigger| {
            // `trigger` rather than `trigger_with` needs a `Selectable`, which a
            // materialized `AnyElement` is not. The hidden builder exists for
            // exactly this — base documents it as the entry point for a
            // higher-level presentation facade, which is what the shell is.
            // The open state it offers is dropped: a controlled popover's
            // script already holds that value and can style the trigger from
            // it, and a trigger that changed shape without the script asking
            // would be presentation the base layer does not own.
            popover.trigger_with(move |_, _, _| trigger)
        })
        .when_some(behavior.open, |popover, open| popover.open(open))
        .when_some(behavior.anchor, |popover, anchor| popover.anchor(anchor))
        .when_some(behavior.mouse_button, |popover, button| {
            popover.mouse_button(button)
        })
        .when_some(behavior.overlay_closable, |popover, closable| {
            popover.overlay_closable(closable)
        })
        .when_some(
            tracked_focus(runtime, &behavior, "Popover"),
            |popover, focus| popover.track_focus(&focus),
        )
        .when_some(content, |popover, content| {
            popover.content(move |_, _, _| content)
        })
        .when_some(behavior.on_open_change, |popover, callback| {
            let runtime = Rc::downgrade(runtime);
            popover.on_open_change(move |open, window, cx| {
                dispatch_change(&runtime, callback, *open, window, cx);
            })
        })
        .into_any_element()
}

/// A hover-driven anchored surface. It owns its own open state — there is no
/// `open` to control and no pointer handler to wire, only the two delays.
pub(in crate::materialize) fn hover_card(
    runtime: &Rc<ShellRuntime>,
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    mut slots: Slots,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "HoverCard");
    warn_unhonoured_a11y(&behavior, "HoverCard", &[]);
    warn_without_surface("HoverCard", &refinement, &states, &children);
    warn_unsupported(
        "HoverCard",
        &[
            ("open", behavior.open.is_some()),
            ("default_open", behavior.default_open),
            ("overlay_closable", behavior.overlay_closable.is_some()),
            ("mouse_button", behavior.mouse_button.is_some()),
        ],
    );

    let trigger = take_slot(&mut slots, "trigger");
    let content = take_slot(&mut slots, "content");
    warn_unread_slots(&slots, "HoverCard");
    if trigger.is_none() {
        tracing::warn!(
            "a HoverCard with no `trigger` renders nothing, and there is nothing to hover"
        );
    }

    HoverCard::new(SharedString::from(id))
        .when_some(trigger, |card, trigger| card.trigger(trigger))
        .when_some(behavior.anchor, |card, anchor| card.anchor(anchor))
        .when_some(behavior.open_delay, |card, delay| card.open_delay(delay))
        .when_some(behavior.close_delay, |card, delay| card.close_delay(delay))
        .when_some(content, |card, content| {
            // Base requires a `Stateful<Div>` here because it hangs the
            // "pointer moved onto the card" listener on it, which is what keeps
            // the card open while the pointer is inside it. The script's own
            // element goes inside that wrapper, so the styles it wrote land on
            // the inner element and the hover region is the wrapper around it.
            card.content(move |_, _, _| div().id("content").child(content))
        })
        .when_some(behavior.on_open_change, |card, callback| {
            let runtime = Rc::downgrade(runtime);
            card.on_open_change(move |open, window, cx| {
                // Base announces the change from `HoverCardState::set_open`,
                // which the delay timers reach through `update_in` — so this
                // runs inside a mutable borrow of that entity. Dispatching
                // straight through would run script under that borrow, and a
                // handler that touched the same card (a `cx.notify()` reaching
                // back into it) would panic on the re-entrant update rather
                // than report anything a script author could act on.
                //
                // Deferring puts the call on the next effect flush, after the
                // borrow has been released, which is where every other shell
                // callback already runs.
                let runtime = runtime.clone();
                let open = *open;
                window.defer(cx, move |window, cx| {
                    dispatch_change(&runtime, callback, open, window, cx);
                });
            })
        })
        .into_any_element()
}
