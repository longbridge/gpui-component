use super::super::color_field::{
    CircleDomain, ColorFieldEvent, ColorFieldState, WhiteMixHueWheelModel,
};
use super::super::color_ring::{ColorRingEvent, ColorRingState, LightnessRingDelegate};
use super::color_hex::{format_rgb_hex, parse_rgb_hex};
use super::color_text_label::ColorTextLabel;
use super::combination_algorithms::{ColorCombination, CombinationSwatch};
use super::hsl_wheel_carrier::HslWheelCarrier;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme, Colorize, IconName, IndexPath, Sizable,
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    h_flex,
    input::{Input, InputEvent, InputState},
    select::{Select, SelectEvent, SelectState},
    v_flex,
};
use std::sync::Arc;

// Story UI composition for harmony controls + ring/wheel presentation.
// Palette math lives in `combination_algorithms.rs`.

pub struct ColorCombinationsState {
    combo_wheel: Entity<ColorFieldState>,
    combo_lightness_ring: Entity<ColorRingState>,
    combo_ring_angle_turns: f32,
    combination_select: Entity<SelectState<Vec<&'static str>>>,
    combo_color_input: Entity<InputState>,
    combo_color_input_programmatic_update: bool,
    combo_color_input_error: Option<SharedString>,
    combo_color: Hsla,
    _subscriptions: Vec<Subscription>,
}

impl ColorCombinationsState {
    pub const PANEL_WIDTH_PX: f32 = 350.0;
    pub const RING_OUTER_SIZE_PX: f32 = 300.0;
    pub const RING_TO_WHEEL_INNER_GAP_PX: f32 = 12.0;
    pub const RING_CANVAS_PADDING_PX: f32 = 20.0;

    fn combo_wheel_outer_size_px() -> f32 {
        let ring_inner_diameter = Self::RING_OUTER_SIZE_PX
            - 2.0 * super::super::color_ring::ring::sizing::RING_THICKNESS_MEDIUM;
        (ring_inner_diameter - Self::RING_TO_WHEEL_INNER_GAP_PX).max(40.0)
    }

