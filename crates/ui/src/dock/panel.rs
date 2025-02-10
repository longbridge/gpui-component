use std::{collections::HashMap, sync::Arc};

use crate::{button::Button, popup_menu::PopupMenu};
use gpui::{
    AnyElement, AnyView, App, Entity, EventEmitter, FocusHandle, Focusable, Global, Hsla,
    IntoElement, Render, SharedString, WeakEntity, Window,
};

use rust_i18n::t;

use super::{DockArea, PanelInfo, PanelState};

pub enum PanelEvent {
    ZoomIn,
    ZoomOut,
    LayoutChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelStyle {
    /// Display the TabBar when there are multiple tabs, otherwise display the simple title.
    Default,
    /// Always display the tab bar.
    TabBar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TitleStyle {
    pub background: Hsla,
    pub foreground: Hsla,
}

#[derive(Clone, Copy, Default)]
pub enum PanelControl {
    Both,
    #[default]
    Menu,
    Toolbar,
}

impl PanelControl {
    #[inline]
    pub fn toolbar_visible(&self) -> bool {
        matches!(self, PanelControl::Both | PanelControl::Toolbar)
    }

    #[inline]
    pub fn menu_visible(&self) -> bool {
        matches!(self, PanelControl::Both | PanelControl::Menu)
    }
}

/// The Panel trait used to define the panel.
#[allow(unused_variables)]
pub trait Panel: EventEmitter<PanelEvent> + Render + Focusable {
    /// The name of the panel used to serialize, deserialize and identify the panel.
    ///
    /// This is used to identify the panel when deserializing the panel.
    /// Once you have defined a panel name, this must not be changed.
    fn panel_name(&self) -> &'static str;

    /// The title of the panel
    fn title(&self, window: &Window, cx: &App) -> AnyElement {
        SharedString::from(t!("Dock.Unnamed")).into_any_element()
    }

    /// The theme of the panel title, default is `None`.
    fn title_style(&self, cx: &App) -> Option<TitleStyle> {
        None
    }

    /// Whether the panel can be closed, default is `true`.
    ///
    /// This method called in Panel render, we should make sure it is fast.
    fn closable(&self, cx: &App) -> bool {
        true
    }

    /// Return `PanelControl` if the panel is zoomable, default is `PanelControl::Menu`.
    ///
    /// This method called in Panel render, we should make sure it is fast.
    fn zoomable(&self, cx: &App) -> Option<PanelControl> {
        Some(PanelControl::Menu)
    }

    /// Return false to hide panel, true to show panel, default is `true`.
    ///
    /// This method called in Panel render, we should make sure it is fast.
    fn visible(&self, cx: &App) -> bool {
        true
    }

    /// Set active state of the panel.
    ///
    /// This method will be called when the panel is active or inactive.
    ///
    /// The last_active_panel and current_active_panel will be touched when the panel is active.
    #[allow(unused_variables)]
    fn set_active(&self, active: bool, window: &Window, cx: &App) {}

    /// Set zoomed state of the panel.
    ///
    /// This method will be called when the panel is zoomed or unzoomed.
    ///
    /// Only current Panel will touch this method.
    fn set_zoomed(&self, zoomed: bool, window: &Window, cx: &App) {}

    /// The addition popup menu of the panel, default is `None`.
    fn popup_menu(&self, this: PopupMenu, window: &Window, cx: &App) -> PopupMenu {
        this
    }

    /// The addition toolbar buttons of the panel used to show in the right of the title bar, default is `None`.
    fn toolbar_buttons(&self, window: &mut Window, cx: &mut App) -> Option<Vec<Button>> {
        None
    }

    /// Dump the panel, used to serialize the panel.
    fn dump(&self, cx: &App) -> PanelState {
        PanelState::new(self)
    }

    /// Whether the panel has inner padding when the panel is in the tabs layout, default is `true`.
    fn inner_padding(&self, cx: &App) -> bool {
        true
    }
}

/// The PanelView trait used to define the panel view.
#[allow(unused_variables)]
pub trait PanelView: 'static + Send + Sync {
    fn panel_name(&self, cx: &App) -> &'static str;
    fn title(&self, window: &Window, cx: &App) -> AnyElement;
    fn title_style(&self, cx: &App) -> Option<TitleStyle>;
    fn closable(&self, cx: &App) -> bool;
    fn zoomable(&self, cx: &App) -> Option<PanelControl>;
    fn visible(&self, cx: &App) -> bool;
    fn set_active(&self, active: bool, window: &mut Window, cx: &mut App);
    fn set_zoomed(&self, zoomed: bool, window: &mut Window, cx: &mut App);
    fn popup_menu(&self, menu: PopupMenu, window: &Window, cx: &App) -> PopupMenu;
    fn toolbar_buttons(&self, window: &mut Window, cx: &mut App) -> Option<Vec<Button>>;
    fn view(&self) -> AnyView;
    fn focus_handle(&self, cx: &App) -> FocusHandle;
    fn dump(&self, cx: &App) -> PanelState;
    fn inner_padding(&self, cx: &App) -> bool;
}

impl<T: Panel> PanelView for Entity<T> {
    fn panel_name(&self, cx: &App) -> &'static str {
        self.read(cx).panel_name()
    }

