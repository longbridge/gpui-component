use gpui::prelude::FluentBuilder;
use gpui::{
    AnyView, App, AppContext as _, Context, Corner, Decorations, Entity, EntityId, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement, MouseButton, ParentElement, Render,
    RenderOnce, SharedString, Styled, Task, Window, WindowControlArea, div, px,
};
use gpui::{ScrollHandle, StatefulInteractiveElement as _};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::list::{List, ListDelegate, ListState};
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::popover::Popover;
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath, InteractiveElementExt as _, Selectable, Sizable, Size,
    h_flex, v_flex,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// ============================================================================
// TabContainer Events
// ============================================================================

/// Events emitted by TabContent
#[derive(Debug, Clone)]
pub enum TabContentEvent {
    /// Tab state changed
    StateChanged,
}

/// Events emitted by TabContainer
#[derive(Debug, Clone)]
pub enum TabContainerEvent {
    /// Layout has changed (tabs added, removed, reordered, or active index changed)
    LayoutChanged,
    /// A tab was activated
    TabActivated { index: usize, id: String },
    /// A tab was closed
    TabClosed { id: String },
}

// ============================================================================
// State Serialization Structures
// ============================================================================

/// Serializable state for TabContainer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TabContainerState {
    /// Version for compatibility checking
    #[serde(default)]
    pub version: Option<usize>,
    /// All tab states
    pub tabs: Vec<TabItemState>,
    /// Currently active tab index
    pub active_index: usize,
    /// Container UI configuration
    #[serde(default)]
    pub config: TabContainerConfig,
}

impl Default for TabContainerState {
    fn default() -> Self {
        Self {
            version: Some(1),
            tabs: Vec::new(),
            active_index: 0,
            config: TabContainerConfig::default(),
        }
    }
}

/// Serializable state for a single tab
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TabItemState {
    /// Unique tab ID
    pub id: SharedString,
    /// Tab From
    pub from: SharedString,
    /// Tab key
    pub key: SharedString,
    /// Tab-specific data (customized by each content type)
    #[serde(default)]
    pub data: serde_json::Value,
}

/// UI configuration for TabContainer
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TabContainerConfig {
    /// Tab size: "xsmall", "small", "medium", "large"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    /// Left padding in pixels
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left_padding: Option<f32>,
    /// Top padding in pixels
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_padding: Option<f32>,
}

// ============================================================================
// TabContent Trait - Static Type Interface (like Panel)
// ============================================================================

/// Trait that defines tab content behavior.
/// Implement this on your Entity type (like Panel).
/// Requires: Render + Focusable + EventEmitter<TabContentEvent>
#[allow(unused_variables)]
pub trait TabContent: EventEmitter<TabContentEvent> + Render + Focusable {
    /// Unique key for this content type (used for serialization)
    fn content_key(&self) -> &'static str;

    /// Get the tab title
    fn title(&self, cx: &App) -> SharedString;

    /// Get optional icon for the tab
    fn icon(&self, cx: &App) -> Option<Icon> {
        None
    }

    /// Check if tab can be closed
    fn closeable(&self, cx: &App) -> bool {
        true
    }

    /// Called when tab becomes active
    fn on_activate(&mut self, window: &mut Window, cx: &mut Context<Self>) {}

    /// Called when tab becomes inactive
    fn on_deactivate(&mut self, window: &mut Window, cx: &mut Context<Self>) {}

    /// Try to close this tab. Returns a Task that resolves to true if close succeeded.
    fn try_close(
        &mut self,
        tab_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        Task::ready(true)
    }

    /// Get tab's preferred width size
    fn width_size(&self, cx: &App) -> Option<Size> {
        None
    }

    /// Dump tab state to serializable data
    fn dump(&self, cx: &App) -> serde_json::Value {
        serde_json::Value::Null
    }
}

// ============================================================================
// TabContentView Trait - Dynamic Type Interface (like PanelView)
// ============================================================================

/// Dynamic trait object interface for TabContent.
/// This allows storing different TabContent types in a single collection.
#[allow(unused_variables)]
pub trait TabContentView: 'static + Send + Sync {
    fn content_key(&self, cx: &App) -> &'static str;
    fn content_id(&self, cx: &App) -> EntityId;
    fn title(&self, cx: &App) -> SharedString;
    fn icon(&self, cx: &App) -> Option<Icon>;
    fn closeable(&self, cx: &App) -> bool;
    fn on_activate(&self, window: &mut Window, cx: &mut App);
    fn on_deactivate(&self, window: &mut Window, cx: &mut App);
    fn try_close(&self, tab_id: &str, window: &mut Window, cx: &mut App) -> Task<bool>;
    fn width_size(&self, cx: &App) -> Option<Size>;
    fn focus_handle(&self, cx: &App) -> FocusHandle;
    fn view(&self) -> AnyView;
    fn dump(&self, cx: &App) -> serde_json::Value;
}

/// Blanket implementation: Entity<T: TabContent> automatically implements TabContentView
impl<T: TabContent> TabContentView for Entity<T> {
    fn content_key(&self, cx: &App) -> &'static str {
        self.read(cx).content_key()
    }

    fn content_id(&self, _cx: &App) -> EntityId {
        self.entity_id()
    }

    fn title(&self, cx: &App) -> SharedString {
        self.read(cx).title(cx)
    }

    fn icon(&self, cx: &App) -> Option<Icon> {
        self.read(cx).icon(cx)
    }

    fn closeable(&self, cx: &App) -> bool {
        self.read(cx).closeable(cx)
    }

    fn on_activate(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |this, cx| this.on_activate(window, cx))
    }

    fn on_deactivate(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |this, cx| this.on_deactivate(window, cx))
    }

    fn try_close(&self, tab_id: &str, window: &mut Window, cx: &mut App) -> Task<bool> {
        let tab_id = tab_id.to_string();
        self.update(cx, |this, cx| this.try_close(&tab_id, window, cx))
    }

    fn width_size(&self, cx: &App) -> Option<Size> {
        self.read(cx).width_size(cx)
    }

    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.read(cx).focus_handle(cx)
    }

    fn view(&self) -> AnyView {
        self.clone().into()
    }

    fn dump(&self, cx: &App) -> serde_json::Value {
        self.read(cx).dump(cx)
    }
}

impl From<&dyn TabContentView> for AnyView {
    fn from(handle: &dyn TabContentView) -> Self {
        handle.view()
    }
}

impl PartialEq for dyn TabContentView {
    fn eq(&self, other: &Self) -> bool {
        self.view() == other.view()
    }
}

// ============================================================================
// TabItem - Represents a single tab with its content
// ============================================================================

pub struct TabItem {
    id: SharedString,
    from: SharedString,
    content: Arc<dyn TabContentView>,
}

impl TabItem {
    pub fn new<T: TabContent>(
        id: impl Into<String>,
        from: impl Into<String>,
        content: Entity<T>,
    ) -> Self {
        Self {
            id: SharedString::from(id.into()),
            from: SharedString::from(from.into()),
            content: Arc::new(content),
        }
    }

    pub fn id(&self) -> SharedString {
        self.id.clone()
    }

    pub fn from(&self) -> SharedString {
        self.from.clone()
    }

    pub fn content(&self) -> &Arc<dyn TabContentView> {
        &self.content
    }
}

// ============================================================================
// TabContentBuilder - Factory trait for rebuilding tabs
// ============================================================================