    fn ring_canvas_size_px() -> f32 {
        Self::RING_OUTER_SIZE_PX + Self::RING_CANVAS_PADDING_PX
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let hsla = parse_rgb_hex("#19D90E").expect("hard-coded default color must be valid");
        let combo_wheel_size = Self::combo_wheel_outer_size_px();
        let combo_wheel_thumb_size = (combo_wheel_size * 0.07).max(10.0);
        let combo_wheel_field_hsv_carrier = HslWheelCarrier::from_hsla(hsla).into_field_hsv();
        let combo_wheel = cx.new(|_cx| {
            ColorFieldState::new(
                "composition_color_wheel_combinations",
                combo_wheel_field_hsv_carrier,
                Arc::new(CircleDomain),
                Arc::new(WhiteMixHueWheelModel),
            )
            .thumb_size(combo_wheel_thumb_size)
            .inside_field()
            .raster_image_prewarmed_square(combo_wheel_size)
        });
        let combo_lightness_ring = cx.new(|cx| {
            ColorRingState::lightness(
                "composition_color_wheel_lightness_ring",
                hsla.l,
                LightnessRingDelegate {
                    hue: hsla.h * 360.0,
                    saturation: hsla.s,
                },
                cx,
            )
            // Keep the wheel-to-ring inner spacing the same while using a thicker Medium ring.
            .with_size(gpui_component::Size::Size(px(Self::RING_OUTER_SIZE_PX)))
            .ring_thickness_size(gpui_component::Size::Medium)
            .thumb_size(16.0)
            .rotation_degrees(180.0)
        });
        let combination_select = cx.new(|cx| {
            let combinations = vec![
                "Monochromatic",
                "Complementary",
                "Analogous",
                "Triadic",
                "Tetradic",
                "Pentadic",
                "Hexadic",
            ];
            let default_combo_row = combinations
                .iter()
                .position(|label| *label == "Tetradic")
                .unwrap_or(0);
            SelectState::new(
                combinations,
                Some(IndexPath::default().row(default_combo_row)),
                window,
                cx,
            )
        });
        let combo_color_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("#RRGGBB")
                .pattern(regex::Regex::new(r"^#[0-9a-fA-F]{0,6}$").unwrap())
                .default_value(format_rgb_hex(hsla))
        });
        let combo_ring_angle_turns = combo_lightness_ring.read(cx).effective_angle_turns();

        let mut _subscriptions = Vec::new();

        _subscriptions.push(
            cx.subscribe_in(
                &combo_wheel,
                window,
                |this, _, event, window, cx| match event {
                    ColorFieldEvent::Change(hsv) | ColorFieldEvent::Release(hsv) => {
                        let mut wheel_hsla = HslWheelCarrier::from_field_hsv(*hsv).to_hsla();
                        wheel_hsla.l = this.combo_color.l;
                        this.combo_color = wheel_hsla;
                        this.sync_combo_color_input(window, cx);
                        this.sync_combo_wheel(cx);
                        this.sync_combo_ring(cx);
                    }
                },
            ),
        );
        _subscriptions.push(cx.subscribe_in(
            &combo_lightness_ring,
            window,
            |this, _, event, window, cx| {
                let value = match event {
                    ColorRingEvent::Change(value) | ColorRingEvent::Release(value) => *value,
                };
                this.combo_color.l = value;
                this.combo_ring_angle_turns =
                    this.combo_lightness_ring.read(cx).effective_angle_turns();
                this.sync_combo_color_input(window, cx);
                this.sync_combo_wheel(cx);
                cx.notify();
            },
        ));

        _subscriptions.push(cx.subscribe_in(
            &combination_select,
            window,
            |this, _, _: &SelectEvent<Vec<&'static str>>, window, cx| {
                // Disabled for now (revisit later):
                // When harmony mode changed we forced the ring thumb to UI 90deg
                // (math-space `angle_turns = 0.0`) by converting that angle to a lightness value,
                // then pushing that value back into ring + wheel + input state.
                // This made combination changes feel surprising, so it is intentionally commented out.
                // let target_angle_turns = 0.0;
                // this.combo_color.l = this.lightness_for_ring_angle(target_angle_turns, cx);
                // this.combo_ring_angle_turns = target_angle_turns;
                // this.sync_combo_color_input(window, cx);
                let _ = window;
                this.sync_combo_wheel(cx);
                cx.notify();
            },
        ));
        _subscriptions.push(cx.subscribe_in(
            &combo_color_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                this.on_combo_color_input_event(event, window, cx);
            },
        ));

        let mut this = Self {
            combo_wheel,
            combo_lightness_ring,
            combo_ring_angle_turns,
            combination_select,
            combo_color_input,
            combo_color_input_programmatic_update: false,
            combo_color_input_error: None,
            combo_color: hsla,
            _subscriptions,
        };
        this.sync_combo_wheel(cx);
        this.sync_combo_ring(cx);
        this
    }

    fn on_combo_color_input_event(
        &mut self,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.combo_color_input_programmatic_update {
            return;
        }
        let input_text = self.combo_color_input.read(cx).value().to_string();
        let current_hex = format_rgb_hex(self.combo_color);
        if input_text.trim().eq_ignore_ascii_case(current_hex.as_ref()) {
            self.combo_color_input_error = None;
            return;
        }
        let strict_validation = matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur);

        match parse_rgb_hex(&input_text) {
            Ok(parsed) => {
                self.combo_color_input_error = None;
                self.combo_color = parsed;
                self.sync_combo_color_input(window, cx);
                self.sync_combo_wheel(cx);
                self.sync_combo_ring(cx);
                cx.notify();
            }
            Err(message) => {
                if strict_validation {
                    self.combo_color_input_error = Some(message.into());
                } else {
                    self.combo_color_input_error = None;
                }
                cx.notify();
            }
        }
    }

    fn sync_combo_color_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = format_rgb_hex(self.combo_color);
        let current = self.combo_color_input.read(cx).value();
        if current == value {
            return;
        }

        self.combo_color_input_programmatic_update = true;
        self.combo_color_input.update(cx, |input, cx| {
            input.set_value(value, window, cx);
        });
        self.combo_color_input_programmatic_update = false;
    }

    fn sync_combo_wheel(&mut self, cx: &mut Context<Self>) {
        let hsla = self.combo_color;
        let field_hsv_carrier = HslWheelCarrier::from_hsla(hsla).into_field_hsv();
        self.combo_wheel.update(cx, |w, cx| {
            w.set_hsv(field_hsv_carrier, cx);
        });
    }

    fn sync_combo_ring(&mut self, cx: &mut Context<Self>) {
        let hsla = self.combo_color;
        let target_angle_turns = self.select_ring_angle_for_lightness(hsla.l, cx);
        self.combo_ring_angle_turns = target_angle_turns;
        self.combo_lightness_ring.update(cx, |ring, cx| {
            ring.set_value_with_angle_turns(hsla.l, target_angle_turns, cx);
            ring.set_delegate(
                Box::new(LightnessRingDelegate {
                    hue: hsla.h * 360.0,
                    saturation: hsla.s,
                }),
                cx,
            );
        });
    }

    fn select_ring_angle_for_lightness(&self, lightness: f32, cx: &App) -> f32 {
        let ring = self.combo_lightness_ring.read(cx);
        let rotation_turns = ring.rotation_turns();
        let to_angle_turns = |position: f32| {
            let mapped_position = if ring.reversed {
                1.0 - position
            } else {
                position
            };
            (mapped_position - 0.25 + rotation_turns).rem_euclid(1.0)
        };

        let canonical_position = (0.5 + lightness.clamp(0.0, 1.0) * 0.5).rem_euclid(1.0);
        let mirrored_position = (1.0 - canonical_position).rem_euclid(1.0);
        let canonical_angle_turns = to_angle_turns(canonical_position);
        let mirrored_angle_turns = to_angle_turns(mirrored_position);

        if turn_distance(canonical_angle_turns, self.combo_ring_angle_turns)
            <= turn_distance(mirrored_angle_turns, self.combo_ring_angle_turns)
        {
            canonical_angle_turns
        } else {
            mirrored_angle_turns
        }
    }

    #[allow(dead_code)] // Kept for the deferred "set ring by fixed angle on mode change" behavior.
    fn lightness_for_ring_angle(&self, angle_turns: f32, cx: &App) -> f32 {
        let ring = self.combo_lightness_ring.read(cx);
        let mut position =
            (angle_turns.rem_euclid(1.0) + 0.25 - ring.rotation_turns()).rem_euclid(1.0);
        if ring.reversed {
            position = 1.0 - position;
        }
        ring.delegate.position_to_value(&ring, position)
    }

    fn selected_combination(&self, cx: &App) -> ColorCombination {
        self.combination_select
            .read(cx)
            .selected_value()
            .map(|value| ColorCombination::from_label(value))
            .unwrap_or_default()
    }
}

