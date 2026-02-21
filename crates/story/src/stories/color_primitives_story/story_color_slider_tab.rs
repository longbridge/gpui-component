use super::color_slider::{
    AlphaDelegate, ChannelDelegate, ColorInterpolation, ColorSliderState, GradientDelegate, Hsl,
    HueDelegate, RgbaSpec, ThumbShape,
};
use crate::section;
use gpui::{App, AppContext, Entity, IntoElement, ParentElement, Render, Styled, Window, div, px};
use gpui_component::{ActiveTheme, Sizable, StyledExt, h_flex, v_flex};

pub struct StoryColorSliderTab {
    pub demo_xsmall: Entity<ColorSliderState>,
    pub demo_small: Entity<ColorSliderState>,
    pub demo_medium: Entity<ColorSliderState>,
    pub demo_large: Entity<ColorSliderState>,
    pub demo_rounded_full: Entity<ColorSliderState>,
    pub demo_rounded_8: Entity<ColorSliderState>,
    pub demo_rounded_4: Entity<ColorSliderState>,
    pub demo_square: Entity<ColorSliderState>,
    pub demo_combo1: Entity<ColorSliderState>,
    pub demo_combo2: Entity<ColorSliderState>,
    pub demo_square_thumb: Entity<ColorSliderState>,
    pub demo_square_thumb_square_corners: Entity<ColorSliderState>,
    pub demo_square_thumb_square_corners_edge: Entity<ColorSliderState>,
    pub demo_bar_thumb: Entity<ColorSliderState>,
    pub demo_bar_thumb_square_corners: Entity<ColorSliderState>,
    pub demo_bar_thumb_square_corners_edge: Entity<ColorSliderState>,
    pub demo_edge_default: Entity<ColorSliderState>,
    pub demo_edge_large: Entity<ColorSliderState>,
    pub demo_edge_square: Entity<ColorSliderState>,
    pub demo_thumb_xsmall_large: Entity<ColorSliderState>,
    pub demo_thumb_small_xsmall: Entity<ColorSliderState>,
    pub demo_thumb_medium_large: Entity<ColorSliderState>,
    pub demo_thumb_large_small: Entity<ColorSliderState>,
    pub demo_interp_rgb: Entity<ColorSliderState>,
    pub demo_interp_hsl: Entity<ColorSliderState>,
    pub demo_interp_lab: Entity<ColorSliderState>,
    pub demo_delegate_hue: Entity<ColorSliderState>,
    pub demo_delegate_alpha: Entity<ColorSliderState>,
    pub demo_delegate_red: Entity<ColorSliderState>,
    pub demo_delegate_green: Entity<ColorSliderState>,
    pub demo_delegate_blue: Entity<ColorSliderState>,
    pub demo_delegate_saturation: Entity<ColorSliderState>,
    pub demo_delegate_lightness: Entity<ColorSliderState>,
    pub demo_delegate_gradient: Entity<ColorSliderState>,
}

