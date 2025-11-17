use std::rc::Rc;

use gpui::{
    prelude::FluentBuilder as _, AnyElement, App, ClickEvent, ElementId, InteractiveElement as _,
    IntoElement, ParentElement as _, RenderOnce, SharedString, Styled, Window,
};

use crate::{
    button::{Button, ButtonVariants},
    h_flex, v_flex, IconName, Sizable as _,
};

/// A trait representing a setting item in the application.
#[derive(IntoElement)]
pub struct SettingItem {
    id: ElementId,
    label: SharedString,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    pub(super) is_default: bool,
    pub(super) on_reset: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
}

impl SettingItem {
    /// Create a new setting item with the given ID.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            id: ElementId::Integer(0),
            label: label.into(),
            description: None,
            children: Vec::new(),
            is_default: true,
            on_reset: Rc::new(|_, _, _| {}),
        }
    }

    pub(super) fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the label of the setting item.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    /// Set the description of the setting item.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set is current setting is default value, default is true.
    ///
    /// If `false`, the reset button will be enabled.
    pub fn is_default(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }

    /// Add a callback to be called when clicking the reset button.
    ///
    /// This method will called by group, setting item level reset,
    /// so please make sure it fast enough.
    pub fn on_reset<F>(mut self, callback: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_reset = Rc::new(callback);
        self
    }

    /// Return true if the setting item matches the given query.
    pub(super) fn is_match(&self, query: &str) -> bool {
        self.label.to_lowercase().contains(query)
            || self
                .description
                .as_ref()
                .map_or(false, |desc| desc.to_lowercase().contains(query))
    }
}

impl RenderOnce for SettingItem {
    fn render(self, _: &mut Window, _: &mut App) -> impl gpui::IntoElement {
        h_flex()
            .id(self.id.clone())
            .gap_4()
            .justify_between()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_1()
                            .child(self.label.clone())
                            .when(!self.is_default, |e| {
                                e.child(
                                    Button::new("reset")
                                        .icon(IconName::Undo2)
                                        .ghost()
                                        .small()
                                        .on_click(move |event, window, cx| {
                                            (self.on_reset)(event, window, cx);
                                        }),
                                )
                            }),
                    )
                    .children(self.description),
            )
            .children(self.children)
    }
}
