use gpui::{
    App, Entity, InteractiveElement as _, IntoElement, ListAlignment, ListState,
    ParentElement as _, SharedString, Styled, Window, div, list, prelude::FluentBuilder as _, px,
};
use rust_i18n::t;

use crate::{
    ActiveTheme, IconName, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    label::Label,
    scroll::ScrollableElement,
    setting::{RenderOptions, SettingGroup, settings::SettingsState},
    v_flex,
};

/// A setting page that can contain multiple setting groups.
#[derive(Clone)]
pub struct SettingPage {
    resettable: bool,
    pub(super) default_open: bool,
    pub(super) title: SharedString,
    pub(super) description: Option<SharedString>,
    pub(super) groups: Vec<SettingGroup>,
}

impl SettingPage {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            resettable: true,
            default_open: false,
            title: title.into(),
            description: None,
            groups: Vec::new(),
        }
    }

    /// Set the title of the setting page.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the description of the setting page, default is None.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the default open state of the setting page, default is false.
    pub fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }

    /// Set whether the setting page is resettable, default is true.
    ///
    /// If true and the items in this page has changed, the reset button will appear.
    pub fn resettable(mut self, resettable: bool) -> Self {
        self.resettable = resettable;
        self
    }

    /// Add a setting group to the page.
    pub fn group(mut self, group: SettingGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// Add multiple setting groups to the page.
    pub fn groups(mut self, groups: impl IntoIterator<Item = SettingGroup>) -> Self {
        self.groups.extend(groups);
        self
    }

    fn is_resettable(&self, cx: &App) -> bool {
        self.resettable && self.groups.iter().any(|group| group.is_resettable(cx))
    }

    fn reset_all(&self, window: &mut Window, cx: &mut App) {
        for group in &self.groups {
            group.reset(window, cx);
        }
    }

    pub(super) fn render(
        &self,
        ix: usize,
        state: &Entity<SettingsState>,
        options: &RenderOptions,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let search_input = state.read(cx).search_input.clone();
        let query = search_input.read(cx).value();
        let groups = self
            .groups
            .iter()
            .filter(|group| group.is_match(&query, cx))
            .cloned()
            .collect::<Vec<_>>();
        let groups_count = groups.len();

        let list_state = window
            .use_keyed_state(
                SharedString::from(format!("list-state:{}", ix)),
                cx,
                |_, _| ListState::new(groups_count, ListAlignment::Top, px(100.)),
            )
            .read(cx)
            .clone();

        if list_state.item_count() != groups_count {
            list_state.reset(groups_count);
        }

        let deferred_scroll_group_ix = state.read(cx).deferred_scroll_group_ix;
        if let Some(ix) = deferred_scroll_group_ix {
            state.update(cx, |state, _| {
                state.deferred_scroll_group_ix = None;
            });
            list_state.scroll_to_reveal_item(ix);
        }

        v_flex()
            .id(ix)
            .size_full()
            .child(
                v_flex()
                    .p_4()
                    .gap_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(h_flex().justify_between().child(self.title.clone()).when(
                        self.is_resettable(cx),
                        |this| {
                            this.child(
                                Button::new("reset")
                                    .icon(IconName::Undo2)
                                    .ghost()
                                    .small()
                                    .tooltip(t!("Settings.Reset All"))
                                    .on_click({
                                        let page = self.clone();
                                        move |_, window, cx| {
                                            page.reset_all(window, cx);
                                        }
                                    }),
                            )
                        },
                    ))
                    .when_some(self.description.clone(), |this, description| {
                        this.child(
                            Label::new(description)
                                .text_sm()
                                .text_color(cx.theme().muted_foreground),
                        )
                    }),
            )
            .child(
                div()
                    .px_4()
                    .relative()
                    .flex_1()
                    .w_full()
                    .child(
                        list(list_state.clone(), {
                            let query = query.clone();
                            let options = *options;
                            move |group_ix, window, cx| {
                                let group = groups[group_ix].clone();
                                group
                                    .py_4()
                                    .render(
                                        &query,
                                        &RenderOptions {
                                            page_ix: ix,
                                            group_ix,
                                            ..options
                                        },
                                        window,
                                        cx,
                                    )
                                    .into_any_element()
                            }
                        })
                        .size_full(),
                    )
                    .vertical_scrollbar(&list_state),
            )
    }
}
