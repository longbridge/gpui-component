//! Dock and panel integration: a script contributes a panel to a docked
//! layout, and draws the dock's chrome.
//!
//! `gpui_base::dock` already has the half of a plugin system that is hard to
//! build: a layout that is pure data, a
//! [`PanelRegistry`](gpui_base::dock::PanelRegistry) that rebuilds a panel
//! from a name in a persisted file, and a per-panel `serde_json::Value` that
//! rides along with it. What it lacks is a way for a panel to come from
//! somewhere other than the host binary. This module is that way — see
//! `docs/gpui-shell.md` §15.
//!
//! Two halves, and they are independent of each other:
//!
//! - [`ScriptPanel`] is a [`Panel`] whose body is a [`ScriptView`]. It carries
//!   the script's own `serialize()` payload through
//!   [`PanelInfo::panel`](gpui_base::dock::PanelInfo::panel), and
//!   [`register_panel`] teaches the registry to rebuild it, so a layout
//!   written before a restart comes back with the panel in place.
//! - [`ScriptDockSkin`] is the appearance. Base draws no chrome at all — an
//!   area with no renderer docks, drags, resizes and persists while painting
//!   nothing but the panels — so every tab bar, dock frame and tile drag bar
//!   a script wants has to come back through the three renderer traits. The
//!   skin forwards each one to [`DockChrome`], and a skin with no chrome is
//!   still a working dock.
//!
//! # Engine independence
//!
//! Nothing here knows what a script value is. The script side of both halves
//! is a trait — [`DockChrome`] for the chrome callbacks, [`PanelScript`] for
//! the panel's build and (de)serialize hooks — that the engine implements.
//! This module deals only in [`Entity<ScriptView>`], [`AnyElement`] and
//! `serde_json::Value`, which is what lets the dock work the same under either
//! engine.

#![allow(dead_code)]
//
// Designed and tested, but nothing in the crate drives it: a script cannot yet
// contribute a panel, because the plugin → contribution → panel path is not
// finished. The module is `pub(crate)` until it is — a public surface no host
// can reach end to end is a promise about an API that has never been used.

use std::{
    collections::HashSet,
    rc::Rc,
    sync::{Arc, Mutex, OnceLock},
};

use gpui::{
    AnyElement, App, AppContext as _, Context, Div, Empty, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, ParentElement as _, Render, Stateful,
    Styled as _, Window, div,
};
use gpui_base::dock::{
    DockAreaRenderer, DockContext, DropIndicator, Panel, PanelBuildContext, PanelEvent, PanelInfo,
    PanelState, PanelView, TabGroupContext, TabGroupRenderer, TileContext, TilesRenderer,
};
use serde_json::{Value, json};

use crate::{
    scope::{self, ScopePhase},
    view::ScriptView,
};

/// The prefix every script panel name carries.
///
/// It is `shell:` rather than the engine's name because the same layout file
/// must still restore after the host switches engines: what wrote the panel is
/// the shell, not QuickJS.
const NAME_PREFIX: &str = "shell:";

/// The interned, process-wide name for one application's panel:
/// `shell:<application>/<panel>`.
///
/// Namespacing is what keeps a script panel from ever colliding with a host
/// panel — no host name begins with `shell:` — and what keeps two
/// applications that both call their panel `inbox` apart in one layout file.
///
/// The returned pointer is stable: calling this twice with the same pair
/// returns the same `&'static str`, so passing a name back through
/// [`ScriptPanel::new`] leaks nothing further.
pub fn panel_name(application: &str, panel: &str) -> &'static str {
    intern(&format!("{NAME_PREFIX}{application}/{panel}"))
}

/// Leaks `name` once and answers with the same pointer forever after.
///
/// [`Panel::panel_name`] must return `&'static str`, and a script's panel name
/// is only known when the application is loaded. There is no way to satisfy
/// the signature without a leak, so the leak is made once per distinct name
/// and bounded by the number of *registered* panels — applications loaded ×
/// panels each, which is a number in the hundreds, tens of bytes apiece.
///
/// Unloading an application does **not** reclaim its name. The design accepts
/// this (§15.4): reclaiming would mean a name could be freed while a persisted
/// layout still refers to it, and the whole point of the name is that it
/// outlives the load that produced it.
fn intern(name: &str) -> &'static str {
    static NAMES: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();

    let mut names = NAMES
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        // A poisoned table still holds valid names: the only mutation is an
        // insert, and a panic between the leak and the insert would leak one
        // name twice at worst.
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(interned) = names.get(name) {
        return interned;
    }

    let interned: &'static str = Box::leak(name.to_owned().into_boxed_str());
    names.insert(interned);
    interned
}

