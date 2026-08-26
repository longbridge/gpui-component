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

use std::{collections::HashSet, rc::Rc};

use smallvec::SmallVec;

use crate::value::Bridged;

/// Index of a node inside a [`SpecArena`].
pub type SpecId = u32;

/// Runtime-unique identifier for a script callback.
pub type CallbackId = u64;

/// Retained description of GPUI's reusable `Background` value.
#[derive(Clone, Debug, PartialEq)]
pub struct BackgroundSpec {
    pub kind: BackgroundKind,
    pub opacity: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BackgroundKind {
    Solid {
        color: String,
    },
    LinearGradient {
        angle: f32,
        from: (String, f32),
        to: (String, f32),
        color_space: String,
    },
    PatternSlash {
        color: String,
        width: f32,
        interval: f32,
    },
    Checkerboard {
        color: String,
        size: f32,
    },
}

/// Which constructor produced a node.
#[derive(Clone, Debug, PartialEq)]
pub enum Component {
    Div,
    HFlex,
    VFlex,
    /// A retained nested script view. The frozen description keeps the entity
    /// itself alive, so releasing the numeric handle cannot invalidate a frame
    /// that was already published.
    ChildView(ChildViewSpec),
    Text(String),
    Button(String),
    Link(String),
    Checkbox(String),
    Switch(String),
    /// A scrollbar the script places itself, driving the scroll area carrying
    /// the same id. Pairing by name is the whole of the wiring — the two share
    /// one `ScrollHandle` in window element state — which is what lets a bar
    /// sit beside a fixed header, span two panes, or scroll a list that paints
    /// no bar of its own.
    Scrollbar(String),
    /// A text input, addressed by its entity handle rather than by an id: the
    /// state is what identifies it, and the state outlives the description.
    Input(crate::entities::EntityHandle),
    /// Multi-line text, addressed by its entity handle for the same reason as
    /// [`Component::Input`]. A separate variant rather than a flag on that one,
    /// because `TextareaState` is a different Rust type: the store cannot hand
    /// out one where the other was asked for, and `Textarea::new` will not take
    /// an `InputState`.
    Textarea(crate::entities::EntityHandle),

