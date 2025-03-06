use std::{cell::Cell, rc::Rc};

use gpui::{
    div, prelude::FluentBuilder as _, AnyElement, App, Div, ElementId, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled as _,
    Window,
};
use smallvec::{smallvec, SmallVec};

use crate::{h_flex, ActiveTheme, Disableable, Icon, Sizable, Size};

#[derive(Default, Copy, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ToggleVariant {
    #[default]
    Ghost,
    Outline,
}

pub trait ToggleVariants: Sized {
    fn with_variant(self, variant: ToggleVariant) -> Self;
    fn ghost(self) -> Self {
        self.with_variant(ToggleVariant::Ghost)
    }
    fn outline(self) -> Self {
        self.with_variant(ToggleVariant::Outline)
    }
}

#[derive(IntoElement)]
pub struct Toggle {
    base: Div,
    checked: bool,
    size: Size,
    variant: ToggleVariant,
    disabled: bool,
    children: SmallVec<[AnyElement; 1]>,
}

#[derive(IntoElement)]
pub struct InteractiveToggle {
    id: ElementId,
    toggle: Toggle,
    on_change: Option<Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
}

impl Toggle {
    fn new() -> Self {
        Self {
            base: div(),
            checked: false,
            size: Size::default(),
            variant: ToggleVariant::default(),
            disabled: false,
            children: smallvec![],
        }
    }

    pub fn label(label: impl Into<SharedString>) -> Self {
        Self::new().child(label.into())
    }

    pub fn icon(icon: impl Into<Icon>) -> Self {
        Self::new().child(icon.into())
    }

    pub fn id(self, id: impl Into<ElementId>) -> InteractiveToggle {
        InteractiveToggle {
            id: id.into(),
            toggle: self,
            on_change: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
}

impl ToggleVariants for Toggle {
    fn with_variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Disableable for Toggle {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Sizable for Toggle {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ParentElement for Toggle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Toggle {
    fn render(self, _: &mut Window, cx: &mut App) -> impl gpui::IntoElement {
        let checked = self.checked;
        let disabled = self.disabled;
        let hoverable = !disabled && !checked;

        self.base
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .map(|this| match self.size {
                Size::XSmall => this.min_w_5().h_5().px_0p5().text_xs(),
                Size::Small => this.min_w_6().h_6().px_1().text_sm(),
                Size::Large => this.min_w_9().h_9().px_3().text_lg(),
                _ => this.min_w_8().h_8().px_2(),
            })
            .rounded(cx.theme().radius)
            .when(self.variant == ToggleVariant::Outline, |this| {
                this.border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .when(cx.theme().shadow, |this| this.shadow_sm())
            })
            .when(hoverable, |this| {
                this.hover(|this| {
                    this.bg(cx.theme().accent)
                        .text_color(cx.theme().accent_foreground)
                })
            })
            .when(checked, |this| {
                this.bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
            })
            .children(self.children)
    }
}

impl InteractiveToggle {
    /// Sets the callback to be invoked when the toggle is clicked.
    ///
    /// The first argument is a boolean indicating whether the toggle is checked.
    pub fn on_change(mut self, on_change: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(on_change));
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.toggle.checked = checked;
        self
    }
}

impl Sizable for InteractiveToggle {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.toggle = self.toggle.with_size(size);
        self
    }
}

impl ToggleVariants for InteractiveToggle {
    fn with_variant(mut self, variant: ToggleVariant) -> Self {
        self.toggle.variant = variant;
        self
    }
}

impl Disableable for InteractiveToggle {
    fn disabled(mut self, disabled: bool) -> Self {
        self.toggle.disabled = disabled;
        self
    }
}

impl ParentElement for InteractiveToggle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.toggle.extend(elements);
    }
}

impl RenderOnce for InteractiveToggle {
    fn render(self, _: &mut Window, _: &mut App) -> impl gpui::IntoElement {
        let checked = self.toggle.checked;
        let disabled = self.toggle.disabled;

        div()
            .id(self.id)
            .child(self.toggle)
            .when(!disabled, |this| {
                this.when_some(self.on_change, |this, on_change| {
                    this.on_click(move |_, window, cx| on_change(&!checked, window, cx))
                })
            })
    }
}

#[derive(IntoElement)]
pub struct ToggleGroup {
    id: ElementId,
    size: Size,
    variant: ToggleVariant,
    disabled: bool,
    items: Vec<Toggle>,
    on_change: Option<Rc<dyn Fn(&Vec<bool>, &mut Window, &mut App) + 'static>>,
}

impl ToggleGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            size: Size::default(),
            variant: ToggleVariant::default(),
            disabled: false,
            items: Vec::new(),
            on_change: None,
        }
    }

    /// Add a child [`Toggle`] to the group.
    pub fn child(mut self, toggle: impl Into<Toggle>) -> Self {
        self.items.push(toggle.into());
        self
    }

    /// Add multiple [`Toggle`]s to the group.
    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Toggle>>) -> Self {
        self.items.extend(children.into_iter().map(Into::into));
        self
    }

    /// Set the callback to be called when the toggle group changes.
    ///
    /// The `&Vec<bool>` parameter represents the check state of each [`Toggle`] in the group.
    pub fn on_change(
        mut self,
        on_change: impl Fn(&Vec<bool>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(on_change));
        self
    }
}

impl Sizable for ToggleGroup {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ToggleVariants for ToggleGroup {
    fn with_variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Disableable for ToggleGroup {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for ToggleGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl gpui::IntoElement {
        let disabled = self.disabled;
        let checks = self
            .items
            .iter()
            .map(|item| item.checked)
            .collect::<Vec<bool>>();
        let state = Rc::new(Cell::new(None));

        h_flex()
            .id(self.id)
            .gap_1()
            .children(self.items.into_iter().enumerate().map({
                |(ix, item)| {
                    let state = state.clone();
                    item.disabled(disabled)
                        .id(ix)
                        .with_size(self.size)
                        .with_variant(self.variant)
                        .on_change(move |_, _, _| {
                            state.set(Some(ix));
                        })
                }
            }))
            .when(!disabled, |this| {
                this.when_some(self.on_change, |this, on_change| {
                    this.on_click(move |_, window, cx| {
                        if let Some(ix) = state.get() {
                            let mut checks = checks.clone();
                            checks[ix] = !checks[ix];
                            on_change(&checks, window, cx);
                        }
                    })
                })
            })
    }
}
