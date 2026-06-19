use std::rc::Rc;

use crate::{
    ActiveTheme, Colorize as _, Disableable, FocusableExt as _, Icon, IconName, Selectable,
    Sizable, Size, StyleSized, StyledExt,
    button::ButtonIcon,
    h_flex,
    tooltip::{ManagedTooltipExt as _, Tooltip},
};
use gpui::{
    AnyElement, App, Background, ClickEvent, Corners, Div, Edges, ElementId, Hsla,
    InteractiveElement, Interactivity, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce,
    SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, px, relative, transparent_white,
};

#[derive(Default, Clone, Copy)]
pub enum ButtonRounded {
    None,
    Small,
    #[default]
    Medium,
    Large,
    Size(Pixels),
}

impl From<Pixels> for ButtonRounded {
    fn from(px: Pixels) -> Self {
        ButtonRounded::Size(px)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ButtonCustomVariant {
    color: Hsla,
    foreground: Hsla,
    shadow: bool,
    hover: Hsla,
    active: Hsla,
}

pub trait ButtonVariants: Sized {
    fn with_variant(self, variant: ButtonVariant) -> Self;

    /// With the primary style for the Button.
    fn primary(self) -> Self {
        self.with_variant(ButtonVariant::Primary)
    }

    /// With the secondary style for the Button.
    fn secondary(self) -> Self {
        self.with_variant(ButtonVariant::Secondary)
    }

    /// With the danger style for the Button.
    fn danger(self) -> Self {
        self.with_variant(ButtonVariant::Danger)
    }

    /// With the warning style for the Button.
    fn warning(self) -> Self {
        self.with_variant(ButtonVariant::Warning)
    }

    /// With the success style for the Button.
    fn success(self) -> Self {
        self.with_variant(ButtonVariant::Success)
    }

    /// With the info style for the Button.
    fn info(self) -> Self {
        self.with_variant(ButtonVariant::Info)
    }

    /// With the ghost style for the Button.
    fn ghost(self) -> Self {
        self.with_variant(ButtonVariant::Ghost)
    }

    /// With the link style for the Button.
    fn link(self) -> Self {
        self.with_variant(ButtonVariant::Link)
    }

    /// With the text style for the Button, it will no padding look like a normal text.
    fn text(self) -> Self {
        self.with_variant(ButtonVariant::Text)
    }

    /// With the custom style for the Button.
    fn custom(self, style: ButtonCustomVariant) -> Self {
        self.with_variant(ButtonVariant::Custom(style))
    }
}

impl ButtonCustomVariant {
    pub fn new(cx: &App) -> Self {
        Self {
            color: cx.theme().transparent,
            foreground: cx.theme().foreground,
            hover: cx.theme().transparent,
            active: cx.theme().transparent,
            shadow: false,
        }
    }

    /// Set background color, default is transparent.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = color;
        self
    }

    /// Set foreground color, default is theme foreground.
    pub fn foreground(mut self, color: Hsla) -> Self {
        self.foreground = color;
        self
    }

    /// Set hover background color, default is transparent.
    pub fn hover(mut self, color: Hsla) -> Self {
        self.hover = color;
        self
    }

    /// Set active background color, default is transparent.
    pub fn active(mut self, color: Hsla) -> Self {
        self.active = color;
        self
    }

    /// Set shadow, default is false.
    pub fn shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }
}

/// The variant of the Button.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ButtonVariant {
    #[default]
    Default,
    Primary,
    Secondary,
    Danger,
    Info,
    Success,
    Warning,
    Ghost,
    Link,
    Text,
    Custom(ButtonCustomVariant),
}

impl ButtonVariant {
    #[inline]
    pub fn is_link(&self) -> bool {
        matches!(self, Self::Link)
    }

    #[inline]
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text)
    }

    #[inline]
    pub fn is_ghost(&self) -> bool {
        matches!(self, Self::Ghost)
    }

    #[inline]
    fn no_padding(&self) -> bool {
        self.is_link() || self.is_text()
    }

    #[inline]
    fn is_default(&self) -> bool {
        matches!(self, Self::Default)
    }
}

/// A Button element.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    icon: Option<ButtonIcon>,
    label: Option<SharedString>,
    children: Vec<AnyElement>,
    disabled: bool,
    pub(crate) selected: bool,
    variant: ButtonVariant,
    rounded: ButtonRounded,
    outline: bool,
    border_corners: Corners<bool>,
    border_edges: Edges<bool>,
    dropdown_caret: bool,
    size: Size,
    compact: bool,
    tooltip: Option<(
        SharedString,
        Option<(Rc<Box<dyn gpui::Action>>, Option<SharedString>)>,
    )>,
    tooltip_builder: Option<Rc<dyn Fn(&mut Window, &mut App) -> gpui::AnyView>>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    on_hover: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    loading: bool,
    loading_icon: Option<Icon>,

    tab_index: isize,
    tab_stop: bool,
}

