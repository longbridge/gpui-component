use std::rc::Rc;

use gpui::{
    prelude::FluentBuilder as _, App, ClickEvent, ElementId, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, SharedString, Styled, Window,
};

use crate::{
    button::{Button, ButtonVariants},
    h_flex,
    label::Label,
    setting::SettingItem,
    v_flex, ActiveTheme, IconName, Sizable as _,
};

/// A setting group that can contain multiple setting items.
#[derive(IntoElement)]
pub struct SettingGroup {
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    items: Vec<SettingItem>,
    query: SharedString,
}

impl SettingGroup {
    /// Create a new setting group with the given title.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            id: ElementId::Integer(0),
            title: title.into(),
            description: None,
            items: Vec::new(),
            query: SharedString::default(),
        }
    }

    pub(super) fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub(super) fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    /// Set the label of the setting group.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the description of the setting group.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a setting field to the group using a closure.
    pub fn field<F>(mut self, f: F) -> Self
    where
        F: FnOnce(SettingItem) -> SettingItem,
    {
        let item = f(SettingItem::new(SharedString::default()));
        self.items.push(item);
        self
    }

    /// Add a setting item to the group.
    pub fn child(mut self, item: SettingItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple setting items to the group.
    pub fn children<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = SettingItem>,
    {
        self.items.extend(items);
        self
    }

    fn is_resetable(&self) -> bool {
        self.items.iter().any(|item| !item.is_default)
    }

    /// Return true if any of the setting items in the group match the given query.
    pub(super) fn is_match(&self, query: &str) -> bool {
        self.items.iter().any(|item| item.is_match(query))
    }

    /// Get all on_reset callbacks for non-default items.
    pub(super) fn on_resets(
        &self,
    ) -> Vec<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>> {
        self.items
            .iter()
            .filter(|item| !item.is_default)
            .map(|item| item.on_reset.clone())
            .collect()
    }
}

impl RenderOnce for SettingGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl gpui::IntoElement {
        let is_resetable = self.is_resetable();
        let on_resets = self
            .items
            .iter()
            .filter(|item| !item.is_default)
            .map(|item| item.on_reset.clone())
            .collect::<Vec<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>>();

        v_flex()
            .id(self.id)
            .gap_4()
            .child(
                h_flex()
                    .justify_between()
                    .child(Label::new(self.title))
                    .when(is_resetable, |this| {
                        this.child(
                            Button::new("reset")
                                .icon(IconName::Undo2)
                                .small()
                                .ghost()
                                .on_click(move |event, window, cx| {
                                    for on_reset in &on_resets {
                                        on_reset(&event, window, cx);
                                    }
                                }),
                        )
                    }),
            )
            .when_some(self.description, |this, description| {
                this.child(
                    Label::new(description)
                        .text_sm()
                        .text_color(cx.theme().muted_foreground),
                )
            })
            .children(self.items.into_iter().enumerate().filter_map(|(ix, item)| {
                if item.is_match(&self.query) {
                    Some(item.id(ix))
                } else {
                    None
                }
            }))
    }
}