impl StoryColorSliderTab {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let _ = window;
        cx.new(|cx| Self::new(cx))
    }

    pub fn new(cx: &mut gpui::Context<Self>) -> Self {
        let demo_xsmall = cx.new(|cx| {
            ColorSliderState::hue("demo_xsmall", 0.5, cx)
                .horizontal()
                .xsmall()
        });

        let demo_small = cx.new(|cx| {
            ColorSliderState::hue("demo_small", 0.5, cx)
                .horizontal()
                .small()
        });

        let demo_medium = cx.new(|cx| {
            ColorSliderState::hue("demo_medium", 0.5, cx)
                .horizontal()
                .thumb_medium()
        });

        let demo_large = cx.new(|cx| {
            ColorSliderState::hue("demo_large", 0.5, cx)
                .horizontal()
                .large()
        });

        let demo_rounded_full =
            cx.new(|cx| ColorSliderState::hue("demo_rounded_full", 0.5, cx).horizontal());

        let demo_rounded_8 = cx.new(|cx| {
            ColorSliderState::hue("demo_rounded_8", 0.5, cx)
                .horizontal()
                .rounded(px(8.0))
        });

        let demo_rounded_4 = cx.new(|cx| {
            ColorSliderState::hue("demo_rounded_4", 0.5, cx)
                .horizontal()
                .rounded(px(4.0))
        });

        let demo_square = cx.new(|cx| {
            ColorSliderState::hue("demo_square", 0.5, cx)
                .horizontal()
                .rounded(px(0.0))
        });

        let demo_combo1 = cx.new(|cx| {
            ColorSliderState::hue("demo_combo1", 0.33, cx)
                .horizontal()
                .large()
                .rounded(px(14.0))
        });

        let demo_combo2 = cx.new(|cx| {
            ColorSliderState::hue("demo_combo2", 0.66, cx)
                .horizontal()
                .small()
                .rounded(px(0.0))
        });

        let demo_square_thumb = cx.new(|cx| {
            ColorSliderState::hue("demo_square_thumb", 0.5, cx)
                .horizontal()
                .thumb_square()
        });

        let demo_square_thumb_square_corners = cx.new(|cx| {
            ColorSliderState::hue("demo_square_thumb_square_corners", 0.5, cx)
                .horizontal()
                .rounded(px(0.0))
                .thumb_square()
        });

        let demo_square_thumb_square_corners_edge = cx.new(|cx| {
            ColorSliderState::hue("demo_square_thumb_square_corners_edge", 0.5, cx)
                .horizontal()
                .rounded(px(0.0))
                .thumb_square()
                .edge_to_edge()
        });

        let demo_bar_thumb = cx.new(|cx| {
            let mut slider = ColorSliderState::hue("demo_bar_thumb", 0.5, cx).horizontal();
            slider.thumb.shape = ThumbShape::Bar;
            slider
        });

        let demo_bar_thumb_square_corners = cx.new(|cx| {
            let mut slider = ColorSliderState::hue("demo_bar_thumb_square_corners", 0.5, cx)
                .horizontal()
                .rounded(px(0.0));
            slider.thumb.shape = ThumbShape::Bar;
            slider
        });

        let demo_bar_thumb_square_corners_edge = cx.new(|cx| {
            let mut slider = ColorSliderState::hue("demo_bar_thumb_square_corners_edge", 0.5, cx)
                .horizontal()
                .rounded(px(0.0))
                .edge_to_edge();
            slider.thumb.shape = ThumbShape::Bar;
            slider
        });

        let demo_edge_default = cx.new(|cx| {
            ColorSliderState::hue("demo_edge_default", 0.5, cx)
                .horizontal()
                .edge_to_edge()
        });

        let demo_edge_large = cx.new(|cx| {
            ColorSliderState::hue("demo_edge_large", 0.5, cx)
                .horizontal()
                .large()
                .edge_to_edge()
        });

        let demo_edge_square = cx.new(|cx| {
            ColorSliderState::hue("demo_edge_square", 0.5, cx)
                .horizontal()
                .rounded(px(0.0))
                .edge_to_edge()
        });

        let demo_thumb_xsmall_large = cx.new(|cx| {
            ColorSliderState::hue("demo_thumb_xsmall_large", 0.5, cx)
                .horizontal()
                .xsmall()
                .thumb_large()
        });

        let demo_thumb_small_xsmall = cx.new(|cx| {
            ColorSliderState::hue("demo_thumb_small_xsmall", 0.5, cx)
                .horizontal()
                .small()
                .thumb_xsmall()
        });

        let demo_thumb_medium_large = cx.new(|cx| {
            ColorSliderState::hue("demo_thumb_medium_large", 0.5, cx)
                .horizontal()
                .thumb_large()
        });

        let demo_thumb_large_small = cx.new(|cx| {
            ColorSliderState::hue("demo_thumb_large_small", 0.5, cx)
                .horizontal()
                .large()
                .thumb_small()
        });

        let red = gpui::hsla(0.0, 1.0, 0.5, 1.0);
        let blue = gpui::hsla(240.0 / 360.0, 1.0, 0.5, 1.0);

        let demo_interp_rgb = cx.new(|cx| {
            ColorSliderState::gradient("demo_interp_rgb", 0.5, vec![red, blue], cx)
                .horizontal()
                .interpolation(ColorInterpolation::Rgb)
        });

        let demo_interp_hsl = cx.new(|cx| {
            ColorSliderState::gradient("demo_interp_hsl", 0.5, vec![red, blue], cx)
                .horizontal()
                .interpolation(ColorInterpolation::Hsl)
        });

        let demo_interp_lab = cx.new(|cx| {
            ColorSliderState::gradient("demo_interp_lab", 0.5, vec![red, blue], cx)
                .horizontal()
                .interpolation(ColorInterpolation::Lab)
        });

        let demo_delegate_hue = cx.new(|cx| {
            ColorSliderState::new("demo_delegate_hue", 180.0, Box::new(HueDelegate), cx)
                .max(360.0)
                .horizontal()
                .thumb_medium()
        });

        let demo_delegate_alpha = cx.new(|cx| {
            let hsl = Hsl {
                h: 200.0,
                s: 0.8,
                l: 0.5,
                a: 1.0,
            };
            ColorSliderState::new(
                "demo_delegate_alpha",
                0.5,
                Box::new(AlphaDelegate { spec: hsl }),
                cx,
            )
            .horizontal()
            .thumb_medium()
        });

        let demo_delegate_red = cx.new(|cx| {
            let rgba = RgbaSpec {
                r: 255.0,
                g: 128.0,
                b: 0.0,
                a: 1.0,
            };
            ColorSliderState::new(
                "demo_delegate_red",
                127.5,
                Box::new(ChannelDelegate {
                    spec: rgba,
                    channel_name: RgbaSpec::RED.into(),
                }),
                cx,
            )
            .min(0.0)
            .max(255.0)
            .horizontal()
            .thumb_medium()
        });

        let demo_delegate_green = cx.new(|cx| {
            let rgba = RgbaSpec {
                r: 128.0,
                g: 255.0,
                b: 0.0,
                a: 1.0,
            };
            ColorSliderState::new(
                "demo_delegate_green",
                127.5,
                Box::new(ChannelDelegate {
                    spec: rgba,
                    channel_name: RgbaSpec::GREEN.into(),
                }),
                cx,
            )
            .min(0.0)
            .max(255.0)
            .horizontal()
            .thumb_medium()
        });

        let demo_delegate_blue = cx.new(|cx| {
            let rgba = RgbaSpec {
                r: 0.0,
                g: 128.0,
                b: 255.0,
                a: 1.0,
            };
            ColorSliderState::new(
                "demo_delegate_blue",
                127.5,
                Box::new(ChannelDelegate {
                    spec: rgba,
                    channel_name: RgbaSpec::BLUE.into(),
                }),
                cx,
            )
            .min(0.0)
            .max(255.0)
            .horizontal()
            .thumb_medium()
        });

        let demo_delegate_saturation = cx.new(|cx| {
            let hsl = Hsl {
                h: 280.0,
                s: 0.5,
                l: 0.5,
                a: 1.0,
            };
            ColorSliderState::new(
                "demo_delegate_saturation",
                0.5,
                Box::new(ChannelDelegate {
                    spec: hsl,
                    channel_name: Hsl::SATURATION.into(),
                }),
                cx,
            )
            .horizontal()
            .thumb_medium()
        });

        let demo_delegate_lightness = cx.new(|cx| {
            let hsl = Hsl {
                h: 40.0,
                s: 0.8,
                l: 0.5,
                a: 1.0,
            };
            ColorSliderState::new(
                "demo_delegate_lightness",
                0.5,
                Box::new(ChannelDelegate {
                    spec: hsl,
                    channel_name: Hsl::LIGHTNESS.into(),
                }),
                cx,
            )
            .horizontal()
            .thumb_medium()
        });

        let demo_delegate_gradient = cx.new(|cx| {
            ColorSliderState::new(
                "demo_delegate_gradient",
                0.5,
                Box::new(GradientDelegate {
                    colors: vec![gpui::red(), gpui::yellow(), gpui::green()],
                }),
                cx,
            )
            .horizontal()
            .thumb_medium()
        });

        Self {
            demo_xsmall,
            demo_small,
            demo_medium,
            demo_large,
            demo_rounded_full,
            demo_rounded_8,
            demo_rounded_4,
            demo_square,
            demo_combo1,
            demo_combo2,
            demo_square_thumb,
            demo_square_thumb_square_corners,
            demo_square_thumb_square_corners_edge,
            demo_bar_thumb,
            demo_bar_thumb_square_corners,
            demo_bar_thumb_square_corners_edge,
            demo_edge_default,
            demo_edge_large,
            demo_edge_square,
            demo_thumb_xsmall_large,
            demo_thumb_small_xsmall,
            demo_thumb_medium_large,
            demo_thumb_large_small,
            demo_interp_rgb,
            demo_interp_hsl,
            demo_interp_lab,
            demo_delegate_hue,
            demo_delegate_alpha,
            demo_delegate_red,
            demo_delegate_green,
            demo_delegate_blue,
            demo_delegate_saturation,
            demo_delegate_lightness,
            demo_delegate_gradient,
        }
    }
}

