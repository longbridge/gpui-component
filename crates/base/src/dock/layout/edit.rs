use gpui::{Bounds, Pixels};

use crate::Placement;

use super::node::{LayoutNode, NodeId, NodeKind, PanelId, TilePanel};
use super::tree::LayoutTree;

/// Where a panel should land.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InsertTarget {
    /// Into an existing tab group, optionally at a specific index.
    Tabs {
        node: NodeId,
        ix: Option<usize>,
        activate: bool,
    },
    /// Beside an existing node, creating a new tab group for the panel.
    Split {
        node: NodeId,
        placement: Placement,
        size: Option<Pixels>,
    },
    /// Onto a tiles canvas at the given bounds.
    Tile {
        node: NodeId,
        bounds: Bounds<Pixels>,
    },
}

/// What one edit changed.
///
/// The caller uses this to decide what to reconcile and whether to persist.
/// Fields are private so new outcomes can be added without breaking callers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditResult {
    changed: bool,
    created_nodes: Vec<NodeId>,
    removed_nodes: Vec<NodeId>,
    removed_panels: Vec<PanelId>,
    activated: Vec<PanelId>,
    deactivated: Vec<PanelId>,
}

impl EditResult {
    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn created_nodes(&self) -> &[NodeId] {
        &self.created_nodes
    }

    pub fn removed_nodes(&self) -> &[NodeId] {
        &self.removed_nodes
    }

    /// Panels that left the tree entirely. A moved panel is absent here: its
    /// entity survives, so it must not receive `on_removed`.
    pub fn removed_panels(&self) -> &[PanelId] {
        &self.removed_panels
    }

    pub fn activated(&self) -> &[PanelId] {
        &self.activated
    }

    pub fn deactivated(&self) -> &[PanelId] {
        &self.deactivated
    }
}

impl LayoutTree {
    pub fn insert_panel(&mut self, panel: PanelId, target: InsertTarget) -> EditResult {
        self.edit(|tree| tree.apply_insert(panel, target))
    }

    pub fn remove_panel(&mut self, panel: PanelId) -> EditResult {
        self.edit(|tree| {
            if tree.detach_panel(panel) {
                vec![panel]
            } else {
                Vec::new()
            }
        })
    }

    /// Move a panel to a new home without ever removing it from the tree's
    /// perspective, so the caller never fires `on_removed` for a drag.
    pub fn move_panel(&mut self, panel: PanelId, target: InsertTarget) -> EditResult {
        self.edit(|tree| {
            tree.detach_panel(panel);
            tree.apply_insert(panel, target);
            Vec::new()
        })
    }

    pub fn split(
        &mut self,
        at: NodeId,
        panel: PanelId,
        placement: Placement,
        size: Option<Pixels>,
    ) -> EditResult {
        self.insert_panel(
            panel,
            InsertTarget::Split {
                node: at,
                placement,
                size,
            },
        )
    }

    pub fn set_active(&mut self, node: NodeId, ix: usize) -> EditResult {
        self.edit(|tree| {
            if let Some(path) = tree.path_of_node(node) {
                if let NodeKind::Tabs { active_ix, .. } = tree.node_at_mut(&path).kind_mut() {
                    *active_ix = ix;
                }
            }
            Vec::new()
        })
    }

    /// Replace a split's slot sizes wholesale.
    ///
    /// A no-op, like every other operation given input it cannot resolve, if
    /// `new_sizes.len()` does not match the split's child count: no rule in
    /// `normalize` repairs a length mismatch, so applying it would otherwise
    /// leave `children.len() != sizes.len()` and trip `normalize`'s
    /// `debug_assert!`.
    pub fn set_sizes(&mut self, node: NodeId, new_sizes: Vec<Option<Pixels>>) -> EditResult {
        self.edit(|tree| {
            if let Some(path) = tree.path_of_node(node) {
                if let NodeKind::Split {
                    children, sizes, ..
                } = tree.node_at_mut(&path).kind_mut()
                {
                    if new_sizes.len() == children.len() {
                        *sizes = new_sizes;
                    }
                }
            }
            Vec::new()
        })
    }