    /// A spinbutton frame over the same `InputState` a [`Component::Input`]
    /// holds: there is no numeric state type, only a text state carrying a
    /// step, a range and a numeric mask. So the handle identifies it exactly as
    /// it identifies an input, and the element carries only what a step button
    /// looks like and what happens when one is pressed.
    NumberInput(crate::entities::EntityHandle),
    /// A fixed-length one-time code, addressed by its entity handle exactly as
    /// [`Component::Input`] is. Unlike every other bound component, its cells
    /// are not described by the script: base draws none, and a described cell
    /// would be frozen at the digit the last script render saw. See
    /// `materialize::components::otp_input`.
    OtpInput(crate::entities::EntityHandle),
    /// A vector image, loaded from the application's own directory.
    Svg(String),
    /// A full-color image, loaded from the application's own directory.
    Image(String),
    /// A native GPUI path. Its geometry is retained in method operations and
    /// resolved against final element bounds during prepaint.
    Path {
        fill: bool,
        background: BackgroundSpec,
        stroke_width: f32,
    },
    /// A tab list. It holds no selection of its own: each `Tab` is told
    /// whether it is selected and reports activation through `on_click`.
    Tabs(String),
    /// One tab in a [`Component::Tabs`].
    Tab(String),
    /// A progress root. It carries the progress role and the announced
    /// `0..=100` value, and draws nothing: the visible bar is a
    /// [`Component::ProgressTrack`] holding a [`Component::ProgressIndicator`],
    /// both styled by the script.
    Progress(String),
    /// The groove of a progress bar. A plain element with no semantics of its
    /// own — the announcement lives on the [`Component::Progress`] around it.
    ProgressTrack,
    /// The filled part of a progress bar, sized by the script from the same
    /// number it gave [`Component::Progress`].
    ProgressIndicator,
    /// The native per-window performance HUD supplied by `gpui-fps`.
    FpsMonitor,
    /// A slider's behavior root, addressed by its entity handle for the same
    /// reason [`Component::Input`] is: the state is what identifies it, and it
    /// outlives the description. It draws nothing at all — the three parts
    /// below are the whole of what is on screen — and announces the value.
    Slider(crate::entities::EntityHandle),
    /// The press and drag surface of a [`Component::Slider`].
    SliderTrack(crate::entities::EntityHandle),
    /// The groove, and the one part that records the box every pointer
    /// position is measured against. A slider without one cannot be moved.
    SliderIndicator(crate::entities::EntityHandle),
    /// The knob. Its position along the axis is read from the state while it
    /// is materialized, never described by the script.
    SliderThumb(crate::entities::EntityHandle),
    /// One option in a radio group. It reports only *becoming* checked: base
    /// drops the handler once the radio is checked or disabled, because a
    /// radio cannot deselect itself.
    Radio(String),
    /// A button that stays down. Controlled through `pressed`.
    Toggle(String),
    /// A set of radios. It holds no selection of its own: each radio is told
    /// whether it is checked and reports the change through `on_change`.
    RadioGroup(String),
    /// A set of toggles, announced as a toolbar. Like [`Component::RadioGroup`]
    /// it holds no state; each toggle carries its own.
    ToggleGroup(String),
    /// A semantic table root. It has no data source and no delegate: the
    /// script nests the groups, rows and cells itself, exactly as HTML does.
    Table(String),
    /// The header row group of a [`Component::Table`].
    TableHeader(String),
    /// The body row group of a [`Component::Table`].
    TableBody(String),
    /// One row, carrying the one-based index it occupies in the whole table so
    /// a screen reader can place it even when only a window of rows is drawn.
    TableRow(String, usize),
    /// One column header, carrying its one-based column index.
    TableHead(String, usize),
    /// One data cell, carrying its one-based column index.
    TableCell(String, usize),
    /// The slot a caption belongs in. It carries no caption role today, so it
    /// says where a caption goes rather than what one means.
    TableCaption(String),
    /// A row or column of panes a user drags apart, carrying the axis its
    /// constructor chose. The axis is part of what the node *is*: base decides
    /// it in `h_resizable` / `v_resizable` and every panel inside reads it, so
    /// there is no builder to change it afterwards.
    Resizable(String, gpui::Axis),
    /// One pane of a [`Component::Resizable`]. It has no id of its own: base
    /// numbers the panels by their position in the group, which is also how the
    /// group's stored sizes are addressed.
    ResizablePanel,
    /// A region whose `content` slot is rendered only while it is open.
    /// Ordinary children are always rendered; the gate applies to the slot
    /// alone.
    Collapsible,
    /// A click-driven anchored surface with a `trigger` slot and a `content`
    /// slot. Controlled: the script holds the open state and is told when the
    /// pointer changed it.
    Popover(String),
    /// A hover-driven anchored surface with the same two slots. It owns its own
    /// open state, so there is nothing to control — only how long the pointer
    /// has to rest before it appears and after it leaves.
    HoverCard(String),
    /// The bare anchored surface underneath [`Component::Popover`]: trigger
    /// measurement, corner arithmetic, deferred paint above the rest of the
    /// window, and window-edge snapping. It holds no open state at all — it
    /// shows whatever is in its `content` slot, so a script opens and closes it
    /// by filling that slot or leaving it empty.
    Popup(String),
    /// A combobox root. It holds no options and no value: it owns the combobox
    /// role, the controlled `open` state, and the transfer of the keyboard
    /// between the trigger and the popup content.
    Select(String),
    /// The same root, announced and keyed as a combobox whose trigger is an
    /// editable field. Base forwards it to `Select` verbatim.
    Combobox(String),
    /// A date-picker root, carrying the focus handle its trigger takes the
    /// keyboard through. The handle is a constructor argument because base's
    /// `DatePicker::new` requires it: a picker without one has no trigger the
    /// keyboard can reach. It holds no date — the calendar does.
    DatePicker(String, crate::entities::EntityHandle),
    /// A virtualized list: the one component whose description is not the whole
    /// of what it draws. Its rows come from a callback GPUI runs during layout,
    /// so this node carries only the list itself. See [`VirtualListSpec`] and
    /// the exception recorded in [`crate::materialize`].
    VirtualList(Rc<VirtualListSpec>),
}

/// The retained entity mounted by one `child_view(handle)` description.
///
/// Equality and diagnostics use the runtime-unique handle. The entity is the
/// frame lease: materialization never looks the handle up again.
#[derive(Clone)]
pub struct ChildViewSpec {
    handle: crate::entities::EntityHandle,
    view: gpui::Entity<crate::view::ScriptView>,
}

impl ChildViewSpec {
    pub(crate) fn new(
        handle: crate::entities::EntityHandle,
        view: gpui::Entity<crate::view::ScriptView>,
    ) -> Self {
        Self { handle, view }
    }

