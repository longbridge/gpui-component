use std::{cell::RefCell, ops::RangeInclusive, rc::Rc};

use gpui::{App, Bounds, EntityId, Hitbox, Pixels, Point, WeakEntity, Window};
use gpui_base::{
    SelectionEndpointSnapshot, SelectionRegionFrame, SelectionScopeId, SelectionSnapshot,
    TextSelectionHost, TextSelectionRegion,
};

use super::TextViewState;

#[derive(Clone, Copy, Debug, PartialEq)]
struct CachedBlockEndpoint {
    endpoint: SelectionEndpointSnapshot,
    block_ix: Option<usize>,
}

#[derive(Default)]
struct VirtualBlockSelection {
    anchor: Option<CachedBlockEndpoint>,
    cursor: Option<CachedBlockEndpoint>,
}

impl VirtualBlockSelection {
    fn update(&mut self, snapshot: Option<SelectionSnapshot>, region_id: EntityId) {
        let Some(snapshot) = snapshot else {
            *self = Self::default();
            return;
        };

        Self::update_endpoint(&mut self.anchor, snapshot.anchor, region_id);
        Self::update_endpoint(&mut self.cursor, snapshot.cursor, region_id);
    }

    fn update_endpoint(
        cached: &mut Option<CachedBlockEndpoint>,
        endpoint: SelectionEndpointSnapshot,
        region_id: EntityId,
    ) {
        if cached.is_some_and(|cached| cached.endpoint == endpoint) {
            return;
        }
        let block_ix = (endpoint.region_id == Some(region_id))
            .then(|| endpoint.virtual_key().map(|key| key as usize))
            .flatten();
        *cached = Some(CachedBlockEndpoint { endpoint, block_ix });
    }

    fn block_range(&self, region_id: EntityId) -> Option<RangeInclusive<usize>> {
        let anchor = self.anchor?;
        let cursor = self.cursor?;
        if anchor.endpoint.region_id != Some(region_id)
            || cursor.endpoint.region_id != Some(region_id)
        {
            return None;
        }

        let (anchor, cursor) = (anchor.block_ix?, cursor.block_ix?);
        Some(anchor.min(cursor)..=anchor.max(cursor))
    }
}

/// TextView's renderer-specific bridge to the base-owned selection host.
#[derive(Clone)]
pub(super) struct TextViewSelectionAdapter {
    region: TextSelectionRegion,
    text_bounds: Vec<Bounds<Pixels>>,
    layout_revision: Option<usize>,
}

impl TextViewSelectionAdapter {
    pub(super) fn new(view: WeakEntity<TextViewState>, cx: &mut App) -> Self {
        let region = TextSelectionRegion::new("", cx);
        let region_id = region.state().entity_id();
        let virtual_blocks = Rc::new(RefCell::new(VirtualBlockSelection::default()));

        region.state().update(cx, |region_state, _| {
            let view_for_selection = view.clone();
            let blocks_for_selection = virtual_blocks.clone();
            region_state.on_selection(move |snapshot, cx| {
                let _ = view_for_selection.update(cx, |state, cx| {
                    blocks_for_selection
                        .borrow_mut()
                        .update(snapshot, region_id);
                    state.is_selecting = snapshot.is_some_and(|snapshot| snapshot.is_selecting);
                    cx.notify();
                });
            });

            let view_for_scroll = view.clone();
            region_state.on_auto_scroll(move |delta, cx| {
                let _ = view_for_scroll.update(cx, |state, cx| {
                    if state.scrollable {
                        state.set_auto_scroll(delta, cx);
                    } else if delta.is_none() {
                        state.stop_auto_scroll();
                    }
                });
            });

            let view_for_clear = view.clone();
            let blocks_for_clear = virtual_blocks.clone();
            region_state.on_clear(move |cx| {
                blocks_for_clear.replace(VirtualBlockSelection::default());
                let _ = view_for_clear.update(cx, |state, cx| {
                    state.reset_selection();
                    cx.notify();
                });
            });

            let view_for_copy = view.clone();
            let blocks_for_copy = virtual_blocks.clone();
            region_state.copy_with(move |cx| {
                let Some(view) = view_for_copy.upgrade() else {
                    return String::new();
                };
                let state = view.read(cx);
                let blocks = blocks_for_copy.borrow().block_range(region_id);
                state.selected_text_in(blocks)
            });

            let view_for_virtual_key = view.clone();
            region_state.on_virtual_key(move |point, cx| {
                let view = view_for_virtual_key.upgrade()?;
                view.read(cx).block_ix_at(point.y).map(|block| block as u64)
            });

            region_state.on_focus(move |window, cx| {
                let Some(view) = view.upgrade() else {
                    return;
                };
                let focus_handle = view.read(cx).focus_handle.clone();
                view.update(cx, |state, _| state.is_selecting = true);
                focus_handle.focus(window, cx);
            });
        });

        Self {
            region,
            text_bounds: Vec::new(),
            layout_revision: None,
        }
    }

    pub(super) fn update_layout_revision(&mut self, revision: usize) -> bool {
        let changed = self
            .layout_revision
            .is_some_and(|previous| previous != revision);
        self.layout_revision = Some(revision);
        changed
    }

    pub(super) fn begin_frame(&mut self) {
        self.text_bounds.clear();
    }

    pub(super) fn register_inline(&mut self, bounds: Vec<Bounds<Pixels>>) {
        self.text_bounds.extend(bounds);
    }

    pub(super) fn register_frame(
        &self,
        hitbox: Hitbox,
        bounds: Bounds<Pixels>,
        scroll_offset: Point<Pixels>,
        scope: SelectionScopeId,
        document_order: u64,
        window: &mut Window,
        cx: &mut App,
    ) {
        TextSelectionHost::install(window, cx).update(cx, |host, cx| {
            host.register_region(
                self.region.clone(),
                SelectionRegionFrame {
                    hitbox,
                    bounds,
                    scroll_offset,
                    scope,
                    document_order,
                    text_bounds: self.text_bounds.clone(),
                },
                cx,
            );
        });
    }

    pub(super) fn selection_points(&self, cx: &App) -> Option<(Point<Pixels>, Point<Pixels>)> {
        self.region.state().read(cx).snapshot()?.resolved_points()
    }

    pub(super) fn set_local_selection(&self, active: bool, cx: &mut App) {
        self.region
            .state()
            .update(cx, |state, _| state.set_local_selection(active));
    }

    pub(super) fn selection_involves_region(&self, cx: &App) -> bool {
        let id = self.region.state().entity_id();
        self.region
            .state()
            .read(cx)
            .snapshot()
            .is_some_and(|snapshot| {
                snapshot.anchor.region_id == Some(id) || snapshot.cursor.region_id == Some(id)
            })
    }

    pub(super) fn has_selection_snapshot(&self, cx: &App) -> bool {
        self.region.state().read(cx).snapshot().is_some()
    }

    #[cfg(test)]
    pub(super) fn text_bounds(&self) -> Vec<Bounds<Pixels>> {
        self.text_bounds.clone()
    }
}
