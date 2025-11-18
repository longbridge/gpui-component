use std::rc::Rc;

use gpui::{
    prelude::FluentBuilder as _, App, ClickEvent, ParentElement as _, SharedString, Styled, Window,
};

use crate::{
    group_box::{GroupBox, GroupBoxVariant},
    label::Label,
    setting::SettingItem,
    v_flex, ActiveTheme,
};

/// A setting group that can contain multiple setting items.
#[derive(Clone)]
pub struct SettingGroup {
    pub title: SharedString,
    pub description: Option<SharedString>,
    pub items: Vec<SettingItem>,
    variant: GroupBoxVariant,
}

impl SettingGroup {
    /// Create a new setting group with the given title.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
            items: Vec::new(),
            variant: GroupBoxVariant::default(),
        }
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

    /// Set the variant of the group box.
    pub(crate) fn with_variant(mut self, variant: GroupBoxVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Add a setting item to the group.
    pub fn item(mut self, item: SettingItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple setting items to the group.
    pub fn items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = SettingItem>,
    {
        self.items.extend(items);
        self
    }

    /// Return true if any of the setting items in the group match the given query.
    pub(super) fn is_match(&self, query: &str) -> bool {
        self.items.iter().any(|item| item.is_match(query))
    }

    /// Get all on_reset callbacks for non-default items.
    pub(super) fn on_resets(
        &self,
    ) -> Vec<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>> {
        return vec![];
        // self.items
        //     .iter()
        //     .filter(|item| !item.is_default)
        //     .map(|item| item.on_reset.clone())
        //     .collect()
    }

    pub(crate) fn render(
        self,
        _ix: usize,
        query: &str,
        window: &mut Window,
        cx: &mut App,
    ) -> impl gpui::IntoElement {
        // let is_resettable = self.is_resettable();
        // let on_resets = self
        //     .items
        //     .iter()
        //     .filter(|item| !item.is_default())
        //     .map(|item| item.on_reset.clone())
        //     .collect::<Vec<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>>();

        GroupBox::new()
            .with_variant(self.variant)
            .title(v_flex().gap_1().child(self.title.clone()).when_some(
                self.description.clone(),
                |this, description| {
                    this.child(
                        Label::new(description)
                            .text_sm()
                            .text_color(cx.theme().muted_foreground),
                    )
                },
            ))
            .children(self.items.iter().filter_map(|item| {
                if item.is_match(&query) {
                    Some(item.clone().render(window, cx))
                } else {
                    None
                }
            }))
    }
}