impl From<Button> for AnyElement {
    fn from(button: Button) -> Self {
        button.into_any_element()
    }
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();

        Self {
            id: id.clone(),
            // ID must be set after div is created;
            // `dropdown_menu` uses this id to create the popup menu.
            base: div().flex_shrink_0().id(id),
            style: StyleRefinement::default(),
            icon: None,
            label: None,
            disabled: false,
            selected: false,
            variant: ButtonVariant::default(),
            rounded: ButtonRounded::Medium,
            border_corners: Corners {
                top_left: true,
                top_right: true,
                bottom_right: true,
                bottom_left: true,
            },
            border_edges: Edges::all(true),
            size: Size::Medium,
            tooltip: None,
            tooltip_builder: None,
            on_click: None,
            on_hover: None,
            loading: false,
            compact: false,
            outline: false,
            children: Vec::new(),
            loading_icon: None,
            dropdown_caret: false,
            tab_index: 0,
            tab_stop: true,
        }
    }

    /// Set the outline style of the Button.
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Set the border radius of the Button.
    pub fn rounded(mut self, rounded: impl Into<ButtonRounded>) -> Self {
        self.rounded = rounded.into();
        self
    }

    /// Set the border corners side of the Button.
    pub(crate) fn border_corners(mut self, corners: impl Into<Corners<bool>>) -> Self {
        self.border_corners = corners.into();
        self
    }

    /// Set the border edges of the Button.
    pub(crate) fn border_edges(mut self, edges: impl Into<Edges<bool>>) -> Self {
        self.border_edges = edges.into();
        self
    }

    /// Set label to the Button, if no label is set, the button will be in Icon Button mode.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the icon of the button, if the Button have no label, the button well in Icon Button mode.
    pub fn icon(mut self, icon: impl Into<ButtonIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the tooltip of the button.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some((tooltip.into(), None));
        self
    }

    /// Set the tooltip of the button with action to show keybinding.
    pub fn tooltip_with_action(
        mut self,
        tooltip: impl Into<SharedString>,
        action: &dyn gpui::Action,
        context: Option<&str>,
    ) -> Self {
        self.tooltip = Some((
            tooltip.into(),
            Some((
                Rc::new(action.boxed_clone()),
                context.map(|c| c.to_string().into()),
            )),
        ));
        self
    }

    /// Set true to show the loading indicator.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set the button to compact mode, then padding will be reduced.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Add click handler.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Add hover handler, the bool parameter indicates whether the mouse is hovering.
    pub fn on_hover(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Rc::new(handler));
        self
    }

    /// Set the loading icon of the button, it will be used when loading is true.
    ///
    /// Default is a spinner icon.
    pub fn loading_icon(mut self, icon: impl Into<Icon>) -> Self {
        self.loading_icon = Some(icon.into());
        self
    }

    /// Set the tab index of the button, it will be used to focus the button by tab key.
    ///
    /// Default is 0.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Set the tab stop of the button, if true, the button will be focusable by tab key.
    ///
    /// Default is true.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Set to show a dropdown caret icon at the end of the button.
    pub fn dropdown_caret(mut self, dropdown_caret: bool) -> Self {
        self.dropdown_caret = dropdown_caret;
        self
    }

    #[inline]
    fn clickable(&self) -> bool {
        !(self.disabled || self.loading) && self.on_click.is_some()
    }

    #[inline]
    fn hoverable(&self) -> bool {
        !(self.disabled || self.loading) && self.on_hover.is_some()
    }
}

impl Disableable for Button {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for Button {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Sizable for Button {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ButtonVariants for Button {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl InteractiveElement for Button {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style: ButtonVariant = self.variant;
        let clickable = self.clickable();
        let is_disabled = self.disabled;
        let hoverable = self.hoverable();
        let normal_style = style.normal(self.outline, cx);
        let icon_size = match self.size {
            Size::Size(v) => Size::Size(v * 0.75),
            _ => self.size,
        };

        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);

        let rounding = match self.rounded {
            ButtonRounded::Small => cx.theme().radius * 0.5,
            ButtonRounded::Medium => cx.theme().radius,
            ButtonRounded::Large => cx.theme().radius * 2.0,
            ButtonRounded::Size(px) => px,
            ButtonRounded::None => Pixels::ZERO,
        };

