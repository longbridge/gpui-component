use gpui::{
    list, prelude::FluentBuilder as _, px, App, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, SharedString, StatefulInteractiveElement, Styled, Window,
};

use crate::{
    button::{Button, ButtonVariants},
    divider::Divider,
    group_box::GroupBoxVariant,
    h_flex,
    label::Label,
    setting::{SettingGroup, SettingsState},
    v_flex, ActiveTheme, IconName, Sizable,
};

/// A setting page that can contain multiple setting groups.
#[derive(Clone)]
pub struct SettingPage {
    pub(super) title: SharedString,
    pub(super) description: Option<SharedString>,
    pub(super) groups: Vec<SettingGroup>,
}

impl SettingPage {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
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

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render(
        &self,
        ix: usize,
        state: &Entity<SettingsState>,
        group_variant: GroupBoxVariant,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let on_resets = self
            .groups
            .iter()
            .flat_map(|group| group.on_resets())
            .collect::<Vec<_>>();

        let query = state.read(cx).query.clone();
        let groups = self
            .groups
            .iter()
            .filter(|group| group.is_match(&query))
            .cloned()
            .collect::<Vec<_>>();
        let groups_count = groups.len();

        let list_state = window
            .use_keyed_state(
                SharedString::from(format!("list-state:{}", ix)),
                cx,
                |_, _| gpui::ListState::new(groups_count, gpui::ListAlignment::Top, px(0.)),
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
            .p_4()
            .size_full()
            .overflow_scroll()
            .child(
                v_flex()
                    .gap_4()
                    .child(
                        h_flex().child(self.title.clone()).justify_between().child(
                            Button::new("reset")
                                .small()
                                .ghost()
                                .icon(IconName::Undo2)
                                .on_click(move |event, window, cx| {
                                    on_resets.iter().for_each(|callback| {
                                        callback(event, window, cx);
                                    });
                                }),
                        ),
                    )
                    .when_some(self.description.clone(), |this, description| {
                        this.child(
                            Label::new(description)
                                .text_sm()
                                .text_color(cx.theme().muted_foreground),
                        )
                    })
                    .child(Divider::horizontal()),
            )
            .child(
                list(list_state.clone(), {
                    let query = query.clone();
                    move |ix, window, cx| {
                        let group = groups[ix].clone();
                        group
                            .pt_6()
                            .with_default_variant(group_variant)
                            .render(ix, &query, window, cx)
                            .into_any_element()
                    }
                })
                .size_full(),
            )
    }
}
