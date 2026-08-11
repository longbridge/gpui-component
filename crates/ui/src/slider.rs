use crate::{ActiveTheme, AxisExt, StyledExt};
pub use gpui_base::slider::{SliderEvent, SliderScale, SliderState, SliderValue};
use gpui_base::{Slider as BaseSlider, SliderIndicator, SliderThumb, SliderTrack};

use gpui::{
    Axis, Background, Corners, DefiniteLength, Entity, IntoElement, IsZero, ParentElement as _,
    RenderOnce, StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, px, relative,
};

/// A Slider element.
#[derive(IntoElement)]
pub struct Slider {
    state: Entity<SliderState>,
    axis: Axis,
    style: StyleRefinement,
    disabled: bool,
    reverse: bool,
}

impl Slider {
    /// Create a new [`Slider`] element bind to the [`SliderState`].
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            axis: Axis::Horizontal,
            state: state.clone(),
            style: StyleRefinement::default(),
            disabled: false,
            reverse: false,
        }
    }

    /// As a horizontal slider.
    pub fn horizontal(mut self) -> Self {
        self.axis = Axis::Horizontal;
        self
    }

    /// As a vertical slider.
    pub fn vertical(mut self) -> Self {
        self.axis = Axis::Vertical;
        self
    }

    /// Set the disabled state of the slider, default: false
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Reverse the filled (highlighted) side of the track, default: false.
    ///
    /// By default the track is filled from the min end to the thumb. With
    /// `reverse`, the fill goes from the thumb to the max end instead — useful
    /// when the slider represents a remaining amount (e.g. time left).
    ///
    /// This only changes the visual fill; values, events and interactions are
    /// unaffected. It applies to single-value sliders and is ignored for
    /// range sliders.
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }
}

impl Styled for Slider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let axis = self.axis;
        let state = self.state.read(cx);
        let is_range = state.value().is_range();
        let percentage = state.percentage();
        let (bar_start, bar_end) = if self.reverse && !is_range {
            // Fill from the thumb to the max end (remaining side).
            (relative(percentage.end), relative(0.))
        } else {
            (relative(percentage.start), relative(1. - percentage.end))
        };
        let rem_size = window.rem_size();

        let bar_color = self
            .style
            .background
            .clone()
            .and_then(|bg| bg.color())
            .unwrap_or(cx.theme().tokens.slider_bar.into());
        let thumb_bg: Background = self
            .style
            .text
            .color
            .map(Into::into)
            .unwrap_or_else(|| cx.theme().tokens.slider_thumb.into());
        let corner_radii = self.style.corner_radii.clone();
        let default_radius = px(999.);
        let mut radius = Corners {
            top_left: corner_radii
                .top_left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
            top_right: corner_radii
                .top_right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
            bottom_left: corner_radii
                .bottom_left
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
            bottom_right: corner_radii
                .bottom_right
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius),
        };
        if cx.theme().radius.is_zero() {
            radius.top_left = px(0.);
            radius.top_right = px(0.);
            radius.bottom_left = px(0.);
            radius.bottom_right = px(0.);
        }

        let thumb = |position: DefiniteLength, start: bool| {
            SliderThumb::new(&self.state)
                .axis(axis)
                .start(start)
                .disabled(self.disabled)
                .when(!self.disabled, |this| {
                    this.absolute()
                        .when(axis.is_horizontal(), |this| {
                            this.top(px(-5.)).left(position).ml(-px(8.))
                        })
                        .when(axis.is_vertical(), |this| {
                            this.bottom(position).left(px(-5.)).mb(-px(8.))
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .flex_shrink_0()
                        .rounded_full()
                        .bg(bar_color.opacity(0.5))
                        .size_4()
                        .p(px(1.))
                        .child(
                            div()
                                .flex_shrink_0()
                                .size_full()
                                .rounded_full()
                                .bg(thumb_bg),
                        )
                })
        };

        BaseSlider::new(&self.state)
            .axis(axis)
            .disabled(self.disabled)
            .flex()
            .flex_1()
            .items_center()
            .justify_center()
            .when(axis.is_vertical(), |this| this.h(px(120.)))
            .when(axis.is_horizontal(), |this| this.w_full())
            .refine_style(&self.style)
            .bg(cx.theme().transparent)
            .text_color(cx.theme().foreground)
            .child(
                SliderTrack::new(&self.state)
                    .axis(axis)
                    .disabled(self.disabled)
                    .flex()
                    .when(axis.is_horizontal(), |this| {
                        this.items_center().h_6().w_full()
                    })
                    .when(axis.is_vertical(), |this| {
                        this.justify_center().w_6().h_full()
                    })
                    .flex_shrink_0()
                    .child(
                        SliderIndicator::new(&self.state)
                            .relative()
                            .when(axis.is_horizontal(), |this| this.w_full().h_1p5())
                            .when(axis.is_vertical(), |this| this.h_full().w_1p5())
                            .bg(bar_color.opacity(0.2))
                            .active(|this| this.bg(bar_color.opacity(0.4)))
                            .corner_radii(radius)
                            .child(
                                div()
                                    .absolute()
                                    .when(axis.is_horizontal(), |this| {
                                        this.h_full().left(bar_start).right(bar_end)
                                    })
                                    .when(axis.is_vertical(), |this| {
                                        this.w_full().bottom(bar_start).top(bar_end)
                                    })
                                    .bg(bar_color)
                                    .when(!cx.theme().radius.is_zero(), |this| this.rounded_full()),
                            )
                            .when(is_range, |this| {
                                this.child(thumb(relative(percentage.start), true))
                            })
                            .child(thumb(relative(percentage.end), false)),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, Context, Modifiers, Render, TestAppContext, point};

    use super::*;

    struct Harness {
        state: Entity<SliderState>,
        disabled: bool,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(100.))
                .h(px(24.))
                .child(Slider::new(&self.state).disabled(self.disabled))
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        disabled: bool,
    ) -> (&mut gpui::VisualTestContext, Entity<SliderState>) {
        cx.update(crate::theme::init);
        let state = cx.new(|_| SliderState::new());
        let result = state.clone();
        let (_, cx) = cx.add_window_view(move |_, _| Harness { state, disabled });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (cx, result)
    }

    #[gpui::test]
    fn pointer_updates_the_migrated_state(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, false);
        cx.simulate_click(point(px(50.), px(12.)), Modifiers::default());
        cx.update(|_, cx| assert!((state.read(cx).value().end() - 50.).abs() < 1.));
    }

    #[gpui::test]
    fn disabled_slider_is_inert(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, true);
        cx.simulate_click(point(px(50.), px(12.)), Modifiers::default());
        cx.update(|_, cx| assert_eq!(state.read(cx).value(), SliderValue::Single(0.)));
    }
}
