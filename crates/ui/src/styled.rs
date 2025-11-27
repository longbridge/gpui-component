use crate::{ActiveTheme, PixelsExt as _};
use gpui::{
    App, BoxShadow, Corners, DefiniteLength, Div, Edges, FocusHandle, Hsla, ParentElement, Pixels,
    Refineable, StyleRefinement, Styled, Window, div, point, px,
};
use serde::{Deserialize, Serialize};

/// Returns a `Div` as horizontal flex layout.
#[inline(always)]
pub fn h_flex() -> Div {
    div().h_flex()
}

/// Returns a `Div` as vertical flex layout.
#[inline(always)]
pub fn v_flex() -> Div {
    div().v_flex()
}

/// Create a [`BoxShadow`] like CSS.
///
/// e.g:
///
/// If CSS is `box-shadow: 0 0 10px 0 rgba(0, 0, 0, 0.1);`
///
/// Then the equivalent in Rust is `box_shadow(0., 0., 10., 0., hsla(0., 0., 0., 0.1))`
#[inline(always)]
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
        color,
    }
}

macro_rules! font_weight {
    ($fn:ident, $const:ident) => {
        /// [docs](https://tailwindcss.com/docs/font-weight)
        #[inline]
        fn $fn(self) -> Self {
            self.font_weight(gpui::FontWeight::$const)
        }
    };
}

/// Extends [`gpui::Styled`] with specific styling methods.
#[cfg_attr(
    any(feature = "inspector", debug_assertions),
    gpui_macros::derive_inspector_reflection
)]
pub trait StyledExt: Styled + Sized {
    /// Refine the style of this element, applying the given style refinement.
    fn refine_style(mut self, style: &StyleRefinement) -> Self {
        self.style().refine(style);
        self
    }

    /// Apply self into a horizontal flex layout.
    #[inline(always)]
    fn h_flex(self) -> Self {
        self.flex().flex_row().items_center()
    }

    /// Apply self into a vertical flex layout.
    #[inline(always)]
    fn v_flex(self) -> Self {
        self.flex().flex_col()
    }

    /// Apply paddings to the element.
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

    /// Apply margins to the element.
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

    /// Render a border with a width of 1px, color red
    fn debug_red(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(crate::red_500())
        } else {
            self
        }
    }

    /// Render a border with a width of 1px, color blue
    fn debug_blue(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(crate::blue_500())
        } else {
            self
        }
    }

    /// Render a border with a width of 1px, color yellow
    fn debug_yellow(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(crate::yellow_500())
        } else {
            self
        }
    }

    /// Render a border with a width of 1px, color green
    fn debug_green(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(crate::green_500())
        } else {
            self
        }
    }

    /// Render a border with a width of 1px, color pink
    fn debug_pink(self) -> Self {
        if cfg!(debug_assertions) {
            self.border_1().border_color(crate::pink_500())
        } else {
            self
        }
    }

    /// Render a 1px blue border, when if the element is focused
    fn debug_focused(self, focus_handle: &FocusHandle, window: &Window, cx: &App) -> Self {
        if cfg!(debug_assertions) {
            if focus_handle.contains_focused(window, cx) {
                self.debug_blue()
            } else {
                self
            }
        } else {
            self
        }
    }

    /// Render a border with a width of 1px, color ring color
    #[inline]
    fn focused_border(self, cx: &App) -> Self {
        self.border_1().border_color(cx.theme().ring)
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

    /// Set as Popover style
    #[inline]
    fn popover_style(self, cx: &App) -> Self {
        self.bg(cx.theme().popover)
            .text_color(cx.theme().popover_foreground)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_lg()
            .rounded(cx.theme().radius)
    }

    /// Set corner radii for the element.
    fn corner_radii(self, radius: Corners<Pixels>) -> Self {
        self.rounded_tl(radius.top_left)
            .rounded_tr(radius.top_right)
            .rounded_bl(radius.bottom_left)
            .rounded_br(radius.bottom_right)
    }
}

impl<E: Styled> StyledExt for E {}

/// A size for elements.
#[derive(Clone, Default, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum Size {
    Size(Pixels),
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
}

impl Size {
    fn as_f32(&self) -> f32 {
        match self {
            Size::Size(val) => val.as_f32(),
            Size::XSmall => 0.,
            Size::Small => 1.,
            Size::Medium => 2.,
            Size::Large => 3.,
        }
    }