/// Trait for building TabContent from serialized state
pub trait TabContentBuilder: Send + Sync {
    fn build(
        &self,
        state: &TabItemState,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Arc<dyn TabContentView>>;
}

/// Function-based builder wrapper
pub struct FnTabContentBuilder<F>(pub F);

impl<F> TabContentBuilder for FnTabContentBuilder<F>
where
    F: Fn(&TabItemState, &mut Window, &mut App) -> Option<Arc<dyn TabContentView>> + Send + Sync,
{
    fn build(
        &self,
        state: &TabItemState,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Arc<dyn TabContentView>> {
        self.0(state, window, cx)
    }
}

// ============================================================================
// TabContentRegistry - Registry for rebuilding tabs from state
// ============================================================================

/// Registry for TabContent builders, used to restore tabs from saved state
#[derive(Clone)]
pub struct TabContentRegistry {
    builders: HashMap<SharedString, Arc<dyn TabContentBuilder>>,
}

impl Default for TabContentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TabContentRegistry {
    pub fn new() -> Self {
        Self {
            builders: HashMap::new(),
        }
    }

    /// Register a builder for a content type
    pub fn register<B: TabContentBuilder + 'static>(
        &mut self,
        content_type: SharedString,
        builder: B,
    ) {
        self.builders.insert(content_type, Arc::new(builder));
    }

    /// Register a builder using a closure
    pub fn register_fn<F>(&mut self, key: SharedString, builder: F)
    where
        F: Fn(&TabItemState, &mut Window, &mut App) -> Option<Arc<dyn TabContentView>>
            + Send
            + Sync
            + 'static,
    {
        self.builders
            .insert(key, Arc::new(FnTabContentBuilder(builder)));
    }

    /// Build a TabContentView from state
    pub fn build(
        &self,
        state: &TabItemState,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Arc<dyn TabContentView>> {
        self.builders.get(&state.key)?.build(state, window, cx)
    }

    /// Check if a builder exists for a content type
    pub fn has_builder(&self, key: &str) -> bool {
        self.builders.contains_key(key)
    }
}

/// Global wrapper for TabContentRegistry
impl gpui::Global for TabContentRegistry {}

// ============================================================================
// TabBarDragState - Window drag state management
// ============================================================================

/// 窗口拖动状态，用于在 Windows 和 Linux 上支持拖动窗口
struct TabBarDragState {
    should_move: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TabBarDragPlan {
    enable_scroll_area_drag: bool,
    enable_single_pinned_tab_drag: bool,
}

fn build_tab_bar_drag_plan(
    show_window_controls: bool,
    has_pinned_tab: bool,
    has_scrollable_tabs: bool,
) -> TabBarDragPlan {
    if !show_window_controls {
        return TabBarDragPlan {
            enable_scroll_area_drag: false,
            enable_single_pinned_tab_drag: false,
        };
    }

    TabBarDragPlan {
        enable_scroll_area_drag: true,
        enable_single_pinned_tab_drag: has_pinned_tab && !has_scrollable_tabs,
    }
}

impl Render for TabBarDragState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

// ============================================================================
// DragTab - Visual representation during drag
// ============================================================================

/// Represents a tab being dragged, used for visual feedback
#[derive(Clone)]
pub struct DragTab {
    pub tab_index: usize,
    pub title: SharedString,
}

impl DragTab {
    pub fn new(tab_index: usize, title: SharedString) -> Self {
        Self { tab_index, title }
    }
}

impl Render for DragTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("drag-tab")
            .cursor_grabbing()
            .py_1()
            .px_3()
            .min_w(px(80.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(px(6.0))
            .text_color(cx.theme().tab_foreground)
            .bg(cx.theme().tab_active)
            .opacity(0.85)
            .shadow_md()
            .text_sm()
            .child(self.title.clone())
    }
}

// ============================================================================
// TabListItem - Custom list item for tab dropdown
// ============================================================================

#[derive(IntoElement)]
pub struct TabListItem {
    tab_index: usize,
    title: SharedString,
    icon: Option<Icon>,
    closeable: bool,
    selected: bool,
    container: Entity<TabContainer>,
}

impl TabListItem {
    pub fn new(
        tab_index: usize,
        title: SharedString,
        icon: Option<Icon>,
        closeable: bool,
        selected: bool,
        container: Entity<TabContainer>,
    ) -> Self {
        Self {
            tab_index,
            title,
            icon,
            closeable,
            selected,
            container,
        }
    }
}

impl Selectable for TabListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for TabListItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let container = self.container.clone();
        let tab_index = self.tab_index;
        let selected = self.selected;
        let drag_border_color = cx.theme().drag_border;
        let drag_title = self.title.clone();

        h_flex()
            .id(SharedString::from(format!("tab-item-{}", tab_index)))
            .w_full()
            .px_2()
            .py_1()
            .rounded(px(4.0))
            .items_center()
            .gap_2()
            .cursor_pointer()
            .when(selected, |el| el.bg(cx.theme().list_active))
            .when(!selected, |el| {
                el.hover(|style| style.bg(cx.theme().list_hover))
            })
            .on_drag(
                DragTab::new(tab_index, drag_title),
                |drag, _, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                },
            )
            .drag_over::<DragTab>(move |el, _, _, _cx| {
                el.border_t_2().border_color(drag_border_color)
            })
            .on_drop(
                window.listener_for(&container, move |this, drag: &DragTab, window, cx| {
                    let from_index = drag.tab_index;
                    let to_index = tab_index;
                    if from_index == to_index {
                        return;
                    }
                    this.move_tab(from_index, to_index, cx);
                    this.set_active_index(to_index, window, cx);
                    if let Some(tab_list) = &this.tab_list {
                        let tabs_data: Vec<(usize, SharedString, Option<Icon>, bool)> = this
                            .tabs
                            .iter()
                            .enumerate()
                            .map(|(idx, tab)| {
                                (
                                    idx,
                                    tab.content().title(cx),
                                    tab.content().icon(cx),
                                    tab.content().closeable(cx),
                                )
                            })
                            .collect();
                        tab_list.update(cx, |state, cx| {
                            let delegate = state.delegate_mut();
                            delegate.tabs = tabs_data.clone();
                            delegate.filtered_tabs = tabs_data;
                            cx.notify();
                        });
                    }
                }),
            )
            .when_some(self.icon, |el, icon| {
                el.child(
                    Icon::new(icon)
                        .size_4()
                        .text_color(cx.theme().muted_foreground),
                )
            })
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(self.title),
            )
            .when(self.closeable, |el| {
                let container = container.clone();
                el.child(
                    div()
                        .id(SharedString::from(format!("close-btn-{}", tab_index)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(16.0))
                        .h(px(16.0))
                        .rounded(px(2.0))
                        .cursor_pointer()
                        .text_color(cx.theme().muted_foreground)
                        .hover(|style| style.bg(cx.theme().muted).text_color(cx.theme().foreground))
                        .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                            container.update(cx, |this, cx| {
                                this.close_tab(tab_index, window, cx).detach();
                            });
                        })
                        .child("×"),
                )
            })
    }
}

// ============================================================================
// TabListDelegate - List delegate for tab dropdown
// ============================================================================

pub struct TabListDelegate {
    container: Entity<TabContainer>,
    tabs: Vec<(usize, SharedString, Option<Icon>, bool)>,
    filtered_tabs: Vec<(usize, SharedString, Option<Icon>, bool)>,
    selected_index: Option<IndexPath>,
}