        self.base
            .when(!self.disabled, |this| {
                this.track_focus(
                    &focus_handle
                        .tab_index(self.tab_index)
                        .tab_stop(self.tab_stop),
                )
            })
            .cursor_default()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .cursor_default()
            .when(self.variant.is_link(), |this| this.cursor_pointer())
            .when(cx.theme().shadow && normal_style.shadow, |this| {
                this.shadow_xs()
            })
            .when(!style.no_padding(), |this| {
                if self.label.is_none() && self.children.is_empty() {
                    // Icon Button
                    match self.size {
                        Size::Size(px) => this.size(px),
                        Size::XSmall => this.size_5(),
                        Size::Small => this.size_6(),
                        Size::Large | Size::Medium => this.size_8(),
                    }
                } else {
                    // Normal Button
                    match self.size {
                        Size::Size(size) => this.px(size * 0.2),
                        Size::XSmall => this.h_5().px_1().when(self.compact, |this| this.min_w_5()),
                        Size::Small => this
                            .h_6()
                            .px_3()
                            .when(self.compact, |this| this.min_w_6().px_1p5()),
                        _ => this
                            .h_8()
                            .px_4()
                            .when(self.compact, |this| this.min_w_8().px_2()),
                    }
                }
            })
            .when(self.border_corners.top_left, |this| {
                this.rounded_tl(rounding)
            })
            .when(self.border_corners.top_right, |this| {
                this.rounded_tr(rounding)
            })
            .when(self.border_corners.bottom_left, |this| {
                this.rounded_bl(rounding)
            })
            .when(self.border_corners.bottom_right, |this| {
                this.rounded_br(rounding)
            })
            .when(self.variant.is_default() || self.outline, |this| {
                this.when(self.border_edges.left, |this| this.border_l_1())
                    .when(self.border_edges.right, |this| this.border_r_1())
                    .when(self.border_edges.top, |this| this.border_t_1())
                    .when(self.border_edges.bottom, |this| this.border_b_1())
            })
            .text_color(normal_style.fg)
            .when(self.selected, |this| {
                let selected_style = style.selected(self.outline, cx);
                this.bg(selected_style.bg)
                    .border_color(selected_style.border)
                    .text_color(selected_style.fg)
            })
            .when(!self.disabled && !self.selected, |this| {
                this.border_color(normal_style.border)
                    .bg(normal_style.bg)
                    .when(normal_style.underline, |this| this.text_decoration_1())
                    .hover(|this| {
                        let hover_style = style.hovered(self.outline, cx);
                        this.bg(hover_style.bg)
                            .border_color(hover_style.border)
                            .text_color(hover_style.fg)
                    })
                    .active(|this| {
                        let active_style = style.active(self.outline, cx);
                        this.bg(active_style.bg)
                            .border_color(active_style.border)
                            .text_color(active_style.fg)
                    })
            })
            .when(self.disabled, |this| {
                let disabled_style = style.disabled(self.outline, cx);
                this.bg(disabled_style.bg)
                    .text_color(disabled_style.fg)
                    .border_color(disabled_style.border)
                    .shadow_none()
            })
            .refine_style(&self.style)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                // Stop handle any click event when disabled.
                // To avoid handle dropdown menu open when button is disabled.
                if is_disabled {
                    cx.stop_propagation();
                    return;
                }

                // Avoid focus on mouse down.
                window.prevent_default();

                // Pressing a button must not start the window-level text selection.
                crate::global_state::GlobalState::suppress_text_selection(cx);
            })
            .when_some(self.on_click, |this, on_click| {
                this.on_click(move |event, window, cx| {
                    // Stop handle any click event when disabled.
                    // To avoid handle dropdown menu open when button is disabled.
                    if !clickable {
                        cx.stop_propagation();
                        return;
                    }

                    on_click(event, window, cx);
                })
            })
            .when_some(self.on_hover.filter(|_| hoverable), |this, on_hover| {
                this.on_hover(move |hovered, window, cx| {
                    on_hover(hovered, window, cx);
                })
            })
            .child({
                h_flex()
                    .id("label")
                    .size_full()
                    .items_center()
                    .justify_center()
                    .button_text_size(self.size)
                    .map(|this| match self.size {
                        Size::XSmall => this.gap_1(),
                        Size::Small => this.gap_1(),
                        _ => this.gap_2(),
                    })
                    .when_some(self.icon, |this, icon| {
                        this.child(
                            icon.loading_icon(self.loading_icon)
                                .loading(self.loading)
                                .with_size(icon_size),
                        )
                    })
                    .when_some(self.label, |this, label| {
                        this.child(div().flex_none().line_height(relative(1.)).child(label))
                    })
                    .children(self.children)
                    .when(self.dropdown_caret, |this| {
                        this.justify_between().child(
                            Icon::new(IconName::ChevronDown).xsmall().text_color(
                                match self.disabled {
                                    true => normal_style.fg.opacity(0.3),
                                    false => normal_style.fg.opacity(0.5),
                                },
                            ),
                        )
                    })
            })
            .when(self.loading && !self.disabled, |this| {
                this.bg(normal_style.bg.opacity(0.8))
                    .border_color(normal_style.border.opacity(0.8))
                    .text_color(normal_style.fg.opacity(0.8))
            })
            .map(|this| {
                if let Some(builder) = self.tooltip_builder {
                    this.managed_tooltip(move |window, cx| builder(window, cx))
                } else if let Some((tooltip, action)) = self.tooltip {
                    this.managed_tooltip(move |window, cx| {
                        Tooltip::new(tooltip.clone())
                            .when_some(action.clone(), |this, (action, context)| {
                                this.action(
                                    action.boxed_clone().as_ref(),
                                    context.as_ref().map(|c| c.as_ref()),
                                )
                            })
                            .build(window, cx)
                    })
                } else {
                    this
                }
            })
            .focus_ring(is_focused, px(0.), window, cx)
    }
}

