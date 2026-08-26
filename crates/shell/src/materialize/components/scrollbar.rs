//! `Scrollbar` — a bar the script places itself, driving a scroll area
//! somewhere else in the tree.
//!
//! `overflow_y_scrollbar()` already paints one, but only over the element it was
//! called on and only along that element's own edges. A bar beside a fixed table
//! header, a bar spanning two panes, or the bar a virtual list needs — base's
//! `VirtualList` deliberately paints none — has to be an element in its own
//! right, positioned by the script like any other.
//!
//! That leaves the question of how a bar over here reaches a scroll area over
//! there. The two are matched **by name**: a scroll area written
//! `v_flex().id("watchlist").overflow_y_scroll()` and a
//! `Scrollbar.vertical("watchlist")` share one [`ScrollHandle`], kept in window
//! element state under `ElementId::Name("watchlist")`. Both sides look it up
//! from inside `materialize`, and that is what makes the name alone enough:
//! `use_keyed_state` keys by the *path* of element ids it is called under, and
//! every node of one description is materialized at the same point in that
//! path, so two lookups of the same string land in the same slot. Calling it
//! from anywhere else — from an element's own `render`, the way
//! [`crate::scroll::Scrollable`] does — puts the ancestors' ids in front of the
//! name and reaches a different slot.
//!
//! The pairing is therefore a contract nothing in the call expresses: nothing in
//! `Scrollbar.vertical("watchlist")` says that some other element has to carry
//! that id. So a bar whose scroll area never turns up says so, rather than
//! sitting there hit-testable and refusing to move.

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, Refineable as _, ScrollHandle,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
};
use gpui_base::{Scrollbar, ScrollbarAxis};

use crate::materialize::{Behavior, Children, StateStyles, warn_ignored_key, warn_unhonoured_a11y};

/// The bar. Its identity comes from `new(id)`, so `id()` is ignored.
pub(in crate::materialize) fn scrollbar(
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    warn_ignored_key(&behavior, "Scrollbar");
    // `Scrollbar` is a painted element, not a control: it has no focus handle,
    // no role and no `Interactivity` for one to be set on.
    warn_unhonoured_a11y(&behavior, "Scrollbar", &[]);
    // Its colors come from the theme's scrollbar tokens and the element around
    // it is a positioning box, so a state style here has nowhere to land.
    // Saying so beats dropping it without a word.
    if states.hover.is_some() || states.active.is_some() || states.focus.is_some() {
        tracing::warn!(
            "state styles on a Scrollbar are ignored; the bar's own colors come from the \
             theme's scrollbar tokens, and the element around it only positions it"
        );
    }

    let target = SharedString::from(id);
    let handle = scroll_position(&ElementId::Name(target.clone()), window, cx);
    warn_if_unclaimed(&target, &handle, window, cx);

    // `new(...)` is both axes, and `Scrollbar.horizontal`/`.vertical` narrow it
    // through the same `axis` a radio group uses — one spelling of orientation
    // in the script API rather than two.
    let axis = behavior
        .axis
        .map_or(ScrollbarAxis::Both, ScrollbarAxis::from);
    let mut bar = Scrollbar::new(&handle)
        // Left alone, the id is `Scrollbar::new`'s call site — this line — which
        // every scrollbar in the application would then share, along with its
        // hover, drag and fade state. The axis is part of the id because one
        // scroll area can carry a horizontal and a vertical bar at once.
        .id((ElementId::Name(target), axis_name(axis)))
        .axis(axis);

    if let Some(mode) = behavior.scrollbar_mode {
        bar = bar.mode(mode);
    }
    if let Some(scroll_size) = behavior.scroll_size {
        bar = bar.scroll_size(scroll_size);
    }
    if behavior.viewport_from_layout {
        bar = bar.viewport_from_layout();
    }

    // `Scrollbar` lays itself out absolutely at the full size of its parent and
    // implements no `Styled` of its own, so where the bar goes and how wide its
    // lane is are styles on the box around it. Children go in the same box: it
    // is an ordinary element, and a bar drawn over a corner button should be
    // able to hold the button.
    let mut frame = div();
    frame.style().refine(&refinement);
    frame.extend(children);
    frame.child(bar).into_any_element()
}