impl ListDelegate for TabListDelegate {
    type Item = TabListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        if query.is_empty() {
            self.filtered_tabs = self.tabs.clone();
        } else {
            let query_lower = query.to_lowercase();
            self.filtered_tabs = self
                .tabs
                .iter()
                .filter(|(_, title, _, _)| title.to_lowercase().contains(&query_lower))
                .cloned()
                .collect();
        }
        cx.notify();
        Task::ready(())
    }

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.filtered_tabs.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let (tab_index, title, icon, closeable) = self.filtered_tabs.get(ix.row)?.clone();
        let active_index = self.container.read(cx).active_index();
        let is_active = tab_index == active_index;

        Some(TabListItem::new(
            tab_index,
            title,
            icon,
            closeable,
            is_active,
            self.container.clone(),
        ))
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        if let Some(ix) = self.selected_index {
            if let Some((tab_index, _, _, _)) = self.filtered_tabs.get(ix.row) {
                let tab_index = *tab_index;
                self.container.update(cx, |this, cx| {
                    this.list_popover_open = false;
                    this.set_active_index(tab_index, window, cx);
                });
            }
        }
    }

    fn cancel(&mut self, _window: &mut Window, cx: &mut Context<ListState<Self>>) {
        self.container.update(cx, |this, cx| {
            this.list_popover_open = false;
            cx.notify();
        });
    }
}

// ============================================================================
// TabContainer - Main container component
// ============================================================================

pub struct TabContainer {
    focus_handle: FocusHandle,
    tabs: Vec<TabItem>,
    active_index: usize,
    size: Size,
    show_menu: bool,
    tab_bar_bg_color: Option<gpui::Hsla>,
    tab_bar_border_color: Option<gpui::Hsla>,
    active_tab_bg_color: Option<gpui::Hsla>,
    inactive_tab_hover_color: Option<gpui::Hsla>,
    inactive_tab_bg_color: Option<gpui::Hsla>,
    tab_text_color: Option<gpui::Hsla>,
    tab_close_button_color: Option<gpui::Hsla>,
    left_padding: Option<gpui::Pixels>,
    top_padding: Option<gpui::Pixels>,
    tab_bar_scroll_handle: ScrollHandle,
    list_popover_open: bool,
    tab_list: Option<Entity<ListState<TabListDelegate>>>,
    closing_tabs: HashSet<SharedString>,
    show_window_controls: bool,
    /// Pinned tab that stays fixed before the scrollable tab list
    pinned_tab: Option<TabItem>,
    /// Whether the pinned tab is currently active (showing its content)
    pinned_tab_active: bool,
}

impl EventEmitter<TabContainerEvent> for TabContainer {}