/// The script side of a panel, implemented by the engine.
///
/// Three hooks, because three things cross the language boundary: building the
/// view a restored panel shows, reading the script's `serialize()`, and handing
/// a persisted payload back to `deserialize(data)`. Everything else about a
/// panel — where it sits, whether it is displayed, what it is called — is the
/// layout's business and never reaches the script.
#[allow(unused_variables)]
pub trait PanelScript: 'static {
    /// Whether the restored panel may be closed.
    ///
    /// Panel options belong to the application definition rather than the
    /// persisted layout, so the registry asks the script again on every load.
    fn closable(&self) -> bool {
        true
    }

    /// Whether the restored panel may fill its dock.
    fn zoomable(&self) -> bool {
        true
    }

    /// Whether the restored panel is currently drawn.
    fn visible(&self) -> bool {
        true
    }

    /// Builds a fresh script view for this panel.
    ///
    /// Called when [`PanelRegistry`](gpui_base::dock::PanelRegistry) rebuilds
    /// the panel from a persisted layout. `None` means the script could not
    /// be instantiated — the application is broken, or its entry point threw —
    /// and the panel's persisted state is carried forward untouched rather
    /// than dropped.
    fn build(&self, window: &mut Window, cx: &mut App) -> Option<Entity<ScriptView>>;

    /// The script's `serialize()`, or `None` for a panel that has none.
    ///
    /// Note the `&App`: [`Panel::dump`] is a read, so there is no `&mut
    /// Window` here and therefore no [`CallScope`](crate::scope) to open. A
    /// script `serialize()` must be a plain value-returning method that calls
    /// nothing back into the host.
    fn serialize(&self, view: &Entity<ScriptView>, cx: &App) -> Option<Value> {
        None
    }

    /// Hands `data` — whatever [`Self::serialize`] wrote last time — back to
    /// the script's `deserialize(data)`.
    ///
    /// Called after [`Self::build`], with a real host call available, so this
    /// one may open a scope and touch entities.
    fn deserialize(
        &self,
        view: &Entity<ScriptView>,
        data: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
    }
}

/// A dockable panel whose body is a script view.
///
/// Only [`gpui_base::dock::Panel`] — behavior — is implemented, not the
/// presentation trait one layer up. A script panel's title, toolbar and menus
/// are drawn by [`ScriptDockSkin`] from the script's own elements, which is
/// what "the script owns presentation" means here.
pub struct ScriptPanel {
    /// Interned, always prefixed. See [`intern`].
    name: &'static str,
    view: Entity<ScriptView>,
    script: Option<Rc<dyn PanelScript>>,
    focus_handle: FocusHandle,
    closable: bool,
    zoomable: bool,
    visible: bool,
}

impl ScriptPanel {
    /// Wraps `view` as a panel named `name`.
    ///
    /// `name` is normally the value [`panel_name`] or [`register_panel`]
    /// returned. A name that does not already carry the `shell:` prefix gets
    /// it, so the namespace guarantee holds however the panel was built.
    pub fn new(name: &str, view: Entity<ScriptView>, cx: &mut App) -> Self {
        let name = if name.starts_with(NAME_PREFIX) {
            intern(name)
        } else {
            intern(&format!("{NAME_PREFIX}{name}"))
        };

        Self {
            name,
            view,
            script: None,
            focus_handle: cx.focus_handle(),
            closable: true,
            zoomable: true,
            visible: true,
        }
    }

    /// Connects the script hooks, without which the panel persists its
    /// position and nothing else.
    pub fn with_script(mut self, script: Rc<dyn PanelScript>) -> Self {
        self.script = Some(script);
        self
    }

    /// Whether the dock may close this panel — the script's
    /// `static options.closable`.
    pub fn with_closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    pub fn with_zoomable(mut self, zoomable: bool) -> Self {
        self.zoomable = zoomable;
        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// The interned name this panel persists under.
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn view(&self) -> &Entity<ScriptView> {
        &self.view
    }
}

impl Panel for ScriptPanel {
    fn panel_name(&self) -> &'static str {
        self.name
    }

    fn visible(&self, _: &App) -> bool {
        self.visible
    }

    fn closable(&self, _: &App) -> bool {
        self.closable
    }

    fn zoomable(&self, _: &App) -> bool {
        self.zoomable
    }

