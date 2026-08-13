use gpui::{
    App, BoxShadow, Corners, DefiniteLength, Div, Edges, FocusHandle, Hsla, ParentElement, Pixels,
    Refineable as _, Role, StyleRefinement, Styled, Window, div, hsla, point, px,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RoleOverride {
    #[default]
    Implicit,
    Presentational,
    Role(Role),
}

impl RoleOverride {
    pub fn resolve(self, default: impl FnOnce() -> Role) -> Option<Role> {
        match self {
            Self::Implicit => Some(default()),
            Self::Presentational => None,
            Self::Role(role) => Some(role),
        }
    }
}
impl From<Role> for RoleOverride {
    fn from(role: Role) -> Self {
        Self::Role(role)
    }
}
impl From<Option<Role>> for RoleOverride {
    fn from(role: Option<Role>) -> Self {
        role.map_or(Self::Presentational, Self::Role)
    }
}

use crate::theme::ActiveTheme as _;

pub fn h_flex() -> Div {
    div().h_flex()
}
pub fn v_flex() -> Div {
    div().v_flex()
}

pub fn box_shadow(
    x: impl Into<Pixels>,
    y: impl Into<Pixels>,
    blur: impl Into<Pixels>,
    spread: impl Into<Pixels>,
    color: Hsla,
) -> BoxShadow {
    BoxShadow {
        offset: point(x.into(), y.into()),
        blur_radius: blur.into(),
        spread_radius: spread.into(),
        inset: false,
        color,
    }
}

macro_rules! font_weight {
    ($method:ident, $weight:ident) => {
        fn $method(self) -> Self {
            self.font_weight(gpui::FontWeight::$weight)
        }
    };
}

#[cfg_attr(
    any(feature = "inspector", debug_assertions),
    gpui_macros::derive_inspector_reflection
)]
pub trait StyledExt: Styled + Sized {
    fn refine_style(mut self, style: &StyleRefinement) -> Self {
        self.style().refine(style);
        self
    }

    fn h_flex(self) -> Self {
        self.flex().flex_row().items_center()
    }

    fn v_flex(self) -> Self {
        self.flex().flex_col()
    }

    fn paddings<L>(self, paddings: impl Into<Edges<L>>) -> Self
    where
        L: Into<DefiniteLength> + Clone + Default + std::fmt::Debug + PartialEq,
    {
        let paddings = paddings.into();
        self.pt(paddings.top.into())
            .pb(paddings.bottom.into())
            .pl(paddings.left.into())
            .pr(paddings.right.into())
    }

    fn margins<L>(self, margins: impl Into<Edges<L>>) -> Self
    where
        L: Into<DefiniteLength> + Clone + Default + std::fmt::Debug + PartialEq,
    {
        let margins = margins.into();
        self.mt(margins.top.into())
            .mb(margins.bottom.into())
            .ml(margins.left.into())
            .mr(margins.right.into())
    }

    fn debug_red(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(hsl(0., 72.2, 50.6))
        } else {
            self
        }
    }

    fn debug_blue(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(hsl(217.2, 91.2, 59.8))
        } else {
            self
        }
    }

    fn debug_yellow(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(hsl(47.9, 95.8, 53.1))
        } else {
            self
        }
    }

    fn debug_green(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(hsl(142.1, 70.6, 45.3))
        } else {
            self
        }
    }

    fn debug_pink(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(hsl(330.4, 81.2, 60.4))
        } else {
            self
        }
    }

    fn debug_focused(self, focus_handle: &FocusHandle, window: &Window, cx: &App) -> Self {
        if cfg!(debug_assertions) && focus_handle.contains_focused(window, cx) {
            self.debug_blue()
        } else {
            self
        }
    }

    fn focused_border(self, cx: &App) -> Self {
        self.border_1().border_color(cx.theme().tokens.colors.ring)
    }

    font_weight!(font_thin, THIN);
    font_weight!(font_extralight, EXTRA_LIGHT);
    font_weight!(font_light, LIGHT);
    font_weight!(font_normal, NORMAL);
    font_weight!(font_medium, MEDIUM);
    font_weight!(font_semibold, SEMIBOLD);
    font_weight!(font_bold, BOLD);
    font_weight!(font_extrabold, EXTRA_BOLD);
    font_weight!(font_black, BLACK);

    fn popover_style(self, cx: &App) -> Self {
        let tokens = cx.theme().tokens;
        self.bg(tokens.colors.surface)
            .text_color(tokens.colors.surface_foreground)
            .border_1()
            .border_color(tokens.colors.border)
            .shadow_lg()
            .rounded(tokens.radius.md)
    }

    fn corner_radii(self, radius: Corners<Pixels>) -> Self {
        self.rounded_tl(radius.top_left)
            .rounded_tr(radius.top_right)
            .rounded_bl(radius.bottom_left)
            .rounded_br(radius.bottom_right)
    }
}

