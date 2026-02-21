use super::color_control_channels::ColorControlChannels;
use super::color_plane_controls::{ColorControlMode, ColorPlaneControls, ColorPlaneControlsState};
use super::color_spec::Hsv;
use super::color_readout::{PublishedColorSpec, render_color_readout_with_spec};
use super::compositions::color_combinations::{ColorCombinations, ColorCombinationsState};
use super::compositions::hsv_photoshop_picker::HsvPhotoshopPicker;
use super::compositions::hue_ring_sl_arcs_picker::{HueRingSlArcsPicker, HueRingSlArcsPickerState};
use super::compositions::hue_ring_sv_square_picker::{
    HueRingSvSquarePicker, HueRingSvSquarePickerState,
};
use super::compositions::hue_ring_sv_triangle_picker::{
    HueRingSvTrianglePicker, HueRingSvTrianglePickerState,
};
use super::compositions::mixers::multimixer::{MultiMixerEvent, MultiMixerState};
use gpui::*;
use gpui_component::{
    ActiveTheme as _,
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex, v_flex,
};

pub struct StoryCompositionsTab {
    color_combinations: Entity<ColorCombinationsState>,
    hue_ring_sv_square_picker: Entity<HueRingSvSquarePickerState>,
    hue_ring_sl_arcs_picker: Entity<HueRingSlArcsPickerState>,
    hue_ring_sv_triangle_picker: Entity<HueRingSvTrianglePickerState>,
    hsv_photoshop_picker: Entity<HsvPhotoshopPicker>,
    photoshop_picker_immediate: Entity<ColorPlaneControlsState>,
    multi_mixer: Entity<MultiMixerState>,
    multi_mixer_color: Option<Hsla>,
    multi_mixer_published_spec: Option<PublishedColorSpec>,
    _subscriptions: Vec<Subscription>,
}

const COMPOSITION_HORIZONTAL_GAP: f32 = 32.0;

impl StoryCompositionsTab {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let initial_color: Hsla = Rgba {
            r: 0.0,
            g: 216.0 / 255.0,
            b: 1.0,
            a: 1.0,
        }
        .into();
        let photoshop_hsla: Hsla = Rgba {
            r: 107.0 / 255.0,
            g: 42.0 / 255.0,
            b: 195.0 / 255.0,
            a: 1.0,
        }
        .into();
        let hsv_plane_hsv = Hsv::from_hsla_ext(
            Rgba {
                r: 1.0,
                g: 251.0 / 255.0,
                b: 23.0 / 255.0,
                a: 1.0,
            }
            .into(),
        );
        let color_combinations = cx.new(|cx| ColorCombinationsState::new(window, cx));
        let hue_ring_sv_square_picker = cx.new(|cx| HueRingSvSquarePickerState::new(window, cx));
        let hue_ring_sl_arcs_picker = cx.new(|cx| HueRingSlArcsPickerState::new(window, cx));
        let hue_ring_sv_triangle_picker =
            cx.new(|cx| HueRingSvTrianglePickerState::new(window, cx));
        let hsv_photoshop_picker = HsvPhotoshopPicker::new(
            cx,
            "test_hsv_combo",
            hsv_plane_hsv,
        );

        let photoshop_picker_immediate = cx.new(|cx| {
            ColorPlaneControlsState::new(window, cx)
                .mode(ColorControlMode::Immediate)
                .default_value(photoshop_hsla, cx)
        });
        let multi_mixer = cx.new(|cx| MultiMixerState::new(window, cx));
        let mut _subscriptions = vec![];
        _subscriptions.push(cx.subscribe(&multi_mixer, |this, _, ev, _| {
            let MultiMixerEvent::Change {
                color,
                published_spec,
            } = ev;
            this.multi_mixer_color = *color;
            this.multi_mixer_published_spec = *published_spec;
        }));

        Self {
            color_combinations,
            hue_ring_sv_square_picker,
            hue_ring_sl_arcs_picker,
            hue_ring_sv_triangle_picker,
            hsv_photoshop_picker,
            photoshop_picker_immediate,
            multi_mixer,
            multi_mixer_color: Some(initial_color),
            multi_mixer_published_spec: Some(PublishedColorSpec::Rgb),
            _subscriptions,
        }
    }
}

