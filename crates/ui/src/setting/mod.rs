mod group;
mod item;
mod page;

pub use group::*;
pub use item::*;
pub use page::*;

use crate::{
    history::{History, HistoryItem},
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    sidebar::{Sidebar, SidebarGroup, SidebarMenu, SidebarMenuItem},
    tree::{tree, TreeItem, TreeState},
    v_flex, Collapsible,
};
use gpui::{
    div, px, App, AppContext as _, ElementId, Entity, IntoElement, ParentElement as _, RenderOnce,
    SharedString, Styled as _, Window,
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
}

impl Settings {
    /// Create a new settings structure with the given ID.
    pub fn new(id: impl Into<ElementId>, pages: Vec<SettingPage>) -> Self {
        Self {
            id: id.into(),
            query: SharedString::default(),
            pages,
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

    fn render_active_page(
        self,
        pages: &Vec<SettingPage>,
        selected_ix: SelectIndex,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        for (ix, page) in pages.into_iter().enumerate() {
            if Some(ix) == selected_ix.page_ix {
                return page.render(ix, &self.query, window, cx).into_any_element();
            }
        }

        return div().into_any_element();
    }

    fn filted_pages(&self) -> Vec<SettingPage> {
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

    fn build_items(&self, pages: &Vec<SettingPage>) -> Vec<TreeItem> {
        let mut items = Vec::new();
        pages.iter().enumerate().for_each(|(page_ix, page)| {
            items.push(
                TreeItem::new(format!("page-{}", page_ix), page.title.clone())
                    .expanded(true)
                    .children(
                        page.groups.iter().enumerate().map(|(ix, group)| {
                            TreeItem::new(format!("{}", ix), group.title.clone())
                        }),
                    ),
            )
        });

        items
    }
}

struct SettingsState {
    history: History<ElementId>,
    tree_state: Entity<TreeState>,
    selected_ix: SelectIndex,
}

#[derive(Clone, Copy, Default)]
struct SelectIndex {
    page_ix: Option<usize>,
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
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| {
            let tree_state = cx.new(|cx| TreeState::new(cx));
            SettingsState {
                tree_state,
                selected_ix: SelectIndex::default(),
                history: History::new().max_undos(1000),
            }
        });

        let filted_pages = self.filted_pages();

        let tree_state = state.read(cx).tree_state.clone();
        let items = self.build_items(&filted_pages);
        tree_state.update(cx, |tree_state, cx| {
            tree_state.set_items(items, cx);
        });
        let selected_ix = state.read(cx).selected_ix;

        h_resizable(self.id.clone())
            .child(
                resizable_panel()
                    .size(px(300.))
                    .child(div().size_full().p_2().child(tree(
                        &tree_state,
                        move |ix, entry, selected, _, _| {
                            ListItem::new(ix)
                                .selected(selected)
                                .pl(px(16.) * entry.depth() + px(12.))
                                .child(entry.item().label.clone())
                                .on_click({
                                    let page_id = entry.item().id.clone();
                                    let state = state.clone();
                                    move |_, _, cx| {
                                        state.update(cx, |state, cx| {
                                            // state.selected_page_id = Some(page_id.clone());
                                            cx.notify();
                                        });
                                    }
                                })
                        },
                    ))),
            )
            .child(resizable_panel().child(self.render_active_page(
                &filted_pages,
                selected_ix,
                window,
                cx,
            )))
    }
}
