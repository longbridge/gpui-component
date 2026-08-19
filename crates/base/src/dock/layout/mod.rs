mod builder;
mod edit;
mod node;
mod normalize;
mod tree;

pub use builder::DockLayout;
pub use edit::{EditResult, InsertTarget};

// `NodeKind` and `NodePath` stay crate-private: `dock_area` reads through
// `NodeKind` to resolve slot sizes before a dump, but nothing outside this
// crate may build a node without going through `LayoutTree`, which is what
// guarantees normalization always runs.
pub(crate) use node::NodeKind;
pub use node::{LayoutNode, NodeId, NodeRef, PanelId, TilePanel};
#[allow(unused_imports)]
pub(crate) use tree::NodePath;
pub use tree::{LayoutTree, RootKind};