struct ButtonVariantStyle {
    bg: Background,
    border: Hsla,
    fg: Hsla,
    underline: bool,
    shadow: bool,
}

#[derive(Clone, Copy)]
enum ButtonStyleState {
    Normal,
    Hovered,
    Active,
}

impl ButtonVariant {
    fn outline_background(&self, state: ButtonStyleState, cx: &mut App) -> Background {
        match (self, state) {
            (Self::Default, ButtonStyleState::Normal) => cx.theme().input_background().into(),
            (Self::Default, ButtonStyleState::Hovered) => cx
                .theme()
                .input
                .mix_oklab(cx.theme().transparent, 0.5)
                .into(),
            (Self::Default, ButtonStyleState::Active) => cx
                .theme()
                .input
                .mix_oklab(cx.theme().transparent, 0.7)
                .into(),
            (Self::Primary, ButtonStyleState::Normal) => {
                cx.theme().tokens.primary.background.opacity(0.1)
            }
            (Self::Primary, ButtonStyleState::Hovered) => {
                cx.theme().tokens.primary_hover.background.opacity(0.2)
            }
            (Self::Primary, ButtonStyleState::Active) => {
                cx.theme().tokens.primary_active.background.opacity(0.4)
            }
            (Self::Secondary, ButtonStyleState::Normal) => {
                cx.theme().tokens.secondary.background.opacity(0.1)
            }
            (Self::Secondary, ButtonStyleState::Hovered) => {
                cx.theme().tokens.secondary_hover.background.opacity(0.2)
            }
            (Self::Secondary, ButtonStyleState::Active) => {
                cx.theme().tokens.secondary_active.background.opacity(0.4)
            }
            (Self::Danger, ButtonStyleState::Normal) => {
                cx.theme().tokens.danger.background.opacity(0.1)
            }
            (Self::Danger, ButtonStyleState::Hovered) => {
                cx.theme().tokens.danger_hover.background.opacity(0.2)
            }
            (Self::Danger, ButtonStyleState::Active) => {
                cx.theme().tokens.danger_active.background.opacity(0.4)
            }
            (Self::Warning, ButtonStyleState::Normal) => {
                cx.theme().tokens.warning.background.opacity(0.1)
            }
            (Self::Warning, ButtonStyleState::Hovered) => {
                cx.theme().tokens.warning_hover.background.opacity(0.2)
            }
            (Self::Warning, ButtonStyleState::Active) => {
                cx.theme().tokens.warning_active.background.opacity(0.4)
            }
            (Self::Success, ButtonStyleState::Normal) => {
                cx.theme().tokens.success.background.opacity(0.1)
            }
            (Self::Success, ButtonStyleState::Hovered) => {
                cx.theme().tokens.success_hover.background.opacity(0.2)
            }
            (Self::Success, ButtonStyleState::Active) => {
                cx.theme().tokens.success_active.background.opacity(0.4)
            }
            (Self::Info, ButtonStyleState::Normal) => {
                cx.theme().tokens.info.background.opacity(0.1)
            }
            (Self::Info, ButtonStyleState::Hovered) => {
                cx.theme().tokens.info_hover.background.opacity(0.2)
            }
            (Self::Info, ButtonStyleState::Active) => {
                cx.theme().tokens.info_active.background.opacity(0.4)
            }
            (Self::Ghost | Self::Link | Self::Text, _) => cx.theme().transparent.into(),
            (Self::Custom(colors), _) => colors.color.mix_oklab(cx.theme().transparent, 0.2).into(),
        }
    }

    fn bg_color(&self, outline: bool, cx: &mut App) -> Background {
        if outline {
            return self.outline_background(ButtonStyleState::Normal, cx);
        }

        match self {
            Self::Default => cx.theme().tokens.button.into(),
            Self::Primary => cx.theme().tokens.button_primary.into(),
            Self::Secondary => cx.theme().tokens.button_secondary.into(),
            Self::Danger => cx.theme().tokens.button_danger.into(),
            Self::Warning => cx.theme().tokens.button_warning.into(),
            Self::Success => cx.theme().tokens.button_success.into(),
            Self::Info => cx.theme().tokens.button_info.into(),
            Self::Ghost | Self::Link | Self::Text => cx.theme().transparent.into(),
            Self::Custom(colors) => colors.color.mix_oklab(cx.theme().transparent, 0.2).into(),
        }
    }

