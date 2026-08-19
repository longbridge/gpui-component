use gpui::Pixels;

use super::node::{LayoutNode, NodeKind};
use super::tree::{LayoutTree, RootKind};

impl LayoutTree {
    /// Collapse the tree to canonical shape.
    ///
    /// One post-order pass repeated to a fixpoint. This is the only place a
    /// container is removed for being empty, replacing the mutually recursive
    /// `remove_self_if_empty` pair the old implementation used. It needs no
    /// parent pointers and no deferred work, so the tree is self-consistent
    /// the instant an edit returns.
    ///
    /// Rules, applied bottom up:
    ///
    /// 1. An empty `Tabs`, `Tiles`, or `Split` is removed from its parent.
    /// 2. A `Split` with one child is replaced by that child. The child keeps
    ///    its own `NodeId` and inherits the split's slot size.
    /// 3. A `Split` whose child is a `Split` of the same axis splices that
    ///    child's children into itself.
    /// 4. `active_ix` is clamped.
    /// 5. The root is preserved according to [`RootKind`].
    ///
    /// Idempotent: `normalize(normalize(t)) == normalize(t)`.
    pub fn normalize(&mut self) {
        // Bounded because every pass that changes anything strictly reduces
        // node count or nesting depth.
        for _ in 0..64 {
            let mut changed = false;
            normalize_node(self.root_mut(), &mut changed);
            collapse_root(self, &mut changed);
            if !changed {
                break;
            }
        }

        debug_assert!(self.is_normalized(), "normalize did not reach a fixpoint");
    }

    /// Whether the tree satisfies every structural invariant.
    pub(crate) fn is_normalized(&self) -> bool {
        let mut ok = true;
        let root_id = self.root().id();
        self.root().walk(&mut |node| match node.kind_ref() {
            NodeKind::Split {
                children,
                sizes,
                axis,
            } => {
                ok &= children.len() == sizes.len();
                // The root may legitimately be an empty or single-child split.
                if node.id() != root_id {
                    ok &= children.len() > 1;
                }
                ok &= !children.iter().any(|child| {
                    matches!(child.kind_ref(), NodeKind::Split { axis: inner, .. } if inner == axis)
                });
            }
            NodeKind::Tabs { panels, active_ix } => {
                ok &= panels.is_empty() || *active_ix < panels.len();
                if node.id() != root_id {
                    ok &= !panels.is_empty();
                }
            }
            NodeKind::Tiles { panels } => {
                if node.id() != root_id {
                    ok &= !panels.is_empty();
                }
            }
        });
        ok
    }
}

fn normalize_node(node: &mut LayoutNode, changed: &mut bool) {
    match node.kind_mut() {
        NodeKind::Tabs { panels, active_ix } => {
            let clamped = (*active_ix).min(panels.len().saturating_sub(1));
            if *active_ix != clamped {
                *active_ix = clamped;
                *changed = true;
            }
        }
        NodeKind::Tiles { .. } => {}
        NodeKind::Split {
            axis,
            children,
            sizes,
        } => {
            let axis = *axis;

            for child in children.iter_mut() {
                normalize_node(child, changed);
            }

            // Rule 1: drop empty children.
            let mut ix = 0;
            while ix < children.len() {
                if is_empty_container(&children[ix]) {
                    children.remove(ix);
                    sizes.remove(ix);
                    *changed = true;
                } else {
                    ix += 1;
                }
            }

            // Rule 2: a single-child split child is replaced by its child,
            // which inherits the slot size the split occupied.
            for ix in 0..children.len() {
                let replacement = match children[ix].kind_ref() {
                    NodeKind::Split {
                        children: inner, ..
                    } if inner.len() == 1 => Some(inner[0].clone()),
                    _ => None,
                };
                if let Some(replacement) = replacement {
                    children[ix] = replacement;
                    *changed = true;
                }
            }

            // Rule 3: splice same-axis nesting.
            let mut ix = 0;
            while ix < children.len() {
                let splice = match children[ix].kind_ref() {
                    NodeKind::Split {
                        axis: inner_axis,
                        children: inner,
                        sizes: inner_sizes,
                    } if *inner_axis == axis => Some((inner.clone(), inner_sizes.clone())),
                    _ => None,
                };

                if let Some((inner, inner_sizes)) = splice {
                    let slot = sizes[ix];
                    let inner_sizes = distribute_slot(slot, inner_sizes);
                    children.splice(ix..=ix, inner.iter().cloned());
                    sizes.splice(ix..=ix, inner_sizes.iter().copied());
                    ix += inner.len();
                    *changed = true;
                } else {
                    ix += 1;
                }
            }
        }
    }
}

/// Spread an outer slot size across the inner sizes that replace it.
///
/// When the outer slot is unconstrained the inner sizes pass through. When it
/// is fixed and every inner size is known, they are scaled to fill the slot;
/// otherwise the slot is dropped, matching how an unconstrained child behaves.
fn distribute_slot(slot: Option<Pixels>, inner: Vec<Option<Pixels>>) -> Vec<Option<Pixels>> {
    let Some(slot) = slot else { return inner };
    // `Option<Pixels>` has no `Sum` impl; fold so one unknown size makes the
    // whole total unknown.
    let total = inner
        .iter()
        .try_fold(Pixels::ZERO, |acc, size| size.map(|size| acc + size));
    match total {
        Some(total) if total > Pixels::ZERO => inner
            .into_iter()
            .map(|size| size.map(|size| size * (slot / total)))
            .collect(),
        _ => inner,
    }
}

