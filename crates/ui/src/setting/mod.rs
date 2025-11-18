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
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    tree::{tree, TreeItem, TreeState},
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

    fn render_active_page(
        self,
        pages: &Vec<SettingPage>,
        selected_ix: SelectIndex,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        for (ix, page) in pages.into_iter().enumerate() {
            if selected_ix.is_page(ix) {
                return page
                    .render(ix, &self.query, self.group_variant, window, cx)
                    .into_any_element();
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
                TreeItem::new(SelectIndex::page_id(page_ix), page.title.clone())
                    .expanded(true)
                    .children(page.groups.iter().enumerate().map(|(ix, group)| {
                        TreeItem::new(SelectIndex::group_id(ix), group.title.clone())
                    })),
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

#[derive(Clone, Default)]
struct SelectIndex {
    page_ix: Option<SharedString>,
    group_ix: Option<SharedString>,
}

impl SelectIndex {
    fn page_id(ix: usize) -> SharedString {
        SharedString::from(format!("page-{}", ix))
    }

    fn group_id(ix: usize) -> SharedString {
        SharedString::from(format!("{}", ix))
    }

    fn is_page(&self, page_ix: usize) -> bool {
        self.page_ix
            .as_ref()
            .map(|ix| ix == &Self::page_id(page_ix))
            .unwrap_or(false)
    }

    fn is_group(&self, group_ix: usize) -> bool {
        self.group_ix
            .as_ref()
            .map(|ix| ix == &Self::group_id(group_ix))
            .unwrap_or(false)
    }
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
            let selected_ix = tree_state.selected_index();
            tree_state.set_items(items, cx);
            tree_state.set_selected_index(selected_ix, cx);
        });
        let selected_ix = state.read(cx).selected_ix.clone();

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
                                    let item_id = entry.item().id.clone();
                                    let depth = entry.depth();
                                    let state = state.clone();
                                    move |_, _, cx| {
                                        state.update(cx, |state, cx| {
                                            if depth == 0 {
                                                state.selected_ix.page_ix = Some(item_id.clone());
                                                state.selected_ix.group_ix = None;
                                            } else if depth == 1 {
                                                state.selected_ix.group_ix = Some(item_id.clone());
                                            }
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