    fn text_color(&self, outline: bool, cx: &mut App) -> Hsla {
        match self {
            Self::Default => cx.theme().button_foreground,
            Self::Primary => {
                if outline {
                    cx.theme().primary
                } else {
                    cx.theme().button_primary_foreground
                }
            }
            Self::Secondary => {
                if outline {
                    cx.theme().secondary_foreground
                } else {
                    cx.theme().button_secondary_foreground
                }
            }
            Self::Ghost => cx.theme().secondary_foreground,
            Self::Danger => {
                if outline {
                    cx.theme().danger
                } else {
                    cx.theme().button_danger_foreground
                }
            }
            Self::Warning => {
                if outline {
                    cx.theme().warning
                } else {
                    cx.theme().button_warning_foreground
                }
            }
            Self::Success => {
                if outline {
                    cx.theme().success
                } else {
                    cx.theme().button_success_foreground
                }
            }
            Self::Info => {
                if outline {
                    cx.theme().info
                } else {
                    cx.theme().button_info_foreground
                }
            }
            Self::Link => cx.theme().link,
            Self::Text => cx.theme().foreground,
            Self::Custom(colors) => colors.color,
        }
    }

    fn border_color(&self, outline: bool, cx: &mut App) -> Hsla {
        match self {
            Self::Default => cx.theme().input,
            Self::Secondary => cx.theme().border,
            Self::Primary => cx.theme().primary,
            Self::Danger => {
                if outline {
                    cx.theme().danger.mix_oklab(transparent_white(), 0.4)
                } else {
                    cx.theme().button_danger
                }
            }
            Self::Info => {
                if outline {
                    cx.theme().info.mix_oklab(transparent_white(), 0.4)
                } else {
                    cx.theme().button_info
                }
            }
            Self::Warning => {
                if outline {
                    cx.theme().warning.mix_oklab(transparent_white(), 0.4)
                } else {
                    cx.theme().button_warning
                }
            }
            Self::Success => {
                if outline {
                    cx.theme().success.mix_oklab(transparent_white(), 0.4)
                } else {
                    cx.theme().button_success
                }
            }
            Self::Ghost | Self::Link | Self::Text => cx.theme().transparent,
            Self::Custom(colors) => {
                if outline {
                    colors.color.mix_oklab(transparent_white(), 0.4)
                } else {
                    colors.color
                }
            }
        }
    }

    fn underline(&self, _: &App) -> bool {
        match self {
            Self::Link => true,
            _ => false,
        }
    }

    fn shadow(&self, outline: bool, _: &App) -> bool {
        match self {
            Self::Default => true,
            Self::Primary | Self::Secondary | Self::Danger => outline,
            Self::Custom(c) => c.shadow,
            _ => false,
        }
    }