impl Render for StoryCompositionsTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let color_combinations_content = h_flex().w_full().justify_center().child(
            div()
                .w(px(ColorCombinationsState::PANEL_WIDTH_PX))
                .max_w_full()
                .child(
                    GroupBox::new()
                        .id("color-combinations-controls")
                        .outline()
                        .content_style(StyleRefinement::default().p_4().gap_6().items_center())
                        .child(ColorCombinations::new(&self.color_combinations)),
                ),
        );

        let donut_content = h_flex().w_full().justify_center().child(
            div()
                .w(px(HueRingSvSquarePickerState::RING_OUTER_SIZE_PX + 40.0))
                .max_w_full()
                .child(
                    GroupBox::new()
                        .id("hsv-color-picker-donut-controls")
                        .outline()
                        .content_style(StyleRefinement::default().p_3().gap_3().items_center())
                        .child(HueRingSvSquarePicker::new(&self.hue_ring_sv_square_picker)),
                ),
        );

        let hsv_photoshop_content = h_flex().w_full().justify_center().child(
            div().max_w_full().child(
                GroupBox::new()
                    .id("hsv-photoshop-picker-controls")
                    .outline()
                    .content_style(StyleRefinement::default().p_3().gap_3().items_center())
                    .child(self.hsv_photoshop_picker.clone()),
            ),
        );

        let arc_ring_content = h_flex().w_full().justify_center().child(
            div()
                .w(px(HueRingSlArcsPickerState::panel_width_px()))
                .max_w_full()
                .child(HueRingSlArcsPicker::new(&self.hue_ring_sl_arcs_picker)),
        );

        let photoshop_content = h_flex().w_full().justify_center().child(
            div().max_w_full().child(
                GroupBox::new()
                    .id("photoshop-picker-controls")
                    .outline()
                    .content_style(StyleRefinement::default().p_3().gap_3().items_center())
                    .child(
                        ColorPlaneControls::new(&self.photoshop_picker_immediate)
                            .channels(ColorControlChannels::photoshop().with_alpha(true))
                            .show_actions(false),
                    ),
            ),
        );

        let photoshop_triangle_content = h_flex().w_full().justify_center().child(
            div()
                .w(px(HueRingSvTrianglePickerState::panel_width_px()))
                .max_w_full()
                .child(
                    GroupBox::new()
                        .id("hue-ring-sv-triangle-picker-controls")
                        .outline()
                        .content_style(StyleRefinement::default().p_3().gap_3().items_center())
                        .child(HueRingSvTrianglePicker::new(
                            &self.hue_ring_sv_triangle_picker,
                        )),
                ),
        );

        let color_combinations_section = transparent_section(
            "Color Combinations",
            "section-color-combinations",
            color_combinations_content,
        );
        let donut_section = transparent_section("HSV Wheel", "section-hue-donut", donut_content);
        let hue_saturation_value_section = transparent_section(
            "HSV Plane (Photoshop)",
            "section-hsv-photoshop",
            hsv_photoshop_content,
        );
        let arc_ring_section = transparent_section(
            "Split-Ring (Pixagram)",
            "section-hue-ring-sl-arcs",
            arc_ring_content,
        );
        let photoshop_section = transparent_section(
            "Color Picker (Photoshop)",
            "section-photoshop-picker",
            photoshop_content,
        );
        let photoshop_triangle_section = transparent_section(
            "Hue Ring + SV Triangle (Photoshop)",
            "section-hue-ring-sv-triangle",
            photoshop_triangle_content,
        );
        let multi_mixer_section = transparent_section(
            "Multi Mixer",
            "section-multi-mixer",
            h_flex()
                .w_full()
                .justify_center()
                .items_start()
                .gap_8()
                .child(self.multi_mixer.clone())
                .child(render_color_readout_with_spec(
                    self.multi_mixer_color.unwrap_or(gpui::red()),
                    cx.theme().mono_font_family.clone(),
                    true,
                    self.multi_mixer_published_spec,
                )),
        );

        let row_one = h_flex()
            .w_full()
            .gap(px(COMPOSITION_HORIZONTAL_GAP))
            .items_start()
            .child(div().flex_1().child(photoshop_triangle_section))
            .child(div().flex_1().child(color_combinations_section))
            .child(div().flex_1().child(donut_section));

        let row_two = h_flex()
            .w_full()
            .gap(px(COMPOSITION_HORIZONTAL_GAP))
            .items_start()
            .child(div().flex_1().child(hue_saturation_value_section))
            .child(div().flex_1().child(arc_ring_section))
            .child(div().flex_1().child(photoshop_section));

        v_flex()
            .w_full()
            .gap_6()
            .child(row_one)
            .child(row_two)
            .child(multi_mixer_section)
    }
}

fn transparent_section(
    title: impl Into<SharedString>,
    id: impl Into<ElementId>,
    content: impl IntoElement,
) -> impl IntoElement {
    let title = title.into();

    GroupBox::new()
        .id(id)
        .normal()
        .title(
            h_flex()
                .justify_between()
                .w_full()
                .gap_4()
                .child(title.clone()),
        )
        .content_style(
            StyleRefinement::default()
                .overflow_x_hidden()
                .items_center()
                .justify_center(),
        )
        .child(
            h_flex()
                .w_full()
                .flex_wrap()
                .justify_center()
                .items_center()
                .gap(px(COMPOSITION_HORIZONTAL_GAP))
                .child(content),
        )
}
