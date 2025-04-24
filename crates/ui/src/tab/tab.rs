use std::sync::Arc;

use crate::{h_flex, ActiveTheme, Icon, IconName, Selectable, Sizable, Size, StyledExt};
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
    Outline,
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
                TabVariant::Underline => px(26.),
                _ => px(20.),
            },
            Size::Small => match self {
                TabVariant::Underline => px(30.),
                _ => px(24.),
            },
            Size::Large => px(36.),
            _ => match self {
                TabVariant::Underline => px(36.),
                _ => px(32.),
            },
        }
    }

    fn inner_height(&self, size: Size) -> Pixels {
        match size {
            Size::XSmall => match self {
                TabVariant::Tab | TabVariant::Outline | TabVariant::Pill => px(20.),
                TabVariant::Segmented => px(16.),
                TabVariant::Underline => px(20.),
            },
            Size::Small => match self {
                TabVariant::Tab | TabVariant::Outline | TabVariant::Pill => px(24.),
                TabVariant::Segmented => px(20.),
                TabVariant::Underline => px(22.),
            },
            Size::Large => match self {
                TabVariant::Tab | TabVariant::Outline | TabVariant::Pill => px(36.),
                TabVariant::Segmented => px(28.),
                TabVariant::Underline => px(32.),
            },
            _ => match self {
                TabVariant::Tab => px(30.),
                TabVariant::Outline | TabVariant::Pill => px(26.),
                TabVariant::Segmented => px(24.),
                TabVariant::Underline => px(26.),
            },
        }
    }

    /// Default px(12) to match panel px_3, See [`crate::dock::TabPanel`]
    fn inner_paddings(&self, size: Size) -> Edges<Pixels> {
        let mut padding_x = match size {
            Size::XSmall => px(8.),
            Size::Small => px(10.),
            Size::Large => px(16.),
            _ => px(12.),
        };

        if matches!(self, TabVariant::Underline) {
            padding_x = padding_x / 2.;
        }

        Edges {
            left: padding_x,
            right: padding_x,
            ..Default::default()
        }
    }

    fn inner_margins(&self, size: Size) -> Edges<Pixels> {
        match size {
            Size::XSmall => match self {
                TabVariant::Underline => Edges {
                    top: px(1.),
                    bottom: px(2.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
            Size::Small => match self {
                TabVariant::Underline => Edges {
                    top: px(2.),
                    bottom: px(3.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
            Size::Large => match self {
                TabVariant::Underline => Edges {
                    top: px(5.),
                    bottom: px(6.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
            _ => match self {
                TabVariant::Underline => Edges {
                    top: px(3.),
                    bottom: px(4.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
        }
    }

    fn normal(&self, cx: &App) -> TabStyle {
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
            TabVariant::Outline => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent,
                borders: Edges::all(px(1.)),
                border_color: cx.theme().border,
                radius: px(99.),
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().foreground,
                bg: cx.theme().transparent,
                radius: px(99.),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent,
                inner_radius: cx.theme().radius,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_foreground,
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

    fn hovered(&self, selected: bool, cx: &App) -> TabStyle {
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
            TabVariant::Outline => TabStyle {
                fg: cx.theme().secondary_foreground,
                bg: cx.theme().secondary_hover,
                borders: Edges::all(px(1.)),
                border_color: cx.theme().border,
                radius: px(99.),
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().secondary_foreground,
                bg: cx.theme().secondary,
                radius: px(99.),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent,
                inner_bg: if selected {
                    cx.theme().background
                } else {
                    cx.theme().transparent
                },
                inner_radius: cx.theme().radius,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_foreground,
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
            TabVariant::Outline => TabStyle {
                fg: cx.theme().primary,
                bg: cx.theme().transparent,
                borders: Edges::all(px(1.)),
                border_color: cx.theme().primary,
                radius: px(99.),
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().primary_foreground,
                bg: cx.theme().primary,
                radius: px(99.),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent,
                inner_radius: cx.theme().radius,
                inner_bg: cx.theme().background,
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
            TabVariant::Outline => TabStyle {
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
            TabVariant::Pill => TabStyle {
                fg: if selected {
                    cx.theme().primary_foreground.opacity(0.5)
                } else {
                    cx.theme().muted_foreground
                },
                bg: if selected {
                    cx.theme().primary.opacity(0.5)
                } else {
                    cx.theme().transparent
                },
                radius: px(99.),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().tab_bar,
                inner_bg: if selected {
                    cx.theme().background
                } else {
                    cx.theme().transparent
                },
                inner_radius: cx.theme().radius,
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
    pub(super) label: SharedString,
    icon: Option<Icon>,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    children: Vec<AnyElement>,
    variant: TabVariant,
    size: Size,
    pub(super) disabled: bool,
    pub(super) selected: bool,
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

impl From<Icon> for Tab {
    fn from(icon: Icon) -> Self {
        Self::icon(icon)
    }
}

impl From<IconName> for Tab {
    fn from(icon_name: IconName) -> Self {
        Self::icon(Icon::new(icon_name))
    }
}

impl Default for Tab {
    fn default() -> Self {
        Self {
            id: ElementId::Integer(0),
            base: div().gap_1(),
            label: SharedString::new(""),
            icon: None,
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
}

impl Tab {
    /// Create a new tab with a label.
    pub fn new(label: impl Into<SharedString>) -> Self {
        let mut this = Self::default();
        this.label = label.into();
        this
    }

    /// Create a Icon tab.
    pub fn icon(icon: impl Into<Icon>) -> Self {
        let mut this = Self::default();
        this.icon = Some(icon.into());
        this
    }

    /// Set id to the tab.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Set Tab Variant.
    pub fn with_variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Use Pill variant.
    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    /// Use outline variant.
    pub fn outline(mut self) -> Self {
        self.variant = TabVariant::Outline;
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
        let mut hover_style = self.variant.hovered(self.selected, cx);
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
            .flex_wrap()
            .items_center()
            .flex_shrink_0()
            .overflow_hidden()
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
                h_flex()
                    .h(inner_height)
                    .line_height(inner_height)
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .margins(inner_margins)
                    .flex_shrink_0()
                    .map(|this| match self.icon {
                        Some(icon) => {
                            this.w(inner_height * 1.25)
                                .child(icon.map(|this| match self.size {
                                    Size::XSmall => this.size_2p5(),
                                    Size::Small => this.size_3p5(),
                                    Size::Large => this.size_4(),
                                    _ => this.size_4(),
                                }))
                        }
                        None => this
                            .paddings(inner_paddings)
                            .when(has_label, |this| this.child(self.label))
                            .when(!has_label, |this| this.children(self.children)),
                    })
                    .bg(tab_style.inner_bg)
                    .rounded(tab_style.inner_radius)
                    .when(tab_style.shadow, |this| this.shadow_sm())
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
