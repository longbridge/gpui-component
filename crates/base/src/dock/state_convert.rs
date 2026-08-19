use gpui::{Axis, Pixels};

use super::layout::{LayoutNode, LayoutTree, NodeKind, NodeRef, PanelId, RootKind, TilePanel};
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
            // `None` is a representation this rewrite introduces, not
            // something the old writer ever produced: the schema's `sizes`
            // field is `Vec<Pixels>`, with no slot for "unconstrained". `0.0`
            // is the sentinel this writer chooses for that case, and the
            // corresponding reader maps a `0.0` it loads back to `None`.
            //
            // That makes a `None` slot safe to persist only transiently: a
            // caller building a tree meant to be written out must resolve
            // every slot to concrete pixels first. An older build reading a
            // persisted `0.0` back has no notion of the sentinel and
            // constructs a real, zero-pixel-wide panel from it.
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

/// Turns a persisted leaf into a live panel id.
///
/// The production implementation (at the `gpui-component` layer, above this
/// crate) consults `PanelRegistry` and falls back to an invalid-panel
/// placeholder that retains the original `PanelState`, so a panel type this
/// build does not know about survives a load/save round trip instead of
/// being erased.
pub trait PanelBuilder {
    fn build(&mut self, state: &PanelState, info: &PanelInfo) -> PanelId;
}

impl LayoutTree {
    /// Read a persisted layout.
    ///
    /// Compatibility rules, all previously implicit in `PanelState::to_item`:
    ///
    /// - a `Tabs` whose children are themselves `Tabs` is flattened;
    /// - a bare `Panel` leaf appearing where a container belongs is wrapped in
    ///   a `Tabs`;
    /// - a node named `TabPanel` carrying `PanelInfo::Panel` is read as an
    ///   empty tab group, recovering data written by the old dump defect (an
    ///   empty `TabPanel` never entered the loop that set its `info` to
    ///   `Tabs`, so it kept `PanelState`'s default `Panel(Value::Null)`). The
    ///   old reader had no such rule: it looked "TabPanel" up in the panel
    ///   registry, found nothing, and rendered an `InvalidPanel` placeholder
    ///   where an empty tab group belonged. This rule is a genuine fix, not a
    ///   preserved behavior;
    /// - a `Tiles` child without a matching meta keeps the default placement.
    ///   The old writer's counterpart (`DockItem::tiles`) hard-asserted
    ///   `items.len() == metas.len()` and panicked the whole load on a short
    ///   `metas` list, so this rule is a new safety net, not a preserved
    ///   graceful-degradation path.
    pub fn from_state(
        state: &PanelState,
        root_kind: RootKind,
        builder: &mut dyn PanelBuilder,
    ) -> Self {
        let mut tree = LayoutTree::new(root_kind);
        let root = build_node(&mut tree, state, builder);

        let root = match (root_kind, &root) {
            (RootKind::Split, node) if !matches!(node.kind_ref(), NodeKind::Split { .. }) => {
                let id = tree.allocate_node_id();
                LayoutNode::new(
                    id,
                    NodeKind::Split {
                        axis: Axis::Horizontal,
                        children: vec![node.clone()],
                        sizes: vec![None],
                    },
                )
            }
            _ => root,
        };

        tree.replace_root(root);
        tree.normalize();
        tree
    }
}

