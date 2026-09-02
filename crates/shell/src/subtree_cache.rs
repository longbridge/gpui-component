//! A script-marked subtree rendered by an entity of its own, so gpui can cache
//! its layout and paint across the frames that change nothing inside it.
//!
//! See `docs/superpowers/specs/2026-09-02-subtree-cache-design.md`.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
};

use gpui::{
    Context, Entity, EntityId, IntoElement, Render, SharedString, StyleRefinement, Styled as _,
    Window, div,
};

use crate::{
    engine::ShellRuntime, materialize::materialize_subtree_cached, snapshot::RenderSnapshot,
    spec::SpecId,
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
    pub(crate) fn get_or_create(
        &self,
        view: EntityId,
        key: &SharedString,
        create: impl FnOnce() -> Entity<SubtreeCache>,
    ) -> Entity<SubtreeCache> {
        let mut by_view = self.by_view.borrow_mut();
        let caches = by_view.entry(view).or_default();
        if let Some(existing) = caches.get(key) {
            return existing.clone();
        }
        let created = create();
        caches.insert(key.clone(), created.clone());
        created
    }

    pub(crate) fn entities(&self, view: EntityId) -> Vec<Entity<SubtreeCache>> {
        self.by_view
            .borrow()
            .get(&view)
            .map(|caches| caches.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Drops every cache of `view` whose id is not in `keys`.
    pub(crate) fn retain(&self, view: EntityId, keys: &HashSet<SharedString>) {
        let mut by_view = self.by_view.borrow_mut();
        if let Some(caches) = by_view.get_mut(&view) {
            caches.retain(|key, _| keys.contains(key));
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
/// `split_loses_nothing_but_the_size` pins the partition: refining a `Style`
/// with the original must equal refining it with the two halves.
pub(crate) fn split_layout_properties(
    style: &StyleRefinement,
) -> (StyleRefinement, StyleRefinement) {
    let mut inner = style.clone();
    let mut outer = StyleRefinement::default();
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
    use gpui::{Refineable as _, Styled as _, px, relative};

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
