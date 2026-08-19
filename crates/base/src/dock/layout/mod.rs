mod node;
mod normalize;
mod tree;

// `NodeKind` and `NodePath` stay crate-private here for the edit operations
// added in a later task; nothing in this crate reaches them through this
// re-export yet. (`normalize.rs` imports both directly from their owning
// submodules instead of through here.)
#[allow(unused_imports)]
pub(crate) use node::NodeKind;
pub use node::{LayoutNode, NodeId, NodeRef, PanelId, TilePanel};
#[allow(unused_imports)]
pub(crate) use tree::NodePath;
pub use tree::{LayoutTree, RootKind};
