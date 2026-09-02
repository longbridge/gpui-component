//! A script-marked subtree rendered by an entity of its own, so gpui can cache
//! its layout and paint across the frames that change nothing inside it.
//!
//! See `docs/superpowers/specs/2026-09-02-subtree-cache-design.md`.

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use gpui::{
    Context, Entity, EntityId, IntoElement, Render, SharedString, StyleRefinement, Styled as _,
    Window, div,
};

use crate::{
    engine::ShellRuntime,
    materialize::materialize_subtree_cached,
    snapshot::RenderSnapshot,
    spec::{CachedNodes, SpecId},
};

/// The entity behind one `.cached()` element.
///
/// It draws from a clone of the owning view's snapshot, which keeps that
/// snapshot's callback generation alive for as long as elements built from it
/// can still be clicked. The view hands it a fresh snapshot and root on every
/// rebuild through [`SubtreeCache::describe`].
pub(crate) struct SubtreeCache {
    runtime: Weak<ShellRuntime>,
    snapshot: RenderSnapshot,
    root: SpecId,
}

impl SubtreeCache {
    pub(crate) fn new(runtime: &Rc<ShellRuntime>, snapshot: RenderSnapshot, root: SpecId) -> Self {
        Self {
            runtime: Rc::downgrade(runtime),
            snapshot,
            root,
        }
    }

    /// Points the subtree at the node it should draw from now on. Does not
    /// notify: the view that calls this has already dirtied the entity, or is
    /// about to, on the path that fits the frame it is in.
    pub(crate) fn describe(&mut self, snapshot: RenderSnapshot, root: SpecId) {
        self.snapshot = snapshot;
        self.root = root;
    }

    #[cfg(test)]
    pub(crate) fn root(&self) -> SpecId {
        self.root
    }
}

impl Render for SubtreeCache {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(runtime) = self.runtime.upgrade() else {
            return div().into_any_element();
        };
        runtime.metrics().record_subtree_rebuild();
        materialize_subtree_cached(&runtime, &self.snapshot, self.root, window, cx)
    }
}

/// Every cached subtree of every view, keyed by the view's entity and the
/// script id of the element. Owned by the runtime so materialization — which
/// runs inside the view's own render and cannot borrow the view — can reach
/// it with the `&Rc<ShellRuntime>` it already has.
#[derive(Default)]
pub(crate) struct SubtreeCaches {
    by_view: RefCell<HashMap<EntityId, HashMap<SharedString, Entity<SubtreeCache>>>>,
}

impl SubtreeCaches {
    /// The entity for one `(view, script id)`, made on first sight.
    ///
    /// `create` runs with no borrow held: it allocates a gpui entity, and this
    /// map must not be locked while another borrow of it could be reached.
    /// That leaves a window in which a second call could have inserted the
    /// same key, so the insert prefers whatever is already there — one id is
    /// one entity, and the loser is dropped rather than published.
    pub(crate) fn get_or_create(
        &self,
        view: EntityId,
        key: &SharedString,
        create: impl FnOnce() -> Entity<SubtreeCache>,
    ) -> Entity<SubtreeCache> {
        if let Some(existing) = self
            .by_view
            .borrow()
            .get(&view)
            .and_then(|caches| caches.get(key))
        {
            return existing.clone();
        }
        let created = create();
        self.by_view
            .borrow_mut()
            .entry(view)
            .or_default()
            .entry(key.clone())
            .or_insert(created)
            .clone()
    }

