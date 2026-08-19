use super::layout::{LayoutNode, LayoutTree, NodeRef, PanelId};
use super::state::{PanelInfo, PanelState, TileMeta};

/// The names written to persisted layouts. These are contract, not type names:
/// they must keep their values even if the Rust types are renamed.
pub(crate) const STACK_PANEL_NAME: &str = "StackPanel";
pub(crate) const TAB_PANEL_NAME: &str = "TabPanel";
pub(crate) const TILES_PANEL_NAME: &str = "Tiles";

/// How the layout tree reads properties of panels it only knows by id.
///
/// Keeping this behind a trait is what lets the whole layout algebra be tested
/// without an `App`.
pub trait PanelSource {
    fn panel_name(&self, id: PanelId) -> &'static str;
    fn is_visible(&self, id: PanelId) -> bool;
    fn dump(&self, id: PanelId) -> PanelState;
}

impl LayoutTree {
    pub fn to_state(&self, source: &dyn PanelSource) -> PanelState {
        node_to_state(self.root(), source)
    }
}

fn node_to_state(node: &LayoutNode, source: &dyn PanelSource) -> PanelState {
    match node.kind() {
        NodeRef::Split {
            axis,
            children,
            sizes,
        } => PanelState {
            panel_name: STACK_PANEL_NAME.to_string(),
            children: children
                .iter()
                .map(|child| node_to_state(child, source))
                .collect(),
            // Slot sizes are persisted as concrete pixels; an unconstrained
            // slot has never been representable in the schema and is written
            // as zero, exactly as `ResizableState::sizes` reported it before.
            info: PanelInfo::stack(
                sizes.iter().map(|size| size.unwrap_or_default()).collect(),
                axis,
            ),
        },
        NodeRef::Tabs { panels, active_ix } => PanelState {
            panel_name: TAB_PANEL_NAME.to_string(),
            children: panels.iter().map(|panel| source.dump(*panel)).collect(),
            // Unconditional, unlike the old writer which assigned this inside
            // its loop and left an empty group looking like a bare panel.
            info: PanelInfo::tabs(active_ix),
        },
        NodeRef::Tiles { panels } => PanelState {
            panel_name: TILES_PANEL_NAME.to_string(),
            children: panels
                .iter()
                .map(|tile| source.dump(tile.panel()))
                .collect(),
            info: PanelInfo::tiles(
                panels
                    .iter()
                    .map(|tile| TileMeta {
                        bounds: tile.bounds(),
                        z_index: tile.z_index(),
                    })
                    .collect(),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::super::layout::{RootKind, TilePanel};
    use super::*;

    /// A `PanelSource` backed by a fixed map, so conversion is testable
    /// without an `App`.
    struct FakePanels(Vec<(PanelId, &'static str)>);

    impl PanelSource for FakePanels {
        fn panel_name(&self, id: PanelId) -> &'static str {
            self.0
                .iter()
                .find(|(p, _)| *p == id)
                .map(|(_, n)| *n)
                .unwrap_or("Unknown")
        }
        fn is_visible(&self, _: PanelId) -> bool {
            true
        }
        fn dump(&self, id: PanelId) -> PanelState {
            PanelState {
                panel_name: self.panel_name(id).to_string(),
                children: Vec::new(),
                info: PanelInfo::panel(serde_json::Value::Null),
            }
        }
    }

    #[test]
    fn a_split_serializes_as_a_stack_panel() {
        let mut tree = LayoutTree::new(RootKind::Split);
        tree.push_tabs_for_test(tree.root().id(), vec![PanelId::from_u64(1)]);
        tree.push_tabs_for_test(tree.root().id(), vec![PanelId::from_u64(2)]);
        tree.normalize();

        let source = FakePanels(vec![
            (PanelId::from_u64(1), "Alpha"),
            (PanelId::from_u64(2), "Beta"),
        ]);
        let state = tree.to_state(&source);

        assert_eq!(state.panel_name, "StackPanel");
        assert!(matches!(state.info, PanelInfo::Stack { .. }));
        assert_eq!(state.children[0].panel_name, "TabPanel");
        assert_eq!(state.children[0].children[0].panel_name, "Alpha");
    }

    #[test]
    fn an_empty_tab_group_serializes_as_tabs_not_as_a_panel() {
        let mut tree = LayoutTree::new(RootKind::Any);
        tree.set_root_tabs_for_test(vec![], 0);

        let state = tree.to_state(&FakePanels(vec![]));

        assert_eq!(state.panel_name, "TabPanel");
        assert!(
            matches!(state.info, PanelInfo::Tabs { active_index: 0 }),
            "the old writer emitted PanelInfo::Panel here, which failed to restore"
        );
    }

    #[test]
    fn an_empty_center_still_serializes_as_a_stack_panel() {
        let tree = LayoutTree::new(RootKind::Split);
        let state = tree.to_state(&FakePanels(vec![]));
        assert_eq!(state.panel_name, "StackPanel");
        assert!(matches!(state.info, PanelInfo::Stack { .. }));
    }

    #[test]
    fn tiles_serialize_with_their_metas_in_order() {
        let mut tree = LayoutTree::new(RootKind::Any);
        let bounds = Bounds {
            origin: point(px(5.), px(6.)),
            size: size(px(7.), px(8.)),
        };
        tree.set_root_tiles_for_test(vec![
            TilePanel::new(PanelId::from_u64(1), bounds).with_z_index(2),
        ]);

        let state = tree.to_state(&FakePanels(vec![(PanelId::from_u64(1), "Alpha")]));

        assert_eq!(state.panel_name, "Tiles");
        let PanelInfo::Tiles { metas } = state.info else {
            panic!()
        };
        assert_eq!(metas[0].bounds, bounds);
        assert_eq!(metas[0].z_index, 2);
    }
}