    pub(crate) fn handle(&self) -> crate::entities::EntityHandle {
        self.handle
    }

    pub(crate) fn view(&self) -> &gpui::Entity<crate::view::ScriptView> {
        &self.view
    }
}

impl std::fmt::Debug for ChildViewSpec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChildViewSpec")
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ChildViewSpec {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

/// What a virtualized list is, beyond its styles.
///
/// Behind an [`Rc`] in [`Component::VirtualList`] because a component is cloned
/// once per node per frame and this is the only variant carrying a vector.
///
/// That vector is the reason the script API is not a literal mirror of
/// `v_virtual_list`. Base wants one `Size` per item, and the *length of that
/// vector is the item count* — so a hundred-thousand-row list would mean a
/// hundred thousand numbers crossing the language boundary on every script
/// render. The script gives a count and either one size or one per item
/// instead, and the vector base wants is built here, once, while the
/// description is being recorded rather than once per frame.
#[derive(Clone, Debug, PartialEq)]
pub struct VirtualListSpec {
    id: String,
    axis: gpui::Axis,
    sizes: Rc<Vec<gpui::Size<gpui::Pixels>>>,
    get_key: CallbackId,
    render_items: CallbackId,
}

impl VirtualListSpec {
    pub fn new(
        id: String,
        axis: gpui::Axis,
        sizes: Rc<Vec<gpui::Size<gpui::Pixels>>>,
        get_key: CallbackId,
        render_items: CallbackId,
    ) -> Self {
        Self {
            id,
            axis,
            sizes,
            get_key,
            render_items,
        }
    }

    /// The name that pairs the list with a `Scrollbar`, and its GPUI identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn axis(&self) -> gpui::Axis {
        self.axis
    }

    /// One extent per item. Its length is the item count.
    pub fn sizes(&self) -> &Rc<Vec<gpui::Size<gpui::Pixels>>> {
        &self.sizes
    }

    /// Resolves the stable domain key for one current item index.
    pub fn get_key(&self) -> CallbackId {
        self.get_key
    }

    /// The handler that describes one window of items.
    pub fn render_items(&self) -> CallbackId {
        self.render_items
    }
}

impl Component {
    pub fn name(&self) -> &'static str {
        match self {
            Component::Div => "div",
            Component::HFlex => "h_flex",
            Component::VFlex => "v_flex",
            Component::ChildView(_) => "child_view",
            Component::Text(_) => "text",
            Component::Button(_) => "Button",
            Component::Link(_) => "Link",
            Component::Checkbox(_) => "Checkbox",
            Component::Switch(_) => "Switch",
            Component::Scrollbar(_) => "Scrollbar",
            Component::Input(_) => "Input",
            Component::Textarea(_) => "Textarea",
            Component::NumberInput(_) => "NumberInput",
            Component::OtpInput(_) => "OtpInput",
            Component::Svg(_) => "svg",
            Component::Image(_) => "image",
            Component::Path { fill: true, .. } => "path fill",
            Component::Path { fill: false, .. } => "path stroke",
            Component::Tabs(_) => "Tabs",
            Component::Tab(_) => "Tab",
            Component::Progress(_) => "Progress",
            Component::ProgressTrack => "ProgressTrack",
            Component::ProgressIndicator => "ProgressIndicator",
            Component::FpsMonitor => "FpsMonitor",
            Component::Slider(_) => "Slider",
            Component::SliderTrack(_) => "SliderTrack",
            Component::SliderIndicator(_) => "SliderIndicator",
            Component::SliderThumb(_) => "SliderThumb",
            Component::Radio(_) => "Radio",
            Component::Toggle(_) => "Toggle",
            Component::RadioGroup(_) => "RadioGroup",
            Component::ToggleGroup(_) => "ToggleGroup",
            Component::Table(_) => "Table",
            Component::TableHeader(_) => "TableHeader",
            Component::TableBody(_) => "TableBody",
            Component::TableRow(..) => "TableRow",
            Component::TableHead(..) => "TableHead",
            Component::TableCell(..) => "TableCell",
            Component::TableCaption(_) => "TableCaption",
            // Named after the constructor rather than the type, because the
            // axis is not a call a reader of the dump could otherwise see.
            Component::Resizable(_, gpui::Axis::Horizontal) => "h_resizable",
            Component::Resizable(_, gpui::Axis::Vertical) => "v_resizable",
            Component::ResizablePanel => "resizable_panel",
            Component::Collapsible => "Collapsible",
            Component::Popover(_) => "Popover",
            Component::HoverCard(_) => "HoverCard",
            Component::Popup(_) => "Popup",
            Component::Select(_) => "Select",
            Component::Combobox(_) => "Combobox",
            Component::DatePicker(..) => "DatePicker",
            // Named after the constructor, as `Resizable` is: the axis is not
            // a call a reader of the dump could otherwise see.
            Component::VirtualList(spec) => match spec.axis() {
                gpui::Axis::Vertical => "v_virtual_list",
                gpui::Axis::Horizontal => "h_virtual_list",
            },
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
    /// A named element slot: an element the component renders in a place of
    /// its own rather than among its children — a `Collapsible`'s content, a
    /// popover's trigger, a number input's buttons.
    ///
    /// The element is detached from the tree when the slot is filled, which is
    /// what stops it from also being drawn as an ordinary child. One `children`
    /// list cannot express any of this: the component has to be able to render
    /// this element somewhere else, or not at all.
    Slot(&'static str, SpecId),
}

/// One described element: what constructed it, what was called on it, and what
/// was put inside it.
///
/// The fields are private and read through methods. A `pub` field on a type that
/// crosses a crate boundary makes every later field a breaking change, and this
/// one will grow — a stable key, dependency metadata, a source span are all
/// things a description might eventually carry (see the "Public Data Types
/// Across the Seam" rule in `docs/ARCHITECTURE.md`).
#[derive(Clone, Debug, Default)]
pub struct SpecNode {
    component: Option<Component>,
    ops: SmallVec<[SpecOp; 8]>,
    children: SmallVec<[SpecId; 4]>,
}

impl SpecNode {
    /// What constructed this node. `None` only for a node that was never
    /// pushed, which a reader should treat as absent.
    pub fn component(&self) -> Option<&Component> {
        self.component.as_ref()
    }

    /// The builder calls recorded on it, in the order the script made them.
    pub fn ops(&self) -> &[SpecOp] {
        &self.ops
    }

    /// The nodes attached to it, in order.
    pub fn children(&self) -> &[SpecId] {
        &self.children
    }
}

/// The descriptions one call to a virtualized list's item renderer produced.
///
/// A batch of rows is described into an arena of its own rather than into the
/// runtime's scratch arena, which belongs to whichever script render is in
/// progress and is reset by the next one. This one is materialized and dropped
/// inside the layout pass that asked for it, so nothing a row described
/// outlives the frame that drew it — and two batches cannot see each other's
/// nodes.
pub struct ItemSpecs {
    arena: SpecArena,
    roots: SmallVec<[SpecId; 16]>,
    keys: Vec<String>,
}

impl ItemSpecs {
    pub(crate) fn new(arena: SpecArena, roots: SmallVec<[SpecId; 16]>, keys: Vec<String>) -> Self {
        Self { arena, roots, keys }
    }

    pub fn arena(&self) -> &SpecArena {
        &self.arena
    }

    /// One root per item, in the order the script returned them.
    pub fn roots(&self) -> &[SpecId] {
        &self.roots
    }

    /// One stable domain key per item, in the same order as [`Self::roots`].
    pub fn keys(&self) -> &[String] {
        &self.keys
    }
}

/// Element descriptions for one script render.
#[derive(Default)]
pub struct SpecArena {
    nodes: Vec<SpecNode>,
    /// Total virtual rows whose native size records this render owns.
    virtual_items: usize,
    /// Nodes already attached to a parent. Re-using one is an error, which is
    /// how Rust's move semantics survive the trip into a garbage-collected
    /// language.
    parented: Vec<bool>,
    /// Nodes consumed by an op rather than by a parent — a state style's
    /// declarations, or the element filling a named slot. They still take ops,
    /// but they can never enter the tree.
    claimed: Vec<bool>,
    /// Retained view handles already described in this snapshot. GPUI cannot
    /// mount one entity at two positions in the same tree.
    mounted_views: HashSet<crate::entities::EntityHandle>,
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
        self.mounted_views.clear();
        self.virtual_items = 0;
    }

    pub(crate) fn claim_virtual_items(&mut self, count: usize, limit: usize) -> bool {
        let Some(total) = self.virtual_items.checked_add(count) else {
            return false;
        };
        if total > limit {
            return false;
        }
        self.virtual_items = total;
        true
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

    /// Records one retained child entity, rejecting a second mount in this
    /// description before any part of the snapshot can be published.
    pub(crate) fn push_child_view(&mut self, child: ChildViewSpec) -> Result<SpecId, SpecError> {
        if !self.mounted_views.insert(child.handle()) {
            return Err(SpecError::DuplicateChildView);
        }
        Ok(self.push(Component::ChildView(child)))
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
        if self.claimed[id as usize] {
            return Err(SpecError::Claimed);
        }
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
            | Component::Link(value)
            | Component::Checkbox(value)
            | Component::Switch(value)
            | Component::Svg(value)
            | Component::Image(value)
            | Component::Tabs(value)
            | Component::Tab(value)
            | Component::Progress(value)
            | Component::Radio(value)
            | Component::Toggle(value)
            | Component::RadioGroup(value)
            | Component::ToggleGroup(value)
            | Component::Popover(value)
            | Component::HoverCard(value)
            | Component::Popup(value)
            | Component::Select(value)
            | Component::Combobox(value) => out.push_str(&format!(" {value:?}")),
            // The focus handle is part of what a `DatePicker` *is* rather than
            // something called on it, so the dump carries it beside the id the
            // way a row carries its index.
            Component::DatePicker(value, handle) => out.push_str(&format!(" {value:?} #{handle}")),
            Component::Table(value)
            | Component::TableHeader(value)
            | Component::TableBody(value)
            | Component::TableCaption(value) => out.push_str(&format!(" {value:?}")),
            // The axis is already in the name, so only the id is left to write.
            Component::Resizable(value, _) => out.push_str(&format!(" {value:?}")),
            // The index is part of what the node *is* rather than something
            // called on it — a cell that lost it announces itself in the wrong
            // column — so the dump carries it beside the id, not among the ops.
            Component::TableRow(value, index)
            | Component::TableHead(value, index)
            | Component::TableCell(value, index) => out.push_str(&format!(" {value:?} #{index}")),
            Component::Scrollbar(value) => out.push_str(&format!(" {value:?}")),
            // The item count, not the item sizes: a dump of a hundred thousand
            // extents is not something a test reads, and the count is the part
            // that says what the list is.
            Component::VirtualList(spec) => {
                out.push_str(&format!(" {:?} \u{d7}{}", spec.id(), spec.sizes().len()))
            }
            Component::ChildView(spec) => out.push_str(&format!(" #{}", spec.handle())),
            Component::Slider(handle)
            | Component::SliderTrack(handle)
            | Component::SliderIndicator(handle)
            | Component::SliderThumb(handle) => out.push_str(&format!(" #{handle}")),
            Component::Input(handle)
            | Component::Textarea(handle)
            | Component::NumberInput(handle)
            | Component::OtpInput(handle) => out.push_str(&format!(" #{handle}")),
            _ => {}
        }
        for op in node.ops() {
            match op {
                SpecOp::NullaryStyle(index) => {
                    out.push_str(&format!(" .{}", crate::style::nullary_name(*index)))
                }
                SpecOp::ParamStyle(name, args) => out.push_str(&format!(" .{name}{args:?}")),
                SpecOp::Method("transition", args) => {
                    if let [
                        Bridged::Str(property),
                        Bridged::Number(duration),
                        Bridged::Number(delay),
                        Bridged::Str(easing),
                    ] = args.as_slice()
                    {
                        out.push_str(&format!(
                            " :transition({property}, {duration}ms, {delay}ms, {easing})"
                        ));
                    } else {
                        out.push_str(" :transition(?)");
                    }
                }
                SpecOp::Method("spring", args) => {
                    if let [
                        Bridged::Str(property),
                        Bridged::Number(response),
                        Bridged::Number(damping),
                        Bridged::Number(epsilon),
                    ] = args.as_slice()
                    {
                        out.push_str(&format!(
                            " :spring({property}, {response}ms, {damping}, {epsilon})"
                        ));
                    } else {
                        out.push_str(" :spring(?)");
                    }
                }
                SpecOp::Method(name, args) => out.push_str(&format!(" :{name}{args:?}")),
                SpecOp::Callback(name, _) => out.push_str(&format!(" :{name}(fn)")),
                SpecOp::StateStyle(name, node) => {
                    out.push_str(&format!(" :{name}("));
                    match self.node(*node) {
                        Some(state) => {
                            for op in state.ops() {
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
                // A slot holds a whole subtree, so it is written under the
                // node instead of on its line of calls.
                SpecOp::Slot(..) => {}
            }
        }
        out.push('\n');
        // A filled slot is detached from `children`, so walking children alone
        // would leave the content out of the dump entirely — and these tests
        // are the only place the description is ever read back.
        for op in node.ops() {
            if let SpecOp::Slot(name, slot) = op {
                out.push_str(&"  ".repeat(depth + 1));
                out.push_str(&format!("@{name}\n"));
                self.write_tree(*slot, depth + 2, out);
            }
        }
        for child in node.children() {
            self.write_tree(*child, depth + 1, out);
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum SpecError {
    /// The node was consumed by a method that takes an element — a state
    /// style's declarations, or a named slot — so it cannot also enter the
    /// tree.
    Claimed,
    /// The node belongs to a previous render pass.
    Expired,
    /// The node was already added to a parent.
    AlreadyParented { component: &'static str },
    /// An element was added to itself.
    SelfParent,
    /// One retained entity was described at two positions in one snapshot.
    DuplicateChildView,
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
                "this element was given to a method that takes one — a state style's \
                 declarations, or a named slot such as content — and cannot also be added \
                 to the tree",
            ),
            SpecError::SelfParent => f.write_str("an element cannot be added to itself"),
            SpecError::DuplicateChildView => f.write_str(
                "a child view handle can be mounted only once in one snapshot; create a second \
                 ViewHandle for a second position",
            ),
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
    fn a_slot_node_can_only_be_claimed_once() {
        let mut arena = SpecArena::new();
        let content = arena.push(Component::Text("body".into()));

        arena.claim(content).unwrap();

        assert_eq!(arena.claim(content).unwrap_err(), SpecError::Claimed);
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

    #[test]
    fn a_filled_slot_is_dumped_under_the_node_holding_it() {
        let mut arena = SpecArena::new();
        let root = arena.push(Component::Collapsible);
        let content = arena.push(Component::Text("body".into()));
        arena.claim(content).unwrap();
        arena
            .push_op(root, SpecOp::Slot("content", content))
            .unwrap();

        assert_eq!(
            arena.debug_tree(root),
            "Collapsible\n  @content\n    text \"body\"\n"
        );
    }
}