    /// The script's own state, under the panel's interned name.
    ///
    /// A panel whose script has no `serialize()` still writes a leaf here, so
    /// its place in the tree survives; only the payload is absent.
    fn dump(&self, cx: &App) -> PanelState {
        let mut state = PanelState::new(self.name);
        if let Some(value) = self
            .script
            .as_ref()
            .and_then(|script| script.serialize(&self.view, cx))
        {
            state.info = PanelInfo::panel(value);
        }
        state
    }
}

impl EventEmitter<PanelEvent> for ScriptPanel {}

impl Focusable for ScriptPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ScriptPanel {
    /// The script's root element, and one focus target around it.
    ///
    /// `size_full` is the only presentation here and it is not a style choice:
    /// a tab group draws its displayed panel as an ordinary child, so a
    /// wrapper that shrink-wrapped would leave the script's own root — which
    /// the script *does* style — with nothing to fill.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .child(self.view.clone())
    }
}

/// Teaches [`PanelRegistry`](gpui_base::dock::PanelRegistry) to rebuild
/// `application`'s `panel` from a persisted layout, and answers with the
/// interned name it registered under.
///
/// Call this when the application is activated, before any
/// [`DockArea::load`](gpui_base::dock::DockArea::load) that might mention the
/// panel. The builder runs [`PanelScript::build`], then hands the persisted
/// `serde_json::Value` to [`PanelScript::deserialize`].
///
/// # An uninstalled application still round-trips
///
/// If the application is *not* loaded, nothing is registered under its name
/// and `DockArea::load` finds no builder. It does not drop the panel: it
/// substitutes a draw-nothing placeholder that answers `Panel::dump` with the
/// `PanelState` it was handed, so the next save writes the panel — name,
/// payload and position — back out unchanged. That is verified base behavior
/// (`dock_area.rs`, `PlaceholderPanel`), and it is what makes uninstalling and
/// reinstalling an application put its panels back where they were. This
/// module keeps the same promise one step further in: a registered panel whose
/// [`PanelScript::build`] fails is carried forward the same way rather than
/// losing its state.
pub fn register_panel(
    application: &str,
    panel: &str,
    script: Rc<dyn PanelScript>,
    cx: &mut App,
) -> &'static str {
    let name = panel_name(application, panel);

    gpui_base::dock::register_panel(cx, name, move |context: PanelBuildContext, window, cx| {
        build_panel(name, &script, &context, window, cx)
    });

    name
}

fn build_panel(
    name: &'static str,
    script: &Rc<dyn PanelScript>,
    context: &PanelBuildContext,
    window: &mut Window,
    cx: &mut App,
) -> Arc<dyn PanelView> {
    let Some(view) = script.build(window, cx) else {
        return Arc::new(cx.new(|cx| RetainedPanel::new(name, context.state().clone(), cx)));
    };

    if let PanelInfo::Panel(data) = context.info() {
        if !data.is_null() {
            script.deserialize(&view, data, window, cx);
        }
    }

    let closable = script.closable();
    let zoomable = script.zoomable();
    let visible = script.visible();
    Arc::new(cx.new(|cx| {
        ScriptPanel::new(name, view, cx)
            .with_script(script.clone())
            .with_closable(closable)
            .with_zoomable(zoomable)
            .with_visible(visible)
    }))
}

/// Stands in for a registered panel whose script would not build.
///
/// It draws nothing and hands back the [`PanelState`] it was given, so a
/// script that throws on load costs the user the panel's *contents* for this
/// session, not its place in the layout or its saved payload. Base does the
/// same for a panel with no builder at all; this covers the case where the
/// builder exists and fails.
struct RetainedPanel {
    name: &'static str,
    state: PanelState,
    focus_handle: FocusHandle,
}

