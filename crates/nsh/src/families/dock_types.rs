//! NSH declarations and native decoding share these Rust types.
use nsh_core::ui::{
    Children, Either, Event, JsHost, JsIn, JsObject, JsOwned, JsValue, PropEnum, PropError, Props,
    Readonly, TsTypes,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, PartialEq, JsObject)]
pub(super) struct DockPanel {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) tab_name: Option<String>,
    #[prop(default)]
    pub(super) zoom_control: ZoomControl,
    #[prop(default = true)]
    pub(super) closable: bool,
    #[prop(default = true)]
    pub(super) zoomable: bool,
    #[prop(default = true)]
    pub(super) visible: bool,
    #[prop(default = true)]
    pub(super) title_bar: bool,
    #[prop(default = true)]
    pub(super) inner_padding: bool,
    #[prop(default)]
    pub(super) data: Value,
    #[prop(default)]
    pub(super) toolbar: Readonly<Vec<DockPanelAction>>,
    #[prop(default)]
    pub(super) menu: Readonly<Vec<DockPanelAction>>,
}
#[derive(Clone, Copy, Default, PartialEq, PropEnum)]
pub(super) enum ZoomControl {
    None,
    #[default]
    Menu,
    Toolbar,
    Both,
}
#[derive(Clone, PartialEq, JsObject)]
pub(super) struct DockPanelAction {
    pub(super) id: String,
    pub(super) label: String,
    #[prop(default)]
    pub(super) disabled: bool,
}
#[derive(Clone, PartialEq, Props)]
pub(super) struct DockAreaProps {
    pub(super) panels: Readonly<Vec<DockPanel>>,
    pub(super) layout: Option<DockLayout>,
    /// Complete native persistence schema; pass it back unchanged to restore.
    pub(super) state: Option<DockState>,
    #[prop(default)]
    pub(super) locked: bool,
    #[prop(default)]
    pub(super) tab_bar: bool,
    #[prop(default)]
    pub(super) close_buttons: bool,
    #[prop(default = true)]
    pub(super) dock_toggles: bool,
    pub(super) children: Children,
}
#[derive(Clone, PartialEq, JsObject, Serialize)]
pub(super) struct DockLayout {
    pub(super) center: DockNode,
    pub(super) left: Option<DockNode>,
    pub(super) right: Option<DockNode>,
    pub(super) bottom: Option<DockNode>,
}
#[derive(Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub(super) enum DockNode {
    Split(DockSplitNode),
    Tabs(DockTabsNode),
}
#[derive(Clone, PartialEq, JsObject, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DockSplitNode {
    axis: DockAxis,
    children: Readonly<Vec<DockNode>>,
    size: Option<f32>,
}
#[derive(Clone, PartialEq, JsObject, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DockTabsNode {
    panels: Readonly<Vec<String>>,
    active_index: Option<u64>,
    size: Option<f32>,
}
#[derive(Clone, Copy, PartialEq, PropEnum, Serialize)]
#[serde(rename_all = "lowercase")]
enum DockAxis {
    Horizontal,
    Vertical,
}
impl JsValue for DockNode {
    fn ts_type(types: &mut TsTypes) -> String {
        types.declare("DockNode", |types| {
            format!(
                "export type DockNode = {};",
                <Either<DockSplitNode, DockTabsNode>>::ts_type(types)
            )
        })
    }
    fn from_js(value: JsIn<'_>) -> Result<Self, PropError> {
        <Either<DockSplitNode, DockTabsNode>>::from_js(value).map(|value| match value {
            Either::Left(node) => Self::Split(node),
            Either::Right(node) => Self::Tabs(node),
        })
    }
    fn to_js<'a>(&self, host: &'a dyn JsHost) -> Result<JsOwned<'a>, PropError> {
        match self {
            Self::Split(node) => node.to_js(host),
            Self::Tabs(node) => node.to_js(host),
        }
    }
}
/// Opaque native persistence schema; preserve it unchanged for restore.
#[derive(Clone, PartialEq)]
pub(super) struct DockState(pub(super) Value);
impl JsValue for DockState {
    fn ts_type(types: &mut TsTypes) -> String {
        types.declare("DockState", |_| {
            "export type DockState = Readonly<Record<string, unknown>>;".to_owned()
        })
    }
    fn from_js(value: JsIn<'_>) -> Result<Self, PropError> {
        Value::from_js(value).map(Self)
    }
    fn to_js<'a>(&self, host: &'a dyn JsHost) -> Result<JsOwned<'a>, PropError> {
        self.0.to_js(host)
    }
}
#[derive(Event)]
pub(super) enum DockEvent {
    LayoutChange(DockState),
    Save(DockState),
    ActiveChange { id: String, active: bool },
    ZoomChange { id: String, zoomed: bool },
    PanelRemoved { id: String },
    PanelAction { id: String, action: String },
    PanelRestore { id: String, data: Value },
    Error(String),
}
#[derive(Clone, PartialEq, JsObject)]
pub(super) struct ResizablePanel {
    pub(super) id: String,
    pub(super) size: Option<f32>,
    pub(super) min_size: Option<f32>,
    pub(super) max_size: Option<f32>,
    #[prop(default)]
    pub(super) fixed: bool,
    #[prop(default = true)]
    pub(super) visible: bool,
}
#[derive(Clone, PartialEq, Props)]
pub(super) struct ResizableProps {
    pub(super) panels: Readonly<Vec<ResizablePanel>>,
    #[prop(default)]
    pub(super) vertical: bool,
    pub(super) children: Children,
}
#[derive(Event)]
pub(super) enum ResizableEvent {
    Resize(Readonly<Vec<f32>>),
    Error(String),
}
#[derive(Clone, PartialEq, Props)]
pub(super) struct ResizeHandleProps {
    #[prop(default)]
    pub(super) vertical: bool,
    #[prop(default)]
    pub(super) trailing: bool,
    #[prop(default)]
    pub(super) disabled: bool,
    #[prop(default = "Resize edge".to_owned())]
    pub(super) accessibility_label: String,
}
#[derive(Event)]
pub(super) enum ResizeHandleEvent {
    Start { delta: f32 },
    Resize { delta: f32 },
    End { delta: f32 },
}