/// Gives a scroll area the position an explicit [`Scrollbar`] reads.
///
/// Called from `flex_element` for every `overflow_*_scroll` element, which is
/// what turns `id("watchlist")` into a name a bar elsewhere can address.
pub(in crate::materialize) fn track_scroll_position<E: StatefulInteractiveElement>(
    element: E,
    identity: &ElementId,
    window: &mut Window,
    cx: &mut App,
) -> E {
    element.track_scroll(&scroll_position(identity, window, cx))
}

/// The scroll position shared by a scroll area and the bars driving it.
///
/// Keyed by the element's own identity rather than by anything the description
/// carries: a [`ScrollHandle`] is not an entity, so it cannot live in the
/// runtime's entity store, and window element state is the one place both sides
/// of the pairing can reach with nothing but a name.
fn scroll_position(identity: &ElementId, window: &mut Window, cx: &mut App) -> ScrollHandle {
    shared_scroll_position(identity, window, cx)
        .read(cx)
        .clone()
}

/// The slot itself, for the one caller that has to *write* it.
///
/// A `VirtualList` brings its own retained handle — base's own type, carrying
/// the pending `scroll_to_item` alongside the offset — so the pairing works the
/// other way round there: the list puts its position in the slot rather than
/// taking one out of it. Everything else only ever reads.
pub(in crate::materialize) fn shared_scroll_position(
    identity: &ElementId,
    window: &mut Window,
    cx: &mut App,
) -> gpui::Entity<ScrollHandle> {
    window.use_keyed_state(identity.clone(), cx, |_, _| ScrollHandle::default())
}

/// Distinguishes the bars over one scroll area. They share a scroll position,
/// but a drag on one is not a drag on the other.
fn axis_name(axis: ScrollbarAxis) -> &'static str {
    match axis {
        ScrollbarAxis::Vertical => "vertical",
        ScrollbarAxis::Horizontal => "horizontal",
        ScrollbarAxis::Both => "both",
    }
}

/// Whether a bar has been through a frame, and whether it has already
/// complained about the scroll area it never found.
#[derive(Default)]
struct ScrollTarget {
    /// The first frame proves nothing: no element has been laid out yet, so the
    /// shared position is empty whether or not a scroll area claims this name.
    rendered_once: bool,
    reported: bool,
}

/// Reports a scrollbar whose named scroll area never turned up.
///
/// Nothing enforces the pairing, so the failure mode is a bar that is laid out,
/// painted and hit-tested and yet completely inert — which reads as a broken
/// scrollbar rather than as a misspelt name. The viewport is the evidence:
/// `track_scroll` fills it in during layout, so from the second frame onwards an
/// empty one means nothing is scrolling under this name. A scroll area that is
/// genuinely collapsed to nothing reports here too, and it should: a bar over a
/// zero-sized viewport cannot move either.
///
/// Reported once. A warning repeated every frame is a warning nobody reads.
fn warn_if_unclaimed(
    target: &SharedString,
    handle: &ScrollHandle,
    window: &mut Window,
    cx: &mut App,
) {
    let viewport = handle.bounds().size;
    let unclaimed = viewport.width <= px(0.) || viewport.height <= px(0.);
    let state = window.use_keyed_state(
        (ElementId::Name(target.clone()), "scroll-target"),
        cx,
        |_, _| ScrollTarget::default(),
    );
    state.update(cx, |state, _| {
        if !state.rendered_once {
            state.rendered_once = true;
        } else if unclaimed && !state.reported {
            state.reported = true;
            tracing::warn!(
                "Scrollbar(\"{target}\") drives nothing: no element with id(\"{target}\") is \
                 scrolling, or the one that is has no size. A bar is paired with its scroll \
                 area by name, and the area owns the scrolling — give it \
                 `.id(\"{target}\").overflow_y_scroll()`. `overflow_y_scrollbar()` is the other \
                 arrangement: it paints its own bar and shares nothing"
            );
        }
    });
}