impl RetainedPanel {
    fn new(name: &'static str, state: PanelState, cx: &mut Context<Self>) -> Self {
        Self {
            name,
            state,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Panel for RetainedPanel {
    fn panel_name(&self) -> &'static str {
        self.name
    }

    fn dump(&self, _: &App) -> PanelState {
        self.state.clone()
    }
}

impl EventEmitter<PanelEvent> for RetainedPanel {}

impl Focusable for RetainedPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RetainedPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// The chrome a script draws for a dock, implemented by the engine.
///
/// One method per place base asks for appearance and a script can plausibly
/// supply it. Every method has a default that reproduces base's own
/// no-chrome behavior, so an application that draws no dock chrome implements
/// none of them and still gets a dock that docks, drags, resizes and persists.
///
/// Each method is handed the *resolved* context. A renderer never sees a drag
/// event, a mouse position or a hit test — base attaches all of that to the
/// elements it gets back — so the script's job is to turn state into elements
/// and to call the context's own callbacks (`select_tab`, `close`,
/// `toggle_zoom`, `resize_to`) rather than reimplement them. [`tab_group_data`],
/// [`dock_data`] and [`tile_data`] convert the state half into plain JSON,
/// which is the form the engine hands to script code.
#[allow(unused_variables)]
pub trait DockChrome: 'static {
    /// The tab bar above a group's displayed panel.
    fn tab_bar(&self, group: &TabGroupContext, window: &mut Window, cx: &mut App) -> AnyElement {
        Empty.into_any_element()
    }

    /// What a group with no displayed panel shows.
    fn empty_group(
        &self,
        group: &TabGroupContext,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        None
    }

    /// The hint showing where a dragged panel would land.
    fn drop_indicator(
        &self,
        indicator: DropIndicator,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        None
    }

    /// One dock's chrome around its content: title strip, collapse
    /// affordance, resize handle. Whatever this returns replaces the content,
    /// so a chrome that wants both must place `content` itself.
    fn dock(
        &self,
        dock: &DockContext,
        content: AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        content
    }

    /// The strip a tile is dragged by. Its height is fixed at
    /// [`DRAG_BAR_HEIGHT`](gpui_base::dock::DRAG_BAR_HEIGHT), which base's
    /// snapping arithmetic assumes and the script must match.
    fn tile_drag_bar(&self, tile: &TileContext, window: &mut Window, cx: &mut App) -> AnyElement {
        Empty.into_any_element()
    }

    /// A tile's resize affordances, whose hit size is
    /// [`HANDLE_SIZE`](gpui_base::dock::HANDLE_SIZE).
    fn tile_resize_handles(
        &self,
        tile: &TileContext,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        None
    }
}

/// A [`DockChrome`] that draws nothing, which is also base's own behavior.
struct BareChrome;

impl DockChrome for BareChrome {}

/// The appearance of a dock area, forwarded to script.
///
/// Install it with
/// [`DockArea::with_renderer`](gpui_base::dock::DockArea::with_renderer). One
/// value implements all three renderer traits and hands out clones of itself
/// for the per-container ones, because the only thing a container renderer
/// needs is the same chrome handle.
pub struct ScriptDockSkin {
    chrome: Rc<dyn DockChrome>,
}

impl ScriptDockSkin {
    pub fn new(chrome: Rc<dyn DockChrome>) -> Self {
        Self { chrome }
    }

    fn clone_skin(&self) -> Self {
        Self {
            chrome: self.chrome.clone(),
        }
    }
}

impl Default for ScriptDockSkin {
    /// A dock with no chrome at all — still draggable, resizable and
    /// persistent, drawing only the panels.
    fn default() -> Self {
        Self::new(Rc::new(BareChrome))
    }
}

/// Runs `f` in a nested [`ScopePhase::Layout`] scope.
///
/// Every chrome callback is reached from inside GPUI's layout/prepaint pass
/// rather than from a view's `render`, and there may already be an outer scope
/// around it — a dock area nested in a script view has one. `Layout` is the
/// phase that says so: it forbids `notify`, entity creation and spawning, and
/// runs on the render time budget. A frame is pushed, not reused, so the
/// budget starts fresh for each callback and a stale `cx` captured from an
/// earlier call is still rejected.
///
/// The scope inherits the enclosing view, if any: the chrome is drawn on
/// behalf of whatever view is being rendered and owns no view of its own.
fn in_layout_scope<R>(
    window: &mut Window,
    cx: &mut App,
    f: impl FnOnce(&mut Window, &mut App) -> R,
) -> R {
    let (_guard, _generation) = scope::enter(window, cx, ScopePhase::Layout, scope::current_view());
    f(window, cx)
}

impl DockAreaRenderer for ScriptDockSkin {
    fn render_dock(
        &self,
        dock: &DockContext,
        content: AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        in_layout_scope(window, cx, |window, cx| {
            self.chrome.dock(dock, content, window, cx)
        })
    }

    fn tab_group_renderer(&self) -> Rc<dyn TabGroupRenderer> {
        Rc::new(self.clone_skin())
    }

    fn tiles_renderer(&self) -> Rc<dyn TilesRenderer> {
        Rc::new(self.clone_skin())
    }
}

impl TabGroupRenderer for ScriptDockSkin {
    fn render_tab_bar(
        &self,
        group: &TabGroupContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        in_layout_scope(window, cx, |window, cx| {
            self.chrome.tab_bar(group, window, cx)
        })
    }

