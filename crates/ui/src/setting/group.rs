use std::rc::Rc;

use gpui::{
    prelude::FluentBuilder as _, App, ClickEvent, ParentElement as _, SharedString,
    StyleRefinement, Styled, Window,
};

use crate::{
    group_box::{GroupBox, GroupBoxVariant, GroupBoxVariants},
    label::Label,
    setting::SettingItem,
    v_flex, ActiveTheme, StyledExt,
};

/// A setting group that can contain multiple setting items.
#[derive(Clone)]
pub struct SettingGroup {
    style: StyleRefinement,
    variant: Option<GroupBoxVariant>,

    pub(super) title: Option<SharedString>,
    pub(super) description: Option<SharedString>,
    pub(super) items: Vec<SettingItem>,
}

impl GroupBoxVariants for SettingGroup {
    fn with_variant(mut self, variant: GroupBoxVariant) -> Self {
        self.variant = Some(variant);
        self
    }
}

impl Styled for SettingGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl SettingGroup {
    /// Create a new setting group with the given title.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            title: None,
            description: None,
            items: Vec::new(),
            variant: None,
        }
    }

    /// Set the label of the setting group.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the description of the setting group.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
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

    pub(super) fn with_default_variant(mut self, variant: GroupBoxVariant) -> Self {
        if self.variant.is_none() {
            self.variant = Some(variant);
        }
        self
    }

    pub(crate) fn render(
        self,
        group_ix: usize,
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
            .id(SharedString::from(format!("group-{}", group_ix)))
            .when_some(self.variant, |this, variant| this.with_variant(variant))
            .when_some(self.title.clone(), |this, title| {
                this.title(v_flex().gap_1().child(title).when_some(
                    self.description.clone(),
                    |this, description| {
                        this.child(
                            Label::new(description)
                                .text_sm()
                                .text_color(cx.theme().muted_foreground),
                        )
                    },
                ))
            })
            .children(self.items.iter().enumerate().filter_map(|(item_ix, item)| {
                if item.is_match(&query) {
                    Some(item.clone().render(item_ix, window, cx))
                } else {
                    None
                }
            }))
            .refine_style(&self.style)
    }
}