impl Render for ColorCombinationsState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ColorCombinations::new(&cx.entity())
    }
}

#[derive(IntoElement)]
pub struct ColorCombinations {
    state: Entity<ColorCombinationsState>,
}

struct ColorCombinationsLayout;

impl ColorCombinationsLayout {
    const COLOR_LABEL: &'static str = "Color";
    const COMBINATION_LABEL: &'static str = "Combination";
    const WHEEL_TO_CONTROLS_GAP_PX: f32 = Self::COMBINATION_ROW_TO_SWATCHES_GAP_PX;
    const COLOR_ROW_TO_COMBINATION_GAP_PX: f32 = 8.0;
    const COMBINATION_ROW_TO_SWATCHES_GAP_PX: f32 = 8.0;
    const FIELD_LABEL_WIDTH_PX: f32 = 92.0;
    const FIELD_ROW_GAP_PX: f32 = 16.0;
    const COLOR_INPUT_WIDTH_PX: f32 = 200.0;
    const COLOR_INPUT_HEIGHT_PX: f32 = 26.0;
    const COLOR_SWATCH_SIZE_PX: f32 = Self::COLOR_INPUT_HEIGHT_PX;
    const INPUT_SWATCH_GAP_PX: f32 = 8.0;
    const FIELD_CONTROL_COLUMN_WIDTH_PX: f32 =
        Self::COLOR_INPUT_WIDTH_PX + Self::INPUT_SWATCH_GAP_PX + Self::COLOR_SWATCH_SIZE_PX;
    const CONTROL_TEXT_SIZE_PX: f32 = 10.0;
    const SELECT_TEXT_SIZE_PX: f32 = 13.0;
    const INPUT_ERROR_TEXT_SIZE_PX: f32 = 11.0;
}