    pub fn set_tile_bounds(&mut self, panel: PanelId, bounds: Bounds<Pixels>) -> EditResult {
        self.edit(|tree| {
            tree.with_tile(panel, |tile| *tile = tile.with_bounds(bounds));
            Vec::new()
        })
    }

    pub fn bring_to_front(&mut self, panel: PanelId) -> EditResult {
        self.edit(|tree| {
            let top = tree.max_z_index();
            tree.with_tile(panel, |tile| *tile = tile.with_z_index(top + 1));
            Vec::new()
        })
    }
}

impl LayoutTree {
    fn edit(&mut self, apply: impl FnOnce(&mut Self) -> Vec<PanelId>) -> EditResult {
        let before = self.clone();
        let before_active = before.active_panels();
        let before_nodes = before.node_ids();

        let removed_panels = apply(self);
        self.normalize();

        let changed = *self != before;
        if !changed {
            return EditResult::default();
        }

        let after_active = self.active_panels();
        let after_nodes = self.node_ids();

        EditResult {
            changed: true,
            created_nodes: after_nodes
                .iter()
                .filter(|id| !before_nodes.contains(id))
                .copied()
                .collect(),
            removed_nodes: before_nodes
                .iter()
                .filter(|id| !after_nodes.contains(id))
                .copied()
                .collect(),
            removed_panels,
            activated: after_active
                .iter()
                .filter(|panel| !before_active.contains(panel))
                .copied()
                .collect(),
            deactivated: before_active
                .iter()
                .filter(|panel| !after_active.contains(panel) && self.contains_panel(**panel))
                .copied()
                .collect(),
        }
    }

    /// The displayed panel of every tab group, plus every tile panel.
    pub(crate) fn active_panels(&self) -> Vec<PanelId> {
        let mut active = Vec::new();
        self.root().walk(&mut |node| match node.kind_ref() {
            NodeKind::Tabs { panels, active_ix } => active.extend(panels.get(*active_ix).copied()),
            NodeKind::Tiles { panels } => active.extend(panels.iter().map(TilePanel::panel)),
            NodeKind::Split { .. } => {}
        });
        active
    }

    pub(crate) fn contains_panel(&self, panel: PanelId) -> bool {
        self.panels().any(|candidate| candidate == panel)
    }
}

impl LayoutTree {
    fn apply_insert(&mut self, panel: PanelId, target: InsertTarget) -> Vec<PanelId> {
        match target {
            InsertTarget::Tabs { node, ix, activate } => {
                if let Some(path) = self.path_of_node(node) {
                    if let NodeKind::Tabs { panels, active_ix } = self.node_at_mut(&path).kind_mut()
                    {
                        let ix = ix.unwrap_or(panels.len()).min(panels.len());
                        panels.insert(ix, panel);
                        if activate {
                            *active_ix = ix;
                        } else if ix <= *active_ix && panels.len() > 1 {
                            // Keep the displayed panel displayed.
                            *active_ix += 1;
                        }
                    }
                }
            }
            InsertTarget::Split {
                node,
                placement,
                size,
            } => self.insert_beside(node, panel, placement, size),
            InsertTarget::Tile { node, bounds } => {
                if let Some(path) = self.path_of_node(node) {
                    let top = self.max_z_index();
                    if let NodeKind::Tiles { panels } = self.node_at_mut(&path).kind_mut() {
                        panels.push(TilePanel::new(panel, bounds).with_z_index(top + 1));
                    }
                }
            }
        }
        Vec::new()
    }

