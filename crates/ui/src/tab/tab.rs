use std::sync::Arc;

use crate::{ActiveTheme, Selectable, Sizable, Size, StyledExt};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, App, ClickEvent, Div, Edges, ElementId, Hsla, InteractiveElement,
    IntoElement, ParentElement, Pixels, RenderOnce, SharedString, StatefulInteractiveElement,
    Styled, Window,
};

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash)]
pub enum TabVariant {
    #[default]
    Tab,
    Pill,
    Segmented,
    Underline,
}

#[allow(dead_code)]
struct TabStyle {
    borders: Edges<Pixels>,
    border_color: Hsla,
    bg: Hsla,
    fg: Hsla,
    radius: Pixels,
    shadow: bool,
    inner_bg: Hsla,
    inner_radius: Pixels,
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
            inner_bg: gpui::transparent_white(),
            inner_radius: px(0.),
        }
    }
}

impl TabVariant {
    fn height(&self, size: Size) -> Pixels {
        match size {
            Size::XSmall => match self {
                TabVariant::Tab => px(22.),
                TabVariant::Pill => px(20.),
                TabVariant::Segmented => px(20.),
                TabVariant::Underline => px(26.),
            },
            Size::Small => match self {
                TabVariant::Tab => px(24.),
                TabVariant::Pill => px(24.),
                TabVariant::Segmented => px(24.),
                TabVariant::Underline => px(30.),
            },
            Size::Large => match self {
                TabVariant::Tab => px(36.),
                TabVariant::Pill => px(36.),
                TabVariant::Segmented => px(36.),
                TabVariant::Underline => px(42.),
            },
            _ => match self {
                TabVariant::Tab => px(30.),
                TabVariant::Pill => px(31.),
                TabVariant::Segmented => px(30.),
                TabVariant::Underline => px(36.),
            },
        }
    }

    fn inner_height(&self, size: Size) -> Pixels {
        match size {
            Size::XSmall => match self {
                TabVariant::Tab => px(20.),
                TabVariant::Pill => px(20.),
                TabVariant::Segmented => px(20.),
                TabVariant::Underline => px(20.),
            },
            Size::Small => match self {
                TabVariant::Tab => px(24.),
                TabVariant::Pill => px(24.),
                TabVariant::Segmented => px(24.),
                TabVariant::Underline => px(22.),
            },
            Size::Large => match self {
                TabVariant::Tab => px(36.),
                TabVariant::Pill => px(36.),
                TabVariant::Segmented => px(36.),
                TabVariant::Underline => px(30.),
            },
            _ => match self {
                TabVariant::Tab => px(30.),
                TabVariant::Pill => px(31.),
                TabVariant::Segmented => px(30.),
                TabVariant::Underline => px(24.),
            },
        }
    }

    fn inner_paddings(&self, size: Size) -> Edges<Pixels> {
        match size {
            Size::XSmall => Edges {
                left: px(8.),
                right: px(8.),
                ..Default::default()
            },
            Size::Small => Edges {
                left: px(12.),
                right: px(12.),
                ..Default::default()
            },
            Size::Large => Edges {
                left: px(20.),
                right: px(20.),
                ..Default::default()
            },
            _ => Edges {
                left: px(16.),
                right: px(16.),
                ..Default::default()
            },
        }
    }