    /// Returns the size as a static string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Size::XSmall => "xs",
            Size::Small => "sm",
            Size::Medium => "md",
            Size::Large => "lg",
            Size::Size(_) => "custom",
        }
    }

    /// Create a Size from a static string.
    ///
    /// - "xs" or "xsmall"
    /// - "sm" or "small"
    /// - "md" or "medium"
    /// - "lg" or "large"
    ///
    /// Any other value will return Size::Medium.
    pub fn from_str(size: &str) -> Self {
        match size.to_lowercase().as_str() {
            "xs" | "xsmall" => Size::XSmall,
            "sm" | "small" => Size::Small,
            "md" | "medium" => Size::Medium,
            "lg" | "large" => Size::Large,
            _ => Size::Medium,
        }
    }

    /// Returns the height for table row.
    #[inline]
    pub fn table_row_height(&self) -> Pixels {
        match self {
            Size::XSmall => px(26.),
            Size::Small => px(30.),
            Size::Large => px(40.),
            _ => px(32.),
        }
    }

    /// Returns the padding for a table cell.
    #[inline]
    pub fn table_cell_padding(&self) -> Edges<Pixels> {
        match self {
            Size::XSmall => Edges {
                top: px(2.),
                bottom: px(2.),
                left: px(4.),
                right: px(4.),
            },
            Size::Small => Edges {
                top: px(3.),
                bottom: px(3.),
                left: px(6.),
                right: px(6.),
            },
            Size::Large => Edges {
                top: px(8.),
                bottom: px(8.),
                left: px(12.),
                right: px(12.),
            },
            _ => Edges {
                top: px(4.),
                bottom: px(4.),
                left: px(8.),
                right: px(8.),
            },
        }
    }

    /// Returns a smaller size.
    pub fn smaller(&self) -> Self {
        match self {
            Size::XSmall => Size::XSmall,
            Size::Small => Size::XSmall,
            Size::Medium => Size::Small,
            Size::Large => Size::Medium,
            Size::Size(val) => Size::Size(*val * 0.2),
        }
    }

    /// Returns a larger size.
    pub fn larger(&self) -> Self {
        match self {
            Size::XSmall => Size::Small,
            Size::Small => Size::Medium,
            Size::Medium => Size::Large,
            Size::Large => Size::Large,
            Size::Size(val) => Size::Size(*val * 1.2),
        }
    }

    /// Return the max size between two sizes.
    ///
    /// e.g. `Size::XSmall.max(Size::Small)` will return `Size::XSmall`.
    pub fn max(&self, other: Self) -> Self {
        match (self, other) {
            (Size::Size(a), Size::Size(b)) => Size::Size(px(a.as_f32().min(b.as_f32()))),
            (Size::Size(a), _) => Size::Size(*a),
            (_, Size::Size(b)) => Size::Size(b),
            (a, b) if a.as_f32() < b.as_f32() => *a,
            _ => other,
        }
    }

    /// Return the min size between two sizes.
    ///
    /// e.g. `Size::XSmall.min(Size::Small)` will return `Size::Small`.
    pub fn min(&self, other: Self) -> Self {
        match (self, other) {
            (Size::Size(a), Size::Size(b)) => Size::Size(px(a.as_f32().max(b.as_f32()))),
            (Size::Size(a), _) => Size::Size(*a),
            (_, Size::Size(b)) => Size::Size(b),
            (a, b) if a.as_f32() > b.as_f32() => *a,
            _ => other,
        }
    }

    /// Returns the horizontal input padding.
    pub fn input_px(&self) -> Pixels {
        match self {
            Self::Large => px(20.),
            Self::Medium => px(12.),
            Self::Small => px(8.),
            Self::XSmall => px(4.),
            _ => px(8.),
        }
    }

    /// Returns the vertical input padding.
    pub fn input_py(&self) -> Pixels {
        match self {
            Size::Large => px(10.),
            Size::Medium => px(5.),
            Size::Small => px(2.),
            Size::XSmall => px(0.),
            _ => px(2.),
        }
    }
}

impl From<Pixels> for Size {
    fn from(size: Pixels) -> Self {
        Size::Size(size)
    }
}

/// A trait for defining element that can be selected.
pub trait Selectable: Sized {
    /// Set the selected state of the element.
    fn selected(self, selected: bool) -> Self;

    /// Returns true if the element is selected.
    fn is_selected(&self) -> bool;