    fn render_empty(
        &self,
        group: &TabGroupContext,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        in_layout_scope(window, cx, |window, cx| {
            self.chrome.empty_group(group, window, cx)
        })
    }

    fn render_drop_indicator(
        &self,
        indicator: DropIndicator,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        in_layout_scope(window, cx, |window, cx| {
            self.chrome.drop_indicator(indicator, window, cx)
        })
    }
}

impl TilesRenderer for ScriptDockSkin {
    fn frame(&self, _: &mut Window, _: &mut App) -> Stateful<Div> {
        div().id("script-tiles")
    }

    fn render_drag_bar(&self, tile: &TileContext, window: &mut Window, cx: &mut App) -> AnyElement {
        in_layout_scope(window, cx, |window, cx| {
            self.chrome.tile_drag_bar(tile, window, cx)
        })
    }

    fn render_resize_handles(
        &self,
        tile: &TileContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        in_layout_scope(window, cx, |window, cx| {
            self.chrome
                .tile_resize_handles(tile, window, cx)
                .unwrap_or_else(|| Empty.into_any_element())
        })
    }
}

/// One tab group's state as plain JSON, the form script code reads:
///
/// ```json
/// {
///   "node": 3,
///   "active_index": 0,
///   "zoomed": false, "collapsed": false, "locked": false,
///   "draggable": true, "droppable": true, "closable": true,
///   "tabs": [
///     { "index": 0, "name": "shell:mail/inbox", "id": 42,
///       "active": true, "visible": true, "closable": true, "zoomable": true }
///   ]
/// }
/// ```
///
/// `index` is the tab's position in the group, which is what the script passes
/// back to [`TabGroupContext::select_tab`]. Hidden panels are included: base
/// keeps them in tab order and a script that wants them gone filters on
/// `visible`, rather than having to re-derive an index into an already
/// filtered list.
///
/// The callbacks are deliberately absent — they cannot be JSON. The engine
/// keeps the [`TabGroupContext`] alongside this value and wires
/// `select_tab`/`close`/`toggle_zoom` onto the elements the script returns.
pub fn tab_group_data(group: &TabGroupContext, cx: &App) -> Value {
    let active = group.active_panel().map(|panel| panel.panel_id(cx));

    let tabs = group
        .panels()
        .iter()
        .enumerate()
        .map(|(index, panel)| {
            let id = panel.panel_id(cx);
            json!({
                "index": index,
                "name": panel.panel_name(cx),
                "id": id.as_u64(),
                "active": Some(id) == active,
                "visible": panel.visible(cx),
                "closable": panel.closable(cx),
                "zoomable": panel.zoomable(cx),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "node": group.node().as_u64(),
        "active_index": group.active_ix(),
        "zoomed": group.is_zoomed(),
        "collapsed": group.is_collapsed(),
        "locked": group.is_locked(),
        "draggable": group.is_draggable(),
        "droppable": group.is_droppable(),
        "closable": group.is_closable(),
        "tabs": tabs,
    })
}

/// One dock's state as plain JSON:
///
/// ```json
/// { "placement": "left", "size": 240.0, "open": true, "collapsible": true }
/// ```
///
/// `placement` uses the same tags as the persisted layout, which are frozen.
/// `size` is the dock's extent along its own axis — width for left and right,
/// height for bottom.
pub fn dock_data(dock: &DockContext) -> Value {
    json!({
        "placement": serde_json::to_value(dock.placement()).unwrap_or(Value::Null),
        "size": f32::from(dock.size()),
        "open": dock.is_open(),
        "collapsible": dock.is_collapsible(),
    })
}

/// One tile's state as plain JSON:
///
/// ```json
/// {
///   "node": 3,
///   "panel": { "name": "shell:mail/inbox", "id": 42, "visible": true },
///   "bounds": { "x": 10.0, "y": 10.0, "width": 200.0, "height": 200.0 },
///   "z_index": 0,
///   "moving": false, "resizing": false,
///   "closable": true, "zoomed": false, "zoomable": true
/// }
/// ```
///
/// `bounds` are already resolved — base snaps, clamps and rounds before a skin
/// sees them, so the script positions nothing itself. A zoomed tile fills the
/// dock and ignores its stored bounds.
pub fn tile_data(tile: &TileContext, cx: &App) -> Value {
    let bounds = tile.bounds();

    json!({
        "node": tile.node().as_u64(),
        "panel": {
            "name": tile.panel().panel_name(cx),
            "id": tile.panel_id().as_u64(),
            "visible": tile.panel().visible(cx),
        },
        "bounds": {
            "x": f32::from(bounds.origin.x),
            "y": f32::from(bounds.origin.y),
            "width": f32::from(bounds.size.width),
            "height": f32::from(bounds.size.height),
        },
        "z_index": tile.z_index(),
        "moving": tile.is_moving(),
        "resizing": tile.is_resizing(),
        "closable": tile.is_closable(),
        "zoomed": tile.is_zoomed(),
        "zoomable": tile.is_zoomable(),
    })
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, ops::Deref as _};

    use gpui::{TestAppContext, VisualTestContext};
    use gpui_base::dock::{DockArea, DockLayout, DockPlacement};

    use super::*;
    use crate::engine::{ShellRuntime, ViewType};

    /// The smallest thing each engine calls a view. The dock tests never look
    /// at what it renders — only at what survives a save and a load.
    #[cfg(feature = "quickjs")]
    const PANEL_SOURCE: &str = r#"
import { View, div } from "gpui";

export default class Inbox extends View {
  render() {
    return div();
  }
}
"#;
    #[cfg(not(feature = "quickjs"))]
    const PANEL_SOURCE: &str = r#"
local gpui = require("gpui")
local Inbox = gpui.view("Inbox")

function Inbox:render(cx)
  return gpui.div()
end

return Inbox
"#;

    /// Stands in for the engine's half of [`PanelScript`]: it builds a real
    /// script view, hands back a fixed payload as the script's `serialize()`,
    /// and records what `deserialize` was given.
    struct Probe {
        /// Declared before the runtime, because fields drop in declaration
        /// order and a script handle released after its runtime aborts the
        /// process. `ShellRuntime` orders its own fields for the same reason.
        view_type: ViewType,
        runtime: Rc<ShellRuntime>,
        payload: Option<Value>,
        restored: Rc<RefCell<Vec<Value>>>,
        closable: bool,
        zoomable: bool,
        visible: bool,
    }

    impl PanelScript for Probe {
        fn build(&self, window: &mut Window, cx: &mut App) -> Option<Entity<ScriptView>> {
            let object = self.runtime.instantiate(&self.view_type, window, cx).ok()?;
            let runtime = self.runtime.clone();
            Some(cx.new(|_| ScriptView::new(runtime, object)))
        }

        fn serialize(&self, _: &Entity<ScriptView>, _: &App) -> Option<Value> {
            self.payload.clone()
        }

        fn deserialize(&self, _: &Entity<ScriptView>, data: &Value, _: &mut Window, _: &mut App) {
            self.restored.borrow_mut().push(data.clone());
        }

        fn closable(&self) -> bool {
            self.closable
        }

        fn zoomable(&self) -> bool {
            self.zoomable
        }

        fn visible(&self) -> bool {
            self.visible
        }
    }

    fn boot(cx: &mut TestAppContext) -> (Rc<ShellRuntime>, ViewType) {
        cx.update(|cx| crate::init(cx));
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        cx.update(|cx| runtime.set_global(cx));
        let view_type = runtime.load_source("inbox", PANEL_SOURCE).expect("load");
        (runtime, view_type)
    }

    fn probe(
        runtime: &Rc<ShellRuntime>,
        view_type: &ViewType,
        payload: Option<Value>,
    ) -> (Rc<Probe>, Rc<RefCell<Vec<Value>>>) {
        let restored = Rc::new(RefCell::new(Vec::new()));
        let script = Rc::new(Probe {
            view_type: view_type.clone(),
            runtime: runtime.clone(),
            payload,
            restored: restored.clone(),
            closable: true,
            zoomable: true,
            visible: true,
        });
        (script, restored)
    }

    #[gpui::test]
    fn registered_panel_restores_its_static_options(cx: &mut TestAppContext) {
        let (runtime, view_type) = boot(cx);
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);

        let (script, _) = probe(&runtime, &view_type, None);
        let mut script = Rc::try_unwrap(script).ok().expect("unique probe");
        script.closable = false;
        script.zoomable = false;
        script.visible = false;
        let script = Rc::new(script);
        let name = context.update(|_, cx| register_panel("mail", "fixed", script.clone(), cx));

        let saved = context.update(|window, cx| {
            let object = runtime
                .instantiate(&view_type, window, cx)
                .expect("instance");
            let view = cx.new(|_| ScriptView::new(runtime.clone(), object));
            let panel = cx.new(|cx| {
                ScriptPanel::new(name, view, cx)
                    .with_script(script.clone())
                    .with_closable(false)
                    .with_zoomable(false)
                    .with_visible(false)
            });
            let area = cx.new(|cx| DockArea::new("workspace", None, window, cx));
            area.update(cx, |area, cx| {
                area.set_center(DockLayout::tabs().panel(panel), window, cx);
                area.dump(cx)
            })
        });

        let area = context.update(|window, cx| {
            let area = cx.new(|cx| DockArea::new("workspace", None, window, cx));
            area.update(cx, |area, cx| {
                area.load(saved, window, cx).expect("load");
            });
            area
        });

        context.read(|cx| {
            let panel_id = area
                .read(cx)
                .layout(DockPlacement::Center)
                .expect("center")
                .panels()
                .next()
                .expect("restored panel");
            let panel = area.read(cx).panel(panel_id).expect("live panel");
            assert!(!panel.closable(cx));
            assert!(!panel.zoomable(cx));
            assert!(!panel.visible(cx));
        });
    }

    /// The persisted leaf for `name`, wherever it sits in the tree.
    fn leaf<'a>(state: &'a PanelState, name: &str) -> Option<&'a PanelState> {
        if state.panel_name == name {
            return Some(state);
        }
        state.children.iter().find_map(|child| leaf(child, name))
    }

    #[test]
    fn a_name_is_namespaced_and_interned_once() {
        let name = panel_name("mail", "inbox");
        assert_eq!(name, "shell:mail/inbox");
        assert!(std::ptr::eq(name, panel_name("mail", "inbox")));
    }

    #[test]
    fn the_same_panel_name_in_two_applications_does_not_collide() {
        assert_ne!(panel_name("mail", "inbox"), panel_name("chat", "inbox"));
        assert_eq!(panel_name("chat", "inbox"), "shell:chat/inbox");
    }

    #[gpui::test]
    fn a_panel_reports_its_interned_name(cx: &mut TestAppContext) {
        let (runtime, view_type) = boot(cx);
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);

        let panel = context.update(|window, cx| {
            let object = runtime
                .instantiate(&view_type, window, cx)
                .expect("instance");
            let view = cx.new(|_| ScriptView::new(runtime.clone(), object));
            cx.new(|cx| ScriptPanel::new(panel_name("mail", "inbox"), view, cx))
        });

        context.read(|cx| {
            let name = panel.read(cx).panel_name();
            assert_eq!(name, "shell:mail/inbox");
            // The panel holds the interned string itself, not a copy of it,
            // which is the whole reason the table exists.
            assert!(std::ptr::eq(name, panel_name("mail", "inbox")));
        });
    }

    /// A bare name still lands in the namespace, so a panel built without
    /// going through `panel_name` cannot shadow a host panel.
    #[gpui::test]
    fn an_unqualified_name_is_still_namespaced(cx: &mut TestAppContext) {
        let (runtime, view_type) = boot(cx);
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);

        let panel = context.update(|window, cx| {
            let object = runtime
                .instantiate(&view_type, window, cx)
                .expect("instance");
            let view = cx.new(|_| ScriptView::new(runtime.clone(), object));
            cx.new(|cx| ScriptPanel::new("TabPanel", view, cx))
        });

        context.read(|cx| assert_eq!(panel.read(cx).panel_name(), "shell:TabPanel"));
    }

    #[gpui::test]
    fn a_round_trip_preserves_the_scripts_own_payload(cx: &mut TestAppContext) {
        let (runtime, view_type) = boot(cx);
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);

        let (script, restored) = probe(
            &runtime,
            &view_type,
            Some(json!({ "filter": "unread", "sort": 2 })),
        );
        let name = context.update(|_, cx| register_panel("mail", "inbox", script.clone(), cx));

        let saved = context.update(|window, cx| {
            let object = runtime
                .instantiate(&view_type, window, cx)
                .expect("instance");
            let view = cx.new(|_| ScriptView::new(runtime.clone(), object));
            let panel = cx.new(|cx| ScriptPanel::new(name, view, cx).with_script(script.clone()));
            let area = cx.new(|cx| DockArea::new("workspace", Some(1), window, cx));
            area.update(cx, |area, cx| {
                area.set_center(DockLayout::tabs().panel(panel), window, cx);
                area.dump(cx)
            })
        });

        let written = leaf(&saved.center, name).expect("the panel is in the saved layout");
        assert_eq!(
            written.info,
            PanelInfo::panel(json!({ "filter": "unread", "sort": 2 }))
        );

        // A fresh area rebuilds the panel through the registry, which is the
        // path a restart takes.
        let reloaded = context.update(|window, cx| {
            let area = cx.new(|cx| DockArea::new("workspace", Some(1), window, cx));
            area.update(cx, |area, cx| {
                area.load(saved.clone(), window, cx).expect("load");
                area.dump(cx)
            })
        });

        assert_eq!(
            restored.borrow().as_slice(),
            [json!({ "filter": "unread", "sort": 2 })],
            "deserialize was handed exactly what serialize wrote"
        );
        assert_eq!(
            leaf(&reloaded.center, name).map(|state| &state.info),
            Some(&PanelInfo::panel(json!({ "filter": "unread", "sort": 2 }))),
            "and saving again writes it back"
        );
    }

    #[gpui::test]
    fn a_panel_without_serialize_still_round_trips_its_position(cx: &mut TestAppContext) {
        let (runtime, view_type) = boot(cx);
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);

        let (script, restored) = probe(&runtime, &view_type, None);
        let name = context.update(|_, cx| register_panel("mail", "outbox", script.clone(), cx));

        let saved = context.update(|window, cx| {
            let object = runtime
                .instantiate(&view_type, window, cx)
                .expect("instance");
            let view = cx.new(|_| ScriptView::new(runtime.clone(), object));
            let panel = cx.new(|cx| ScriptPanel::new(name, view, cx).with_script(script.clone()));
            let area = cx.new(|cx| DockArea::new("workspace", None, window, cx));
            area.update(cx, |area, cx| {
                area.set_dock(
                    DockPlacement::Bottom,
                    DockLayout::tabs().panel(panel),
                    window,
                    cx,
                );
                area.dump(cx)
            })
        });

        let bottom = saved.bottom_dock.clone().expect("a bottom dock was saved");
        let written = leaf(bottom.panel(), name).expect("the panel is in the saved layout");
        assert_eq!(written.info, PanelInfo::panel(Value::Null));

        let reloaded = context.update(|window, cx| {
            let area = cx.new(|cx| DockArea::new("workspace", None, window, cx));
            area.update(cx, |area, cx| {
                area.load(saved.clone(), window, cx).expect("load");
                area.dump(cx)
            })
        });

        assert!(
            restored.borrow().is_empty(),
            "an absent payload is not handed to deserialize"
        );
        assert!(
            leaf(
                reloaded
                    .bottom_dock
                    .expect("the bottom dock came back")
                    .panel(),
                name
            )
            .is_some(),
            "the panel came back in the dock it was saved in"
        );
    }

    /// The layout must survive a script that cannot be built at all — the
    /// same promise base makes for a panel it has no builder for.
    #[gpui::test]
    fn a_panel_whose_script_fails_keeps_its_persisted_state(cx: &mut TestAppContext) {
        struct Broken;

        impl PanelScript for Broken {
            fn build(&self, _: &mut Window, _: &mut App) -> Option<Entity<ScriptView>> {
                None
            }
        }

        let (runtime, view_type) = boot(cx);
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);

        let (script, _) = probe(&runtime, &view_type, Some(json!({ "filter": "all" })));
        let name = context.update(|_, cx| register_panel("mail", "drafts", script.clone(), cx));

        let saved = context.update(|window, cx| {
            let object = runtime
                .instantiate(&view_type, window, cx)
                .expect("instance");
            let view = cx.new(|_| ScriptView::new(runtime.clone(), object));
            let panel = cx.new(|cx| ScriptPanel::new(name, view, cx).with_script(script.clone()));
            let area = cx.new(|cx| DockArea::new("workspace", None, window, cx));
            area.update(cx, |area, cx| {
                area.set_center(DockLayout::tabs().panel(panel), window, cx);
                area.dump(cx)
            })
        });

        // The application reloads with a script that throws on construction.
        context.update(|_, cx| register_panel("mail", "drafts", Rc::new(Broken), cx));

        let reloaded = context.update(|window, cx| {
            let area = cx.new(|cx| DockArea::new("workspace", None, window, cx));
            area.update(cx, |area, cx| {
                area.load(saved.clone(), window, cx).expect("load");
                area.dump(cx)
            })
        });

        assert_eq!(
            leaf(&reloaded.center, name).map(|state| &state.info),
            Some(&PanelInfo::panel(json!({ "filter": "all" }))),
            "a failed build carries the payload forward instead of erasing it"
        );
    }
}
