//! The gpui-component appearance for the dock.
//!
//! The layout tree, the persisted schema, the drag geometry, the active-panel
//! state machine and the container entities all live in
//! [`gpui_base::dock`]. This module is the skin over them: it re-exports the
//! types a consumer needs, adds the presentation half of the panel traits
//! (see [`panel`]), and implements base's three renderer traits.
//!
//! ```ignore
//! let area = cx.new(|cx| {
//!     DockArea::new("main", Some(1), window, cx).with_renderer(DockSkin::new(cx))
//! });
//! ```
//!
//! A [`DockArea`] built without [`DockSkin`] still docks, drags and persists —
//! it simply draws no chrome at all.

mod dock;
mod invalid_panel;
mod panel;
mod tab_panel;
mod tiles;

use std::{cell::Cell, rc::Rc};

use gpui::{App, AppContext as _, Context, Entity, SharedString, WeakEntity, Window, actions};

/// The behavior half of the panel traits, which every panel implements
/// alongside [`Panel`]. Exported under this name because `Panel` in this
/// module is the presentation half that extends it.
pub use gpui_base::dock::Panel as BasePanel;
/// The object-safe counterpart of [`BasePanel`], for the same reason.
pub use gpui_base::dock::PanelView as BasePanelView;
pub use gpui_base::dock::{
    AnyDrag, DockArea, DockAreaRenderer, DockAreaState, DockContext, DockEvent, DockLayout,
    DockPlacement, DockState, DragPanel, DropIndicator, DropPlaceholderBounds, DropTarget,
    EditResult, InsertTarget, LayoutNode, LayoutTree, NodeId, NodeRef, PanelBuildContext,
    PanelEvent, PanelId, PanelInfo, PanelRegistry, PanelState, TabGroup, TabGroupContext,
    TabGroupRenderer, TileContext, TileMeta, TilePanel, TilesRenderer, register_panel,
};
pub use panel::*;
pub use tab_panel::DragPanelPreview;

actions!(dock, [ToggleZoom, ClosePanel]);

pub(crate) fn init(cx: &mut App) {
    // `gpui_base::dock::PanelRegistry::init` is crate-private, but the global
    // it installs is not: `DockArea::new` and `register_panel` both create it
    // on demand, and this keeps the old guarantee that it exists as soon as
    // `gpui_component::init` has run.
    if cx.try_global::<PanelRegistry>().is_none() {
        cx.set_global(PanelRegistry::new());
    }
}

/// What every part of the skin reads, and the dock area it belongs to.
///
/// The renderer is the only skin-owned object in the picture, so the settings
/// the old `DockArea` carried — the panel style, whether dock collapse
/// affordances are offered at all — live here. It is shared by reference with
/// the per-container renderers, which are built once each and outlive any one
/// frame.
pub(crate) struct SkinShared {
    area: WeakEntity<DockArea>,
    panel_style: Cell<PanelStyle>,
    toggle_button_visible: Cell<bool>,
    /// The dock whose resize handle is being dragged, if any. Only one can be.
    resizing_dock: Cell<Option<DockPlacement>>,
}

impl SkinShared {
    pub(crate) fn area(&self) -> &WeakEntity<DockArea> {
        &self.area
    }

    pub(crate) fn panel_style(&self) -> PanelStyle {
        self.panel_style.get()
    }

    pub(crate) fn is_toggle_button_visible(&self) -> bool {
        self.toggle_button_visible.get()
    }

    pub(crate) fn resizing_dock(&self) -> &Cell<Option<DockPlacement>> {
        &self.resizing_dock
    }

    /// Redraw the area after a setting changed. The skin is not an entity, so
    /// nothing else would notice.
    fn notify(&self, cx: &mut App) {
        _ = self.area.update(cx, |_, cx| cx.notify());
    }
}

/// The gpui-component appearance for a [`DockArea`], and the handle its
/// settings are changed through.
///
/// Install it at construction, where the area's own weak handle is available:
///
/// ```ignore
/// let skin = DockSkin::new(cx);
/// DockArea::new("main", None, window, cx).with_renderer(skin)
/// ```
///
/// Keep the returned handle to change a setting later; it is an `Rc`, so a
/// clone and the installed renderer are the same skin.
pub struct DockSkin {
    shared: Rc<SkinShared>,
}

impl DockSkin {
    /// Build a [`DockArea`] wearing this appearance, together with the handle
    /// its settings are changed through.
    ///
    /// The skin needs the area's own weak handle, so it can only be built
    /// while the area is being constructed; this is that dance done once.
    pub fn dock_area(
        id: impl Into<SharedString>,
        version: Option<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> (Entity<DockArea>, Rc<Self>) {
        let mut skin = None;
        let area = cx.new(|cx| {
            let this = Self::new(cx);
            skin = Some(this.clone());
            DockArea::new(id, version, window, cx).with_renderer(this)
        });
        // The closure above runs before `cx.new` returns.
        (
            area,
            skin.expect("DockSkin::new ran inside the constructor"),
        )
    }

    pub fn new(cx: &mut Context<DockArea>) -> Rc<Self> {
        Rc::new(Self {
            shared: Rc::new(SkinShared {
                area: cx.weak_entity(),
                panel_style: Cell::new(PanelStyle::default()),
                toggle_button_visible: Cell::new(true),
                resizing_dock: Cell::new(None),
            }),
        })
    }

    pub(crate) fn shared(&self) -> &Rc<SkinShared> {
        &self.shared
    }

    /// Whether a single-panel tab group draws a plain title or a full tab bar.
    pub fn panel_style(&self) -> PanelStyle {
        self.shared.panel_style()
    }

    pub fn set_panel_style(&self, style: PanelStyle, cx: &mut App) {
        self.shared.panel_style.set(style);
        self.shared.notify(cx);
    }

    /// Whether tab bars offer the affordance that collapses a neighbouring
    /// dock.
    pub fn is_toggle_button_visible(&self) -> bool {
        self.shared.is_toggle_button_visible()
    }

    pub fn set_toggle_button_visible(&self, visible: bool, cx: &mut App) {
        self.shared.toggle_button_visible.set(visible);
        self.shared.notify(cx);
    }
}