fn build_node(
    tree: &mut LayoutTree,
    state: &PanelState,
    builder: &mut dyn PanelBuilder,
) -> LayoutNode {
    let id = tree.allocate_node_id();

    match &state.info {
        PanelInfo::Stack { sizes, axis } => {
            let axis = if *axis == 0 {
                Axis::Horizontal
            } else {
                Axis::Vertical
            };
            let children: Vec<LayoutNode> = state
                .children
                .iter()
                .map(|child| build_node(tree, child, builder))
                .collect();
            let sizes = (0..children.len())
                .map(|ix| sizes.get(ix).copied().filter(|size| *size > Pixels::ZERO))
                .collect();
            LayoutNode::new(
                id,
                NodeKind::Split {
                    axis,
                    children,
                    sizes,
                },
            )
        }
        PanelInfo::Tabs { active_index } => {
            let panels = collect_tab_panels(&state.children, builder);
            LayoutNode::new(
                id,
                NodeKind::Tabs {
                    panels,
                    active_ix: *active_index,
                },
            )
        }
        PanelInfo::Tiles { metas } => {
            let panels = state
                .children
                .iter()
                .enumerate()
                .map(|(ix, child)| {
                    let meta = metas.get(ix).copied().unwrap_or_default();
                    TilePanel::new(builder.build(child, &child.info), meta.bounds)
                        .with_z_index(meta.z_index)
                })
                .collect();
            LayoutNode::new(id, NodeKind::Tiles { panels })
        }
        PanelInfo::Panel(_) => {
            // A container name carrying a leaf info means the writer that
            // produced this file had the empty-group defect.
            let panels = if state.panel_name == TAB_PANEL_NAME {
                Vec::new()
            } else {
                vec![builder.build(state, &state.info)]
            };
            LayoutNode::new(
                id,
                NodeKind::Tabs {
                    panels,
                    active_ix: 0,
                },
            )
        }
    }
}

