pub use crate::component_traits::{Collapsible, Disableable, Selectable};
pub use crate::sizing::{Sizable, Size, StyleSized};
use gpui::{App, Corners, Edges, ParentElement, Pixels, StyleRefinement, Styled, Window, div, px};
pub use gpui_base::{FocusableExt, RoleOverride, StyledExt, box_shadow, h_flex, v_flex};

use crate::ActiveTheme as _;

const FOCUS_RING_WIDTH: Pixels = px(3.);
const FOCUS_RING_OPACITY: f32 = 0.5;

pub(crate) trait FocusRingStyleExt<T: ParentElement + Styled + Sized> {
    fn draw_focus_ring(self, visible: bool, margin: Pixels, window: &Window, cx: &App) -> Self;
}

impl<T: ParentElement + Styled + Sized> FocusRingStyleExt<T> for T {
    fn draw_focus_ring(mut self, visible: bool, margin: Pixels, window: &Window, cx: &App) -> Self {
        if !visible {
            return self;
        }

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
        };
        let outer_radius = radius.map(|value| *value + FOCUS_RING_WIDTH);
        let mut border_style = StyleRefinement::default();
        border_style.corner_radii.top_left = Some(radius.top_left.into());
        border_style.corner_radii.top_right = Some(radius.top_right.into());
        border_style.corner_radii.bottom_left = Some(radius.bottom_left.into());
        border_style.corner_radii.bottom_right = Some(radius.bottom_right.into());
        let mut ring_style = StyleRefinement::default();
        ring_style.corner_radii.top_left = Some(outer_radius.top_left.into());
        ring_style.corner_radii.top_right = Some(outer_radius.top_right.into());
        ring_style.corner_radii.bottom_left = Some(outer_radius.bottom_left.into());
        ring_style.corner_radii.bottom_right = Some(outer_radius.bottom_right.into());
        let inset = FOCUS_RING_WIDTH + margin;

        self.child(
            div()
                .flex_none()
                .absolute()
                .top(-border_widths.top)
                .left(-border_widths.left)
                .right(-border_widths.right)
                .bottom(-border_widths.bottom)
                .border_1()
                .border_color(cx.theme().ring)
                .refine_style(&border_style),
        )
        .child(
            div()
                .flex_none()
                .absolute()
                .top(-(inset + border_widths.top))
                .left(-(inset + border_widths.left))
                .right(-(inset + border_widths.right))
                .bottom(-(inset + border_widths.bottom))
                .border(FOCUS_RING_WIDTH)
                .border_color(cx.theme().ring.alpha(FOCUS_RING_OPACITY))
                .refine_style(&ring_style),
        )
    }
}
