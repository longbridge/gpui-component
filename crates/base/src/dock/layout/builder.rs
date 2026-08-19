//! Describing a layout without building it.

use std::sync::Arc;

use gpui::{Axis, Bounds, Entity, Pixels};

use super::node::{LayoutNode, NodeKind, PanelId, TilePanel};
use super::tree::{LayoutTree, RootKind};
use crate::dock::panel::{Panel, PanelView};

/// Describes a layout without constructing any entity.
///
/// Building a layout used to require `window` and `cx` because every node was
/// an entity. Now a layout is data, and `DockArea` reconciles it into entities
/// when it is installed.
///
/// Misuse — a panel added to a split, a child container added to a tab group,
/// an active index on anything but a tab group — trips a `debug_assert!` and
/// is otherwise ignored, matching how the old `DockItem::active_index` guarded
/// itself. A type-state builder would reject the same mistakes at compile
/// time, but it needs four marker types and leaks into every consumer
/// signature; the runtime assertion is proportionate for a builder whose
/// misuse shows up on the first debug run.
pub struct DockLayout {
    kind: BuilderKind,
}

enum BuilderKind {
    Split {
        axis: Axis,
        children: Vec<(DockLayout, Option<Pixels>)>,
    },
    Tabs {
        panels: Vec<(PanelId, Arc<dyn PanelView>)>,
        active_ix: usize,
    },
    Tiles {
        panels: Vec<(PanelId, Arc<dyn PanelView>, Bounds<Pixels>)>,
    },
}

impl DockLayout {
    /// A split whose children sit side by side.
    pub fn h_split() -> Self {
        Self::split(Axis::Horizontal)
    }

    /// A split whose children stack.
    pub fn v_split() -> Self {
        Self::split(Axis::Vertical)
    }

    pub fn tabs() -> Self {
        Self {
            kind: BuilderKind::Tabs {
                panels: Vec::new(),
                active_ix: 0,
            },
        }
    }

    pub fn tiles() -> Self {
        Self {
            kind: BuilderKind::Tiles { panels: Vec::new() },
        }
    }

    fn split(axis: Axis) -> Self {
        Self {
            kind: BuilderKind::Split {
                axis,
                children: Vec::new(),
            },
        }
    }

    /// Add a child container to a split. `size` is the child's slot along the
    /// split's axis; `None` leaves it unconstrained.
    pub fn child(mut self, child: DockLayout, size: Option<Pixels>) -> Self {
        debug_assert!(
            matches!(self.kind, BuilderKind::Split { .. }),
            "child() is only valid on h_split() or v_split()"
        );
        if let BuilderKind::Split { children, .. } = &mut self.kind {
            children.push((child, size));
        }
        self
    }

    /// Add a panel to a tab group.
    pub fn panel<P: Panel>(mut self, panel: Entity<P>) -> Self {
        debug_assert!(
            matches!(self.kind, BuilderKind::Tabs { .. }),
            "panel() is only valid on tabs()"
        );
        if let BuilderKind::Tabs { panels, .. } = &mut self.kind {
            panels.push((PanelId::from(panel.entity_id()), Arc::new(panel)));
        }
        self
    }

    /// Place a panel on a tiles canvas.
    pub fn tile<P: Panel>(mut self, panel: Entity<P>, bounds: Bounds<Pixels>) -> Self {
        debug_assert!(
            matches!(self.kind, BuilderKind::Tiles { .. }),
            "tile() is only valid on tiles()"
        );
        if let BuilderKind::Tiles { panels } = &mut self.kind {
            panels.push((PanelId::from(panel.entity_id()), Arc::new(panel), bounds));
        }
        self
    }

    /// Which tab is displayed. Out-of-range values are clamped by
    /// `normalize` once the layout is installed.
    pub fn active_index(mut self, ix: usize) -> Self {
        debug_assert!(
            matches!(self.kind, BuilderKind::Tabs { .. }),
            "active_index() is only valid on tabs()"
        );
        if let BuilderKind::Tabs { active_ix, .. } = &mut self.kind {
            *active_ix = ix;
        }
        self
    }

    /// Lower into a tree plus the panel views the area must register.
    ///
    /// The views come back paired with the ids the tree was built from rather
    /// than as bare views: recovering an id from a view means calling
    /// [`PanelView::panel_id`], and nothing here would notice if a `PanelView`
    /// implementation ever answered with something other than its entity id.
    pub(crate) fn build(
        self,
        tree: &mut LayoutTree,
    ) -> (LayoutNode, Vec<(PanelId, Arc<dyn PanelView>)>) {
        let mut panels = Vec::new();
        let node = self.build_node(tree, &mut panels);
        (node, panels)
    }