    /// Set is the element mouse right clicked, default do nothing.
    fn secondary_selected(self, _: bool) -> Self {
        self
    }
}

/// A trait for defining element that can be disabled.
pub trait Disableable {
    /// Set the disabled state of the element.
    fn disabled(self, disabled: bool) -> Self;
}

/// A trait for setting the size of an element.
/// Size::Medium is use by default.
pub trait Sizable: Sized {
    /// Set the ui::Size of this element.
    ///
    /// Also can receive a `ButtonSize` to convert to `IconSize`,
    /// Or a `Pixels` to set a custom size: `px(30.)`
    fn with_size(self, size: impl Into<Size>) -> Self;

    /// Set to Size::XSmall
    #[inline(always)]
    fn xsmall(self) -> Self {
        self.with_size(Size::XSmall)
    }

    /// Set to Size::Small
    #[inline(always)]
    fn small(self) -> Self {
        self.with_size(Size::Small)
    }

    /// Set to Size::Large
    #[inline(always)]
    fn large(self) -> Self {
        self.with_size(Size::Large)
    }
}

#[allow(unused)]
pub trait StyleSized<T: Styled> {
    fn input_text_size(self, size: Size) -> Self;
    fn input_size(self, size: Size) -> Self;
    fn input_pl(self, size: Size) -> Self;
    fn input_pr(self, size: Size) -> Self;
    fn input_px(self, size: Size) -> Self;
    fn input_py(self, size: Size) -> Self;
    fn input_h(self, size: Size) -> Self;
    fn list_size(self, size: Size) -> Self;
    fn list_px(self, size: Size) -> Self;
    fn list_py(self, size: Size) -> Self;
    /// Apply size with the given `Size`.
    fn size_with(self, size: Size) -> Self;
    /// Apply the table cell size (Font size, padding) with the given `Size`.
    fn table_cell_size(self, size: Size) -> Self;
    fn button_text_size(self, size: Size) -> Self;
}

impl<T: Styled> StyleSized<T> for T {
    #[inline]
    fn input_text_size(self, size: Size) -> Self {
        match size {
            Size::XSmall => self.text_xs(),
            Size::Small => self.text_sm(),
            Size::Medium => self.text_base(),
            Size::Large => self.text_lg(),
            Size::Size(size) => self.text_size(size),
        }
    }

    #[inline]
    fn input_size(self, size: Size) -> Self {
        self.input_px(size).input_py(size).input_h(size)
    }

    #[inline]
    fn input_pl(self, size: Size) -> Self {
        self.pl(size.input_px())
    }

    #[inline]
    fn input_pr(self, size: Size) -> Self {
        self.pr(size.input_px())
    }

    #[inline]
    fn input_px(self, size: Size) -> Self {
        self.px(size.input_px())
    }

    #[inline]
    fn input_py(self, size: Size) -> Self {
        self.py(size.input_py())
    }

    #[inline]
    fn input_h(self, size: Size) -> Self {
        match size {
            Size::Large => self.h_11(),
            Size::Medium => self.h_8(),
            Size::Small => self.h(px(24.)),
            Size::XSmall => self.h(px(20.)),
            _ => self.h(px(24.)),
        }
        .input_text_size(size)
    }

    #[inline]
    fn list_size(self, size: Size) -> Self {
        self.list_px(size).list_py(size).input_text_size(size)
    }

    #[inline]
    fn list_px(self, size: Size) -> Self {
        match size {
            Size::Small => self.px_2(),
            _ => self.px_3(),
        }
    }

    #[inline]
    fn list_py(self, size: Size) -> Self {
        match size {
            Size::Large => self.py_2(),
            Size::Medium => self.py_1(),
            Size::Small => self.py_0p5(),
            _ => self.py_1(),
        }
    }

    #[inline]
    fn size_with(self, size: Size) -> Self {
        match size {
            Size::Large => self.size_11(),
            Size::Medium => self.size_8(),
            Size::Small => self.size_5(),
            Size::XSmall => self.size_4(),
            Size::Size(size) => self.size(size),
        }
    }

    #[inline]
    fn table_cell_size(self, size: Size) -> Self {
        let padding = size.table_cell_padding();
        match size {
            Size::XSmall => self.text_sm(),
            Size::Small => self.text_sm(),
            _ => self,
        }
        .pl(padding.left)
        .pr(padding.right)
        .pt(padding.top)
        .pb(padding.bottom)
    }

