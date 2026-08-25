//! The element description buffer.
//!
//! GPUI elements are values that are consumed when used: `RenderOnce::render`
//! takes `self`, and `ParentElement::child` takes its child by value. A script
//! object therefore cannot *be* an element. Instead a script builder records
//! operations into this arena, and [`crate::materialize`] replays them into real
//! elements inside `Render::render`.
//!
//! One arena is the runtime's scratch space, reset at the start of every script
//! render; a successful render freezes it into a
//! [`crate::snapshot::RenderSnapshot`] and leaves a fresh one behind. Reading it
//! is therefore non-destructive: the same description is replayed by every GPUI
//! frame that materializes the snapshot, which is what keeps repainting off the
//! VM.

use smallvec::SmallVec;

use crate::value::Bridged;

/// Index of a node inside a [`SpecArena`].
pub type SpecId = u32;

/// Index of a script callback, within the snapshot generation that registered it.
pub type CallbackId = u32;

/// Which constructor produced a node.
#[derive(Clone, Debug, PartialEq)]
pub enum Component {
    Div,
    HFlex,
    VFlex,
    Text(String),
    Button(String),
    Checkbox(String),
    Switch(String),
    /// A text input, addressed by its entity handle rather than by an id: the
    /// state is what identifies it, and the state outlives the description.
    Input(crate::entities::EntityHandle),
    /// A vector image, loaded from the application's own directory.
    Svg(String),
}

impl Component {
    pub fn name(&self) -> &'static str {
        match self {
            Component::Div => "div",
            Component::HFlex => "h_flex",
            Component::VFlex => "v_flex",
            Component::Text(_) => "text",
            Component::Button(_) => "Button",
            Component::Checkbox(_) => "Checkbox",
            Component::Switch(_) => "Switch",
            Component::Input(_) => "Input",
            Component::Svg(_) => "svg",
        }
    }
}