impl<E: Styled> StyledExt for E {}

#[cfg(any(feature = "inspector", debug_assertions))]
pub fn styled_ext_reflection_methods<T: Styled + 'static>()
-> Vec<gpui::inspector_reflection::FunctionReflection<T>> {
    styled_ext_reflection::methods::<T>()
}

pub trait FocusableExt<T: ParentElement + Styled + Sized> {
    fn focus_ring(self, is_focused: bool, margins: Pixels, window: &Window, cx: &App) -> Self;
}

impl<T: ParentElement + Styled + Sized> FocusableExt<T> for T {
    fn focus_ring(mut self, is_focused: bool, margins: Pixels, window: &Window, cx: &App) -> Self {
        if !is_focused {
            return self;
        }
        const RING_BORDER_WIDTH: Pixels = px(1.5);
        let rem_size = window.rem_size();
        let style = self.style();
        let border_widths = Edges::<Pixels> {
            top: style
                .border_widths
                .top
                .map(|value| value.to_pixels(rem_size))
                .unwrap_or_default(),
            bottom: style
                .border_widths
                .bottom
                .map(|value| value.to_pixels(rem_size))
                .unwrap_or_default(),
            left: style
                .border_widths
                .left
                .map(|value| value.to_pixels(rem_size))
                .unwrap_or_default(),
            right: style
                .border_widths
                .right
                .map(|value| value.to_pixels(rem_size))
                .unwrap_or_default(),
        };
        let radius = Corners::<Pixels> {
            top_left: style
                .corner_radii
                .top_left
                .map(|value| value.to_pixels(rem_size))
                .unwrap_or_default(),
            top_right: style
                .corner_radii
                .top_right
                .map(|value| value.to_pixels(rem_size))
                .unwrap_or_default(),
            bottom_left: style
                .corner_radii
                .bottom_left
                .map(|value| value.to_pixels(rem_size))
                .unwrap_or_default(),
            bottom_right: style
                .corner_radii
                .bottom_right
                .map(|value| value.to_pixels(rem_size))
                .unwrap_or_default(),
        }
        .map(|value| *value + RING_BORDER_WIDTH);
        let mut inner_style = StyleRefinement::default();
        inner_style.corner_radii.top_left = Some(radius.top_left.into());
        inner_style.corner_radii.top_right = Some(radius.top_right.into());
        inner_style.corner_radii.bottom_left = Some(radius.bottom_left.into());
        inner_style.corner_radii.bottom_right = Some(radius.bottom_right.into());
        let inset = RING_BORDER_WIDTH + margins;
        let ring = cx.theme().tokens.colors.ring;
        self.child(
            div()
                .flex_none()
                .absolute()
                .top(-(inset + border_widths.top))
                .left(-(inset + border_widths.left))
                .right(-(inset + border_widths.right))
                .bottom(-(inset + border_widths.bottom))
                .border(RING_BORDER_WIDTH)
                .border_color(ring.alpha(0.2))
                .refine_style(&inner_style),
        )
    }
}

fn hsl(hue: f32, saturation: f32, lightness: f32) -> Hsla {
    hsla(hue / 360., saturation / 100., lightness / 100., 1.)
}
