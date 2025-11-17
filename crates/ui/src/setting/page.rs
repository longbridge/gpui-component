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
    v_flex, ActiveTheme, IconName,
};

/// A setting page that can contain multiple setting groups.
#[derive(IntoElement)]
pub struct SettingPage {
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    groups: Vec<SettingGroup>,
    query: SharedString,
}

impl SettingPage {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            title: SharedString::default(),
            description: None,
            query: SharedString::default(),
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

    /// Add a setting group to the page using a closure.
    pub fn group<F>(mut self, f: F) -> Self
    where
        F: FnOnce(SettingGroup) -> SettingGroup,
    {
        let group = f(SettingGroup::new(SharedString::default()));
        self.groups.push(group);
        self
    }

    /// Add a setting group to the page.
    pub fn child(mut self, group: SettingGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// Add multiple setting groups to the page.
    pub fn children<I>(mut self, groups: I) -> Self
    where
        I: IntoIterator<Item = SettingGroup>,
    {
        self.groups.extend(groups);
        self
    }

    /// Set the search query for filtering setting groups.
    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        let query: SharedString = query.into();
        self.query = query.to_lowercase().into();
        self
    }
}

impl RenderOnce for SettingPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_resets = self
            .groups
            .iter()
            .flat_map(|group| group.on_resets())
            .collect::<Vec<_>>();

        v_flex()
            .id(self.id)
            .size_full()
            .gap_5()
            .child(
                v_flex()
                    .gap_4()
                    .child(h_flex().child(self.title.clone()).justify_between().child(
                        Button::new("reset").ghost().icon(IconName::Undo2).on_click(
                            move |event, window, cx| {
                                on_resets.iter().for_each(|callback| {
                                    callback(event, window, cx);
                                });
                            },
                        ),
                    ))
                    .when_some(self.description, |this, description| {
                        this.child(
                            Label::new(description)
                                .text_sm()
                                .text_color(cx.theme().muted_foreground),
                        )
                    }),
            )
            .child(Divider::horizontal())
            .children(
                self.groups
                    .into_iter()
                    .enumerate()
                    .filter_map(|(ix, group)| {
                        if group.is_match(&self.query) {
                            Some(group.id(ix).query(self.query.clone()))
                        } else {
                            None
                        }
                    }),
            )
    }
}