    fn title(&self, window: &Window, cx: &App) -> AnyElement {
        self.read(cx).title(window, cx)
    }

    fn title_style(&self, cx: &App) -> Option<TitleStyle> {
        self.read(cx).title_style(cx)
    }

    fn closable(&self, cx: &App) -> bool {
        self.read(cx).closable(cx)
    }

    fn zoomable(&self, cx: &App) -> Option<PanelControl> {
        self.read(cx).zoomable(cx)
    }

    fn visible(&self, cx: &App) -> bool {
        self.read(cx).visible(cx)
    }

    fn set_active(&self, active: bool, window: &mut Window, cx: &mut App) {
        self.update(cx, |this, cx| {
            this.set_active(active, window, cx);
        })
    }

    fn set_zoomed(&self, zoomed: bool, window: &mut Window, cx: &mut App) {
        self.update(cx, |this, cx| {
            this.set_zoomed(zoomed, window, cx);
        })
    }

    fn popup_menu(&self, menu: PopupMenu, window: &Window, cx: &App) -> PopupMenu {
        self.read(cx).popup_menu(menu, window, cx)
    }

    fn toolbar_buttons(&self, window: &mut Window, cx: &mut App) -> Option<Vec<Button>> {
        self.update(cx, |this, cx| this.toolbar_buttons(window, cx))
    }

    fn view(&self) -> AnyView {
        self.clone().into()
    }

    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.read(cx).focus_handle(cx)
    }

    fn dump(&self, cx: &App) -> PanelState {
        self.read(cx).dump(cx)
    }

    fn inner_padding(&self, cx: &App) -> bool {
        self.read(cx).inner_padding(cx)
    }
}

impl From<&dyn PanelView> for AnyView {
    fn from(handle: &dyn PanelView) -> Self {
        handle.view()
    }
}

impl<T: Panel> From<&dyn PanelView> for Entity<T> {
    fn from(value: &dyn PanelView) -> Self {
        value.view().downcast::<T>().unwrap()
    }
}

impl PartialEq for dyn PanelView {
    fn eq(&self, other: &Self) -> bool {
        self.view() == other.view()
    }
}

pub struct PanelRegistry {
    pub(super) items: HashMap<
        String,
        Arc<
            dyn Fn(
                WeakEntity<DockArea>,
                &PanelState,
                &PanelInfo,
                &mut Window,
                &mut App,
            ) -> Box<dyn PanelView>,
        >,
    >,
}
impl PanelRegistry {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }
}
impl Global for PanelRegistry {}

/// Register the Panel init by panel_name to global registry.
pub fn register_panel<F>(cx: &mut App, panel_name: &str, deserialize: F)
where
    F: Fn(
            WeakEntity<DockArea>,
            &PanelState,
            &PanelInfo,
            &mut Window,
            &mut App,
        ) -> Box<dyn PanelView>
        + 'static,
{
    if let None = cx.try_global::<PanelRegistry>() {
        cx.set_global(PanelRegistry::new());
    }

    cx.global_mut::<PanelRegistry>()
        .items
        .insert(panel_name.to_string(), Arc::new(deserialize));
}