impl TabContainer {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _ = window;
        Self {
            focus_handle: cx.focus_handle(),
            tabs: Vec::new(),
            active_index: 0,
            size: Size::Large,
            show_menu: false,
            tab_bar_bg_color: None,
            tab_bar_border_color: None,
            active_tab_bg_color: None,
            inactive_tab_hover_color: None,
            inactive_tab_bg_color: None,
            tab_text_color: None,
            tab_close_button_color: None,
            left_padding: None,
            top_padding: None,
            tab_bar_scroll_handle: ScrollHandle::new(),
            list_popover_open: false,
            tab_list: None,
            closing_tabs: HashSet::new(),
            show_window_controls: false,
            pinned_tab: None,
            pinned_tab_active: false,
        }
    }

    pub fn with_inactive_tab_bg_color(mut self, color: impl Into<Option<gpui::Hsla>>) -> Self {
        self.inactive_tab_bg_color = color.into();
        self
    }

    pub fn with_tab_bar_colors(
        mut self,
        bg_color: impl Into<Option<gpui::Hsla>>,
        border_color: impl Into<Option<gpui::Hsla>>,
    ) -> Self {
        self.tab_bar_bg_color = bg_color.into();
        self.tab_bar_border_color = border_color.into();
        self
    }

    pub fn with_tab_item_colors(
        mut self,
        active_color: impl Into<Option<gpui::Hsla>>,
        hover_color: impl Into<Option<gpui::Hsla>>,
    ) -> Self {
        self.active_tab_bg_color = active_color.into();
        self.inactive_tab_hover_color = hover_color.into();
        self
    }

    pub fn with_tab_content_colors(
        mut self,
        text_color: impl Into<Option<gpui::Hsla>>,
        close_button_color: impl Into<Option<gpui::Hsla>>,
    ) -> Self {
        self.tab_text_color = text_color.into();
        self.tab_close_button_color = close_button_color.into();
        self
    }

    pub fn with_left_padding(mut self, padding: gpui::Pixels) -> Self {
        self.left_padding = Some(padding);
        self
    }

    pub fn with_top_padding(mut self, padding: gpui::Pixels) -> Self {
        self.top_padding = Some(padding);
        self
    }

    pub fn with_window_controls(mut self, show: bool) -> Self {
        self.show_window_controls = show;
        self
    }

    /// Set a pinned tab that stays fixed before the scrollable tab list.
    /// The pinned tab is always visible and cannot be scrolled away.
    pub fn set_pinned_tab(&mut self, tab: TabItem, cx: &mut Context<Self>) {
        self.pinned_tab = Some(tab);
        self.pinned_tab_active = self.tabs.is_empty();
        cx.notify();
    }

    /// Returns whether the pinned tab is currently active.
    pub fn is_pinned_tab_active(&self) -> bool {
        self.pinned_tab_active
    }

    /// Returns whether a pinned tab exists.
    pub fn has_pinned_tab(&self) -> bool {
        self.pinned_tab.is_some()
    }

    /// Activate the pinned tab (deactivate regular tabs visually).
    pub fn activate_pinned_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pinned_tab.is_some() {
            self.pinned_tab_active = true;
            if let Some(pinned) = &self.pinned_tab {
                pinned.content().focus_handle(cx).focus(window, cx);
            }
            cx.notify();
        }
    }

    pub fn set_tab_bar_bg_color(
        &mut self,
        color: impl Into<Option<gpui::Hsla>>,
        cx: &mut Context<Self>,
    ) {
        self.tab_bar_bg_color = color.into();
        cx.notify();
    }

    pub fn set_tab_bar_border_color(
        &mut self,
        color: impl Into<Option<gpui::Hsla>>,
        cx: &mut Context<Self>,
    ) {
        self.tab_bar_border_color = color.into();
        cx.notify();
    }

    pub fn set_active_tab_bg_color(
        &mut self,
        color: impl Into<Option<gpui::Hsla>>,
        cx: &mut Context<Self>,
    ) {
        self.active_tab_bg_color = color.into();
        cx.notify();
    }

    pub fn set_inactive_tab_hover_color(
        &mut self,
        color: impl Into<Option<gpui::Hsla>>,
        cx: &mut Context<Self>,
    ) {
        self.inactive_tab_hover_color = color.into();
        cx.notify();
    }

    /// Add a new tab
    pub fn add_tab(&mut self, tab: TabItem, cx: &mut Context<Self>) {
        self.tabs.push(tab);
        cx.emit(TabContainerEvent::LayoutChanged);
        cx.notify();
    }

    /// Add a new tab and activate it
    pub fn add_and_activate_tab(&mut self, tab: TabItem, cx: &mut Context<Self>) {
        let id = tab.id().to_string();
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        self.pinned_tab_active = false;
        self.tab_bar_scroll_handle
            .scroll_to_item(self.tabs.len() - 1);
        cx.emit(TabContainerEvent::TabActivated {
            index: self.active_index,
            id,
        });
        cx.emit(TabContainerEvent::LayoutChanged);
        cx.notify();
    }

    /// Activate existing tab by ID, or create and activate if not exists (lazy loading)
    pub fn activate_or_add_tab_lazy<F>(
        &mut self,
        tab_id: impl Into<String>,
        create_fn: F,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        F: FnOnce(&mut Window, &mut Context<Self>) -> TabItem,
    {
        let tab_id = tab_id.into();

        if let Some(index) = self.tabs.iter().position(|t| t.id() == tab_id) {
            // 激活现有 tab，复用 set_active_index 逻辑
            self.set_active_index(index, window, cx);
        } else {
            // 创建新 tab 并激活
            let tab = create_fn(window, cx);
            self.add_and_activate_tab_with_focus(tab, window, cx);
        }
    }

    /// Add a new tab, activate it, and focus its content
    pub fn add_and_activate_tab_with_focus(
        &mut self,
        tab: TabItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = tab.id().to_string();
        let focus_handle = tab.content.focus_handle(cx);
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        self.pinned_tab_active = false;
        self.tab_bar_scroll_handle
            .scroll_to_item(self.tabs.len() - 1);

        // 激活新 tab 的 content
        if let Some(new_tab) = self.tabs.get(self.active_index) {
            new_tab.content().on_activate(window, cx);
        }

        // 让 content 获取焦点
        focus_handle.focus(window, cx);

        cx.emit(TabContainerEvent::TabActivated {
            index: self.active_index,
            id,
        });
        cx.emit(TabContainerEvent::LayoutChanged);
        cx.notify();
    }

    /// Close a tab by index
    pub fn close_tab(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        if index >= self.tabs.len() || !self.tabs[index].content().closeable(cx) {
            return Task::ready(false);
        }

        let tab_id = self.tabs[index].id();

        if self.closing_tabs.contains(&tab_id) {
            return Task::ready(false);
        }

        self.closing_tabs.insert(tab_id.clone());

        let tab_id_string = tab_id.to_string();
        let content = self.tabs[index].content().clone();
        let entity = cx.entity();

        let close_task = content.try_close(&tab_id_string, window, cx);

        cx.spawn(async move |_handle, cx| {
            let can_close = close_task.await;
            if can_close {
                let _ = entity.update(cx, |this, cx| {
                    this.do_remove_tab_by_id(&tab_id_string, cx);
                });
            } else {
                let _ = entity.update(cx, |this, _cx| {
                    this.closing_tabs.remove(&tab_id);
                });
            }
            can_close
        })
    }

    fn do_remove_tab_by_id(&mut self, tab_id: &str, cx: &mut Context<Self>) {
        if let Some(index) = self.tabs.iter().position(|t| t.id() == tab_id) {
            let removed_tab_id = self.tabs[index].id();
            self.tabs.remove(index);
            self.closing_tabs.remove(&removed_tab_id);

            if self.tabs.is_empty() {
                // All regular tabs closed, activate pinned tab if present
                self.active_index = 0;
                if self.pinned_tab.is_some() {
                    self.pinned_tab_active = true;
                }
            } else if index < self.active_index {
                self.active_index -= 1;
            } else if index == self.active_index {
                if self.active_index >= self.tabs.len() {
                    self.active_index = self.tabs.len() - 1;
                }
            }

            cx.emit(TabContainerEvent::TabClosed {
                id: tab_id.to_string(),
            });
            cx.emit(TabContainerEvent::LayoutChanged);
            cx.notify();
        }
    }

    /// Close all tabs except the one at the given index
    pub fn close_other_tabs(
        &mut self,
        keep_index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        if keep_index >= self.tabs.len() {
            return Task::ready(true);
        }

        let keep_id = self.tabs[keep_index].id().to_string();
        let tab_ids: Vec<String> = self
            .tabs
            .iter()
            .filter(|t| t.id() != keep_id && t.content().closeable(cx))
            .map(|t| t.id().to_string())
            .collect();

        if tab_ids.is_empty() {
            return Task::ready(true);
        }

        let entity = cx.entity();
        let window_id = cx.active_window();

        cx.spawn(async move |_handle, cx| {
            for tab_id in tab_ids {
                let should_close =
                    cx.update_window(window_id.expect("No active window"), |_, window, cx| {
                        entity.update(cx, |this, cx| {
                            if let Some(index) = this.tabs.iter().position(|t| t.id() == tab_id) {
                                this.set_active_index(index, window, cx);
                                let content = this.tabs[index].content().clone();
                                Some(content.try_close(&tab_id, window, cx))
                            } else {
                                None
                            }
                        })
                    });

                match should_close {
                    Ok(Some(task)) => {
                        let can_close = task.await;
                        if !can_close {
                            return false;
                        }
                        let _ = entity.update(cx, |this, cx| {
                            this.do_remove_tab_by_id(&tab_id, cx);
                        });
                    }
                    Ok(None) => continue,
                    Err(_) => return false,
                }
            }
            true
        })
    }

    /// Close all tabs
    pub fn close_all_tabs(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> Task<bool> {
        let tab_ids: Vec<String> = self
            .tabs
            .iter()
            .filter(|t| t.content().closeable(cx))
            .map(|t| t.id().to_string())
            .collect();

        if tab_ids.is_empty() {
            return Task::ready(true);
        }

        let entity = cx.entity();
        let window_id = cx.active_window();

        cx.spawn(async move |_handle, cx| {
            for tab_id in tab_ids {
                let should_close =
                    cx.update_window(window_id.expect("No active window"), |_, window, cx| {
                        entity.update(cx, |this, cx| {
                            if let Some(index) = this.tabs.iter().position(|t| t.id() == tab_id) {
                                this.set_active_index(index, window, cx);
                                let content = this.tabs[index].content().clone();
                                Some(content.try_close(&tab_id, window, cx))
                            } else {
                                None
                            }
                        })
                    });

                match should_close {
                    Ok(Some(task)) => {
                        let can_close = task.await;
                        if !can_close {
                            return false;
                        }
                        let _ = entity.update(cx, |this, cx| {
                            this.do_remove_tab_by_id(&tab_id, cx);
                        });
                    }
                    Ok(None) => continue,
                    Err(_) => return false,
                }
            }
            true
        })
    }

    /// Close all tabs to the left of the given index
    pub fn close_tabs_to_left(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        if index == 0 || index >= self.tabs.len() {
            return Task::ready(true);
        }

        let tab_ids: Vec<String> = self
            .tabs
            .iter()
            .take(index)
            .filter(|t| t.content().closeable(cx))
            .map(|t| t.id().to_string())
            .collect();

        if tab_ids.is_empty() {
            return Task::ready(true);
        }

        let entity = cx.entity();
        let window_id = cx.active_window();

        cx.spawn(async move |_handle, cx| {
            for tab_id in tab_ids {
                let should_close =
                    cx.update_window(window_id.expect("No active window"), |_, window, cx| {
                        entity.update(cx, |this, cx| {
                            if let Some(idx) = this.tabs.iter().position(|t| t.id() == tab_id) {
                                this.set_active_index(idx, window, cx);
                                let content = this.tabs[idx].content().clone();
                                Some(content.try_close(&tab_id, window, cx))
                            } else {
                                None
                            }
                        })
                    });

                match should_close {
                    Ok(Some(task)) => {
                        let can_close = task.await;
                        if !can_close {
                            return false;
                        }
                        let _ = entity.update(cx, |this, cx| {
                            this.do_remove_tab_by_id(&tab_id, cx);
                        });
                    }
                    Ok(None) => continue,
                    Err(_) => return false,
                }
            }
            true
        })
    }

    /// Close all tabs to the right of the given index
    pub fn close_tabs_to_right(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        if index >= self.tabs.len() - 1 {
            return Task::ready(true);
        }

        let tab_ids: Vec<String> = self
            .tabs
            .iter()
            .skip(index + 1)
            .filter(|t| t.content().closeable(cx))
            .map(|t| t.id().to_string())
            .collect();

        if tab_ids.is_empty() {
            return Task::ready(true);
        }

        let entity = cx.entity();
        let window_id = cx.active_window();

        cx.spawn(async move |_handle, cx| {
            for tab_id in tab_ids {
                let should_close =
                    cx.update_window(window_id.expect("No active window"), |_, window, cx| {
                        entity.update(cx, |this, cx| {
                            if let Some(idx) = this.tabs.iter().position(|t| t.id() == tab_id) {
                                this.set_active_index(idx, window, cx);
                                let content = this.tabs[idx].content().clone();
                                Some(content.try_close(&tab_id, window, cx))
                            } else {
                                None
                            }
                        })
                    });

                match should_close {
                    Ok(Some(task)) => {
                        let can_close = task.await;
                        if !can_close {
                            return false;
                        }
                        let _ = entity.update(cx, |this, cx| {
                            this.do_remove_tab_by_id(&tab_id, cx);
                        });
                    }
                    Ok(None) => continue,
                    Err(_) => return false,
                }
            }
            true
        })
    }

    /// Close a tab by ID
    pub fn close_tab_by_id(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        if let Some(index) = self.tabs.iter().position(|t| t.id() == id) {
            self.close_tab(index, window, cx)
        } else {
            Task::ready(false)
        }
    }

    /// Close all tabs from a specific source
    pub fn close_tabs_by_tab_from(
        &mut self,
        tab_from: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        let tab_ids: Vec<String> = self
            .tabs
            .iter()
            .filter(|t| t.from() == tab_from && t.content().closeable(cx))
            .map(|t| t.id().to_string())
            .collect();

        if tab_ids.is_empty() {
            return Task::ready(true);
        }

        let entity = cx.entity();
        let window_id = cx.active_window();

        cx.spawn(async move |_handle, cx| {
            for tab_id in tab_ids {
                let should_close =
                    cx.update_window(window_id.expect("No active window"), |_, window, cx| {
                        entity.update(cx, |this, cx| {
                            if let Some(index) = this.tabs.iter().position(|t| t.id() == tab_id) {
                                this.set_active_index(index, window, cx);
                                let content = this.tabs[index].content().clone();
                                Some(content.try_close(&tab_id, window, cx))
                            } else {
                                None
                            }
                        })
                    });

                match should_close {
                    Ok(Some(task)) => {
                        let can_close = task.await;
                        if !can_close {
                            return false;
                        }
                        let _ = entity.update(cx, |this, cx| {
                            this.do_remove_tab_by_id(&tab_id, cx);
                        });
                    }
                    Ok(None) => continue,
                    Err(_) => return false,
                }
            }
            true
        })
    }

    /// Force close a tab by ID, skipping try_close
    pub fn force_close_tab_by_id(&mut self, id: &str, cx: &mut Context<Self>) {
        self.do_remove_tab_by_id(id, cx);
    }

    /// Set the active tab by index
    pub fn set_active_index(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index < self.tabs.len() && (index != self.active_index || self.pinned_tab_active) {
            if self.pinned_tab_active {
                // Deactivate pinned tab
                if let Some(pinned) = &self.pinned_tab {
                    pinned.content().on_deactivate(window, cx);
                }
                self.pinned_tab_active = false;
            } else if let Some(old_tab) = self.tabs.get(self.active_index) {
                old_tab.content().on_deactivate(window, cx);
            }

            self.tab_bar_scroll_handle.scroll_to_item(index);
            self.active_index = index;

            let tab_id = if let Some(new_tab) = self.tabs.get(self.active_index) {
                new_tab.content().on_activate(window, cx);
                new_tab.content().focus_handle(cx).focus(window, cx);
                new_tab.id().to_string()
            } else {
                String::new()
            };

            cx.emit(TabContainerEvent::TabActivated { index, id: tab_id });
            cx.emit(TabContainerEvent::LayoutChanged);
            cx.notify();
        }
    }

    /// Set the active tab by ID
    pub fn set_active_by_id(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.tabs.iter().position(|t| t.id() == id) {
            self.set_active_index(index, window, cx);
        }
    }

    /// Get the active tab
    pub fn active_tab(&self) -> Option<&TabItem> {
        self.tabs.get(self.active_index)
    }

    pub fn set_size(&mut self, size: Size, cx: &mut Context<Self>) {
        self.size = size;
        cx.notify();
    }

    pub fn set_show_menu(&mut self, show: bool, cx: &mut Context<Self>) {
        self.show_menu = show;
        cx.notify();
    }

    pub fn tabs(&self) -> &[TabItem] {
        &self.tabs
    }

    pub fn active_index(&self) -> usize {
        self.active_index
    }

    pub fn dump(&self, cx: &App) -> TabContainerState {
        let tabs = self
            .tabs
            .iter()
            .map(|tab| TabItemState {
                id: tab.id(),
                from: tab.from(),
                key: SharedString::from(tab.content().content_key(cx)),
                data: tab.content().dump(cx),
            })
            .collect();

        TabContainerState {
            version: Some(1),
            tabs,
            active_index: self.active_index,
            config: self.dump_config(),
        }
    }

    fn dump_config(&self) -> TabContainerConfig {
        TabContainerConfig {
            size: Some(self.size_to_string()),
            left_padding: self.left_padding.map(|p| f32::from(p)),
            top_padding: self.top_padding.map(|p| f32::from(p)),
        }
    }

    fn size_to_string(&self) -> String {
        match self.size {
            Size::XSmall => "xsmall".to_string(),
            Size::Small => "small".to_string(),
            Size::Medium => "medium".to_string(),
            Size::Large => "large".to_string(),
            Size::Size(pixels) => format!("{}px", f32::from(pixels)),
        }
    }

    fn parse_size(s: &str) -> Size {
        match s {
            "xsmall" => Size::XSmall,
            "small" => Size::Small,
            "medium" => Size::Medium,
            "large" => Size::Large,
            s if s.ends_with("px") => s
                .trim_end_matches("px")
                .parse::<f32>()
                .map(|v| Size::Size(px(v)))
                .unwrap_or(Size::Large),
            _ => Size::Large,
        }
    }

    pub fn load(
        &mut self,
        state: TabContainerState,
        registry: &TabContentRegistry,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.tabs.clear();

        for tab_state in &state.tabs {
            if let Some(content) = registry.build(tab_state, window, cx) {
                self.tabs.push(TabItem {
                    id: tab_state.id.clone(),
                    from: tab_state.from.clone(),
                    content,
                });
            }
        }

        self.active_index = if self.tabs.is_empty() {
            0 // Empty list: active_index is 0 by convention (active_tab() will return None)
        } else {
            state.active_index.min(self.tabs.len() - 1)
        };

        self.load_config(&state.config);
    }

    fn load_config(&mut self, config: &TabContainerConfig) {
        if let Some(size) = &config.size {
            self.size = Self::parse_size(size);
        }
        if let Some(left_padding) = config.left_padding {
            let default_padding = self.left_padding.map(f32::from).unwrap_or(left_padding);
            self.left_padding = Some(px(left_padding.max(default_padding)));
        }
        if let Some(top_padding) = config.top_padding {
            let default_padding = self.top_padding.map(f32::from).unwrap_or(top_padding);
            self.top_padding = Some(px(top_padding.max(default_padding)));
        }
    }

    pub fn move_tab(&mut self, from_index: usize, to_index: usize, cx: &mut Context<Self>) {
        if from_index >= self.tabs.len() || to_index >= self.tabs.len() || from_index == to_index {
            return;
        }

        let tab = self.tabs.remove(from_index);
        self.tabs.insert(to_index, tab);

        if self.active_index == from_index {
            self.active_index = to_index;
        } else {
            match (
                from_index.cmp(&self.active_index),
                to_index.cmp(&self.active_index),
            ) {
                (Ordering::Less, Ordering::Greater | Ordering::Equal) => {
                    self.active_index -= 1;
                }
                (Ordering::Greater, Ordering::Less | Ordering::Equal) => {
                    self.active_index += 1;
                }
                _ => {}
            }
        }

        cx.emit(TabContainerEvent::LayoutChanged);
        cx.notify();
    }

    fn get_tab_width(&self, tab: &TabItem, cx: &App) -> gpui::Pixels {
        let size = tab.content().width_size(cx).unwrap_or(self.size);
        self.size_to_pixels(size)
    }

    fn size_to_pixels(&self, size: Size) -> gpui::Pixels {
        match size {
            Size::Size(pixels) => pixels,
            Size::XSmall => px(60.0),
            Size::Small => px(100.0),
            Size::Medium => px(140.0),
            Size::Large => px(180.0),
        }
    }

    pub fn render_tab_content(&self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let active_view = if self.pinned_tab_active {
            self.pinned_tab.as_ref().map(|tab| tab.content().view())
        } else {
            self.active_tab().map(|tab| tab.content().view())
        };

        div()
            .flex_1()
            .w_full()
            .overflow_hidden()
            .when_some(active_view, |el, view| el.child(view))
    }

    pub fn render_tab_bar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity();

        let theme = cx.theme();
        let bg_color = self.tab_bar_bg_color.unwrap_or(theme.tab);
        let border_color = self.tab_bar_border_color.unwrap_or(theme.border);
        let active_tab_color = self.active_tab_bg_color.unwrap_or(theme.tab_active);
        let hover_tab_color = self
            .inactive_tab_hover_color
            .unwrap_or(theme.tab.opacity(0.8));
        let inactive_tab_color = self.inactive_tab_bg_color.unwrap_or(theme.tab.opacity(0.5));
        let text_color = self.tab_text_color.unwrap_or(theme.tab_foreground);
        let close_btn_color = self
            .tab_close_button_color
            .unwrap_or(theme.muted_foreground);
        let drag_border_color = theme.drag_border;
        let active_index = self.active_index;
        let left_padding = self.left_padding.unwrap_or(px(8.0));

        let tab_list = self.tab_list.clone();

        // 窗口拖动状态管理（仅在 Windows/Linux 上需要，且启用窗口控件时）
        let is_linux = cfg!(target_os = "linux");
        let is_macos = cfg!(target_os = "macos");
        let is_windows = cfg!(target_os = "windows");
        let is_client_decorated = matches!(window.window_decorations(), Decorations::Client { .. });
        let show_window_controls = self.show_window_controls;
        // Windows 下 tab 重排拖拽会与窗口拖动区域冲突，可能导致白屏/崩溃。
        // 先禁用 Windows 的 tab 重排拖拽，保证标题栏拖动稳定性。
        let allow_tab_drag = !is_macos && !is_windows;
        let drag_plan = build_tab_bar_drag_plan(
            show_window_controls,
            self.pinned_tab.is_some(),
            !self.tabs.is_empty(),
        );

        // 使用状态管理窗口拖动
        let drag_state = window.use_state(cx, |_, _| TabBarDragState { should_move: false });

        h_flex()
            .id("tab-bar")
            .w_full()
            .h(px(40.0))
            .bg(bg_color)
            .items_center()
            .border_b_1()
            .border_color(border_color)
            // 窗口拖动支持：仅在非 macOS 且启用窗口控件时生效
            .when(show_window_controls, |this| {
                this.when(is_linux, |this| {
                    this.on_double_click(|_, window, _| window.zoom_window())
                })
                .when(is_macos, |this| {
                    this.on_double_click(|_, window, _| window.titlebar_double_click())
                })
                .on_mouse_down_out(window.listener_for(&drag_state, |state, _, _, _| {
                    state.should_move = false;
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    window.listener_for(&drag_state, |state, _, _, _| {
                        state.should_move = true;
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    window.listener_for(&drag_state, |state, _, _, _| {
                        state.should_move = false;
                    }),
                )
                .on_mouse_move(window.listener_for(
                    &drag_state,
                    |state, _, window, _| {
                        if state.should_move {
                            state.should_move = false;
                            window.start_window_move();
                        }
                    },
                ))
            })
            .when(is_macos, |this| {
                this.child(
                    div()
                        .flex_shrink_0()
                        .h_full()
                        .w(left_padding)
                        .when_some(self.top_padding, |div, padding| div.pt(padding)),
                )
            })
            // Pinned tab (fixed, not scrollable)
            .when_some(self.pinned_tab.as_ref(), |this, pinned| {
                let pinned_title = pinned.content().title(cx);
                let pinned_icon = pinned.content().icon(cx);
                let is_pinned_active = self.pinned_tab_active;
                let view_for_pinned = view.clone();
                let top_padding = self.top_padding;

                this.child(
                    div()
                        .id("pinned-tab")
                        .flex()
                        .flex_shrink_0()
                        .overflow_hidden()
                        .items_center()
                        .gap_2()
                        .h(px(32.0))
                        .px_3()
                        .when(!is_macos, |el| el.ml(left_padding))
                        .when_some(top_padding, |el, padding| el.mt(padding))
                        .rounded(px(6.0))
                        .when(is_pinned_active, |el| el.bg(active_tab_color))
                        .when(!is_pinned_active, |el| {
                            el.hover(move |style| style.bg(hover_tab_color))
                                .bg(inactive_tab_color)
                        })
                        .when(drag_plan.enable_single_pinned_tab_drag, |el| {
                            el.window_control_area(WindowControlArea::Drag)
                                .on_mouse_down_out(window.listener_for(
                                    &drag_state,
                                    |state, _, _, _| {
                                        state.should_move = false;
                                    },
                                ))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    window.listener_for(&drag_state, |state, _, _, _| {
                                        state.should_move = true;
                                    }),
                                )
                                .on_mouse_up(
                                    MouseButton::Left,
                                    window.listener_for(&drag_state, |state, _, _, _| {
                                        state.should_move = false;
                                    }),
                                )
                                .on_mouse_move(window.listener_for(
                                    &drag_state,
                                    |state, _, window, _| {
                                        if state.should_move {
                                            state.should_move = false;
                                            window.start_window_move();
                                        }
                                    },
                                ))
                        })
                        .when(!drag_plan.enable_single_pinned_tab_drag, |el| {
                            el.cursor_pointer()
                                // pinned tab 在存在普通 tab 时只是一个普通可点击页签，
                                // 需要阻止事件冒泡到标题栏拖动区域。
                                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                    window.prevent_default();
                                    cx.stop_propagation();
                                })
                                .on_mouse_move(|_, window, cx| {
                                    window.prevent_default();
                                    cx.stop_propagation();
                                })
                                .on_click(move |_, window, cx| {
                                    view_for_pinned.update(cx, |this, cx| {
                                        this.activate_pinned_tab(window, cx);
                                    });
                                })
                        })
                        .when_some(pinned_icon, |el, icon| {
                            el.child(div().flex_shrink_0().flex().items_center().child(icon))
                        })
                        .child(
                            div()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_sm()
                                .text_color(text_color)
                                .text_ellipsis()
                                .child(pinned_title.to_string()),
                        ),
                )
                // Separator between pinned tab and scrollable tabs
                .child(
                    div()
                        .flex_shrink_0()
                        .mx_1()
                        .when_some(top_padding, |el, padding| el.mt(padding))
                        .w(px(1.0))
                        .h(px(16.0))
                        .bg(border_color),
                )
            })
            .child(
                h_flex()
                    .id("tabs")
                    .flex_1()
                    // 仅在启用窗口控件时设置拖动区域（用于 Windows 原生拖动）
                    .when(drag_plan.enable_scroll_area_drag, |this| {
                        this.window_control_area(WindowControlArea::Drag)
                            .on_mouse_down_out(window.listener_for(
                                &drag_state,
                                |state, _, _, _| {
                                    state.should_move = false;
                                },
                            ))
                            .on_mouse_down(
                                MouseButton::Left,
                                window.listener_for(&drag_state, |state, _, _, _| {
                                    state.should_move = true;
                                }),
                            )
                            .on_mouse_up(
                                MouseButton::Left,
                                window.listener_for(&drag_state, |state, _, _, _| {
                                    state.should_move = false;
                                }),
                            )
                            .on_mouse_move(window.listener_for(
                                &drag_state,
                                |state, _, window, _| {
                                    if state.should_move {
                                        state.should_move = false;
                                        window.start_window_move();
                                    }
                                },
                            ))
                    })
                    .overflow_x_scroll()
                    .when(!is_macos && self.pinned_tab.is_none(), |this| {
                        this.pl(left_padding)
                    })
                    .when_some(self.top_padding, |div, padding| div.pt(padding))
                    .pr_2()
                    .gap_1()
                    .track_scroll(&self.tab_bar_scroll_handle)
                    // Linux 客户端装饰模式下，右键显示窗口菜单
                    .when(
                        is_linux && is_client_decorated && show_window_controls,
                        |this| {
                            this.child(
                                div()
                                    .top_0()
                                    .left_0()
                                    .absolute()
                                    .size_full()
                                    .h_full()
                                    .on_mouse_down(MouseButton::Right, move |ev, window, _| {
                                        window.show_window_menu(ev.position)
                                    }),
                            )
                        },
                    )
                    .children(self.tabs.iter().enumerate().map(|(idx, tab)| {
                        let title = tab.content().title(cx);
                        let icon = tab.content().icon(cx);
                        let closeable = tab.content().closeable(cx);
                        let is_active = idx == active_index;
                        let view_clone = view.clone();
                        let title_clone = title.clone();
                        let tab_width = self.get_tab_width(tab, cx);

                        div()
                            .id(idx)
                            .flex()
                            .flex_shrink_0()
                            .overflow_hidden()
                            .items_center()
                            .gap_2()
                            .h(px(32.0))
                            .text_ellipsis()
                            .w(tab_width)
                            .px_3()
                            .rounded(px(6.0))
                            .when(is_active, |el| el.bg(active_tab_color))
                            .when(!is_active, |el| {
                                el.hover(move |style| style.bg(hover_tab_color))
                                    .bg(inactive_tab_color)
                            })
                            // 普通 tab 不应把拖动/按下事件冒泡为窗口拖动。
                            .on_mouse_down(MouseButton::Left, move |_evt, window: &mut Window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                            })
                            .on_mouse_move(move |_evt, window: &mut Window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                            })
                            .when(allow_tab_drag, |el| {
                                el.cursor_grab()
                                    .on_drag(
                                        DragTab::new(idx, title.clone()),
                                        |drag, _, window, cx| {
                                            window.prevent_default();
                                            cx.stop_propagation();
                                            cx.new(|_| drag.clone())
                                        },
                                    )
                                    .drag_over::<DragTab>(move |el, _, _, _cx| {
                                        el.border_l_2().border_color(drag_border_color)
                                    })
                                    .on_drop(cx.listener(
                                        move |this, drag: &DragTab, window, cx| {
                                            let from_idx = drag.tab_index;
                                            let to_idx = idx;
                                            if from_idx != to_idx {
                                                this.move_tab(from_idx, to_idx, cx);
                                            }
                                            this.set_active_index(to_idx, window, cx);
                                        },
                                    ))
                            })
                            .on_click(cx.listener(move |this, _event, window, cx| {
                                window.prevent_default();
                                this.set_active_index(idx, window, cx);
                            }))
                            .when_some(icon, |el, icon| {
                                el.child(div().flex_shrink_0().flex().items_center().child(icon))
                            })
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_sm()
                                    .text_color(text_color)
                                    .text_ellipsis()
                                    .child(title_clone.to_string()),
                            )
                            .when(closeable, |el| {
                                let view_clone = view_clone.clone();
                                el.child(
                                    div()
                                        .flex_shrink_0()
                                        .w(px(16.0))
                                        .h(px(16.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(2.0))
                                        .cursor_pointer()
                                        .text_color(close_btn_color)
                                        .hover(|style| {
                                            style.bg(gpui::rgb(0x5a5a5a)).text_color(text_color)
                                        })
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            move |_event, window, cx| {
                                                cx.stop_propagation();
                                                view_clone.update(cx, |this, cx| {
                                                    this.close_tab(idx, window, cx).detach();
                                                });
                                            },
                                        )
                                        .child("×"),
                                )
                            })
                            .context_menu(move |menu, window, cx| {
                                let view_for_menu = view_clone.clone();
                                let tab_count = view_for_menu.read(cx).tabs.len();
                                let has_tabs_left = idx > 0;
                                let has_tabs_right = idx < tab_count - 1;
                                let closeable = view_for_menu
                                    .read(cx)
                                    .tabs
                                    .get(idx)
                                    .map(|tab| tab.content().closeable(cx))
                                    .unwrap_or(false);

                                menu.item(
                                    PopupMenuItem::new("Close").disabled(!closeable).on_click(
                                        window.listener_for(
                                            &view_for_menu,
                                            move |this, _, window, cx| {
                                                this.close_tab(idx, window, cx).detach();
                                            },
                                        ),
                                    ),
                                )
                                .item(PopupMenuItem::new("Close All").on_click(
                                    window.listener_for(
                                        &view_for_menu,
                                        move |this, _, window, cx| {
                                            this.close_all_tabs(window, cx).detach();
                                        },
                                    ),
                                ))
                                .item(
                                    PopupMenuItem::new("Close Others")
                                        .disabled(tab_count <= 1)
                                        .on_click(window.listener_for(
                                            &view_for_menu,
                                            move |this, _, window, cx| {
                                                this.close_other_tabs(idx, window, cx).detach();
                                            },
                                        )),
                                )
                                .item(
                                    PopupMenuItem::new("Close Tabs To The Left")
                                        .disabled(!has_tabs_left)
                                        .on_click(window.listener_for(
                                            &view_for_menu,
                                            move |this, _, window, cx| {
                                                this.close_tabs_to_left(idx, window, cx).detach();
                                            },
                                        )),
                                )
                                .item(
                                    PopupMenuItem::new("Close Tabs To The Right")
                                        .disabled(!has_tabs_right)
                                        .on_click(window.listener_for(
                                            &view_for_menu,
                                            move |this, _, window, cx| {
                                                this.close_tabs_to_right(idx, window, cx).detach();
                                            },
                                        )),
                                )
                            })
                    })),
            )
            .child(
                Popover::new("tab-list-popover")
                    .anchor(Corner::TopRight)
                    .p_0()
                    .open(self.list_popover_open)
                    .on_open_change(cx.listener(move |this, open, window, cx| {
                        this.list_popover_open = *open;
                        if *open {
                            let tabs_data: Vec<(usize, SharedString, Option<Icon>, bool)> = this
                                .tabs
                                .iter()
                                .enumerate()
                                .map(|(idx, tab)| {
                                    (
                                        idx,
                                        tab.content().title(cx),
                                        tab.content().icon(cx),
                                        tab.content().closeable(cx),
                                    )
                                })
                                .collect();
                            let container = cx.entity();

                            if let Some(tab_list) = &this.tab_list {
                                tab_list.update(cx, |state, _| {
                                    let delegate = state.delegate_mut();
                                    delegate.tabs = tabs_data.clone();
                                    delegate.filtered_tabs = tabs_data;
                                });
                            } else {
                                this.tab_list = Some(cx.new(|cx| {
                                    ListState::new(
                                        TabListDelegate {
                                            container,
                                            tabs: tabs_data.clone(),
                                            filtered_tabs: tabs_data,
                                            selected_index: None,
                                        },
                                        window,
                                        cx,
                                    )
                                    .searchable(true)
                                }));
                            }
                        }
                        cx.notify();
                    }))
                    .when_some(tab_list.as_ref(), |popover, list| {
                        popover.track_focus(&list.focus_handle(cx))
                    })
                    .trigger(
                        Button::new("tab-dropdown-btn")
                            .icon(IconName::ChevronDown)
                            .ghost()
                            .compact(),
                    )
                    .when_some(tab_list, |popover, list| {
                        popover.child(
                            List::new(&list)
                                .w(px(280.0))
                                .max_h(px(300.0))
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded(cx.theme().radius),
                        )
                    }),
            )
            .when(
                cfg!(not(target_os = "macos")) && self.show_window_controls,
                |el| el.child(self.render_window_controls(window)),
            )
    }

    fn render_window_controls(&self, window: &mut Window) -> impl IntoElement {
        let is_linux = cfg!(target_os = "linux");
        let is_windows = cfg!(target_os = "windows");
        let is_maximized = window.is_maximized();

        h_flex()
            .id("window-controls")
            .items_center()
            .flex_shrink_0()
            .h_full()
            .child(self.render_control_button(
                "minimize",
                IconName::WindowMinimize,
                WindowControlArea::Min,
                is_linux,
                is_windows,
                false,
            ))
            .child(self.render_control_button(
                if is_maximized { "restore" } else { "maximize" },
                if is_maximized {
                    IconName::WindowRestore
                } else {
                    IconName::WindowMaximize
                },
                WindowControlArea::Max,
                is_linux,
                is_windows,
                false,
            ))
            .child(self.render_control_button(
                "close",
                IconName::WindowClose,
                WindowControlArea::Close,
                is_linux,
                is_windows,
                true,
            ))
    }

    fn render_control_button(
        &self,
        id: &'static str,
        icon: IconName,
        control_area: WindowControlArea,
        is_linux: bool,
        is_windows: bool,
        is_close: bool,
    ) -> impl IntoElement {
        div()
            .id(id)
            .flex()
            .w(px(34.0))
            .h_full()
            .flex_shrink_0()
            .justify_center()
            .content_center()
            .items_center()
            .text_color(gpui::white())
            .hover(move |style| {
                if is_close {
                    style.bg(gpui::rgb(0xe81123)).text_color(gpui::white())
                } else {
                    style.bg(gpui::rgb(0x3a3a3a)).text_color(gpui::white())
                }
            })
            .active(move |style| {
                if is_close {
                    style.bg(gpui::rgb(0xc50f1f)).text_color(gpui::white())
                } else {
                    style.bg(gpui::rgb(0x2a2a2a)).text_color(gpui::white())
                }
            })
            .when(is_windows, move |this| {
                // Windows 依赖系统原生标题栏控件行为：
                // 仅声明 control area，避免手动 on_click 干扰最大化/还原切换。
                this.window_control_area(control_area)
            })
            .when(is_linux, move |this| {
                this.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    match control_area {
                        WindowControlArea::Min => window.minimize_window(),
                        WindowControlArea::Max => window.zoom_window(),
                        WindowControlArea::Close => window.remove_window(),
                        _ => {}
                    }
                })
            })
            .child(Icon::new(icon).with_size(Size::Small))
    }
}

