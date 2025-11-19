mod fields;
mod group;
mod item;
mod page;

pub use fields::NumberFieldOptions;
pub use group::*;
pub use item::*;
pub use page::*;

use crate::{
    group_box::GroupBoxVariant,
    history::{History, HistoryItem},
    resizable::{h_resizable, resizable_panel},
    sidebar::{Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
};
use gpui::{
    div, px, relative, App, AppContext as _, ElementId, Entity, IntoElement, ParentElement as _,
    RenderOnce, SharedString, Styled as _, Window,
};

/// The settings structure containing multiple sections for app settings.
///
/// The hierarchy of settings is as follows:
///
/// ```ignore
/// Settings
///   SettingPage     <- The single active page displayed
///     SettingGroup
///       SettingItem
///         Label
///         SettingField (e.g., Switch, Dropdown, Input)
/// ```
#[derive(IntoElement)]
pub struct Settings {
    id: ElementId,
    query: SharedString,
    pages: Vec<SettingPage>,
    group_variant: GroupBoxVariant,
}

impl Settings {
    /// Create a new settings structure with the given ID.
    pub fn new(id: impl Into<ElementId>, pages: Vec<SettingPage>) -> Self {
        Self {
            id: id.into(),
            query: SharedString::default(),
            pages,
            group_variant: GroupBoxVariant::default(),
        }
    }

    /// Set the search query for filtering settings.
    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    /// Add pages to the settings.
    pub fn pages(mut self, pages: impl IntoIterator<Item = SettingPage>) -> Self {
        self.pages = pages.into_iter().collect();
        self
    }

    /// Set the variant for all setting groups.
    pub fn group_variant(mut self, variant: GroupBoxVariant) -> Self {
        self.group_variant = variant;
        self
    }

    fn filtered_pages(&self) -> Vec<SettingPage> {
        if self.query.is_empty() {
            return self.pages.clone();
        }

        self.pages
            .iter()
            .filter_map(|page| {
                let filtered_groups: Vec<SettingGroup> = page
                    .groups
                    .iter()
                    .filter_map(|group| {
                        let mut group = group.clone();
                        group.items = group
                            .items
                            .iter()
                            .filter(|item| item.is_match(&self.query))
                            .cloned()
                            .collect();
                        if group.items.is_empty() {
                            None
                        } else {
                            Some(group)
                        }
                    })
                    .collect();
                let mut page = page.clone();
                page.groups = filtered_groups;
                if page.groups.is_empty() {
                    None
                } else {
                    Some(page)
                }
            })
            .collect()
    }

    fn render_active_page(
        &self,
        state: &Entity<SettingsState>,
        pages: &Vec<SettingPage>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let selected_index = state.read(cx).selected_index;

        for (ix, page) in pages.into_iter().enumerate() {
            if selected_index.page_ix == ix {
                return page
                    .render(ix, &self.query, self.group_variant, window, cx)
                    .into_any_element();
            }
        }

        return div().into_any_element();
    }

    fn render_sidebar(
        &self,
        state: &Entity<SettingsState>,
        pages: &Vec<SettingPage>,
        _: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let selected_index = state.read(cx).selected_index;
        Sidebar::left()
            .width(relative(1.))
            .border_width(px(0.))
            .collapsed(false)
            .header(SidebarHeader::new().child("Search Input"))
            .child(
                SidebarGroup::new("Settings").child(SidebarMenu::new().children(
                    pages.iter().enumerate().map(|(page_ix, page)| {
                        let is_page_active = selected_index.page_ix == page_ix;
                        SidebarMenuItem::new(page.title.clone())
                            .active(is_page_active)
                            .on_click({
                                let state = state.clone();
                                move |_, _, cx| {
                                    state.update(cx, |state, cx| {
                                        state.selected_index = SelectIndex {
                                            page_ix,
                                            ..Default::default()
                                        };
                                        cx.notify();
                                    })
                                }
                            })
                            .children(page.groups.iter().enumerate().map(|(group_ix, group)| {
                                let is_active = selected_index.page_ix == page_ix
                                    && selected_index.group_ix == Some(group_ix);
                                SidebarMenuItem::new(group.title.clone())
                                    .active(is_active)
                                    .on_click({
                                        let state = state.clone();
                                        move |_, _, cx| {
                                            state.update(cx, |state, cx| {
                                                state.selected_index = SelectIndex {
                                                    page_ix,
                                                    group_ix: Some(group_ix),
                                                };
                                                cx.notify();
                                            })
                                        }
                                    })
                            }))
                    }),
                )),
            )
    }
}

struct SettingsState {
    history: History<ElementId>,
    selected_index: SelectIndex,
}

#[derive(Clone, Copy, Default)]
struct SelectIndex {
    page_ix: usize,
    group_ix: Option<usize>,
}

impl HistoryItem for ElementId {
    fn version(&self) -> usize {
        0
    }

    fn set_version(&mut self, _: usize) {}
}

impl RenderOnce for Settings {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let filtered_pages = self.filtered_pages();
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| SettingsState {
            selected_index: SelectIndex::default(),
            history: History::new().max_undos(1000),
        });

        h_resizable(self.id.clone())
            .child(resizable_panel().size(px(300.)).child(self.render_sidebar(
                &state,
                &filtered_pages,
                window,
                cx,
            )))
            .child(resizable_panel().child(self.render_active_page(
                &state,
                &filtered_pages,
                window,
                cx,
            )))
    }
}