/// One recorded builder call.
#[derive(Clone, Debug, PartialEq)]
pub enum SpecOp {
    /// A no-argument style method, addressed by index into the reflection table.
    NullaryStyle(u16),
    /// A style method that takes arguments.
    ParamStyle(&'static str, SmallVec<[Bridged; 2]>),
    /// A component behavior method.
    Method(&'static str, SmallVec<[Bridged; 2]>),
    /// An event handler pointing into the callback arena.
    Callback(&'static str, CallbackId),
    /// A state style — hover, active, focus — whose declarations were recorded
    /// into a detached node. Reusing the ordinary style methods there is what
    /// keeps state styling from needing a second value grammar.
    StateStyle(&'static str, SpecId),
}

#[derive(Clone, Debug, Default)]
pub struct SpecNode {
    pub component: Option<Component>,
    pub ops: SmallVec<[SpecOp; 8]>,
    pub children: SmallVec<[SpecId; 4]>,
}

/// Element descriptions for one script render.
#[derive(Default)]
pub struct SpecArena {
    nodes: Vec<SpecNode>,
    /// Nodes already attached to a parent. Re-using one is an error, which is
    /// how Rust's move semantics survive the trip into a garbage-collected
    /// language.
    parented: Vec<bool>,
    /// Nodes consumed by an op rather than by a parent — a state style's
    /// declarations, for instance. They still take style ops, but they can
    /// never enter the tree.
    claimed: Vec<bool>,
}

impl SpecArena {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drops every node. Called at the start of each script render, on the
    /// runtime's scratch arena — never on a published snapshot.
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.parented.clear();
        self.claimed.clear();
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn push(&mut self, component: Component) -> SpecId {
        self.nodes.push(SpecNode {
            component: Some(component),
            ..Default::default()
        });
        self.parented.push(false);
        self.claimed.push(false);
        (self.nodes.len() - 1) as SpecId
    }

    pub fn node(&self, id: SpecId) -> Option<&SpecNode> {
        self.nodes.get(id as usize)
    }

    pub fn push_op(&mut self, id: SpecId, op: SpecOp) -> Result<(), SpecError> {
        self.check_live(id)?;
        self.nodes[id as usize].ops.push(op);
        Ok(())
    }

    /// Attaches `child` to `parent`, consuming the child.
    /// Marks a node as consumed by an op rather than by a parent, so it cannot
    /// also be added to the tree.
    pub fn claim(&mut self, id: SpecId) -> Result<(), SpecError> {
        self.check_live(id)?;
        self.claimed[id as usize] = true;
        Ok(())
    }

    pub fn attach(&mut self, parent: SpecId, child: SpecId) -> Result<(), SpecError> {
        self.check_live(parent)?;
        self.check_live(child)?;
        if self.claimed[child as usize] {
            return Err(SpecError::Claimed);
        }
        if parent == child {
            return Err(SpecError::SelfParent);
        }
        self.parented[child as usize] = true;
        self.nodes[parent as usize].children.push(child);
        Ok(())
    }

    fn check_live(&self, id: SpecId) -> Result<(), SpecError> {
        let index = id as usize;
        if index >= self.nodes.len() || self.nodes[index].component.is_none() {
            return Err(SpecError::Expired);
        }
        if self.parented[index] {
            return Err(SpecError::AlreadyParented {
                component: self.nodes[index]
                    .component
                    .as_ref()
                    .map(Component::name)
                    .unwrap_or("element"),
            });
        }
        Ok(())
    }

    /// A stable textual dump, used by snapshot tests. Rendering never needs a
    /// GPU to be verified: the description is plain data.
    pub fn debug_tree(&self, root: SpecId) -> String {
        let mut out = String::new();
        self.write_tree(root, 0, &mut out);
        out
    }

    fn write_tree(&self, id: SpecId, depth: usize, out: &mut String) {
        let Some(node) = self.node(id) else {
            return;
        };
        let Some(component) = node.component.as_ref() else {
            return;
        };
        out.push_str(&"  ".repeat(depth));
        out.push_str(component.name());
        match component {
            Component::Text(value)
            | Component::Button(value)
            | Component::Checkbox(value)
            | Component::Switch(value)
            | Component::Svg(value) => out.push_str(&format!(" {value:?}")),
            Component::Input(handle) => out.push_str(&format!(" #{handle}")),
            _ => {}
        }
        for op in &node.ops {
            match op {
                SpecOp::NullaryStyle(index) => {
                    out.push_str(&format!(" .{}", crate::style::nullary_name(*index)))
                }
                SpecOp::ParamStyle(name, args) => out.push_str(&format!(" .{name}{args:?}")),
                SpecOp::Method(name, args) => out.push_str(&format!(" :{name}{args:?}")),
                SpecOp::Callback(name, _) => out.push_str(&format!(" :{name}(fn)")),
                SpecOp::StateStyle(name, node) => {
                    out.push_str(&format!(" :{name}("));
                    match self.node(*node) {
                        Some(state) => {
                            for op in &state.ops {
                                match op {
                                    SpecOp::NullaryStyle(index) => out.push_str(&format!(
                                        ".{}",
                                        crate::style::nullary_name(*index)
                                    )),
                                    SpecOp::ParamStyle(name, args) => {
                                        out.push_str(&format!(".{name}{args:?}"))
                                    }
                                    _ => {}
                                }
                            }
                        }
                        None => out.push_str("?"),
                    }
                    out.push(')');
                }
            }
        }
        out.push('\n');
        for child in &node.children {
            self.write_tree(*child, depth + 1, out);
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum SpecError {
    /// The node holds a state style's declarations and cannot enter the tree.
    Claimed,
    /// The node belongs to a previous render pass.
    Expired,
    /// The node was already added to a parent.
    AlreadyParented { component: &'static str },
    /// An element was added to itself.
    SelfParent,
}

impl std::fmt::Display for SpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecError::Expired => f.write_str(
                "this element belongs to a previous render pass; elements are single-use values \
                 and must be rebuilt each time render runs",
            ),
            SpecError::AlreadyParented { component } => write!(
                f,
                "element `{component}` was already added to a parent; elements are single-use values"
            ),
            SpecError::Claimed => f.write_str(
                "this element holds the declarations of a state style and cannot be added \
                 to the tree",
            ),
            SpecError::SelfParent => f.write_str("an element cannot be added to itself"),
        }
    }
}

impl std::error::Error for SpecError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attaching_an_element_twice_is_an_error() {
        let mut arena = SpecArena::new();
        let parent = arena.push(Component::Div);
        let other_parent = arena.push(Component::Div);
        let child = arena.push(Component::Text("hi".into()));

        arena.attach(parent, child).unwrap();
        let error = arena.attach(other_parent, child).unwrap_err();

        assert!(matches!(error, SpecError::AlreadyParented { .. }));
    }

    #[test]
    fn a_parented_element_can_no_longer_take_ops() {
        let mut arena = SpecArena::new();
        let parent = arena.push(Component::Div);
        let child = arena.push(Component::Div);
        arena.attach(parent, child).unwrap();

        assert_eq!(
            arena.push_op(child, SpecOp::NullaryStyle(0)).unwrap_err(),
            SpecError::AlreadyParented { component: "div" }
        );
    }

    #[test]
    fn a_claimed_node_still_takes_styles_but_cannot_be_attached() {
        let mut arena = SpecArena::new();
        let parent = arena.push(Component::Div);
        let state = arena.push(Component::Div);
        arena.claim(state).unwrap();

        assert!(arena.push_op(state, SpecOp::NullaryStyle(0)).is_ok());
        assert_eq!(arena.attach(parent, state).unwrap_err(), SpecError::Claimed);
    }

    #[test]
    fn reset_expires_every_node() {
        let mut arena = SpecArena::new();
        let node = arena.push(Component::Div);
        arena.reset();

        assert_eq!(
            arena.push_op(node, SpecOp::NullaryStyle(0)).unwrap_err(),
            SpecError::Expired
        );
    }

    #[test]
    fn debug_tree_renders_structure_without_a_gpu() {
        let mut arena = SpecArena::new();
        let root = arena.push(Component::VFlex);
        let label = arena.push(Component::Text("Save".into()));
        arena.attach(root, label).unwrap();

        assert_eq!(arena.debug_tree(root), "v_flex\n  text \"Save\"\n");
    }
}