struct ColorCombinationsViewData {
    combination_palette: Vec<CombinationSwatch>,
    combo_lightness_ring: Entity<ColorRingState>,
    combo_wheel: Entity<ColorFieldState>,
    combo_wheel_size: f32,
    combo_color: Hsla,
    combo_color_input: Entity<InputState>,
    combo_color_value: String,
    combo_color_input_error: Option<SharedString>,
    combination_select: Entity<SelectState<Vec<&'static str>>>,
}

impl ColorCombinationsViewData {
    fn from_state(state: &ColorCombinationsState, cx: &App) -> Self {
        let combination = state.selected_combination(cx);
        Self {
            combination_palette: combination.palette(state.combo_color),
            combo_lightness_ring: state.combo_lightness_ring.clone(),
            combo_wheel: state.combo_wheel.clone(),
            combo_wheel_size: ColorCombinationsState::combo_wheel_outer_size_px(),
            combo_color: state.combo_color,
            combo_color_input: state.combo_color_input.clone(),
            combo_color_value: state.combo_color_input.read(cx).value().to_string(),
            combo_color_input_error: state.combo_color_input_error.clone(),
            combination_select: state.combination_select.clone(),
        }
    }
}

impl ColorCombinations {
    pub fn new(state: &Entity<ColorCombinationsState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for ColorCombinations {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let view = ColorCombinationsViewData::from_state(&state, cx);
        let control_font_family = cx.theme().mono_font_family.clone();

        v_flex()
            .w_full()
            .gap_0()
            .child(render_color_combinations_canvas(&view, cx))
            .child(render_color_combinations_controls(
                &view,
                control_font_family,
                cx,
            ))
    }
}

fn render_color_combinations_canvas(
    view: &ColorCombinationsViewData,
    cx: &mut App,
) -> impl IntoElement {
    let harmony_points = harmony_points_from_swatches(&view.combination_palette);
    h_flex().w_full().justify_center().child(
        div()
            .size(px(ColorCombinationsState::ring_canvas_size_px()))
            .flex_shrink_0()
            .relative()
            // Order matters: ring below + wheel above preserves center wheel mouse interaction.
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(view.combo_lightness_ring.clone()),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(render_combo_wheel_layer(
                        view.combo_wheel.clone(),
                        view.combo_wheel_size,
                        harmony_points,
                        cx,
                    )),
            ),
    )
}

