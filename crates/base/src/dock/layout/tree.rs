use smallvec::SmallVec;

use super::node::{LayoutNode, NodeId, NodeKind, NodeRef, PanelId, TilePanel};

/// Whether the root of this tree is pinned to a split.
///
/// The center of a `DockArea` must serialize as a `StackPanel` even when
/// empty, which `RootKind::Split` guarantees. A dock's root is unconstrained.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RootKind {
    Split,
    Any,
}

/// Path from the root as a sequence of child indices.
pub(crate) type NodePath = SmallVec<[usize; 8]>;

#[derive(Clone, PartialEq, Debug)]
pub struct LayoutTree {
    root: LayoutNode,
    next_node_id: u64,
    root_kind: RootKind,
}

impl LayoutTree {
    pub fn new(root_kind: RootKind) -> Self {
        let root = LayoutNode::new(
            NodeId::from_u64(0),
            NodeKind::Split {
                axis: gpui::Axis::Horizontal,
                children: Vec::new(),
                sizes: Vec::new(),
            },
        );
        Self {
            root,
            next_node_id: 1,
            root_kind,
        }
    }

    pub fn root(&self) -> &LayoutNode {
        &self.root
    }

    /// Mutable access to the root, for normalization's post-order pass.
    pub(crate) fn root_mut(&mut self) -> &mut LayoutNode {
        &mut self.root
    }

    pub fn root_kind(&self) -> RootKind {
        self.root_kind
    }

    // Only called by `#[cfg(test)]` seeders today; edit operations become
    // real callers in a later task.
    #[allow(dead_code)]
    pub(crate) fn allocate_node_id(&mut self) -> NodeId {
        let id = NodeId::from_u64(self.next_node_id);
        self.next_node_id += 1;
        id
    }

    /// Every container id in the tree, in pre-order.
    pub fn node_ids(&self) -> Vec<NodeId> {
        let mut ids = Vec::new();
        self.root.walk(&mut |node| ids.push(node.id()));
        ids
    }

    /// Every panel in the tree, in pre-order.
    pub fn panels(&self) -> impl Iterator<Item = PanelId> {
        let mut found = Vec::new();
        self.root.walk(&mut |node| match node.kind() {
            NodeRef::Tabs { panels, .. } => found.extend_from_slice(panels),
            NodeRef::Tiles { panels } => found.extend(panels.iter().map(TilePanel::panel)),
            NodeRef::Split { .. } => {}
        });
        found.into_iter()
    }

    pub fn find_node(&self, id: NodeId) -> Option<&LayoutNode> {
        self.path_of_node(id).map(|path| self.node_at(&path))
    }

    /// The tab or tiles node holding `panel`.
    pub fn find_panel_node(&self, panel: PanelId) -> Option<NodeId> {
        let mut found = None;
        self.root.walk(&mut |node| {
            let holds = match node.kind() {
                NodeRef::Tabs { panels, .. } => panels.contains(&panel),
                NodeRef::Tiles { panels } => panels.iter().any(|p| p.panel() == panel),
                NodeRef::Split { .. } => false,
            };
            if holds {
                found = Some(node.id());
            }
        });
        found
    }

    pub(crate) fn path_of_node(&self, id: NodeId) -> Option<NodePath> {
        fn search(node: &LayoutNode, id: NodeId, path: &mut NodePath) -> bool {
            if node.id() == id {
                return true;
            }
            if let NodeKind::Split { children, .. } = node.kind_ref() {
                for (ix, child) in children.iter().enumerate() {
                    path.push(ix);
                    if search(child, id, path) {
                        return true;
                    }
                    path.pop();
                }
            }
            false
        }

        let mut path = NodePath::new();
        search(&self.root, id, &mut path).then_some(path)
    }

    pub(crate) fn node_at(&self, path: &NodePath) -> &LayoutNode {
        let mut node = &self.root;
        for ix in path {
            let NodeKind::Split { children, .. } = node.kind_ref() else {
                unreachable!("path traverses a non-split node");
            };
            node = &children[*ix];
        }
        node
    }

    // Only called by `#[cfg(test)]` seeders today; edit operations become
    // real callers in a later task.
    #[allow(dead_code)]
    pub(crate) fn node_at_mut(&mut self, path: &NodePath) -> &mut LayoutNode {
        let mut node = &mut self.root;
        for ix in path {
            let NodeKind::Split { children, .. } = node.kind_mut() else {
                unreachable!("path traverses a non-split node");
            };
            node = &mut children[*ix];
        }
        node
    }

    /// Replace the whole tree with `node`, keeping `node`'s own id and
    /// `root_kind` unchanged. Used by normalization's root-collapse rule.
    pub(crate) fn replace_root(&mut self, node: LayoutNode) {
        self.root = node;
    }
}

#[cfg(test)]
use gpui::{Axis, Pixels};