impl Render for StoryColorSliderTab {
    fn render(&mut self, _: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        section("Customization Examples")
            .max_w_full()
            .child(
                v_flex()
                    .w_full()
                    .gap_6()
	                    .child(
	                        // Slider Delegate Examples
	                        v_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Slider Delegate Examples"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Different delegates provide various color calculation and visualization behaviors"),
                            )
                            .child(
                                v_flex()
                                    .w_full()
                                    .gap_4()
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .gap_6()
                                            .items_center()
                                            .child(
                                                v_flex()
                                                    .gap_2()
                                                    .flex_1()
                                                    .child(div().text_xs().child("HueDelegate"))
                                                    .child(self.demo_delegate_hue.clone()),
                                            )
                                            .child(
                                                v_flex()
                                                    .gap_2()
                                                    .flex_1()
                                                    .child(div().text_xs().child("AlphaDelegate"))
                                                    .child(self.demo_delegate_alpha.clone()),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .gap_6()
                                            .items_center()
	                                            .child(
	                                                v_flex()
	                                                    .gap_2()
	                                                    .flex_1()
	                                                    .child(div().text_xs().child("ChannelDelegate (Red Sweep)"))
	                                                    .child(
	                                                        div()
	                                                            .text_xs()
	                                                            .text_color(cx.theme().muted_foreground)
	                                                            .child("Red: RGB(0,128,0) -> RGB(255,128,0)"),
	                                                    )
	                                                    .child(self.demo_delegate_red.clone()),
	                                            )
	                                            .child(
	                                                v_flex()
	                                                    .gap_2()
	                                                    .flex_1()
	                                                    .child(div().text_xs().child("ChannelDelegate (Green Sweep)"))
	                                                    .child(
	                                                        div()
	                                                            .text_xs()
	                                                            .text_color(cx.theme().muted_foreground)
	                                                            .child("Green: RGB(128,0,0) -> RGB(128,255,0)"),
	                                                    )
	                                                    .child(self.demo_delegate_green.clone()),
	                                            )
	                                            .child(
	                                                v_flex()
	                                                    .gap_2()
	                                                    .flex_1()
	                                                    .child(div().text_xs().child("ChannelDelegate (Blue Sweep)"))
	                                                    .child(
	                                                        div()
	                                                            .text_xs()
	                                                            .text_color(cx.theme().muted_foreground)
	                                                            .child("Blue: RGB(0,128,0) -> RGB(0,128,255)"),
	                                                    )
	                                                    .child(self.demo_delegate_blue.clone()),
	                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .gap_6()
                                            .items_center()
                                            .child(
                                                v_flex()
                                                    .gap_2()
                                                    .flex_1()
                                                    .child(div().text_xs().child("ChannelDelegate (Saturation)"))
                                                    .child(self.demo_delegate_saturation.clone()),
                                            )
                                            .child(
                                                v_flex()
                                                    .gap_2()
                                                    .flex_1()
                                                    .child(div().text_xs().child("ChannelDelegate (Lightness)"))
                                                    .child(self.demo_delegate_lightness.clone()),
                                            )
                                            .child(
                                                v_flex()
                                                    .gap_2()
                                                    .flex_1()
                                                    .child(div().text_xs().child("GradientDelegate"))
                                                    .child(self.demo_delegate_gradient.clone()),
                                            ),
	                                    ),
	                            ),
	                    )
	                    .child(
	                        // Color Interpolation Variations
	                        v_flex()
	                            .w_full()
	                            .gap_4()
	                            .child(
	                                div()
	                                    .text_sm()
	                                    .font_semibold()
	                                    .text_color(cx.theme().foreground)
	                                    .child("Color Interpolation (Red to Blue)"),
	                            )
	                            .child(
	                                div()
	                                    .text_xs()
	                                    .text_color(cx.theme().muted_foreground)
	                                    .child("Compare RGB vs HSL vs Lab interpolation for better color transitions"),
	                            )
	                            .child(
	                                h_flex()
	                                    .w_full()
	                                    .gap_6()
	                                    .items_center()
	                                    .child(
	                                        v_flex()
	                                            .gap_2()
	                                            .flex_1()
	                                            .child(div().text_xs().child("RGB (Muddy)"))
	                                            .child(self.demo_interp_rgb.clone()),
	                                    )
	                                    .child(
	                                        v_flex()
	                                            .gap_2()
	                                            .flex_1()
	                                            .child(div().text_xs().child("HSL (Vibrant)"))
	                                            .child(self.demo_interp_hsl.clone()),
	                                    )
	                                    .child(
	                                        v_flex()
	                                            .gap_2()
	                                            .flex_1()
	                                            .child(div().text_xs().child("Lab (Uniform)"))
	                                            .child(self.demo_interp_lab.clone()),
	                                    ),
	                            ),
	                    )
	                    .child(
	                        // Size variations
	                        v_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Size Variations"),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_6()
                                    .items_center()
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("XSmall"))
                                            .child(self.demo_xsmall.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Small"))
                                            .child(self.demo_small.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Medium (Default)"))
                                            .child(self.demo_medium.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Large"))
                                            .child(self.demo_large.clone()),
                                    ),
                            ),
                    )
                    .child(
                        // Corner radius variations
                        v_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Corner Radius Variations"),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_6()
                                    .items_center()
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Rounded (Default)"))
                                            .child(self.demo_rounded_full.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Rounded 8px"))
                                            .child(self.demo_rounded_8.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Rounded 4px"))
                                            .child(self.demo_rounded_4.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Square (0px)"))
                                            .child(self.demo_square.clone()),
                                    ),
                            ),
                    )
                    .child(
                        // Combined customizations
                        v_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Combined Customizations"),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_6()
                                    .items_center()
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Large + Rounded 14px"))
                                            .child(self.demo_combo1.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Small + Square"))
                                            .child(self.demo_combo2.clone()),
                                    ),
                            ),
                    )
                    .child(
                        // Thumb Size Variations
                        v_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Independent Thumb Sizing"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Thumb size can be set independently from slider track size"),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_6()
                                    .items_center()
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("XSmall + Large Thumb"))
                                            .child(self.demo_thumb_xsmall_large.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Small + XSmall Thumb"))
                                            .child(self.demo_thumb_small_xsmall.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Medium + Large Thumb"))
                                            .child(self.demo_thumb_medium_large.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Large + Small Thumb"))
                                            .child(self.demo_thumb_large_small.clone()),
                                    ),
                            ),
                    )
                    .child(
                        // Thumb Shape Variations
                        v_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Thumb Shape Variations"),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_6()
                                    .items_center()
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Square Thumb"))
                                            .child(self.demo_square_thumb.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Square Thumb + Square Corners"))
                                            .child(self.demo_square_thumb_square_corners.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .child("Square Thumb + Square Corners + Edge-to-Edge"),
                                            )
                                            .child(self.demo_square_thumb_square_corners_edge.clone()),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_6()
                                    .items_center()
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Bar Thumb"))
                                            .child(self.demo_bar_thumb.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Bar Thumb + Square Corners"))
                                            .child(self.demo_bar_thumb_square_corners.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .child("Bar Thumb + Square Corners + Edge-to-Edge"),
                                            )
                                            .child(self.demo_bar_thumb_square_corners_edge.clone()),
                                    ),
                            ),
                    )
                    .child(
                        // Edge-to-Edge Thumb Positioning
                        v_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Edge-to-Edge Thumb Positioning"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Thumb extends to the edges of the slider track"),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_6()
                                    .items_center()
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Edge-to-Edge (Default Track)"))
                                            .child(self.demo_edge_default.clone()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .flex_1()
                                            .child(div().text_xs().child("Edge-to-Edge (Large Track)"))
                                            .child(self.demo_edge_large.clone()),
                                    )
                            )
                            .child(
                                h_flex().w_full().gap_6().items_center().child(
                                    v_flex()
                                        .gap_2()
                                        .flex_1()
                                        .child(div().text_xs().child("Edge-to-Edge + Square Corners"))
                                        .child(self.demo_edge_square.clone()),
                                ),
                            ),
                    )
	            )
    }
}