fn is_empty_container(node: &LayoutNode) -> bool {
    match node.kind_ref() {
        NodeKind::Split { children, .. } => children.is_empty(),
        NodeKind::Tabs { panels, .. } => panels.is_empty(),
        NodeKind::Tiles { panels } => panels.is_empty(),
    }
}

/// Rule 5. A `RootKind::Split` tree keeps a split root no matter what, so an
/// empty center still serializes as a `StackPanel`. A `RootKind::Any` tree
/// lets rule 2 collapse the root like any other node.
fn collapse_root(tree: &mut LayoutTree, changed: &mut bool) {
    if tree.root_kind() == RootKind::Split {
        return;
    }

    let replacement = match tree.root().kind_ref() {
        NodeKind::Split { children, .. } if children.len() == 1 => Some(children[0].clone()),
        _ => None,
    };

    if let Some(replacement) = replacement {
        tree.replace_root(replacement);
        *changed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use gpui::{Axis, px};

    fn panel(n: u64) -> PanelId {
        PanelId::from_u64(n)
    }

    #[test]
    fn empty_tab_groups_are_dropped() {
        let mut tree = LayoutTree::new(RootKind::Split);
        let root = tree.root().id();
        tree.push_tabs_for_test(root, vec![]);
        tree.push_tabs_for_test(root, vec![panel(1)]);

        tree.normalize();

        // The empty tab group is dropped by rule 1, leaving the root split
        // holding the one surviving child.
        assert!(
            matches!(tree.root().kind(), NodeRef::Split { children, .. } if children.len() == 1)
        );
        assert_eq!(tree.panels().collect::<Vec<_>>(), vec![panel(1)]);
    }

    #[test]
    fn a_single_child_split_is_replaced_by_its_child_keeping_the_child_id() {
        let mut tree = LayoutTree::new(RootKind::Any);
        let outer = tree.set_root_split_for_test(Axis::Horizontal);
        let inner = tree.push_split_for_test(outer, Axis::Vertical, Some(px(120.)));
        let tabs = tree.push_tabs_for_test(inner, vec![panel(1)]);

        tree.normalize();

        assert_eq!(tree.root().id(), tabs, "child keeps its own NodeId");
        assert!(tree.find_node(inner).is_none());
    }

    #[test]
    fn a_collapsing_split_hands_its_slot_size_to_the_child() {
        let mut tree = LayoutTree::new(RootKind::Split);
        let root = tree.root().id();
        let inner = tree.push_split_for_test(root, Axis::Vertical, Some(px(300.)));
        tree.push_tabs_for_test(inner, vec![panel(1)]);
        tree.push_tabs_for_test(root, vec![panel(2)]);

        tree.normalize();

        let NodeRef::Split { sizes, .. } = tree.root().kind() else {
            panic!()
        };
        assert_eq!(
            sizes[0],
            Some(px(300.)),
            "the child inherits the collapsed split's slot"
        );
    }

    #[test]
    fn same_axis_nesting_is_spliced_into_the_parent() {
        let mut tree = LayoutTree::new(RootKind::Split);
        tree.set_root_axis_for_test(Axis::Horizontal);
        let root = tree.root().id();
        tree.push_tabs_for_test(root, vec![panel(1)]);
        let inner = tree.push_split_for_test(root, Axis::Horizontal, None);
        tree.push_tabs_for_test(inner, vec![panel(2)]);
        tree.push_tabs_for_test(inner, vec![panel(3)]);

        tree.normalize();

        let NodeRef::Split { children, axis, .. } = tree.root().kind() else {
            panic!()
        };
        assert_eq!(axis, Axis::Horizontal);
        assert_eq!(
            children.len(),
            3,
            "the inner split's children are spliced in"
        );
        assert_eq!(
            tree.panels().collect::<Vec<_>>(),
            vec![panel(1), panel(2), panel(3)],
            "order is preserved"
        );
    }

    #[test]
    fn active_index_is_clamped_to_the_panel_count() {
        let mut tree = LayoutTree::new(RootKind::Any);
        let tabs = tree.set_root_tabs_for_test(vec![panel(1), panel(2)], 9);

        tree.normalize();

        let NodeRef::Tabs { active_ix, .. } = tree.find_node(tabs).unwrap().kind() else {
            panic!()
        };
        assert_eq!(active_ix, 1);
    }

    #[test]
    fn a_split_root_survives_being_emptied() {
        let mut tree = LayoutTree::new(RootKind::Split);
        let root = tree.root().id();
        tree.push_tabs_for_test(root, vec![]);

        tree.normalize();

        assert!(
            matches!(tree.root().kind(), NodeRef::Split { children, .. } if children.is_empty()),
            "the center must still serialize as a StackPanel when empty"
        );
    }

    #[test]
    fn normalize_is_idempotent() {
        let mut tree = LayoutTree::new(RootKind::Split);
        let root = tree.root().id();
        let inner = tree.push_split_for_test(root, Axis::Horizontal, None);
        tree.push_tabs_for_test(inner, vec![panel(1)]);
        tree.push_tabs_for_test(inner, vec![]);
        tree.push_tabs_for_test(root, vec![panel(2)]);

        tree.normalize();
        let once = tree.clone();
        tree.normalize();

        assert_eq!(once, tree);
    }
}
