use gpui::{
    App, AsyncApp, Context, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Task, WeakEntity, Window, div,
};
use std::collections::HashSet;
use std::rc::Rc;

use gpui_component::list::{ListDelegate, ListState};
use gpui_component::tooltip::Tooltip;
use gpui_component::{ActiveTheme, IndexPath, Selectable, checkbox::Checkbox, h_flex, label::Label};

#[derive(Clone, Debug)]
pub struct FilterValue {
    pub value: String,
    pub count: usize,
    pub checked: bool,
    pub selected: bool,
}

impl FilterValue {
    pub fn new(value: String, count: usize) -> Self {
        Self {
            value,
            count,
            checked: false,
            selected: false,
        }
    }
}

#[derive(IntoElement)]
pub struct FilterListItem {
    pub value: Rc<FilterValue>,
    pub selected: bool,
    pub on_toggle: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl FilterListItem {
    pub fn new(value: Rc<FilterValue>, selected: bool) -> Self {
        Self {
            value,
            selected,
            on_toggle: None,
        }
    }

    pub fn on_toggle(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl Selectable for FilterListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for FilterListItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_toggle = self.on_toggle.clone();
        let value_str = self.value.value.clone();
        let checked = self.value.checked;

        h_flex()
            .id(SharedString::from(format!("filter-item-{}", value_str)))
            .w_full()
            .px_2()
            .py_1()
            .items_center()
            .justify_between()
            .gap_2()
            .cursor_pointer()
            .on_click(move |_, window, cx| {
                if let Some(handler) = on_toggle.as_ref() {
                    handler(window, cx);
                }
            })
            .child(
                h_flex()
                    .id(SharedString::from(format!("label-{}", value_str)))
                    .flex_1()
                    .gap_2()
                    .items_center()
                    .overflow_x_hidden()
                    .child(
                        Checkbox::new(SharedString::from(format!("filter-{}", value_str)))
                            .checked(checked),
                    )
                    .child(Label::new(self.value.value.clone()))
                    .tooltip(move |window, cx| Tooltip::new(value_str.clone()).build(window, cx)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("({})", self.value.count)),
            )
    }
}

pub struct FilterPanel {
    pub(crate) values: Vec<FilterValue>,
    selected_index: Option<IndexPath>,
    confirmed_index: Option<IndexPath>,
    on_toggle: Option<Rc<dyn Fn(&str, &mut Window, &mut App)>>,
    filtered_values: Vec<FilterValue>,
}

impl FilterPanel {
    pub fn new(values: Vec<FilterValue>) -> Self {
        let filtered_values = values.clone();
        Self {
            values,
            selected_index: None,
            confirmed_index: None,
            on_toggle: None,
            filtered_values,
        }
    }

    pub fn on_toggle(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    pub fn get_selected_values(&self) -> HashSet<String> {
        self.values
            .iter()
            .filter(|v| v.selected)
            .map(|v| v.value.clone())
            .collect()
    }

    pub fn toggle_value(&mut self, value: &str) {
        if let Some(v) = self.values.iter_mut().find(|v| v.value == value) {
            v.selected = !v.selected;
            v.checked = v.selected;
        }

        if let Some(v) = self.filtered_values.iter_mut().find(|v| v.value == value) {
            v.selected = !v.selected;
            v.checked = v.selected;
        }
    }

    pub fn select_all(&mut self) {
        let visible_values: HashSet<String> = self
            .filtered_values
            .iter()
            .map(|v| v.value.clone())
            .collect();

        for v in &mut self.values {
            if visible_values.contains(&v.value) {
                v.selected = true;
                v.checked = true;
            }
        }

        for v in &mut self.filtered_values {
            v.selected = true;
            v.checked = true;
        }
    }

    pub fn deselect_all(&mut self) {
        let visible_values: HashSet<String> = self
            .filtered_values
            .iter()
            .map(|v| v.value.clone())
            .collect();

        for v in &mut self.values {
            if visible_values.contains(&v.value) {
                v.selected = false;
                v.checked = false;
            }
        }

        for v in &mut self.filtered_values {
            v.selected = false;
            v.checked = false;
        }
    }

    fn update_filtered_values(&mut self, search_query: String) {
        if search_query.is_empty() {
            self.filtered_values = self.values.clone();
        } else {
            let query_lower = search_query.to_lowercase();
            self.filtered_values = self
                .values
                .iter()
                .filter(|v| v.value.to_lowercase().contains(&query_lower))
                .cloned()
                .collect();
        }
    }

    pub fn filtered_values(&self) -> &[FilterValue] {
        &self.filtered_values
    }
}

impl ListDelegate for FilterPanel {
    type Item = FilterListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        let query = query.to_string();
        cx.spawn(
            async move |entity: WeakEntity<ListState<Self>>, cx: &mut AsyncApp| {
                let result = entity.update(cx, move |this, _cx| {
                    this.delegate_mut().update_filtered_values(query);
                });
                result.unwrap_or_else(|_| {
                    eprint!("Failed to update search query");
                })
            },
        )
    }

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.filtered_values.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(value) = self.filtered_values.get(ix.row) {
            let value_rc = Rc::from(value.clone());
            let mut item = FilterListItem::new(value_rc.clone(), selected);

            if let Some(on_toggle) = self.on_toggle.as_ref() {
                let on_toggle = on_toggle.clone();
                let value_str = value.value.clone();
                item = item.on_toggle(move |window, cx| {
                    on_toggle(&value_str, window, cx);
                });
            }

            return Some(item);
        }
        None
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }
}