    pub(crate) fn entities(&self, view: EntityId) -> Vec<Entity<SubtreeCache>> {
        self.by_view
            .borrow()
            .get(&view)
            .map(|caches| caches.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Drops every cache of `view` whose id the new description no longer
    /// marks.
    pub(crate) fn retain(&self, view: EntityId, cached: &CachedNodes) {
        let mut by_view = self.by_view.borrow_mut();
        if let Some(caches) = by_view.get_mut(&view) {
            caches.retain(|key, _| cached.holds(key));
            if caches.is_empty() {
                by_view.remove(&view);
            }
        }
    }

    pub(crate) fn remove_view(&self, view: EntityId) {
        self.by_view.borrow_mut().remove(&view);
    }
}

/// Splits a `.cached()` element's style into the part Taffy resolves against
/// the parent — the box the element occupies in the parent's flow — and the
/// part it resolves inside that box. The outer half goes on a plain `div` in
/// the parent's tree; the inner half, with `size_full()` in place of the
/// size, goes on the element the [`SubtreeCache`] renders.
///
/// Two tests pin the partition. `split_loses_nothing_but_the_size` proves
/// nothing is dropped: refining a `Style` with the original must equal
/// refining it with the two halves. `split_layout_properties_places_every_style_field`
/// names every field of `StyleRefinement` and says which half it lands on, so
/// a field added to gpui stops the build until it has been placed.
pub(crate) fn split_layout_properties(
    style: &StyleRefinement,
) -> (StyleRefinement, StyleRefinement) {
    let mut inner = style.clone();
    let mut outer = StyleRefinement::default();
    // `display: none` is the only display value the *parent* resolves: it says
    // the node takes no part in the parent's flow. Left inside, it would
    // collapse the contents while the outer `div` went on occupying its box.
    // Every other value describes how this box lays its own children out.
    if inner.display == Some(gpui::Display::None) {
        outer.display = inner.display.take();
    }
    outer.position = inner.position.take();
    outer.visibility = inner.visibility.take();
    outer.inset = std::mem::take(&mut inner.inset);
    outer.size = std::mem::take(&mut inner.size);
    outer.min_size = std::mem::take(&mut inner.min_size);
    outer.max_size = std::mem::take(&mut inner.max_size);
    outer.aspect_ratio = inner.aspect_ratio.take();
    outer.margin = std::mem::take(&mut inner.margin);
    outer.align_self = inner.align_self.take();
    outer.flex_basis = inner.flex_basis.take();
    outer.flex_grow = inner.flex_grow.take();
    outer.flex_shrink = inner.flex_shrink.take();
    outer.grid_location = inner.grid_location.take();
    let inner = inner.size_full();
    (outer, inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Display, Refineable as _, px, relative};

    fn sample() -> StyleRefinement {
        StyleRefinement::default()
            .flex_1()
            .min_h(px(0.))
            .w(px(200.))
            .m_2()
            .p_4()
            .gap_2()
            .bg(gpui::red())
            .rounded_md()
            .flex()
            .flex_col()
            .items_center()
            .opacity(0.5)
            .absolute()
            .top_0()
            .left_0()
    }

    #[test]
    fn split_keeps_the_box_outside_and_everything_else_inside() {
        let style = sample();
        let (outer, inner) = split_layout_properties(&style);

        assert_eq!(
            outer.flex_grow,
            Some(1.),
            "flex_grow belongs to the outer box"
        );
        assert_eq!(outer.min_size.height, style.min_size.height);
        assert_eq!(outer.margin.top, style.margin.top);
        assert_eq!(outer.position, style.position);
        assert_eq!(outer.inset.top, style.inset.top);
        assert!(outer.padding.top.is_none(), "padding stays inside");
        assert!(outer.background.is_none(), "background stays inside");
        assert!(
            outer.flex_direction.is_none(),
            "flex_direction stays inside"
        );

        assert!(inner.flex_grow.is_none());
        assert!(inner.margin.top.is_none());
        assert!(inner.position.is_none());
        assert_eq!(inner.padding.top, style.padding.top);
        assert_eq!(inner.background, style.background);
        assert_eq!(inner.opacity, style.opacity);
        assert_eq!(
            inner.size.width,
            Some(relative(1.).into()),
            "the inner element fills the box the outer one was given"
        );
        assert_eq!(inner.size.height, Some(relative(1.).into()));
    }

    /// A `StyleRefinement` with every field set to something other than its
    /// default, so the table below can tell "landed on this half" apart from
    /// "was never set at all".
    fn every_field() -> StyleRefinement {
        let mut style = StyleRefinement::default();
        style.display = Some(Display::Flex);
        style.visibility = Some(gpui::Visibility::Hidden);
        style.overflow.x = Some(gpui::Overflow::Scroll);
        style.overflow.y = Some(gpui::Overflow::Scroll);
        style.scrollbar_width = Some(px(6.).into());
        style.allow_concurrent_scroll = Some(true);
        style.restrict_scroll_to_axis = Some(true);
        style.position = Some(gpui::Position::Absolute);
        style.inset.top = Some(px(1.).into());
        style.size.width = Some(px(200.).into());
        style.size.height = Some(px(100.).into());
        style.min_size.height = Some(px(0.).into());
        style.max_size.width = Some(px(400.).into());
        style.aspect_ratio = Some(1.5);
        style.margin.left = Some(px(2.).into());
        style.padding.top = Some(px(4.).into());
        style.border_widths.bottom = Some(px(1.).into());
        style.align_items = Some(gpui::AlignItems::Center);
        style.align_self = Some(gpui::AlignSelf::End);
        style.align_content = Some(gpui::AlignContent::SpaceBetween);
        style.justify_content = Some(gpui::JustifyContent::Center);
        style.gap.width = Some(px(8.).into());
        style.flex_direction = Some(gpui::FlexDirection::Column);
        style.flex_wrap = Some(gpui::FlexWrap::Wrap);
        style.flex_basis = Some(px(10.).into());
        style.flex_grow = Some(1.);
        style.flex_shrink = Some(0.);
        style.background = Some(gpui::red().into());
        style.border_color = Some(gpui::blue());
        style.border_style = Some(gpui::BorderStyle::Dashed);
        style.corner_radii.top_left = Some(px(4.).into());
        style.box_shadow = Some(vec![gpui::BoxShadow::new(px(1.), px(1.), gpui::black())]);
        style.text.color = Some(gpui::green());
        style.mouse_cursor = Some(gpui::CursorStyle::PointingHand);
        style.opacity = Some(0.5);
        style.grid_cols = Some(gpui::GridTemplate {
            repeat: 2,
            ..Default::default()
        });
        style.grid_rows = Some(gpui::GridTemplate {
            repeat: 3,
            ..Default::default()
        });
        style.grid_location = Some(gpui::GridLocation {
            row: gpui::GridPlacement::Line(1)..gpui::GridPlacement::Span(2),
            column: gpui::GridPlacement::Line(1)..gpui::GridPlacement::Span(2),
        });
        #[cfg(debug_assertions)]
        {
            style.debug = Some(true);
            style.debug_below = Some(true);
        }
        style
    }

    /// Whether a refinement field carries anything, for the one comparison the
    /// table below makes of every field regardless of its type.
    fn is_set<T: Default + PartialEq>(value: &T) -> bool {
        *value != T::default()
    }

    /// The table the design asks for: every field of `StyleRefinement` named,
    /// and for each of them which half of a `.cached()` element's split it
    /// belongs to.
    ///
    /// The destructuring is the half of this test that a new gpui release
    /// runs into: `StyleRefinement` is taken apart without `..`, so a field
    /// added upstream stops this file compiling until somebody names it here,
    /// sets it in `every_field`, and decides which side of the boundary it is
    /// resolved on.
    #[test]
    fn split_layout_properties_places_every_style_field() {
        let style = every_field();
        let (outer, inner) = split_layout_properties(&style);

        macro_rules! placed {
            ($field:ident, outer) => {{
                assert!(
                    is_set(&$field),
                    concat!("every_field() must set `", stringify!($field), "`")
                );
                assert!(
                    is_set(&outer.$field),
                    concat!("`", stringify!($field), "` is resolved against the parent")
                );
                assert!(
                    !is_set(&inner.$field),
                    concat!("`", stringify!($field), "` must not stay on the inner half")
                );
            }};
            ($field:ident, inner) => {{
                assert!(
                    is_set(&$field),
                    concat!("every_field() must set `", stringify!($field), "`")
                );
                assert!(
                    is_set(&inner.$field),
                    concat!("`", stringify!($field), "` is resolved inside the box")
                );
                assert!(
                    !is_set(&outer.$field),
                    concat!("`", stringify!($field), "` must not reach the outer half")
                );
            }};
        }

        let StyleRefinement {
            display,
            visibility,
            overflow,
            scrollbar_width,
            allow_concurrent_scroll,
            restrict_scroll_to_axis,
            position,
            inset,
            size,
            min_size,
            max_size,
            aspect_ratio,
            margin,
            padding,
            border_widths,
            align_items,
            align_self,
            align_content,
            justify_content,
            gap,
            flex_direction,
            flex_wrap,
            flex_basis,
            flex_grow,
            flex_shrink,
            background,
            border_color,
            border_style,
            corner_radii,
            box_shadow,
            text,
            mouse_cursor,
            opacity,
            grid_cols,
            grid_rows,
            grid_location,
            #[cfg(debug_assertions)]
            debug,
            #[cfg(debug_assertions)]
            debug_below,
        } = style.clone();

        // The box the element occupies in its parent's flow.
        placed!(visibility, outer);
        placed!(position, outer);
        placed!(inset, outer);
        placed!(min_size, outer);
        placed!(max_size, outer);
        placed!(aspect_ratio, outer);
        placed!(margin, outer);
        placed!(align_self, outer);
        placed!(flex_basis, outer);
        placed!(flex_grow, outer);
        placed!(flex_shrink, outer);
        placed!(grid_location, outer);

        // Everything Taffy resolves inside that box, plus everything that only
        // paints.
        placed!(overflow, inner);
        placed!(scrollbar_width, inner);
        placed!(allow_concurrent_scroll, inner);
        placed!(restrict_scroll_to_axis, inner);
        placed!(padding, inner);
        placed!(border_widths, inner);
        placed!(align_items, inner);
        placed!(align_content, inner);
        placed!(justify_content, inner);
        placed!(gap, inner);
        placed!(flex_direction, inner);
        placed!(flex_wrap, inner);
        placed!(background, inner);
        placed!(border_color, inner);
        placed!(border_style, inner);
        placed!(corner_radii, inner);
        placed!(box_shadow, inner);
        placed!(text, inner);
        placed!(mouse_cursor, inner);
        placed!(opacity, inner);
        placed!(grid_cols, inner);
        placed!(grid_rows, inner);
        #[cfg(debug_assertions)]
        placed!(debug, inner);
        #[cfg(debug_assertions)]
        placed!(debug_below, inner);

        // Two fields the halves do not simply divide.
        //
        // `size` is the one property both halves carry: the outer element
        // takes the size the script asked for, and the inner one is told to
        // fill it, because a cached view is a layout leaf and would otherwise
        // draw at nothing.
        assert!(is_set(&size), "every_field() must set `size`");
        assert_eq!(
            outer.size, style.size,
            "the box keeps the size it was given"
        );
        assert_eq!(
            inner.size.width,
            Some(relative(1.).into()),
            "the inner element fills that box rather than re-deciding it"
        );
        assert_eq!(inner.size.height, Some(relative(1.).into()));

        // `display` is split by value: `none` is the parent's answer to
        // "does this node take part in my flow", every other value is how
        // this box lays its own children out. See
        // `split_moves_display_none_outside_and_leaves_the_others_in`.
        assert!(is_set(&display), "every_field() must set `display`");
        assert_eq!(inner.display, Some(Display::Flex));
        assert_eq!(outer.display, None);
    }

    #[test]
    fn split_moves_display_none_outside_and_leaves_the_others_in() {
        // `display: none` is the one display value the parent resolves: it
        // says the node takes no part in the parent's flow. Left on the inner
        // half it would collapse the contents while the outer `div` kept its
        // box, so `.cached().hidden()` would reserve space for nothing.
        let (outer, inner) = split_layout_properties(&StyleRefinement::default().hidden());
        assert_eq!(outer.display, Some(Display::None));
        assert_eq!(inner.display, None);

        // Every other display value describes how the box lays its own
        // children out, which is the inner half's business.
        for display in [Display::Block, Display::Flex, Display::Grid] {
            let mut style = StyleRefinement::default();
            style.display = Some(display);
            let (outer, inner) = split_layout_properties(&style);
            assert_eq!(outer.display, None, "{display:?} is resolved inside");
            assert_eq!(inner.display, Some(display));
        }
    }

    #[test]
    fn split_loses_nothing_but_the_size() {
        let style = sample();
        let (outer, mut inner) = split_layout_properties(&style);
        // Put the size back so the two halves describe exactly the original.
        inner.size = style.size.clone();

        let mut whole = gpui::Style::default();
        whole.refine(&style);
        let mut halves = gpui::Style::default();
        halves.refine(&outer);
        halves.refine(&inner);

        assert_eq!(format!("{whole:?}"), format!("{halves:?}"));
    }
}