    /// Place `panel` in a new tab group beside `node`.
    ///
    /// When the parent split already runs along the placement's axis the new
    /// group becomes a sibling. Otherwise `node` is wrapped in a fresh split of
    /// the placement's axis. Rule 3 of `normalize` then flattens any redundant
    /// nesting this creates, which is why no "reuse the parent split" special
    /// case is needed here.
    fn insert_beside(
        &mut self,
        node: NodeId,
        panel: PanelId,
        placement: Placement,
        size: Option<Pixels>,
    ) {
        let Some(path) = self.path_of_node(node) else {
            return;
        };
        let group_id = self.allocate_node_id();
        let group = LayoutNode::new(
            group_id,
            NodeKind::Tabs {
                panels: vec![panel],
                active_ix: 0,
            },
        );
        let before = matches!(placement, Placement::Left | Placement::Top);

        if let Some((parent_path, ix)) = split_parent_of(&path) {
            let parent_axis = match self.node_at(&parent_path).kind_ref() {
                NodeKind::Split { axis, .. } => Some(*axis),
                _ => None,
            };

            if parent_axis == Some(placement.axis()) {
                let NodeKind::Split {
                    children, sizes, ..
                } = self.node_at_mut(&parent_path).kind_mut()
                else {
                    return;
                };
                let at = if before { ix } else { ix + 1 };
                children.insert(at, group);
                sizes.insert(at, size);
                return;
            }
        }

        // Wrap the target in a new split of the placement's axis.
        let wrapper_id = self.allocate_node_id();
        let target = self.node_at(&path).clone();
        let (children, sizes) = if before {
            (vec![group, target], vec![size, None])
        } else {
            (vec![target, group], vec![None, size])
        };
        let wrapper = LayoutNode::new(
            wrapper_id,
            NodeKind::Split {
                axis: placement.axis(),
                children,
                sizes,
            },
        );
        *self.node_at_mut(&path) = wrapper;
    }

    /// Remove `panel` wherever it lives. Returns whether it was found.
    fn detach_panel(&mut self, panel: PanelId) -> bool {
        let Some(node) = self.find_panel_node(panel) else {
            return false;
        };
        let Some(path) = self.path_of_node(node) else {
            return false;
        };

        match self.node_at_mut(&path).kind_mut() {
            NodeKind::Tabs { panels, active_ix } => {
                let Some(ix) = panels.iter().position(|p| *p == panel) else {
                    return false;
                };
                panels.remove(ix);
                if ix < *active_ix {
                    *active_ix -= 1;
                }
                true
            }
            NodeKind::Tiles { panels } => {
                let Some(ix) = panels.iter().position(|p| p.panel() == panel) else {
                    return false;
                };
                panels.remove(ix);
                true
            }
            NodeKind::Split { .. } => false,
        }
    }

    fn with_tile(&mut self, panel: PanelId, f: impl FnOnce(&mut TilePanel)) {
        let Some(node) = self.find_panel_node(panel) else {
            return;
        };
        let Some(path) = self.path_of_node(node) else {
            return;
        };
        if let NodeKind::Tiles { panels } = self.node_at_mut(&path).kind_mut() {
            if let Some(tile) = panels.iter_mut().find(|tile| tile.panel() == panel) {
                f(tile);
            }
        }
    }

    /// Highest z-index across every tiles canvas in the tree.
    fn max_z_index(&self) -> usize {
        let mut top = 0;
        self.root().walk(&mut |node| {
            if let NodeKind::Tiles { panels } = node.kind_ref() {
                top = top.max(panels.iter().map(TilePanel::z_index).max().unwrap_or(0));
            }
        });
        top
    }
}

