use std::{rc::Rc, time::Duration};

use crate::animation::{Lerp, ease_in_out_cubic};
use crate::{ActiveTheme, Icon, IconName, Selectable, Sizable, Size, StyledExt, h_flex};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Background, ClickEvent, Div, Edges, ElementId,
    Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px, relative, rems,
};

/// Tab variants.
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash)]
pub enum TabVariant {
    #[default]
    Tab,
    Outline,
    Pill,
    Segmented,
    Underline,
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
            Size::Large => match self {
                TabVariant::Underline => px(44.),
                _ => px(36.),
            },
            _ => match self {
                TabVariant::Underline => px(36.),
                _ => px(32.),
            },
        }
    }

    pub(super) fn inner_height(&self, size: Size) -> Pixels {
        match size {
            Size::XSmall => match self {
                TabVariant::Tab | TabVariant::Outline | TabVariant::Pill => px(18.),
                TabVariant::Segmented => px(16.),
                TabVariant::Underline => px(20.),
            },
            Size::Small => match self {
                TabVariant::Tab | TabVariant::Outline | TabVariant::Pill => px(22.),
                TabVariant::Segmented => px(18.),
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
            padding_x = px(0.);
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
                bg: cx.theme().transparent.into(),
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges::all(px(1.)),
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().foreground,
                bg: cx.theme().transparent.into(),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: cx.theme().transparent.into(),
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
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().secondary_foreground,
                bg: cx.theme().tokens.secondary_hover.into(),
                borders: Edges::all(px(1.)),
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().secondary_foreground,
                bg: cx.theme().tokens.secondary.into(),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: if selected {
                    cx.theme().tokens.background.into()
                } else {
                    cx.theme().transparent.into()
                },
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: cx.theme().transparent.into(),
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
                bg: cx.theme().tokens.tab_active.into(),
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().primary,
                bg: cx.theme().transparent.into(),
                borders: Edges::all(px(1.)),
                border_color: cx.theme().primary,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().primary_foreground,
                bg: cx.theme().tokens.primary.into(),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: cx.theme().tokens.background.into(),
                shadow: true,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
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
                bg: cx.theme().transparent.into(),
                border_color: if selected {
                    cx.theme().border
                } else {
                    cx.theme().transparent
                },
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges::all(px(1.)),
                border_color: if selected {
                    cx.theme().primary
                } else {
                    cx.theme().border
                },
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: if selected {
                    cx.theme().primary_foreground.opacity(0.5)
                } else {
                    cx.theme().muted_foreground
                },
                bg: if selected {
                    cx.theme().primary.opacity(0.5).into()
                } else {
                    cx.theme().transparent.into()
                },
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().tokens.tab_bar.into(),
                inner_bg: if selected {
                    cx.theme().tokens.background.into()
                } else {
                    cx.theme().transparent.into()
                },
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent.into(),
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

    pub(super) fn tab_bar_radius(&self, size: Size, cx: &App) -> Pixels {
        if *self != TabVariant::Segmented {
            return px(0.);
        }

        match size {
            Size::XSmall | Size::Small => cx.theme().radius,
            Size::Large => cx.theme().radius_lg,
            _ => cx.theme().radius_lg,
        }
    }

    fn radius(&self, size: Size, cx: &App) -> Pixels {
        match self {
            TabVariant::Outline | TabVariant::Pill => px(99.),
            TabVariant::Segmented => match size {
                Size::XSmall | Size::Small => cx.theme().radius,
                Size::Large => cx.theme().radius_lg,
                _ => cx.theme().radius_lg,
            },
            _ => px(0.),
        }
    }

    pub(super) fn inner_radius(&self, size: Size, cx: &App) -> Pixels {
        match self {
            TabVariant::Segmented => match size {
                Size::Large => self.tab_bar_radius(size, cx) - px(3.),
                _ => self.tab_bar_radius(size, cx) - px(2.),
            },
            _ => px(0.),
        }
    }
}

#[allow(dead_code)]
struct TabStyle {
    borders: Edges<Pixels>,
    border_color: Hsla,
    bg: Background,
    fg: Hsla,
    shadow: bool,
    inner_bg: Background,
}

impl Default for TabStyle {
    fn default() -> Self {
        TabStyle {
            borders: Edges::all(px(0.)),
            border_color: gpui::transparent_white(),
            bg: gpui::transparent_white().into(),
            fg: gpui::transparent_white(),
            shadow: false,
            inner_bg: gpui::transparent_white().into(),
        }
    }
}

/// A Tab element for the [`super::TabBar`].
#[derive(IntoElement)]
pub struct Tab {
    ix: usize,
    base: Div,
    pub(super) label: Option<SharedString>,
    aria_label: Option<SharedString>,
    pub(super) icon: Option<Icon>,
    prefix: Option<AnyElement>,
    pub(super) tab_bar_prefix: Option<bool>,
    suffix: Option<AnyElement>,
    children: Vec<AnyElement>,
    variant: TabVariant,
    size: Size,
    pub(super) disabled: bool,
    pub(super) selected: bool,
    pub(super) indicator_active: bool,
    pub(super) indicator_ready: bool,
    /// Animation epoch of the [`super::TabBar`] indicator; increments on every
    /// tab switch. Used to key the selected tab's text color fade so it
    /// restarts in sync with the indicator slide.
    pub(super) indicator_epoch: u64,
    pub(super) max_tab_width: Option<Pixels>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl From<&'static str> for Tab {
    fn from(label: &'static str) -> Self {
        Self::new().label(label)
    }
}

impl From<String> for Tab {
    fn from(label: String) -> Self {
        Self::new().label(label)
    }
}

impl From<SharedString> for Tab {
    fn from(label: SharedString) -> Self {
        Self::new().label(label)
    }
}

impl From<Icon> for Tab {
    fn from(icon: Icon) -> Self {
        Self::default().icon(icon)
    }
}

impl From<IconName> for Tab {
    fn from(icon_name: IconName) -> Self {
        Self::default().icon(Icon::new(icon_name))
    }
}

impl Default for Tab {
    fn default() -> Self {
        Self {
            ix: 0,
            base: div(),
            label: None,
            aria_label: None,
            icon: None,
            tab_bar_prefix: None,
            children: Vec::new(),
            disabled: false,
            selected: false,
            indicator_active: false,
            indicator_ready: true,
            indicator_epoch: 0,
            prefix: None,
            suffix: None,
            variant: TabVariant::default(),
            size: Size::default(),
            max_tab_width: None,
            on_click: None,
        }
    }
}

impl Tab {
    /// Create a new tab with a label.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set label for the tab.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the accessible label for the tab.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    fn a11y_label(&self) -> Option<SharedString> {
        self.aria_label.clone().or_else(|| self.label.clone())
    }

    /// Set icon for the tab.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
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
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Set the right side of the tab
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Set disabled state to the tab, default false.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the click handler for the tab.
    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// Set index to the tab.
    pub(crate) fn ix(mut self, ix: usize) -> Self {
        self.ix = ix;
        self
    }

    /// Set if the tab bar has a prefix.
    pub(crate) fn tab_bar_prefix(mut self, tab_bar_prefix: bool) -> Self {
        self.tab_bar_prefix = Some(tab_bar_prefix);
        self
    }

    pub(super) fn max_tab_width(mut self, max_tab_width: Option<Pixels>) -> Self {
        self.max_tab_width = max_tab_width;
        self
    }
}

/// Measure the label's rendered width, rounded up to a whole pixel.
///
/// The truncating label wrapper needs an explicit integer width: if it were
/// auto-sized, its width would equal the fractional text width exactly, and
/// whole-pixel layout rounding could leave it under by <1px — enough for the
/// ellipsis to kick in on a label that actually fits. Whole-pixel widths
/// survive rounding unchanged.
///
/// The font size mirrors the `text_xs`/`text_sm`/`text_base` classes applied
/// in `render` (0.75/0.875/1.0 rem); keep the two in sync.
fn measured_label_width(label: &SharedString, size: Size, window: &mut Window) -> Pixels {
    if label.is_empty() {
        return px(0.);
    }

    let font_size = match size {
        Size::XSmall => rems(0.75),
        Size::Large => rems(1.0),
        _ => rems(0.875),
    }
    .to_pixels(window.rem_size());

    let text_style = window.text_style();
    let width = window
        .text_system()
        .shape_line(
            label.clone(),
            font_size,
            &[text_style.to_run(label.len())],
            None,
        )
        .width();
    width.ceil()
}

impl ParentElement for Tab {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Selectable for Tab {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
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
        let tab_bar_prefix = self.tab_bar_prefix.unwrap_or_default();
        if !tab_bar_prefix {
            if self.ix == 0 && self.variant == TabVariant::Tab {
                tab_style.borders.left = px(0.);
                hover_style.borders.left = px(0.);
            }
        }
        let radius = self.variant.radius(self.size, cx);
        let inner_radius = self.variant.inner_radius(self.size, cx);
        let inner_paddings = self.variant.inner_paddings(self.size);
        let inner_margins = self.variant.inner_margins(self.size);
        let inner_height = self.variant.inner_height(self.size);
        let height = self.variant.height(self.size);
        let aria_label = self.a11y_label();

        let segmented_indicator_active =
            self.variant == TabVariant::Segmented && self.indicator_active;
        let has_inline_inner_bg =
            self.selected && segmented_indicator_active && !self.indicator_ready;
        let inline_inner_bg = tab_style.inner_bg;
        let (inner_bg, hover_inner_bg) = if segmented_indicator_active && self.indicator_ready {
            (cx.theme().transparent.into(), cx.theme().transparent.into())
        } else if has_inline_inner_bg {
            (inline_inner_bg, inline_inner_bg)
        } else {
            (tab_style.inner_bg, hover_style.inner_bg)
        };
        let inner_shadow = tab_style.shadow && !segmented_indicator_active;

        // When a sliding indicator is active and ready, it alone represents the
        // selected state. Suppress the selected tab's own active background/border
        // so the two don't overlap during the switch animation (Segmented already
        // does this for its `inner_bg` above). Skip disabled tabs so a
        // disabled-selected tab keeps its dimmed styling instead of the
        // full-strength indicator color.
        let suppress_active_visual =
            self.selected && !self.disabled && self.indicator_active && self.indicator_ready;
        // Pill paints its active state via the outer `bg`.
        let outer_bg = if suppress_active_visual && self.variant == TabVariant::Pill {
            cx.theme().transparent.into()
        } else {
            tab_style.bg
        };
        // Underline paints its active state via the bottom `border_color`.
        let outer_border_color = if suppress_active_visual && self.variant == TabVariant::Underline
        {
            cx.theme().transparent
        } else {
            tab_style.border_color
        };

        // For Pill, the newly selected tab's text color (`primary_foreground`)
        // would otherwise snap to white instantly while the indicator is still
        // sliding into place. Fade it from the normal color in sync with the
        // indicator slide (keyed on the indicator epoch so it restarts on each
        // switch). `epoch == 0` is the initial layout (no slide), so we skip it.
        let animate_fg = self.selected
            && !self.disabled
            && self.variant == TabVariant::Pill
            && self.indicator_active
            && self.indicator_ready
            && self.indicator_epoch > 0;
        let fg_from = self.variant.normal(cx).fg;
        let fg_to = tab_style.fg;
        // Icon-only tabs are fixed-size and exempt from `max_tab_width`.
        let max_tab_width = self.max_tab_width.filter(|_| self.icon.is_none());

        let inner_content = h_flex()
            .h(inner_height)
            .line_height(relative(1.))
            .whitespace_nowrap()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .margins(inner_margins)
            .when(max_tab_width.is_none(), |this| this.flex_shrink_0())
            .map(|this| match self.icon {
                Some(icon) => this
                    .w(inner_height * 1.25)
                    .child(icon.map(|this| match self.size {
                        Size::XSmall => this.size_2p5(),
                        Size::Small => this.size_3p5(),
                        Size::Large => this.size_4(),
                        _ => this.size_4(),
                    })),
                None => this
                    .paddings(inner_paddings)
                    .map(|this| match (self.label, max_tab_width.is_some()) {
                        (Some(label), true) => {
                            // 2px of slack so whole-pixel layout rounding in the
                            // ancestor chain lands in empty space instead of
                            // shrinking the wrapper below the text width.
                            let label_width =
                                measured_label_width(&label, self.size, window) + px(2.);
                            this.child(div().w(label_width).min_w_0().truncate().child(label))
                        }
                        (Some(label), false) => this.child(label),
                        (None, _) => this,
                    })
                    .children(self.children),
            })
            .bg(inner_bg)
            .rounded(inner_radius)
            .when(inner_shadow, |this| this.shadow_xs())
            .hover(|this| this.bg(hover_inner_bg).rounded(inner_radius));

        let inner_element = if animate_fg {
            inner_content
                .with_animation(
                    ElementId::NamedInteger("tab-fg".into(), self.indicator_epoch),
                    Animation::new(Duration::from_millis(200)).with_easing(ease_in_out_cubic),
                    move |this, delta| this.text_color(Lerp::lerp(&fg_from, &fg_to, delta)),
                )
                .into_any_element()
        } else {
            inner_content.into_any_element()
        };

        // When width-constrained, group the prefix and label into a collapsible
        // wrapper that absorbs all of the clipping, so the suffix (e.g. a close
        // button) always stays fully visible at the trailing edge.
        let mut prefix = self.prefix;
        let content = if max_tab_width.is_some() && (prefix.is_some() || self.suffix.is_some()) {
            h_flex()
                .flex_grow_1()
                .min_w_0()
                .overflow_hidden()
                .gap_1()
                .when_some(prefix.take(), |this, prefix| {
                    this.child(div().flex_shrink_0().child(prefix))
                })
                .child(inner_element)
                .into_any_element()
        } else {
            inner_element
        };

        self.base
            .id(self.ix)
            .role(Role::Tab)
            .when_some(aria_label, |this, label| this.aria_label(label))
            .aria_selected(self.selected)
            .relative()
            .flex()
            .map(|this| {
                if max_tab_width.is_some() {
                    this.flex_nowrap()
                } else {
                    this.flex_wrap()
                }
            })
            .gap_1()
            .items_center()
            .flex_shrink_0()
            .when_some(max_tab_width, |this, max_width| this.max_w(max_width))
            .h(height)
            .overflow_hidden()
            .text_color(tab_style.fg)
            .map(|this| match self.size {
                Size::XSmall => this.text_xs(),
                Size::Large => this.text_base(),
                _ => this.text_sm(),
            })
            .bg(outer_bg)
            .border_l(tab_style.borders.left)
            .border_r(tab_style.borders.right)
            .border_t(tab_style.borders.top)
            .border_b(tab_style.borders.bottom)
            .border_color(outer_border_color)
            .rounded(radius)
            .when(!self.selected && !self.disabled, |this| {
                this.hover(|this| {
                    this.text_color(hover_style.fg)
                        .bg(hover_style.bg)
                        .border_l(hover_style.borders.left)
                        .border_r(hover_style.borders.right)
                        .border_t(hover_style.borders.top)
                        .border_b(hover_style.borders.bottom)
                        .border_color(hover_style.border_color)
                        .rounded(radius)
                })
            })
            .when(has_inline_inner_bg, |this| {
                this.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top_0()
                        .bottom_0()
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .w_full()
                                .h(inner_height)
                                .bg(inline_inner_bg)
                                .rounded(inner_radius)
                                .when(tab_style.shadow, |this| this.shadow_xs()),
                        ),
                )
            })
            .when_some(prefix, |this, prefix| this.child(prefix))
            .child(content)
            .when_some(self.suffix, |this, suffix| {
                if max_tab_width.is_some() {
                    this.child(div().flex_shrink_0().child(suffix))
                } else {
                    this.child(suffix)
                }
            })
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                // Stop propagation behavior, for works on TitleBar.
                // https://github.com/longbridge/gpui-component/issues/1836
                cx.stop_propagation();
            })
            .when(!self.disabled, |this| {
                this.when_some(self.on_click.clone(), |this, on_click| {
                    this.on_click(move |event, window, cx| on_click(event, window, cx))
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn a11y_label_defaults_to_visible_label(_cx: &mut gpui::TestAppContext) {
        let tab = Tab::new().label("Account");

        assert_eq!(tab.a11y_label(), Some("Account".into()));
    }

    #[gpui::test]
    fn explicit_a11y_label_overrides_visible_label(_cx: &mut gpui::TestAppContext) {
        let tab = Tab::new().label("Acct").aria_label("Account settings");

        assert_eq!(tab.a11y_label(), Some("Account settings".into()));
    }

    #[gpui::test]
    fn max_tab_width_caps_rendered_tab_bounds(cx: &mut gpui::TestAppContext) {
        use crate::tab::TabBar;
        use gpui::{Context, Render};

        struct TabsView;

        impl Render for TabsView {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                TabBar::new("tabs")
                    .max_tab_width(px(100.))
                    .selected_index(0)
                    .child(
                        Tab::new()
                            .label("Account Settings & Preferences")
                            .debug_selector(|| "long-tab".into()),
                    )
                    .child(Tab::new().label("Go").debug_selector(|| "short-tab".into()))
            }
        }

        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, _| TabsView);
        cx.run_until_parked();

        let long = cx.debug_bounds("long-tab").expect("long tab not rendered");
        assert!(
            long.size.width <= px(100.),
            "long tab width {:?} exceeds max_tab_width",
            long.size.width
        );

        let short = cx
            .debug_bounds("short-tab")
            .expect("short tab not rendered");
        assert!(
            short.size.width < long.size.width,
            "short tab ({:?}) should shrink to its content, not fill max width ({:?})",
            short.size.width,
            long.size.width
        );
    }

    #[gpui::test]
    fn max_tab_width_keeps_suffix_visible_and_right_aligned(cx: &mut gpui::TestAppContext) {
        use crate::tab::TabBar;
        use crate::{Icon, IconName};
        use gpui::{Context, Render};

        struct TabsView {
            max_width: Pixels,
        }

        impl Render for TabsView {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                TabBar::new("tabs")
                    .segmented()
                    .max_tab_width(self.max_width)
                    .selected_index(0)
                    .child(
                        Tab::new()
                            .px_2()
                            .prefix(Icon::new(IconName::BookOpen))
                            .label("A Very Long Dynamic Tab Label")
                            .suffix(
                                div()
                                    .w(px(16.))
                                    .h(px(16.))
                                    .debug_selector(|| "suffix".into()),
                            )
                            .selected(true)
                            .debug_selector(|| "tab".into()),
                    )
            }
        }

        // 60 leaves no room for the label at all; the suffix must survive both.
        for max_width in [60., 100.] {
            cx.update(crate::init);
            let (_, cx) = cx.add_window_view(|_, _| TabsView {
                max_width: px(max_width),
            });
            cx.run_until_parked();

            let tab = cx.debug_bounds("tab").expect("tab not rendered");
            let suffix = cx.debug_bounds("suffix").expect("suffix not rendered");
            let tab_right = tab.origin.x + tab.size.width;
            let suffix_right = suffix.origin.x + suffix.size.width;

            assert!(tab.size.width <= px(max_width));
            assert_eq!(
                suffix.size.width,
                px(16.),
                "suffix squeezed at max_width {max_width}"
            );
            assert!(
                suffix_right <= tab_right,
                "suffix clipped at max_width {max_width}: suffix right {suffix_right:?} > tab right {tab_right:?}"
            );
            assert!(
                tab_right - suffix_right <= px(12.),
                "suffix not right-aligned at max_width {max_width}: gap {:?}",
                tab_right - suffix_right
            );
        }
    }

    /// Tabs with identical content must lay out identically regardless of
    /// position or selection. This regressed once via under-measured intrinsic
    /// widths (basis-0 nesting) that sporadically ellipsized labels that fit.
    #[gpui::test]
    fn max_tab_width_gives_equal_tabs_equal_widths(cx: &mut gpui::TestAppContext) {
        use crate::tab::TabBar;
        use crate::{Icon, IconName};
        use gpui::{Context, Render};

        struct TabsView;

        impl Render for TabsView {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                TabBar::new("tabs")
                    .segmented()
                    .max_tab_width(px(140.))
                    .selected_index(0)
                    .children((0..3).map(|id| {
                        Tab::new()
                            .px_2()
                            .prefix(Icon::new(IconName::BookOpen))
                            .label(format!("Tab {id}"))
                            .suffix(div().w(px(16.)).h(px(16.)))
                            .debug_selector(move || format!("tab-{id}"))
                    }))
            }
        }

        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, _| TabsView);
        cx.run_until_parked();

        let widths: Vec<Pixels> = ["tab-0", "tab-1", "tab-2"]
            .iter()
            .map(|name| cx.debug_bounds(name).expect("tab not rendered").size.width)
            .collect();

        assert!(
            widths[1] < px(140.),
            "tab should shrink to content below max width, got {:?}",
            widths[1]
        );
        assert_eq!(
            widths[0], widths[1],
            "first tab must match its identical siblings"
        );
        assert_eq!(widths[1], widths[2]);
    }
}
