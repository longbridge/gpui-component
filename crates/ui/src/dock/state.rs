use gpui::{AppContext, Axis, Pixels, View, VisualContext as _, WeakView, WindowContext};
use itertools::Itertools as _;
use serde::{Deserialize, Serialize};

use super::{
    invalid_panel::InvalidPanel, Dock, DockArea, DockItem, DockPlacement, Panel, PanelRegistry,
};

/// Used to serialize and deserialize the DockArea
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct DockAreaState {
    /// The version is used to mark this persisted state is compatible with the current version
    /// For example, some times we many totally changed the structure of the Panel,
    /// then we can compare the version to decide whether we can use the state or ignore.
    #[serde(default)]
    pub version: Option<usize>,
    pub center: DockItemState,
    pub left_dock: Option<DockState>,
    pub right_dock: Option<DockState>,
    pub bottom_dock: Option<DockState>,
}

/// Used to serialize and deserialize the Dock
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DockState {
    panel: DockItemState,
    placement: DockPlacement,
    size: Pixels,
    open: bool,
}

impl DockState {
    pub fn new(dock: View<Dock>, cx: &AppContext) -> Self {
        let dock = dock.read(cx);

        Self {
            placement: dock.placement,
            size: dock.size,
            open: dock.open,
            panel: dock.panel.view().dump(cx),
        }
    }

    /// Convert the DockState to Dock
    pub fn to_dock(&self, dock_area: WeakView<DockArea>, cx: &mut WindowContext) -> View<Dock> {
        let item = self.panel.to_item(dock_area.clone(), cx);
        cx.new_view(|cx| {
            Dock::from_state(
                dock_area.clone(),
                self.placement,
                self.size,
                item,
                self.open,
                cx,
            )
        })
    }
}

/// Used to serialize and deserialize the DockerItem
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DockItemState {
    pub panel_name: String,
    pub children: Vec<DockItemState>,
    pub info: DockItemInfo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DockItemInfo {
    #[serde(rename = "stack")]
    Stack {
        sizes: Vec<Pixels>,
        /// The axis of the stack, 0 is horizontal, 1 is vertical
        axis: usize,
    },
    #[serde(rename = "tabs")]
    Tabs { active_index: usize },
    #[serde(rename = "panel")]
    Panel(serde_json::Value),
}

impl DockItemInfo {
    pub fn stack(sizes: Vec<Pixels>, axis: Axis) -> Self {
        Self::Stack {
            sizes,
            axis: if axis == Axis::Horizontal { 0 } else { 1 },
        }
    }

    pub fn tabs(active_index: usize) -> Self {
        Self::Tabs { active_index }
    }

    pub fn panel(value: serde_json::Value) -> Self {
        Self::Panel(value)
    }

    pub fn axis(&self) -> Option<Axis> {
        match self {
            Self::Stack { axis, .. } => Some(if *axis == 0 {
                Axis::Horizontal
            } else {
                Axis::Vertical
            }),
            _ => None,
        }
    }

    pub fn sizes(&self) -> Option<&Vec<Pixels>> {
        match self {
            Self::Stack { sizes, .. } => Some(sizes),
            _ => None,
        }
    }

    pub fn active_index(&self) -> Option<usize> {
        match self {
            Self::Tabs { active_index } => Some(*active_index),
            _ => None,
        }
    }
}

impl Default for DockItemState {
    fn default() -> Self {
        Self {
            panel_name: "".to_string(),
            children: Vec::new(),
            info: DockItemInfo::Panel(serde_json::Value::Null),
        }
    }
}

impl DockItemState {
    pub fn new<P: Panel>(panel: &P) -> Self {
        Self {
            panel_name: panel.panel_name().to_string(),
            ..Default::default()
        }
    }

    pub fn add_child(&mut self, panel: DockItemState) {
        self.children.push(panel);
    }

    pub fn to_item(&self, dock_area: WeakView<DockArea>, cx: &mut WindowContext) -> DockItem {
        let info = self.info.clone();

        let items: Vec<DockItem> = self
            .children
            .iter()
            .map(|child| child.to_item(dock_area.clone(), cx))
            .collect();

        match info {
            DockItemInfo::Stack { sizes, axis } => {
                let axis = if axis == 0 {
                    Axis::Horizontal
                } else {
                    Axis::Vertical
                };
                let sizes = sizes.iter().map(|s| Some(*s)).collect_vec();
                DockItem::split_with_sizes(axis, items, sizes, &dock_area, cx)
            }
            DockItemInfo::Tabs { active_index } => {
                if items.len() == 1 {
                    return items[0].clone();
                }

                let items = items
                    .iter()
                    .flat_map(|item| match item {
                        DockItem::Tabs { items, .. } => items.clone(),
                        _ => {
                            unreachable!("Invalid DockItem type in DockItemInfo::Tabs")
                        }
                    })
                    .collect_vec();

                DockItem::tabs(items, Some(active_index), &dock_area, cx)
            }
            DockItemInfo::Panel(_) => {
                let view = if let Some(f) = cx
                    .global::<PanelRegistry>()
                    .items
                    .get(&self.panel_name)
                    .cloned()
                {
                    f(dock_area.clone(), self, &info, cx)
                } else {
                    // Show an invalid panel if the panel is not registered.
                    Box::new(
                        cx.new_view(|cx| InvalidPanel::new(&self.panel_name, info.clone(), cx)),
                    )
                };

                DockItem::tabs(vec![view.into()], None, &dock_area, cx)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::*;
    #[test]
    fn test_deserialize_item_state() {
        let json = include_str!("../../tests/fixtures/layout.json");
        let state: DockAreaState = serde_json::from_str(json).unwrap();
        assert_eq!(state.version, None);
        assert_eq!(state.center.panel_name, "StackPanel");
        assert_eq!(state.center.children.len(), 2);
        assert_eq!(state.center.children[0].panel_name, "TabPanel");
        assert_eq!(state.center.children[1].children.len(), 1);
        assert_eq!(
            state.center.children[1].children[0].panel_name,
            "StoryContainer"
        );
        assert_eq!(state.center.children[1].panel_name, "TabPanel");

        let left_dock = state.left_dock.unwrap();
        assert_eq!(left_dock.open, true);
        assert_eq!(left_dock.size, px(350.0));
        assert_eq!(left_dock.placement, DockPlacement::Left);
        assert_eq!(left_dock.panel.panel_name, "TabPanel");
        assert_eq!(left_dock.panel.children.len(), 1);
        assert_eq!(left_dock.panel.children[0].panel_name, "StoryContainer");

        let bottom_dock = state.bottom_dock.unwrap();
        assert_eq!(bottom_dock.open, true);
        assert_eq!(bottom_dock.size, px(200.0));
        assert_eq!(bottom_dock.panel.panel_name, "TabPanel");
        assert_eq!(bottom_dock.panel.children.len(), 2);
        assert_eq!(bottom_dock.panel.children[0].panel_name, "StoryContainer");

        let right_dock = state.right_dock.unwrap();
        assert_eq!(right_dock.open, true);
        assert_eq!(right_dock.size, px(320.0));
        assert_eq!(right_dock.panel.panel_name, "TabPanel");
        assert_eq!(right_dock.panel.children.len(), 1);
        assert_eq!(right_dock.panel.children[0].panel_name, "StoryContainer");
    }
}