/// Split the path into its parent path and the child index, or `None` at the root.
fn split_parent_of(path: &super::tree::NodePath) -> Option<(super::tree::NodePath, usize)> {
    let (&ix, parent) = path.split_last()?;
    Some((parent.iter().copied().collect(), ix))
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::Placement;
    use gpui::{Axis, Bounds, Pixels, point, px, size};

    fn panel(n: u64) -> PanelId {
        PanelId::from_u64(n)
    }

    fn tree_with_one_group() -> (LayoutTree, NodeId) {
        let mut tree = LayoutTree::new(RootKind::Split);
        let tabs = tree.push_tabs_for_test(tree.root().id(), vec![panel(1)]);
        tree.normalize();
        (tree, tabs)
    }

    #[test]
    fn inserting_into_a_tab_group_appends_and_can_activate() {
        let (mut tree, tabs) = tree_with_one_group();
        let result = tree.insert_panel(
            panel(2),
            InsertTarget::Tabs {
                node: tabs,
                ix: None,
                activate: true,
            },
        );

        assert!(result.changed());
        let NodeRef::Tabs { panels, active_ix } = tree.find_node(tabs).unwrap().kind() else {
            panic!()
        };
        assert_eq!(panels, [panel(1), panel(2)]);
        assert_eq!(active_ix, 1);
        assert_eq!(result.activated(), &[panel(2)]);
        assert_eq!(result.deactivated(), &[panel(1)]);
    }

    #[test]
    fn a_background_insert_leaves_the_active_panel_alone() {
        let (mut tree, tabs) = tree_with_one_group();
        let result = tree.insert_panel(
            panel(2),
            InsertTarget::Tabs {
                node: tabs,
                ix: None,
                activate: false,
            },
        );

        assert!(result.activated().is_empty());
        assert!(result.deactivated().is_empty());
    }

    #[test]
    fn removing_the_last_panel_collapses_the_group_and_reports_it() {
        let (mut tree, tabs) = tree_with_one_group();
        let result = tree.remove_panel(panel(1));

        assert!(result.changed());
        assert_eq!(result.removed_panels(), &[panel(1)]);
        assert!(result.removed_nodes().contains(&tabs));
        assert!(
            matches!(tree.root().kind(), NodeRef::Split { children, .. } if children.is_empty())
        );
    }

    #[test]
    fn splitting_creates_a_sibling_group_on_the_requested_side() {
        let (mut tree, tabs) = tree_with_one_group();
        let result = tree.split(tabs, panel(2), Placement::Right, Some(px(240.)));

        assert!(result.changed());
        let NodeRef::Split {
            axis,
            children,
            sizes,
        } = tree.root().kind()
        else {
            panic!()
        };
        assert_eq!(axis, Axis::Horizontal);
        assert_eq!(children.len(), 2);
        assert_eq!(sizes[1], Some(px(240.)));

        let NodeRef::Tabs { panels, .. } = children[0].kind() else {
            panic!()
        };
        assert_eq!(panels, [panel(1)]);
        let NodeRef::Tabs { panels, .. } = children[1].kind() else {
            panic!()
        };
        assert_eq!(panels, [panel(2)]);
    }

    #[test]
    fn splitting_left_puts_the_new_group_first() {
        let (mut tree, tabs) = tree_with_one_group();
        tree.split(tabs, panel(2), Placement::Left, None);

        let NodeRef::Split { children, .. } = tree.root().kind() else {
            panic!()
        };
        let NodeRef::Tabs { panels, .. } = children[0].kind() else {
            panic!()
        };
        assert_eq!(panels, [panel(2)]);
    }

    #[test]
    fn splitting_across_the_parent_axis_nests_a_new_split() {
        let (mut tree, tabs) = tree_with_one_group();
        tree.push_tabs_for_test(tree.root().id(), vec![panel(9)]);
        tree.normalize();

        tree.split(tabs, panel(2), Placement::Bottom, None);

        let NodeRef::Split { axis, children, .. } = tree.root().kind() else {
            panic!()
        };
        assert_eq!(axis, Axis::Horizontal);
        let NodeRef::Split {
            axis: inner,
            children: inner_children,
            ..
        } = children[0].kind()
        else {
            panic!("the split target is wrapped in a vertical split")
        };
        assert_eq!(inner, Axis::Vertical);
        assert_eq!(inner_children.len(), 2);

        // `Bottom` puts the new group after the original target: the
        // wrapper's first child is still the target, the second is new.
        let NodeRef::Tabs { panels, .. } = inner_children[0].kind() else {
            panic!()
        };
        assert_eq!(panels, [panel(1)], "the original target stays first");
        let NodeRef::Tabs { panels, .. } = inner_children[1].kind() else {
            panic!()
        };
        assert_eq!(panels, [panel(2)], "the new group lands second, below");
    }

    #[test]
    fn splitting_top_across_the_parent_axis_puts_the_new_group_first() {
        let (mut tree, tabs) = tree_with_one_group();
        tree.push_tabs_for_test(tree.root().id(), vec![panel(9)]);
        tree.normalize();

        tree.split(tabs, panel(2), Placement::Top, None);

        let NodeRef::Split { children, .. } = tree.root().kind() else {
            panic!()
        };
        let NodeRef::Split {
            axis: inner,
            children: inner_children,
            ..
        } = children[0].kind()
        else {
            panic!("the split target is wrapped in a vertical split")
        };
        assert_eq!(inner, Axis::Vertical);
        assert_eq!(inner_children.len(), 2);

        // `Top` is the mirror of `Bottom`: the new group lands first, above
        // the original target.
        let NodeRef::Tabs { panels, .. } = inner_children[0].kind() else {
            panic!()
        };
        assert_eq!(panels, [panel(2)], "the new group lands first, above");
        let NodeRef::Tabs { panels, .. } = inner_children[1].kind() else {
            panic!()
        };
        assert_eq!(panels, [panel(1)], "the original target moves second");
    }

    #[test]
    fn the_split_target_keeps_its_node_id_so_its_entity_survives() {
        let (mut tree, tabs) = tree_with_one_group();
        tree.split(tabs, panel(2), Placement::Right, None);

        assert!(
            tree.find_node(tabs).is_some(),
            "the target group is reused, not rebuilt"
        );
    }

    #[test]
    fn moving_a_panel_between_groups_preserves_its_identity() {
        let (mut tree, tabs) = tree_with_one_group();
        let result = tree.split(tabs, panel(2), Placement::Right, None);
        let other = result.created_nodes()[0];

        let result = tree.move_panel(
            panel(1),
            InsertTarget::Tabs {
                node: other,
                ix: None,
                activate: true,
            },
        );

        assert!(result.changed());
        assert!(
            result.removed_panels().is_empty(),
            "a move is not a removal; the panel entity must survive"
        );
        assert_eq!(tree.panels().collect::<Vec<_>>(), vec![panel(2), panel(1)]);
        assert!(
            tree.find_node(tabs).is_none(),
            "the emptied group collapses"
        );
    }

    #[test]
    fn a_no_op_edit_reports_no_change() {
        let (mut tree, tabs) = tree_with_one_group();
        let result = tree.set_active(tabs, 0);
        assert!(!result.changed());
    }

    #[test]
    fn set_sizes_replaces_a_matching_length_vector() {
        let (mut tree, tabs) = tree_with_one_group();
        tree.split(tabs, panel(2), Placement::Right, None);
        let root = tree.root().id();

        let result = tree.set_sizes(root, vec![Some(px(100.)), Some(px(200.))]);

        assert!(result.changed());
        let NodeRef::Split { sizes, .. } = tree.root().kind() else {
            panic!()
        };
        assert_eq!(sizes, &[Some(px(100.)), Some(px(200.))]);
    }

    #[test]
    fn set_sizes_ignores_a_mismatched_length_vector() {
        let (mut tree, tabs) = tree_with_one_group();
        tree.split(tabs, panel(2), Placement::Right, None);
        let root = tree.root().id();
        let NodeRef::Split {
            sizes: before_sizes,
            ..
        } = tree.root().kind()
        else {
            panic!()
        };
        let before_sizes = before_sizes.to_vec();

        // The split has 2 children; hand it 3 sizes.
        let result = tree.set_sizes(root, vec![Some(px(10.)), Some(px(20.)), Some(px(30.))]);

        assert!(
            !result.changed(),
            "a mismatched vector must not report a change"
        );
        let NodeRef::Split { sizes, .. } = tree.root().kind() else {
            panic!()
        };
        assert_eq!(
            sizes,
            before_sizes.as_slice(),
            "the mismatched vector is rejected"
        );
    }

    fn tile_bounds(x: f32) -> Bounds<Pixels> {
        Bounds {
            origin: point(px(x), px(0.)),
            size: size(px(10.), px(10.)),
        }
    }

    #[test]
    fn inserting_a_tile_places_it_at_the_given_bounds_on_top() {
        let mut tree = LayoutTree::new(RootKind::Any);
        let canvas = tree.set_root_tiles_for_test(vec![
            TilePanel::new(panel(1), tile_bounds(0.)).with_z_index(4),
        ]);

        let result = tree.insert_panel(
            panel(2),
            InsertTarget::Tile {
                node: canvas,
                bounds: tile_bounds(70.),
            },
        );

        assert!(result.changed());
        let NodeRef::Tiles { panels } = tree.root().kind() else {
            panic!("the canvas stays a canvas rather than being split")
        };
        assert_eq!(panels.len(), 2);
        let added = panels.iter().find(|tile| tile.panel() == panel(2)).unwrap();
        assert_eq!(added.bounds(), tile_bounds(70.), "the bounds are honoured");
        assert!(
            added.z_index() > panels[0].z_index(),
            "a new tile lands on top of the ones already there"
        );
    }

    #[test]
    fn set_tile_bounds_moves_one_tile_and_leaves_its_peers_alone() {
        let mut tree = LayoutTree::new(RootKind::Any);
        tree.set_root_tiles_for_test(vec![
            TilePanel::new(panel(1), tile_bounds(0.)).with_z_index(1),
            TilePanel::new(panel(2), tile_bounds(40.)).with_z_index(2),
        ]);

        let result = tree.set_tile_bounds(panel(1), tile_bounds(90.));

        assert!(result.changed());
        let NodeRef::Tiles { panels } = tree.root().kind() else {
            panic!()
        };
        let moved = panels.iter().find(|tile| tile.panel() == panel(1)).unwrap();
        let other = panels.iter().find(|tile| tile.panel() == panel(2)).unwrap();
        assert_eq!(moved.bounds(), tile_bounds(90.));
        assert_eq!(
            moved.z_index(),
            1,
            "moving a tile does not raise it; that is `bring_to_front`'s job"
        );
        assert_eq!(other.bounds(), tile_bounds(40.), "its peer does not move");
    }

    #[test]
    fn bring_to_front_raises_the_tile_above_its_peers() {
        let mut tree = LayoutTree::new(RootKind::Any);
        let bounds = Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(10.), px(10.)),
        };
        tree.set_root_tiles_for_test(vec![
            TilePanel::new(panel(1), bounds).with_z_index(0),
            TilePanel::new(panel(2), bounds).with_z_index(1),
        ]);

        tree.bring_to_front(panel(1));

        let NodeRef::Tiles { panels } = tree.root().kind() else {
            panic!()
        };
        let raised = panels.iter().find(|p| p.panel() == panel(1)).unwrap();
        let other = panels.iter().find(|p| p.panel() == panel(2)).unwrap();
        assert!(raised.z_index() > other.z_index());
    }

    #[test]
    fn every_edit_leaves_the_tree_normalized() {
        let (mut tree, tabs) = tree_with_one_group();
        tree.split(tabs, panel(2), Placement::Bottom, None);
        tree.insert_panel(
            panel(3),
            InsertTarget::Tabs {
                node: tabs,
                ix: None,
                activate: false,
            },
        );
        tree.remove_panel(panel(2));
        tree.remove_panel(panel(3));

        assert!(tree.is_normalized());
    }
}
