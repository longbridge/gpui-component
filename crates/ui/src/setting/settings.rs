use crate::{
    group_box::GroupBoxVariant,
    resizable::{h_resizable, resizable_panel},
    setting::{SettingGroup, SettingPage},
    sidebar::{Sidebar, SidebarHeader, SidebarMenu, SidebarMenuItem},
};
use gpui::{
    div, prelude::FluentBuilder as _, px, relative, App, ElementId, Entity, IntoElement,
    ParentElement as _, RenderOnce, SharedString, Styled as _, Window,
};

/// The settings structure containing multiple pages for app settings.
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
    pages: Vec<SettingPage>,
    group_variant: GroupBoxVariant,
}

impl Settings {
    /// Create a new settings with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            pages: vec![],
            group_variant: GroupBoxVariant::default(),
        }
    }

    /// Add a page to the settings.
    pub fn page(mut self, page: SettingPage) -> Self {
        self.pages.push(page);
        self
    }

    /// Add pages to the settings.
    pub fn pages(mut self, pages: impl IntoIterator<Item = SettingPage>) -> Self {
        self.pages.extend(pages);
        self
    }

    /// Set the default variant for all setting groups.
    ///
    /// All setting groups will use this variant unless overridden individually.
    pub fn group_variant(mut self, variant: GroupBoxVariant) -> Self {
        self.group_variant = variant;
        self
    }

    fn filtered_pages(&self, query: &str) -> Vec<SettingPage> {
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
                            .filter(|item| item.is_match(&query))
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
                    .render(ix, state, self.group_variant, window, cx)
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
                SidebarMenu::new()
                    .p_2()
                    .children(pages.iter().enumerate().map(|(page_ix, page)| {
                        let is_page_active =
                            selected_index.page_ix == page_ix && selected_index.group_ix.is_none();
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
                            .when(page.groups.len() > 1, |this| {
                                this.children(
                                    page.groups
                                        .iter()
                                        .filter(|g| g.title.is_some())
                                        .enumerate()
                                        .map(|(group_ix, group)| {
                                            let is_active = selected_index.page_ix == page_ix
                                                && selected_index.group_ix == Some(group_ix);
                                            let title = group.title.clone().unwrap_or_default();

                                            SidebarMenuItem::new(title).active(is_active).on_click(
                                                {
                                                    let state = state.clone();
                                                    move |_, _, cx| {
                                                        state.update(cx, |state, cx| {
                                                            state.selected_index = SelectIndex {
                                                                page_ix,
                                                                group_ix: Some(group_ix),
                                                            };
                                                            state.deferred_scroll_group_ix =
                                                                Some(group_ix);
                                                            cx.notify();
                                                        })
                                                    }
                                                },
                                            )
                                        }),
                                )
                            })
                    })),
            )
    }
}

pub(super) struct SettingsState {
    pub(super) query: SharedString,
    pub(super) selected_index: SelectIndex,
    /// If set, defer scrolling to this group index after rendering.
    pub(super) deferred_scroll_group_ix: Option<usize>,
}

#[derive(Clone, Copy, Default)]
pub(super) struct SelectIndex {
    page_ix: usize,
    group_ix: Option<usize>,
}

impl RenderOnce for Settings {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| SettingsState {
            query: SharedString::default(),
            selected_index: SelectIndex::default(),
            deferred_scroll_group_ix: None,
        });
        let query = state.read(cx).query.clone();
        let filtered_pages = self.filtered_pages(&query);

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