#[cfg(test)]
impl LayoutTree {
    pub(crate) fn push_tabs_for_test(&mut self, parent: NodeId, panels: Vec<PanelId>) -> NodeId {
        let id = self.allocate_node_id();
        let path = self.path_of_node(parent).expect("parent must exist");
        let NodeKind::Split {
            children, sizes, ..
        } = self.node_at_mut(&path).kind_mut()
        else {
            panic!("parent must be a split");
        };
        children.push(LayoutNode::new(
            id,
            NodeKind::Tabs {
                panels,
                active_ix: 0,
            },
        ));
        sizes.push(None);
        id
    }

    /// Like [`Self::push_tabs_for_test`], but with a concrete slot size
    /// instead of always pushing `None`. Needed to exercise the scaling
    /// arithmetic in `normalize`'s same-axis splice rule, which only runs
    /// when every sibling size is known.
    pub(crate) fn push_sized_tabs_for_test(
        &mut self,
        parent: NodeId,
        panels: Vec<PanelId>,
        size: Option<Pixels>,
    ) -> NodeId {
        let id = self.allocate_node_id();
        let path = self.path_of_node(parent).expect("parent must exist");
        let NodeKind::Split {
            children, sizes, ..
        } = self.node_at_mut(&path).kind_mut()
        else {
            panic!("parent must be a split");
        };
        children.push(LayoutNode::new(
            id,
            NodeKind::Tabs {
                panels,
                active_ix: 0,
            },
        ));
        sizes.push(size);
        id
    }

    pub(crate) fn set_root_tiles_for_test(&mut self, panels: Vec<TilePanel>) -> NodeId {
        let id = self.allocate_node_id();
        self.root = LayoutNode::new(id, NodeKind::Tiles { panels });
        id
    }

    pub(crate) fn set_root_split_for_test(&mut self, axis: Axis) -> NodeId {
        let id = self.allocate_node_id();
        self.root = LayoutNode::new(
            id,
            NodeKind::Split {
                axis,
                children: Vec::new(),
                sizes: Vec::new(),
            },
        );
        id
    }

    pub(crate) fn set_root_axis_for_test(&mut self, new_axis: Axis) {
        if let NodeKind::Split { axis, .. } = self.root.kind_mut() {
            *axis = new_axis;
        }
    }

    pub(crate) fn set_root_tabs_for_test(
        &mut self,
        panels: Vec<PanelId>,
        active_ix: usize,
    ) -> NodeId {
        let id = self.allocate_node_id();
        self.root = LayoutNode::new(id, NodeKind::Tabs { panels, active_ix });
        id
    }

    pub(crate) fn push_split_for_test(
        &mut self,
        parent: NodeId,
        axis: Axis,
        size: Option<Pixels>,
    ) -> NodeId {
        let id = self.allocate_node_id();
        let path = self.path_of_node(parent).expect("parent must exist");
        let NodeKind::Split {
            children, sizes, ..
        } = self.node_at_mut(&path).kind_mut()
        else {
            panic!("parent must be a split");
        };
        children.push(LayoutNode::new(
            id,
            NodeKind::Split {
                axis,
                children: Vec::new(),
                sizes: Vec::new(),
            },
        ));
        sizes.push(size);
        id
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::*;

    fn panel_id(n: u64) -> PanelId {
        PanelId::from_u64(n)
    }

    #[test]
    fn a_fresh_split_root_tree_has_no_panels() {
        let tree = LayoutTree::new(RootKind::Split);
        assert!(
            matches!(tree.root().kind(), NodeRef::Split { children, .. } if children.is_empty())
        );
        assert_eq!(tree.panels().count(), 0);
    }

    #[test]
    fn node_ids_are_unique_and_resolvable() {
        let mut tree = LayoutTree::new(RootKind::Split);
        let root = tree.root().id();
        let tabs = tree.push_tabs_for_test(root, vec![panel_id(1)]);
        assert_ne!(tabs, tree.root().id());
        assert!(tree.find_node(tabs).is_some());
        assert_eq!(tree.find_panel_node(panel_id(1)), Some(tabs));
    }

    #[test]
    fn tile_panels_carry_bounds_and_z_index() {
        let mut tree = LayoutTree::new(RootKind::Any);
        let tiles = tree.set_root_tiles_for_test(vec![
            TilePanel::new(
                panel_id(7),
                Bounds {
                    origin: point(px(10.), px(20.)),
                    size: size(px(100.), px(50.)),
                },
            )
            .with_z_index(3),
        ]);
        let NodeRef::Tiles { panels } = tree.find_node(tiles).unwrap().kind() else {
            panic!("expected tiles root");
        };
        assert_eq!(panels[0].panel(), panel_id(7));
        assert_eq!(panels[0].z_index(), 3);
        assert_eq!(panels[0].bounds().size.width, px(100.));
    }
}
