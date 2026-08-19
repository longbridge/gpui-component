mod active;
mod dock_area;
mod drag;
pub mod layout;
mod panel;
mod registry;
mod state;
mod state_convert;
mod tab_group;

pub use dock_area::DockArea;
pub use drag::{
    AnyDrag, DragPanel, DropIndicator, DropPlaceholderBounds, DropTarget, split_placement_at,
};
pub use layout::{LayoutNode, LayoutTree, NodeId, NodeRef, PanelId, RootKind, TilePanel};
pub use panel::{Panel, PanelEvent, PanelView};
pub use registry::{PanelBuildContext, PanelRegistry, register_panel};
pub use state::{DockAreaState, DockPlacement, DockState, PanelInfo, PanelState, TileMeta};
pub use state_convert::PanelSource;
pub use tab_group::TabGroup;