fn render_color_combinations_controls(
    view: &ColorCombinationsViewData,
    control_font_family: SharedString,
    cx: &mut App,
) -> impl IntoElement {
    v_flex()
        .w_full()
        .pt(px(ColorCombinationsLayout::WHEEL_TO_CONTROLS_GAP_PX))
        .child(
            h_flex()
                .w_full()
                .mb(px(ColorCombinationsLayout::COLOR_ROW_TO_COMBINATION_GAP_PX))
                .justify_start()
                .items_center()
                .gap(px(ColorCombinationsLayout::FIELD_ROW_GAP_PX))
                .child(
                    div()
                        .w(px(ColorCombinationsLayout::FIELD_LABEL_WIDTH_PX))
                        .text_right()
                        .font_family(control_font_family.clone())
                        .text_size(px(ColorCombinationsLayout::CONTROL_TEXT_SIZE_PX))
                        .child(ColorCombinationsLayout::COLOR_LABEL),
                )
                .child(
                    v_flex()
                        .w(px(ColorCombinationsLayout::FIELD_CONTROL_COLUMN_WIDTH_PX))
                        .gap_1()
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(ColorCombinationsLayout::INPUT_SWATCH_GAP_PX))
                                .child(
                                    Input::new(&view.combo_color_input)
                                        .w(px(ColorCombinationsLayout::COLOR_INPUT_WIDTH_PX))
                                        .with_size(gpui_component::Size::Small)
                                        .font_family(control_font_family.clone())
                                        .text_size(px(
                                            ColorCombinationsLayout::CONTROL_TEXT_SIZE_PX,
                                        ))
                                        .suffix(
                                            Clipboard::new("color-combinations-color-copy")
                                                .value(view.combo_color_value.clone()),
                                        )
                                        .when(view.combo_color_input_error.is_some(), |input| {
                                            input.border_color(cx.theme().danger)
                                        }),
                                )
                                .child(
                                    div()
                                        .size(px(ColorCombinationsLayout::COLOR_SWATCH_SIZE_PX))
                                        .rounded_sm()
                                        .bg(view.combo_color)
                                        .border_1()
                                        .border_color(cx.theme().border),
                                ),
                        )
                        .when_some(view.combo_color_input_error.clone(), |this, error| {
                            this.child(
                                div()
                                    .text_size(px(
                                        ColorCombinationsLayout::INPUT_ERROR_TEXT_SIZE_PX,
                                    ))
                                    .text_color(cx.theme().danger)
                                    .child(error),
                            )
                        }),
                ),
        )
        .child(
            h_flex()
                .w_full()
                .mb(px(
                    ColorCombinationsLayout::COMBINATION_ROW_TO_SWATCHES_GAP_PX,
                ))
                .justify_start()
                .items_center()
                .gap(px(ColorCombinationsLayout::FIELD_ROW_GAP_PX))
                .child(
                    div()
                        .w(px(ColorCombinationsLayout::FIELD_LABEL_WIDTH_PX))
                        .text_right()
                        .font_family(control_font_family.clone())
                        .text_size(px(ColorCombinationsLayout::CONTROL_TEXT_SIZE_PX))
                        .child(ColorCombinationsLayout::COMBINATION_LABEL),
                )
                .child(
                    div()
                        .w(px(ColorCombinationsLayout::FIELD_CONTROL_COLUMN_WIDTH_PX))
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(ColorCombinationsLayout::INPUT_SWATCH_GAP_PX))
                                .child(
                                    Select::new(&view.combination_select)
                                        .with_size(px(ColorCombinationsLayout::SELECT_TEXT_SIZE_PX))
                                        .w(px(ColorCombinationsLayout::COLOR_INPUT_WIDTH_PX)),
                                )
                                .child(
                                    div().size(px(ColorCombinationsLayout::COLOR_SWATCH_SIZE_PX)),
                                ),
                        ),
                ),
        )
        .child(render_color_combinations_swatches(
            &view.combination_palette,
            cx,
        ))
}

fn render_color_combinations_swatches(
    swatches: &[CombinationSwatch],
    cx: &App,
) -> impl IntoElement {
    h_flex()
        .w_full()
        .gap_1()
        .children(swatches.iter().enumerate().map(|(index, swatch)| {
            let swatch_hex = swatch.color.to_hex().to_uppercase();
            let swatch_hex_for_copy = swatch_hex.clone();
            let icon_foreground = swatch_icon_foreground(swatch.color, cx.theme().muted_foreground);
            v_flex()
                .flex_1()
                .min_w(px(0.0))
                .gap_1()
                .child(
                    div()
                        .h(px(40.0))
                        .rounded_md()
                        .bg(swatch.color)
                        .border_1()
                        .relative()
                        .child(
                            div().absolute().right(px(4.0)).bottom(px(4.0)).child(
                                Button::new(format!("color-combination-swatch-copy-icon-{index}"))
                                    .icon(IconName::Copy)
                                    .ghost()
                                    .xsmall()
                                    .text_color(icon_foreground)
                                    .on_click(move |_, _, cx| {
                                        cx.stop_propagation();
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            swatch_hex_for_copy.to_string(),
                                        ));
                                    }),
                            ),
                        ),
                )
                .child(
                    ColorTextLabel::new(
                        format!("color-combination-swatch-copy-{index}"),
                        swatch_hex.into(),
                    )
                    .show_icon(false),
                )
                .into_any_element()
        }))
}