    fn normal(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg = self.bg_color(outline, cx);
        let border = self.border_color(outline, cx);
        let fg = self.text_color(outline, cx);
        let underline = self.underline(cx);
        let shadow = self.shadow(outline, cx);

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline,
            shadow,
        }
    }

    fn hovered(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg: Background = match self {
            Self::Default => {
                if outline {
                    self.outline_background(ButtonStyleState::Hovered, cx)
                } else {
                    cx.theme().tokens.button_hover.into()
                }
            }
            Self::Primary => {
                if outline {
                    self.outline_background(ButtonStyleState::Hovered, cx)
                } else {
                    cx.theme().tokens.button_primary_hover.into()
                }
            }
            Self::Secondary => {
                if outline {
                    self.outline_background(ButtonStyleState::Hovered, cx)
                } else {
                    cx.theme().tokens.button_secondary_hover.into()
                }
            }
            Self::Danger => {
                if outline {
                    self.outline_background(ButtonStyleState::Hovered, cx)
                } else {
                    cx.theme().tokens.button_danger_hover.into()
                }
            }
            Self::Warning => {
                if outline {
                    self.outline_background(ButtonStyleState::Hovered, cx)
                } else {
                    cx.theme().tokens.button_warning_hover.into()
                }
            }
            Self::Success => {
                if outline {
                    self.outline_background(ButtonStyleState::Hovered, cx)
                } else {
                    cx.theme().tokens.button_success_hover.into()
                }
            }
            Self::Info => {
                if outline {
                    self.outline_background(ButtonStyleState::Hovered, cx)
                } else {
                    cx.theme().tokens.button_info_hover.into()
                }
            }
            Self::Custom(colors) => if outline {
                colors.color.mix_oklab(cx.theme().transparent, 0.2)
            } else {
                colors.color.mix_oklab(cx.theme().transparent, 0.3)
            }
            .into(),
            Self::Ghost => if cx.theme().mode.is_dark() {
                cx.theme().secondary.lighten(0.1).opacity(0.8)
            } else {
                cx.theme().secondary.darken(0.1).opacity(0.8)
            }
            .into(),
            Self::Link => cx.theme().transparent.into(),
            Self::Text => cx.theme().transparent.into(),
        };

        let border = self.border_color(outline, cx);
        let fg = match self {
            Self::Link => cx.theme().link_hover,
            _ => self.text_color(outline, cx),
        };

        let underline = self.underline(cx);
        let shadow = self.shadow(outline, cx);

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline,
            shadow,
        }
    }

    fn active(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg = match self {
            Self::Default => {
                if outline {
                    self.outline_background(ButtonStyleState::Active, cx)
                } else {
                    cx.theme().tokens.button_active.into()
                }
            }
            Self::Primary => {
                if outline {
                    self.outline_background(ButtonStyleState::Active, cx)
                } else {
                    cx.theme().tokens.button_primary_active.into()
                }
            }
            Self::Secondary => {
                if outline {
                    self.outline_background(ButtonStyleState::Active, cx)
                } else {
                    cx.theme().tokens.button_secondary_active.into()
                }
            }
            Self::Ghost => if cx.theme().mode.is_dark() {
                cx.theme().secondary.lighten(0.2).opacity(0.8)
            } else {
                cx.theme().secondary.darken(0.2).opacity(0.8)
            }
            .into(),
            Self::Danger => {
                if outline {
                    self.outline_background(ButtonStyleState::Active, cx)
                } else {
                    cx.theme().tokens.button_danger_active.into()
                }
            }
            Self::Warning => {
                if outline {
                    self.outline_background(ButtonStyleState::Active, cx)
                } else {
                    cx.theme().tokens.button_warning_active.into()
                }
            }
            Self::Success => {
                if outline {
                    self.outline_background(ButtonStyleState::Active, cx)
                } else {
                    cx.theme().tokens.button_success_active.into()
                }
            }
            Self::Info => {
                if outline {
                    self.outline_background(ButtonStyleState::Active, cx)
                } else {
                    cx.theme().tokens.button_info_active.into()
                }
            }
            Self::Custom(colors) => colors.color.mix_oklab(cx.theme().transparent, 0.4).into(),
            Self::Link => cx.theme().transparent.into(),
            Self::Text => cx.theme().transparent.into(),
        };
        let border = self.border_color(outline, cx);
        let fg = match self {
            Self::Link => cx.theme().link_active,
            Self::Text => cx.theme().foreground.opacity(0.7),
            _ => self.text_color(outline, cx),
        };
        let underline = self.underline(cx);
        let shadow = self.shadow(outline, cx);

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline,
            shadow,
        }
    }

    fn selected(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        if outline {
            let active_style = self.active(outline, cx);

            return ButtonVariantStyle {
                fg: self.text_color(outline, cx),
                ..active_style
            };
        }

        let bg = match self {
            Self::Default => cx.theme().tokens.button_active.into(),
            Self::Primary => cx.theme().tokens.button_primary_active.into(),
            Self::Secondary => cx.theme().tokens.button_secondary_active.into(),
            Self::Ghost => cx.theme().tokens.secondary_active.into(),
            Self::Danger => cx.theme().tokens.button_danger_active.into(),
            Self::Warning => cx.theme().tokens.button_warning_active.into(),
            Self::Success => cx.theme().tokens.button_success_active.into(),
            Self::Info => cx.theme().tokens.button_info_active.into(),
            Self::Link => cx.theme().transparent.into(),
            Self::Text => cx.theme().transparent.into(),
            Self::Custom(colors) => colors.active.into(),
        };

        let border = self.border_color(outline, cx);
        let fg = match self {
            Self::Link => cx.theme().link_active,
            Self::Text => cx.theme().foreground.opacity(0.7),
            _ => self.text_color(false, cx),
        };
        let underline = self.underline(cx);
        let shadow = self.shadow(outline, cx);

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline,
            shadow,
        }
    }

    fn disabled(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        let bg = match self {
            Self::Default | Self::Link | Self::Ghost | Self::Text => cx.theme().transparent.into(),
            Self::Primary => cx.theme().tokens.button_primary.background.opacity(0.15),
            Self::Danger => cx.theme().tokens.button_danger.background.opacity(0.15),
            Self::Warning => cx.theme().tokens.button_warning.background.opacity(0.15),
            Self::Success => cx.theme().tokens.button_success.background.opacity(0.15),
            Self::Info => cx.theme().tokens.button_info.background.opacity(0.15),
            Self::Secondary => cx.theme().tokens.button_secondary.background.opacity(1.5),
            Self::Custom(style) => style.color.opacity(0.15).into(),
        };
        let fg = cx.theme().muted_foreground.opacity(0.5);
        let (bg, border) = if outline {
            (
                self.outline_background(ButtonStyleState::Normal, cx)
                    .opacity(0.5),
                self.border_color(true, cx).opacity(0.5),
            )
        } else if let Self::Default = self {
            (
                cx.theme().input_background().opacity(0.5).into(),
                cx.theme().input.opacity(0.5),
            )
        } else {
            let border = match self {
                Self::Primary => cx.theme().button_primary.opacity(0.15),
                Self::Secondary => cx.theme().button_secondary.opacity(1.5),
                Self::Danger => cx.theme().button_danger.opacity(0.15),
                Self::Warning => cx.theme().button_warning.opacity(0.15),
                Self::Success => cx.theme().button_success.opacity(0.15),
                Self::Info => cx.theme().button_info.opacity(0.15),
                Self::Custom(style) => style.color.opacity(0.15),
                Self::Default | Self::Link | Self::Ghost | Self::Text => cx.theme().transparent,
            };
            (bg, border)
        };

        let underline = self.underline(cx);
        let shadow = false;

        ButtonVariantStyle {
            bg,
            border,
            fg,
            underline,
            shadow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{linear_color_stop, linear_gradient};

    #[gpui::test]
    fn test_button_builder(_cx: &mut gpui::TestAppContext) {
        let button = Button::new("complex-button")
            .label("Save Changes")
            .primary()
            .outline()
            .large()
            .tooltip("Click to save")
            .compact()
            .loading(false)
            .disabled(false)
            .selected(false)
            .tab_index(1)
            .tab_stop(true)
            .dropdown_caret(false)
            .rounded(ButtonRounded::Medium)
            .on_click(|_, _, _| {});

        assert_eq!(button.label, Some("Save Changes".into()));
        assert_eq!(button.variant, ButtonVariant::Primary);
        assert!(button.outline);
        assert_eq!(button.size, Size::Large);
        assert!(button.tooltip.is_some());
        assert!(button.compact);
        assert!(!button.loading);
        assert!(!button.disabled);
        assert!(!button.selected);
        assert_eq!(button.tab_index, 1);
        assert!(button.tab_stop);
        assert!(!button.dropdown_caret);
        assert!(matches!(button.rounded, ButtonRounded::Medium));
    }

    #[gpui::test]
    fn test_button_clickable_logic(_cx: &mut gpui::TestAppContext) {
        // Button with click handler should be clickable
        let clickable = Button::new("test").on_click(|_, _, _| {});
        assert!(clickable.clickable());

        // Disabled button should not be clickable
        let disabled = Button::new("test").disabled(true).on_click(|_, _, _| {});
        assert!(!disabled.clickable());

        // Loading button should not be clickable
        let loading = Button::new("test").loading(true).on_click(|_, _, _| {});
        assert!(!loading.clickable());
    }

    #[gpui::test]
    fn test_button_variant_methods(_cx: &mut gpui::TestAppContext) {
        // Test variant check methods
        assert!(ButtonVariant::Link.is_link());
        assert!(ButtonVariant::Text.is_text());
        assert!(ButtonVariant::Ghost.is_ghost());

        // Test no_padding logic
        assert!(ButtonVariant::Link.no_padding());
        assert!(ButtonVariant::Text.no_padding());
        assert!(!ButtonVariant::Ghost.no_padding());
    }

    #[gpui::test]
    fn test_outline_selected_uses_outline_active_style(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|_, cx| {
            let variant = ButtonVariant::Danger;
            let active_style = variant.active(true, cx);
            let selected_style = variant.selected(true, cx);

            assert_eq!(selected_style.bg, active_style.bg);
            assert_eq!(selected_style.border, active_style.border);
            assert_eq!(selected_style.fg, cx.theme().danger);
            assert_ne!(selected_style.bg, cx.theme().tokens.danger_active.into());
        });
    }

    #[gpui::test]
    fn test_primary_button_uses_gradient_background_tokens(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|_, cx| {
            let config = serde_json::from_value::<crate::ThemeConfig>(serde_json::json!({
                "name": "Gradient",
                "mode": "light",
                "colors": {
                    "button.primary.background": "linear-gradient(135deg, #4F46E5, #06B6D4)",
                    "button.primary.hover.background": "linear-gradient(145deg, #4338CA, #0891B2)",
                    "button.primary.active.background": "linear-gradient(155deg, #3730A3, #0E7490)"
                }
            }))
            .unwrap();
            crate::Theme::global_mut(cx).apply_config(&std::rc::Rc::new(config));

            assert_eq!(
                ButtonVariant::Primary.normal(false, cx).bg,
                cx.theme().tokens.button_primary.into()
            );
            assert_eq!(
                ButtonVariant::Primary.hovered(false, cx).bg,
                cx.theme().tokens.button_primary_hover.into()
            );
            assert_eq!(
                ButtonVariant::Primary.active(false, cx).bg,
                cx.theme().tokens.button_primary_active.into()
            );
        });
    }

    #[gpui::test]
    fn test_outline_primary_keeps_original_depth(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|_, cx| {
            let config = serde_json::from_value::<crate::ThemeConfig>(serde_json::json!({
                "name": "Outline Depth",
                "mode": "light",
                "colors": {
                    "primary.background": "linear-gradient(180deg, #111827, #020617)",
                    "primary.hover.background": "linear-gradient(180deg, #1F2937, #111827)",
                    "primary.active.background": "linear-gradient(180deg, #020617, #000000)"
                }
            }))
            .unwrap();
            crate::Theme::global_mut(cx).apply_config(&std::rc::Rc::new(config));

            assert_eq!(
                ButtonVariant::Primary.normal(true, cx).bg,
                cx.theme().tokens.primary.background.opacity(0.1)
            );
            assert_eq!(
                ButtonVariant::Primary.hovered(true, cx).bg,
                cx.theme().tokens.primary_hover.background.opacity(0.2)
            );
            assert_eq!(
                ButtonVariant::Primary.active(true, cx).bg,
                cx.theme().tokens.primary_active.background.opacity(0.4)
            );
        });
    }

    #[gpui::test]
    fn test_outline_buttons_use_semantic_gradient_tokens(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|_, cx| {
            let config = serde_json::from_value::<crate::ThemeConfig>(serde_json::json!({
                "name": "Outline Gradient",
                "mode": "light",
                "colors": {
                    "primary.background": "linear-gradient(180deg, #111827, #020617)",
                    "primary.hover.background": "linear-gradient(180deg, #1F2937, #111827)",
                    "primary.active.background": "linear-gradient(180deg, #020617, #000000)",
                    "button.primary.background": "linear-gradient(180deg, #FFFFFF, #E5E7EB)",
                    "button.primary.hover.background": "linear-gradient(180deg, #F9FAFB, #E5E7EB)",
                    "button.primary.active.background": "linear-gradient(180deg, #E5E7EB, #D1D5DB)",
                    "danger.background": "linear-gradient(180deg, #EF4444, #DC2626)",
                    "danger.hover.background": "linear-gradient(180deg, #F87171, #EF4444)",
                    "danger.active.background": "linear-gradient(180deg, #DC2626, #B91C1C)",
                    "button.danger.background": "linear-gradient(180deg, #FEF2F2, #FEE2E2)",
                    "button.danger.hover.background": "linear-gradient(180deg, #FEE2E2, #FECACA)",
                    "button.danger.active.background": "linear-gradient(180deg, #FECACA, #FCA5A5)"
                }
            }))
            .unwrap();
            crate::Theme::global_mut(cx).apply_config(&std::rc::Rc::new(config));

            assert_eq!(
                ButtonVariant::Primary.normal(true, cx).bg,
                cx.theme().tokens.primary.background.opacity(0.1)
            );
            assert_eq!(
                ButtonVariant::Danger.normal(true, cx).bg,
                cx.theme().tokens.danger.background.opacity(0.1)
            );
            assert_eq!(
                ButtonVariant::Danger.hovered(true, cx).bg,
                cx.theme().tokens.danger_hover.background.opacity(0.2)
            );
            assert_eq!(
                ButtonVariant::Danger.active(true, cx).bg,
                cx.theme().tokens.danger_active.background.opacity(0.4)
            );
            assert_eq!(
                ButtonVariant::Primary.normal(false, cx).bg,
                cx.theme().tokens.button_primary.into()
            );
            assert_eq!(
                ButtonVariant::Danger.normal(false, cx).bg,
                linear_gradient(
                    180.,
                    linear_color_stop(crate::try_parse_color("#FEF2F2").unwrap(), 0.),
                    linear_color_stop(crate::try_parse_color("#FEE2E2").unwrap(), 1.)
                )
            );
        });
    }

    #[gpui::test]
    fn test_disabled_outline_buttons_keep_semantic_backgrounds(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|_, cx| {
            let config = serde_json::from_value::<crate::ThemeConfig>(serde_json::json!({
                "name": "Disabled Outline Gradient",
                "mode": "light",
                "colors": {
                    "primary.background": "linear-gradient(180deg, #111827, #020617)",
                    "button.primary.background": "linear-gradient(180deg, #FFFFFF, #E5E7EB)",
                    "danger.background": "linear-gradient(180deg, #EF4444, #DC2626)",
                    "button.danger.background": "linear-gradient(180deg, #E5E7EB, #D1D5DB)"
                }
            }))
            .unwrap();
            crate::Theme::global_mut(cx).apply_config(&std::rc::Rc::new(config));

            assert_eq!(
                ButtonVariant::Primary.disabled(true, cx).bg,
                cx.theme()
                    .tokens
                    .primary
                    .background
                    .opacity(0.1)
                    .opacity(0.5)
            );
            assert_eq!(
                ButtonVariant::Danger.disabled(true, cx).bg,
                cx.theme()
                    .tokens
                    .danger
                    .background
                    .opacity(0.1)
                    .opacity(0.5)
            );
            assert_ne!(
                ButtonVariant::Danger.disabled(true, cx).bg,
                cx.theme().input_background().opacity(0.5).into()
            );
            assert_ne!(
                ButtonVariant::Danger.disabled(true, cx).bg,
                cx.theme().tokens.button_danger.background.opacity(0.15)
            );
        });
    }
}
