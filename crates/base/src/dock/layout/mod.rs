mod node;
mod normalize;
mod tree;

// `NodeKind` and `NodePath` stay crate-private here for sibling modules
// (normalization, edit operations) added in later tasks; nothing in this
// task's code reaches them through this re-export yet.
#[allow(unused_imports)]
pub(crate) use node::NodeKind;
pub use node::{LayoutNode, NodeId, NodeRef, PanelId, TilePanel};
#[allow(unused_imports)]
pub(crate) use tree::NodePath;
pub use tree::{LayoutTree, RootKind};