    fn build_node(
        self,
        tree: &mut LayoutTree,
        collected: &mut Vec<(PanelId, Arc<dyn PanelView>)>,
    ) -> LayoutNode {
        let id = tree.allocate_node_id();
        match self.kind {
            BuilderKind::Split { axis, children } => {
                let mut nodes = Vec::with_capacity(children.len());
                let mut sizes = Vec::with_capacity(children.len());
                for (child, size) in children {
                    nodes.push(child.build_node(tree, collected));
                    sizes.push(size);
                }
                LayoutNode::new(
                    id,
                    NodeKind::Split {
                        axis,
                        children: nodes,
                        sizes,
                    },
                )
            }
            BuilderKind::Tabs { panels, active_ix } => {
                let ids = panels.iter().map(|(id, _)| *id).collect();
                collected.extend(panels);
                LayoutNode::new(
                    id,
                    NodeKind::Tabs {
                        panels: ids,
                        active_ix,
                    },
                )
            }
            BuilderKind::Tiles { panels } => {
                let tiles = panels
                    .iter()
                    .enumerate()
                    .map(|(ix, (panel, _, bounds))| {
                        TilePanel::new(*panel, *bounds).with_z_index(ix)
                    })
                    .collect();
                collected.extend(panels.into_iter().map(|(id, view, _)| (id, view)));
                LayoutNode::new(id, NodeKind::Tiles { panels: tiles })
            }
        }
    }
}

impl LayoutTree {
    /// Build a whole tree from a described layout.
    ///
    /// The `RootKind::Split` wrap mirrors the one in
    /// [`LayoutTree::from_state`](crate::dock::LayoutTree::from_state): a
    /// center whose described root is a tab group or a tiles canvas still has
    /// to serialize as a `StackPanel`.
    pub(crate) fn from_layout(
        layout: DockLayout,
        root_kind: RootKind,
    ) -> (Self, Vec<(PanelId, Arc<dyn PanelView>)>) {
        let mut tree = LayoutTree::new(root_kind);
        let (root, panels) = layout.build(&mut tree);

        let root = match (root_kind, root.kind_ref()) {
            (RootKind::Split, NodeKind::Split { .. }) | (RootKind::Any, _) => root,
            (RootKind::Split, _) => {
                let id = tree.allocate_node_id();
                LayoutNode::new(
                    id,
                    NodeKind::Split {
                        axis: Axis::Horizontal,
                        children: vec![root],
                        sizes: vec![None],
                    },
                )
            }
        };

        tree.replace_root(root);
        tree.normalize();
        (tree, panels)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{TestAppContext, px};

    use super::super::NodeRef;
    use super::*;
    use crate::dock::test_support::TestPanel;

    #[gpui::test]
    fn a_described_split_lowers_to_a_split_of_tab_groups(cx: &mut TestAppContext) {
        let (tree, panels) = cx.update(|cx| {
            let alpha = TestPanel::new("Alpha", cx);
            let beta = TestPanel::new("Beta", cx);
            LayoutTree::from_layout(
                DockLayout::h_split()
                    .child(DockLayout::tabs().panel(alpha), Some(px(300.)))
                    .child(DockLayout::tabs().panel(beta), None),
                RootKind::Split,
            )
        });

        assert_eq!(panels.len(), 2);
        let NodeRef::Split {
            axis,
            children,
            sizes,
        } = tree.root().kind()
        else {
            panic!("expected a split root");
        };
        assert_eq!(axis, gpui::Axis::Horizontal);
        assert_eq!(children.len(), 2);
        assert_eq!(sizes, &[Some(px(300.)), None]);
        assert!(matches!(children[0].kind(), NodeRef::Tabs { panels, .. } if panels.len() == 1));
    }

    #[gpui::test]
    fn a_bare_tab_group_is_wrapped_for_a_split_root(cx: &mut TestAppContext) {
        let (tree, _) = cx.update(|cx| {
            let alpha = TestPanel::new("Alpha", cx);
            LayoutTree::from_layout(DockLayout::tabs().panel(alpha), RootKind::Split)
        });

        assert!(matches!(tree.root().kind(), NodeRef::Split { .. }));
    }

    #[gpui::test]
    fn an_active_index_selects_the_displayed_tab(cx: &mut TestAppContext) {
        let (tree, _) = cx.update(|cx| {
            let alpha = TestPanel::new("Alpha", cx);
            let beta = TestPanel::new("Beta", cx);
            LayoutTree::from_layout(
                DockLayout::tabs().panel(alpha).panel(beta).active_index(1),
                RootKind::Any,
            )
        });

        let NodeRef::Tabs { active_ix, .. } = tree.root().kind() else {
            panic!("expected a tab group root");
        };
        assert_eq!(active_ix, 1);
    }

    #[gpui::test]
    fn tiles_are_stacked_in_the_order_they_are_placed(cx: &mut TestAppContext) {
        let bounds = Bounds {
            origin: gpui::point(px(0.), px(0.)),
            size: gpui::size(px(100.), px(100.)),
        };
        let (tree, _) = cx.update(|cx| {
            let alpha = TestPanel::new("Alpha", cx);
            let beta = TestPanel::new("Beta", cx);
            LayoutTree::from_layout(
                DockLayout::tiles().tile(alpha, bounds).tile(beta, bounds),
                RootKind::Any,
            )
        });

        let NodeRef::Tiles { panels } = tree.root().kind() else {
            panic!("expected a tiles root");
        };
        assert_eq!(panels[0].z_index(), 0);
        assert_eq!(panels[1].z_index(), 1);
    }
}
