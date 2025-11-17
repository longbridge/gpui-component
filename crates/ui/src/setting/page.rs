use gpui::{
    prelude::FluentBuilder as _, App, ElementId, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, SharedString, Styled, Window,
};

use crate::{
    button::{Button, ButtonVariants},
    divider::Divider,
    h_flex,
    label::Label,
    setting::SettingGroup,
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

    pub fn render(
        &self,
        ix: usize,
        query: &str,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let on_resets = self
            .groups
            .iter()
            .flat_map(|group| group.on_resets())
            .collect::<Vec<_>>();

        v_flex()
            .id(ix)
            .p_4()
            .size_full()
            .gap_5()
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
                    }),
            )
            .child(Divider::horizontal())
            .children(self.groups.iter().enumerate().filter_map(|(ix, group)| {
                if group.is_match(&query) {
                    Some(group.render(ix, query, window, cx))
                } else {
                    None
                }
            }))
    }
}