/// Flatten one level of tab nesting, which the old writer could produce.
fn collect_tab_panels(children: &[PanelState], builder: &mut dyn PanelBuilder) -> Vec<PanelId> {
    children
        .iter()
        .flat_map(|child| match &child.info {
            PanelInfo::Tabs { .. } => collect_tab_panels(&child.children, builder),
            PanelInfo::Panel(_) if child.panel_name == TAB_PANEL_NAME => Vec::new(),
            _ => vec![builder.build(child, &child.info)],
        })
        .collect()
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
        let PanelInfo::Stack { sizes, .. } = &state.info else {
            panic!("expected Stack info");
        };
        // Both slots were pushed with no explicit size (`push_tabs_for_test`
        // always pushes `None`), so both serialize as the zero sentinel.
        assert_eq!(sizes, &vec![px(0.), px(0.)]);
        assert_eq!(state.children[0].panel_name, "TabPanel");
        assert_eq!(state.children[0].children[0].panel_name, "Alpha");
    }

    #[test]
    fn an_unresolved_slot_size_serializes_as_the_zero_sentinel() {
        let mut tree = LayoutTree::new(RootKind::Split);
        let root = tree.root().id();
        tree.push_sized_tabs_for_test(root, vec![PanelId::from_u64(1)], Some(px(120.)));
        tree.push_sized_tabs_for_test(root, vec![PanelId::from_u64(2)], None);
        tree.normalize();

        let source = FakePanels(vec![
            (PanelId::from_u64(1), "Alpha"),
            (PanelId::from_u64(2), "Beta"),
        ]);
        let state = tree.to_state(&source);

        let PanelInfo::Stack { sizes, .. } = state.info else {
            panic!("expected Stack info");
        };
        // The `Some` slot keeps its concrete value; only the genuinely
        // unresolved (`None`) slot is written as the `0.0` sentinel.
        assert_eq!(sizes, vec![px(120.), px(0.)]);
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

    /// Assigns each leaf `PanelState` an id in encounter order, so the reader
    /// can be tested without a registry or an `App`.
    #[derive(Default)]
    struct RecordingBuilder {
        built: Vec<String>,
    }

    impl PanelBuilder for RecordingBuilder {
        fn build(&mut self, state: &PanelState, _: &PanelInfo) -> PanelId {
            self.built.push(state.panel_name.clone());
            PanelId::from_u64(self.built.len() as u64)
        }
    }

    fn tabs_state(children: Vec<PanelState>, active_index: usize) -> PanelState {
        PanelState {
            panel_name: TAB_PANEL_NAME.to_string(),
            children,
            info: PanelInfo::tabs(active_index),
        }
    }

    fn panel_state(name: &str) -> PanelState {
        PanelState {
            panel_name: name.to_string(),
            children: Vec::new(),
            info: PanelInfo::panel(serde_json::Value::Null),
        }
    }

    #[test]
    fn nested_tab_groups_are_flattened() {
        let state = tabs_state(
            vec![
                tabs_state(vec![panel_state("Alpha")], 0),
                tabs_state(vec![panel_state("Beta")], 0),
            ],
            1,
        );

        let mut builder = RecordingBuilder::default();
        let tree = LayoutTree::from_state(&state, RootKind::Any, &mut builder);

        assert_eq!(builder.built, vec!["Alpha", "Beta"]);
        let NodeRef::Tabs { panels, active_ix } = tree.root().kind() else {
            panic!()
        };
        assert_eq!(panels.len(), 2);
        assert_eq!(active_ix, 1);
    }

    #[test]
    fn a_bare_panel_leaf_is_wrapped_in_a_tab_group() {
        let mut builder = RecordingBuilder::default();
        let tree = LayoutTree::from_state(&panel_state("Alpha"), RootKind::Any, &mut builder);

        assert!(matches!(tree.root().kind(), NodeRef::Tabs { panels, .. } if panels.len() == 1));
    }

    #[test]
    fn a_tab_panel_carrying_panel_info_is_read_as_an_empty_group() {
        // What the old `TabPanel::dump` wrote for an empty tab group.
        let state = PanelState {
            panel_name: TAB_PANEL_NAME.to_string(),
            children: Vec::new(),
            info: PanelInfo::panel(serde_json::Value::Null),
        };

        let mut builder = RecordingBuilder::default();
        let tree = LayoutTree::from_state(&state, RootKind::Any, &mut builder);

        assert!(
            builder.built.is_empty(),
            "no panel is built for the phantom leaf"
        );
        assert!(matches!(tree.root().kind(), NodeRef::Tabs { panels, .. } if panels.is_empty()));
    }

    #[test]
    fn a_split_root_is_forced_even_when_the_state_is_a_tab_group() {
        let state = tabs_state(vec![panel_state("Alpha")], 0);
        let mut builder = RecordingBuilder::default();
        let tree = LayoutTree::from_state(&state, RootKind::Split, &mut builder);

        assert!(matches!(tree.root().kind(), NodeRef::Split { .. }));
    }

    #[test]
    fn tile_metas_are_paired_with_children_by_index() {
        let bounds = Bounds {
            origin: point(px(1.), px(2.)),
            size: size(px(3.), px(4.)),
        };
        let state = PanelState {
            panel_name: TILES_PANEL_NAME.to_string(),
            children: vec![panel_state("Alpha")],
            info: PanelInfo::tiles(vec![TileMeta { bounds, z_index: 5 }]),
        };

        let mut builder = RecordingBuilder::default();
        let tree = LayoutTree::from_state(&state, RootKind::Any, &mut builder);

        let NodeRef::Tiles { panels } = tree.root().kind() else {
            panic!()
        };
        assert_eq!(panels[0].bounds(), bounds);
        assert_eq!(panels[0].z_index(), 5);
    }

    #[test]
    fn a_tile_child_missing_its_meta_falls_back_to_the_default_placement() {
        let state = PanelState {
            panel_name: TILES_PANEL_NAME.to_string(),
            children: vec![panel_state("Alpha"), panel_state("Beta")],
            info: PanelInfo::tiles(vec![TileMeta::default()]),
        };

        let mut builder = RecordingBuilder::default();
        let tree = LayoutTree::from_state(&state, RootKind::Any, &mut builder);

        let NodeRef::Tiles { panels } = tree.root().kind() else {
            panic!()
        };
        assert_eq!(panels.len(), 2, "a short metas list must not drop panels");
    }
}
