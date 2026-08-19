mod active;
mod dock_area;
mod dock_placement;
mod drag;
pub mod layout;
mod panel;
mod registry;
mod state;
mod state_convert;
mod tab_group;
#[cfg(test)]
pub(crate) mod test_support;
mod tiles_geometry;
mod tiles_state;

pub use dock_area::{DockArea, DockAreaRenderer, DockContext, DockEvent};
pub use dock_placement::{Dock, DockSizing};
pub use drag::{
    AnyDrag, DragPanel, DropIndicator, DropPlaceholderBounds, DropTarget, split_placement_at,
};
pub use layout::{
    DockLayout, EditResult, InsertTarget, LayoutNode, LayoutTree, NodeId, NodeRef, PanelId,
    RootKind, TilePanel,
};
pub use panel::{Panel, PanelEvent, PanelView};
pub use registry::{PanelBuildContext, PanelRegistry, register_panel};
pub use state::{DockAreaState, DockPlacement, DockState, PanelInfo, PanelState, TileMeta};
pub use state_convert::PanelSource;
pub use tab_group::{
    TabGroup, TabGroupConstraints, TabGroupContext, TabGroupEvent, TabGroupRenderer,
};
pub use tiles_geometry::{
    DRAG_BAR_HEIGHT, HANDLE_SIZE, MINIMUM_SIZE, ResizeDrag, ResizeSide, TileChange,
    apply_boundary_constraints, compute_resized_bounds, content_size, magnetic_snap,
    round_point_to_grid, round_to_grid, snap_edge,
};
pub use tiles_state::{TileContext, TilesEvent, TilesRenderer, TilesState};