    fn button_text_size(self, size: Size) -> Self {
        match size {
            Size::XSmall => self.text_xs(),
            Size::Small => self.text_sm(),
            _ => self.text_base(),
        }
    }
}

pub(crate) trait FocusableExt<T: ParentElement + Styled + Sized> {
    /// Add focus ring to the element.
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
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
            bottom: style
                .border_widths
                .bottom
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
            left: style
                .border_widths
                .left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
            right: style
                .border_widths
                .right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
        };

        // Update the radius based on element's corner radii and the ring border width.
        let radius = Corners::<Pixels> {
            top_left: style
                .corner_radii
                .top_left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
            top_right: style
                .corner_radii
                .top_right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
            bottom_left: style
                .corner_radii
                .bottom_left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
            bottom_right: style
                .corner_radii
                .bottom_right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or_default(),
        }
        .map(|v| *v + RING_BORDER_WIDTH);

        let mut inner_style = StyleRefinement::default();
        inner_style.corner_radii.top_left = Some(radius.top_left.into());
        inner_style.corner_radii.top_right = Some(radius.top_right.into());
        inner_style.corner_radii.bottom_left = Some(radius.bottom_left.into());
        inner_style.corner_radii.bottom_right = Some(radius.bottom_right.into());

        let inset = RING_BORDER_WIDTH + margins;

        self.child(
            div()
                .flex_none()
                .absolute()
                .top(-(inset + border_widths.top))
                .left(-(inset + border_widths.left))
                .right(-(inset + border_widths.right))
                .bottom(-(inset + border_widths.bottom))
                .border(RING_BORDER_WIDTH)
                .border_color(cx.theme().ring.alpha(0.2))
                .refine_style(&inner_style),
        )
    }
}

/// A trait for defining element that can be collapsed.
pub trait Collapsible {
    fn collapsed(self, collapsed: bool) -> Self;
    fn is_collapsed(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use crate::Size;

    #[test]
    fn test_size_max_min() {
        assert_eq!(Size::Small.min(Size::XSmall), Size::Small);
        assert_eq!(Size::XSmall.min(Size::Small), Size::Small);
        assert_eq!(Size::Small.min(Size::Medium), Size::Medium);
        assert_eq!(Size::Medium.min(Size::Large), Size::Large);
        assert_eq!(Size::Large.min(Size::Small), Size::Large);

        assert_eq!(
            Size::Size(px(10.)).min(Size::Size(px(20.))),
            Size::Size(px(20.))
        );

        // Min
        assert_eq!(Size::Small.max(Size::XSmall), Size::XSmall);
        assert_eq!(Size::XSmall.max(Size::Small), Size::XSmall);
        assert_eq!(Size::Small.max(Size::Medium), Size::Small);
        assert_eq!(Size::Medium.max(Size::Large), Size::Medium);
        assert_eq!(Size::Large.max(Size::Small), Size::Small);

        assert_eq!(
            Size::Size(px(10.)).max(Size::Size(px(20.))),
            Size::Size(px(10.))
        );
    }

    #[test]
    fn test_size_as_str() {
        assert_eq!(Size::XSmall.as_str(), "xs");
        assert_eq!(Size::Small.as_str(), "sm");
        assert_eq!(Size::Medium.as_str(), "md");
        assert_eq!(Size::Large.as_str(), "lg");
        assert_eq!(Size::Size(px(15.)).as_str(), "custom");
    }

    #[test]
    fn test_size_from_str() {
        assert_eq!(Size::from_str("xs"), Size::XSmall);
        assert_eq!(Size::from_str("xsmall"), Size::XSmall);
        assert_eq!(Size::from_str("sm"), Size::Small);
        assert_eq!(Size::from_str("small"), Size::Small);
        assert_eq!(Size::from_str("md"), Size::Medium);
        assert_eq!(Size::from_str("medium"), Size::Medium);
        assert_eq!(Size::from_str("lg"), Size::Large);
        assert_eq!(Size::from_str("large"), Size::Large);
        assert_eq!(Size::from_str("unknown"), Size::Medium);

        // Case insensitive
        assert_eq!(Size::from_str("XS"), Size::XSmall);
        assert_eq!(Size::from_str("SMALL"), Size::Small);
        assert_eq!(Size::from_str("Md"), Size::Medium);
    }
}
