use crate::{ActiveTheme, Selectable};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, App, Div, Edges, ElementId, Hsla, InteractiveElement, IntoElement,
    ParentElement as _, Pixels, RenderOnce, Stateful, StatefulInteractiveElement, Styled, Window,
};

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash)]
pub enum TabVariant {
    #[default]
    Tab,
    Pill,
    Underline,
}

struct TabStyle {
    borders: Edges<Pixels>,
    border_color: Hsla,
    bg: Hsla,
    fg: Hsla,
    radius: Pixels,
    shadow: bool,
}

impl Default for TabStyle {
    fn default() -> Self {
        TabStyle {
            borders: Edges::all(px(0.)),
            border_color: gpui::transparent_white(),
            bg: gpui::transparent_white(),
            fg: gpui::transparent_white(),
            radius: px(0.),
            shadow: false,
        }
    }
}

impl TabVariant {
    fn normal(&self, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().foreground,
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().foreground,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().foreground,
                ..Default::default()
            },
        }
    }

    fn hovered(&self, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().foreground,
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().accent_foreground,
                bg: cx.theme().accent,
                radius: cx.theme().radius,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().accent_foreground,
                bg: cx.theme().accent,
                radius: cx.theme().radius,
                ..Default::default()
            },
        }
    }

    fn selected(&self, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().tab_active,
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().tab_active,
                radius: cx.theme().radius,
                shadow: true,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().primary,
                bg: cx.theme().transparent,
                radius: cx.theme().radius,
                border_color: cx.theme().primary,
                ..Default::default()
            },
        }
    }

    fn disabled(&self, selected: bool, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().tab,
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: if selected {
                    cx.theme().tab_active
                } else {
                    cx.theme().tab
                },
                radius: cx.theme().radius,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent,
                radius: cx.theme().radius,
                border_color: cx.theme().border,
                ..Default::default()
            },
        }
    }
}

#[derive(IntoElement)]
pub struct Tab {
    id: ElementId,
    base: Stateful<Div>,
    label: AnyElement,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    variant: TabVariant,
    disabled: bool,
    selected: bool,
}

impl Tab {
    pub fn new(id: impl Into<ElementId>, label: impl IntoElement) -> Self {
        let id: ElementId = id.into();
        Self {
            id: id.clone(),
            base: div().id(id).gap_1().py_1p5().px_3().h(px(30.)),
            label: label.into_any_element(),
            disabled: false,
            selected: false,
            prefix: None,
            suffix: None,
            variant: TabVariant::default(),
        }
    }

    pub fn variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    pub fn underline(mut self) -> Self {
        self.variant = TabVariant::Underline;
        self
    }

    /// Set the left side of the tab
    pub fn prefix(mut self, prefix: impl Into<AnyElement>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Set the right side of the tab
    pub fn suffix(mut self, suffix: impl Into<AnyElement>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Set disabled state to the tab
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for Tab {
    fn element_id(&self) -> &ElementId {
        &self.id
    }

    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl InteractiveElement for Tab {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Tab {}

impl Styled for Tab {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Tab {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut tab_style = if self.selected {
            self.variant.selected(cx)
        } else {
            self.variant.normal(cx)
        };
        let mut hover_style = self.variant.hovered(cx);
        if self.disabled {
            tab_style = self.variant.disabled(self.selected, cx);
            hover_style = self.variant.disabled(self.selected, cx);
        }

        self.base
            .flex()
            .items_center()
            .flex_shrink_0()
            .cursor_pointer()
            .overflow_hidden()
            .text_color(tab_style.fg)
            .bg(tab_style.bg)
            .border_l(tab_style.borders.left)
            .border_r(tab_style.borders.right)
            .border_t(tab_style.borders.top)
            .border_b(tab_style.borders.bottom)
            .border_color(tab_style.border_color)
            .rounded(tab_style.radius)
            .when(!tab_style.shadow, |this| this.shadow_sm())
            .when(!self.selected && !self.disabled, |this| {
                this.hover(|this| {
                    this.text_color(hover_style.fg)
                        .bg(hover_style.bg)
                        .border_l(hover_style.borders.left)
                        .border_r(hover_style.borders.right)
                        .border_t(hover_style.borders.top)
                        .border_b(hover_style.borders.bottom)
                        .border_color(hover_style.border_color)
                        .rounded(tab_style.radius)
                })
            })
            .text_sm()
            .when_some(self.prefix, |this, prefix| this.child(prefix))
            .child(div().text_ellipsis().child(self.label))
            .when_some(self.suffix, |this, suffix| this.child(suffix))
    }
}