fn harmony_points_from_swatches(swatches: &[CombinationSwatch]) -> Vec<(f32, f32)> {
    swatches
        .iter()
        .filter(|swatch| swatch.role != "Base")
        .map(|swatch| (swatch.color.h * 360.0, swatch.color.s.clamp(0.0, 1.0)))
        .collect()
}

fn render_combo_wheel_layer(
    combo_wheel: Entity<ColorFieldState>,
    combo_wheel_size: f32,
    harmony_points: Vec<(f32, f32)>,
    cx: &mut App,
) -> impl IntoElement {
    let center = combo_wheel_size * 0.5;
    let marker_size = (((combo_wheel_size * 0.07).max(10.0)) * 0.64).max(8.0);
    let marker_half = marker_size * 0.5;
    let marker_radius_max = (center - marker_half).max(0.0);

    div()
        .size(px(combo_wheel_size))
        .relative()
        .child(combo_wheel)
        .children(harmony_points.into_iter().map(|(hue_degrees, radius)| {
            let angle = hue_degrees.rem_euclid(360.0).to_radians();
            let marker_radius = marker_radius_max * radius.clamp(0.0, 1.0);
            let left = center + marker_radius * angle.cos() - marker_half;
            let top = center - marker_radius * angle.sin() - marker_half;
            let marker_color = white_mix_wheel_color(hue_degrees, radius);

            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                .size(px(marker_size))
                .rounded_full()
                .bg(marker_color)
                .border_1()
                .border_color(cx.theme().border)
                .shadow_sm()
                .into_any_element()
        }))
}

fn white_mix_wheel_color(hue_degrees: f32, saturation: f32) -> Hsla {
    let hue_rgb = hsla(
        (hue_degrees.rem_euclid(360.0) / 360.0).rem_euclid(1.0),
        1.0,
        0.5,
        1.0,
    )
    .to_rgb();
    let sat = saturation.clamp(0.0, 1.0);

    Rgba {
        r: (1.0 - sat) + hue_rgb.r * sat,
        g: (1.0 - sat) + hue_rgb.g * sat,
        b: (1.0 - sat) + hue_rgb.b * sat,
        a: 1.0,
    }
    .into()
}

fn turn_distance(a: f32, b: f32) -> f32 {
    let delta = (a - b).abs().rem_euclid(1.0);
    delta.min(1.0 - delta)
}

fn swatch_icon_foreground(swatch: Hsla, default_foreground: Hsla) -> Hsla {
    const MIN_ICON_CONTRAST_RATIO: f32 = 3.0;
    if contrast_ratio(swatch, default_foreground) >= MIN_ICON_CONTRAST_RATIO {
        return default_foreground;
    }

    let white = hsla(0.0, 0.0, 1.0, 1.0);
    let black = hsla(0.0, 0.0, 0.0, 1.0);
    if contrast_ratio(swatch, white) >= contrast_ratio(swatch, black) {
        white
    } else {
        black
    }
}

fn contrast_ratio(a: Hsla, b: Hsla) -> f32 {
    let luminance_a = relative_luminance(a);
    let luminance_b = relative_luminance(b);
    let (lighter, darker) = if luminance_a >= luminance_b {
        (luminance_a, luminance_b)
    } else {
        (luminance_b, luminance_a)
    };
    (lighter + 0.05) / (darker + 0.05)
}

fn relative_luminance(color: Hsla) -> f32 {
    let rgb = color.to_rgb();
    0.2126 * srgb_channel_to_linear(rgb.r)
        + 0.7152 * srgb_channel_to_linear(rgb.g)
        + 0.0722 * srgb_channel_to_linear(rgb.b)
}

fn srgb_channel_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}