impl Focusable for TabContainer {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        if self.pinned_tab_active {
            if let Some(pinned) = &self.pinned_tab {
                return pinned.content().focus_handle(cx);
            }
        }
        if let Some(active_tab) = self.active_tab() {
            active_tab.content().focus_handle(cx)
        } else {
            self.focus_handle.clone()
        }
    }
}

impl Render for TabContainer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle(cx);

        div()
            .id("tab-container")
            .track_focus(&focus_handle)
            .relative()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(self.render_tab_bar(window, cx))
                    .child(self.render_tab_content(window, cx)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{TabBarDragPlan, build_tab_bar_drag_plan};

    #[test]
    fn build_tab_bar_drag_plan_enables_pinned_drag_for_single_home_tab() {
        let plan = build_tab_bar_drag_plan(true, true, false);

        assert_eq!(
            plan,
            TabBarDragPlan {
                enable_scroll_area_drag: true,
                enable_single_pinned_tab_drag: true,
            }
        );
    }

    #[test]
    fn build_tab_bar_drag_plan_keeps_pinned_drag_disabled_when_other_tabs_exist() {
        let plan = build_tab_bar_drag_plan(true, true, true);

        assert_eq!(
            plan,
            TabBarDragPlan {
                enable_scroll_area_drag: true,
                enable_single_pinned_tab_drag: false,
            }
        );
    }

    #[test]
    fn build_tab_bar_drag_plan_disables_all_drag_without_window_controls() {
        let plan = build_tab_bar_drag_plan(false, true, false);

        assert_eq!(
            plan,
            TabBarDragPlan {
                enable_scroll_area_drag: false,
                enable_single_pinned_tab_drag: false,
            }
        );
    }
}