    fn inner_margins(&self, size: Size) -> Edges<Pixels> {
        match size {
            Size::XSmall => Edges::all(px(0.)),
            Size::Small => match self {
                TabVariant::Underline => Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
            _ => match self {
                TabVariant::Underline => Edges {
                    top: px(5.),
                    bottom: px(3.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
        }
    }

    fn normal(&self, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().foreground,
                bg: cx.theme().transparent,
                borders: Edges {
                    top: px(1.),
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().foreground,
                bg: cx.theme().transparent,
                borders: Edges::all(px(1.)),
                border_color: cx.theme().border,
                radius: px(99.),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().foreground,
                bg: cx.theme().transparent,
                radius: cx.theme().radius,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().foreground,
                bg: cx.theme().transparent,
                radius: px(0.),
                inner_bg: cx.theme().transparent,
                inner_radius: cx.theme().radius,
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
        }
    }

    fn hovered(&self, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent,
                borders: Edges {
                    top: px(1.),
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
                borders: Edges::all(px(1.)),
                border_color: cx.theme().border,
                radius: px(99.),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent,
                radius: cx.theme().radius,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent,
                radius: px(0.),
                inner_bg: cx.theme().accent,
                inner_radius: cx.theme().radius,
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
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
                    top: px(1.),
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().primary,
                bg: cx.theme().transparent,
                borders: Edges::all(px(1.)),
                border_color: cx.theme().primary,
                radius: px(99.),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().background,
                radius: cx.theme().radius,
                shadow: true,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent,
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
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
                border_color: if selected {
                    cx.theme().border
                } else {
                    cx.theme().transparent
                },
                borders: Edges {
                    top: px(1.),
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent,
                borders: Edges::all(px(1.)),
                border_color: if selected {
                    cx.theme().primary
                } else {
                    cx.theme().border
                },
                radius: px(99.),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: if selected {
                    cx.theme().background
                } else {
                    cx.theme().transparent
                },
                radius: cx.theme().radius,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent,
                radius: cx.theme().radius,
                border_color: if selected {
                    cx.theme().border
                } else {
                    cx.theme().transparent
                },
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }
}

#[derive(IntoElement)]
pub struct Tab {
    id: ElementId,
    base: Div,
    label: SharedString,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    children: Vec<AnyElement>,
    variant: TabVariant,
    size: Size,
    disabled: bool,
    selected: bool,
    on_click: Option<Arc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl From<&'static str> for Tab {
    fn from(label: &'static str) -> Self {
        let label = SharedString::from(label);
        Self::new(label)
    }
}

impl From<String> for Tab {
    fn from(label: String) -> Self {
        let label = SharedString::from(label);
        Self::new(label)
    }
}

impl From<SharedString> for Tab {
    fn from(label: SharedString) -> Self {
        Self::new(label)
    }
}

impl Tab {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            id: ElementId::Integer(0),
            base: div().gap_1(),
            label: label.into(),
            children: Vec::new(),
            disabled: false,
            selected: false,
            prefix: None,
            suffix: None,
            variant: TabVariant::default(),
            size: Size::default(),
            on_click: None,
        }
    }

    /// Set id to the tab.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Set Tab Variant.
    pub fn variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Use Pill variant.
    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    /// Use Segmented variant.
    pub fn segmented(mut self) -> Self {
        self.variant = TabVariant::Segmented;
        self
    }

    /// Use Underline variant.
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

    /// Set the click handler for the tab.
    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Arc::new(on_click));
        self
    }
}

impl ParentElement for Tab {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
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

impl Sizable for Tab {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
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
        let inner_paddings = self.variant.inner_paddings(self.size);
        let inner_margins = self.variant.inner_margins(self.size);
        let inner_height = self.variant.inner_height(self.size);
        let height = self.variant.height(self.size);
        let has_label = !self.label.is_empty();

        self.base
            .id(self.id)
            .flex()
            .items_center()
            .flex_shrink_0()
            .cursor_pointer()
            .overflow_hidden()
            .line_height(height)
            .h(height)
            .overflow_hidden()
            .text_color(tab_style.fg)
            .map(|this| match self.size {
                Size::XSmall => this.text_xs(),
                Size::Large => this.text_base(),
                _ => this.text_sm(),
            })
            .bg(tab_style.bg)
            .border_l(tab_style.borders.left)
            .border_r(tab_style.borders.right)
            .border_t(tab_style.borders.top)
            .border_b(tab_style.borders.bottom)
            .border_color(tab_style.border_color)
            .rounded(tab_style.radius)
            .when(tab_style.shadow, |this| this.shadow_sm())
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
            .when_some(self.prefix, |this, prefix| this.child(prefix))
            .child(
                div()
                    .h(inner_height)
                    .line_height(inner_height)
                    .paddings(inner_paddings)
                    .margins(inner_margins)
                    .text_ellipsis()
                    .flex_shrink_0()
                    .when(has_label, |this| this.child(self.label))
                    .when(!has_label, |this| this.children(self.children))
                    .bg(tab_style.inner_bg)
                    .rounded(tab_style.inner_radius)
                    .hover(|this| {
                        this.bg(hover_style.inner_bg)
                            .rounded(hover_style.inner_radius)
                    }),
            )
            .when_some(self.suffix, |this, suffix| this.child(suffix))
            .when(!self.disabled, |this| {
                this.when_some(self.on_click.clone(), |this, on_click| {
                    this.on_click(move |event, window, cx| on_click(event, window, cx))
                })
            })
    }
}
