mod edit;
mod node;
mod normalize;
mod tree;

pub use edit::{EditResult, InsertTarget};

// `NodeKind` and `NodePath` stay crate-private here for a later task; the
// edit operations added in this task reach `node`/`tree` directly as
// `super::node::NodeKind` / `super::tree::NodePath` instead of through this
// re-export, so it still has no caller.
#[allow(unused_imports)]
pub(crate) use node::NodeKind;
pub use node::{LayoutNode, NodeId, NodeRef, PanelId, TilePanel};
#[allow(unused_imports)]
pub(crate) use tree::NodePath;
pub use tree::{LayoutTree, RootKind};
