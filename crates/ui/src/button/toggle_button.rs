use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Window};

use crate::{Disableable, Icon, Selectable as _, Sizable, Size};

use super::{Button, ButtonGroup, ButtonVariant, ButtonVariants};

/// A Toggle Button can be used to group related options.
#[derive(IntoElement)]
pub struct ToggleButtonGroup {
    /// The indices of the checked buttons.
    checkeds: Vec<bool>,
    inner: ButtonGroup,

    items: Vec<ToggleButton>,
    on_change: Option<Box<dyn Fn(&Vec<bool>, &mut Window, &mut App) + 'static>>,
}

enum ToggleButtonItem {
    Icon(Icon),
    Label(SharedString),
}

pub struct ToggleButton {
    item: ToggleButtonItem,
    checked: bool,
}

impl ToggleButton {
    pub fn label(label: impl Into<SharedString>) -> Self {
        Self {
            item: ToggleButtonItem::Label(label.into()),
            checked: false,
        }
    }

    pub fn icon(icon: impl Into<Icon>) -> Self {
        Self {
            item: ToggleButtonItem::Icon(icon.into()),
            checked: false,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    fn render(self, id: usize) -> Button {
        match self.item {
            ToggleButtonItem::Icon(icon) => Button::new(id).icon(icon).compact(),
            ToggleButtonItem::Label(label) => Button::new(id).label(label).compact(),
        }
    }
}

impl ToggleButtonGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            checkeds: Vec::new(),
            inner: ButtonGroup::new(id),
            items: Vec::new(),
            on_change: None,
        }
    }

    /// Sets the checked state of all items.
    pub fn checked(mut self, checkeds: Vec<bool>) -> Self {
        self.checkeds = checkeds;
        self
    }

    /// With the multiple selection mode.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.inner = self.inner.multiple(multiple);
        self
    }

    /// Add child button to the ToggleButton.
    pub fn child(mut self, child: impl Into<ToggleButton>) -> Self {
        let btn: ToggleButton = child.into();
        self.checkeds.push(btn.checked);
        self.items.push(btn);
        self
    }

    /// Add children buttons to the ToggleButton.
    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<ToggleButton>>) -> Self {
        let btns = children
            .into_iter()
            .map(Into::into)
            .collect::<Vec<ToggleButton>>();
        self.checkeds.extend(btns.iter().map(|btn| btn.checked));
        self.items.extend(btns);
        self
    }

    /// Sets the on_change handler for the ToggleButton.
    ///
    /// The handler first argument is a vector of the checked item indices.
    ///
    /// The `&Vec<bool>` is contains check state of all items, the Vec length is equal to the number of items.
    ///
    /// ```rust
    /// ToggleButton::new("toggle-button")
    ///    .children(ve!["A", "B", "C"])
    ///    .on_click(cx.listener(|view, checkeds: &Vec<bool>, _, cx| {
    ///        println!("item checks: {:?}", checkeds);
    ///        cx.notify();
    ///    }))
    /// ```
    pub fn on_change(
        mut self,
        handler: impl Fn(&Vec<bool>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
}

impl Sizable for ToggleButtonGroup {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.inner = self.inner.with_size(size);
        self
    }
}

impl ButtonVariants for ToggleButtonGroup {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.inner = self.inner.with_variant(variant);
        self
    }
}

impl Disableable for ToggleButtonGroup {
    fn disabled(mut self, disabled: bool) -> Self {
        self.inner = self.inner.disabled(disabled);
        self
    }
}

impl RenderOnce for ToggleButtonGroup {
    fn render(self, _: &mut gpui::Window, _: &mut gpui::App) -> impl IntoElement {
        let checkeds = self.checkeds;
        let items_len = self.items.len();

        self.inner
            .children(
                self.items
                    .into_iter()
                    .enumerate()
                    .map(|(ix, item)| item.render(ix).selected(checkeds[ix])),
            )
            .on_click(move |selecteds: &Vec<usize>, window, cx| {
                let mut checkeds = vec![false; items_len];
                for i in 0..items_len {
                    checkeds[i] = selecteds.contains(&i);
                }
                if let Some(handler) = &self.on_change {
                    handler(&checkeds, window, cx);
                }
            })
    }
}
