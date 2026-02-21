use super::color_arc::{
    ColorArc, ColorArcEvent, ColorArcRasterState, ColorArcRenderer, ColorArcState,
};
use crate::section;
use gpui::*;
use gpui_component::{h_flex, v_flex, Sizable};

const ARC_GAP_DEGREES: f32 = 8.0;
const ARC_ROTATION_DEGREES: f32 = 90.0;
const ARC_SWEEP_DEGREES: f32 = 180.0 - ARC_GAP_DEGREES;
const ARC_SWEEP_270_DEGREES: f32 = 270.0;
const ARC_HORIZONTAL_OFFSET_PX: f32 = 5.0;
const ARC_HORIZONTAL_OFFSET_THICKNESS_PX: f32 = 14.0;
const ARC_PAIR_READOUT_COUNT: usize = 10;
const ARC_SINGLE_READOUT_COUNT: usize = 3;

#[derive(Clone, Copy)]
struct ArcPairReadout {
    event_name: &'static str,
    event_value: f32,
}

impl ArcPairReadout {
    const DEFAULT: Self = Self {
        event_name: "Release",
        event_value: 0.0,
    };
}

#[derive(Clone, Copy)]
enum ArcPairReadoutKey {
    CompareVector,
    CompareRaster,
    SizeXSmall,
    SizeSmall,
    SizeMedium,
    SizeLarge,
    ThicknessXSmall,
    ThicknessSmall,
    ThicknessMedium,
    ThicknessLarge,
}

impl ArcPairReadoutKey {
    const fn index(self) -> usize {
        match self {
            Self::CompareVector => 0,
            Self::CompareRaster => 1,
            Self::SizeXSmall => 2,
            Self::SizeSmall => 3,
            Self::SizeMedium => 4,
            Self::SizeLarge => 5,
            Self::ThicknessXSmall => 6,
            Self::ThicknessSmall => 7,
            Self::ThicknessMedium => 8,
            Self::ThicknessLarge => 9,
        }
    }
}

#[derive(Clone, Copy)]
enum ArcSingleReadoutKey {
    Hue270,
    Saturation270,
    Lightness270,
}

impl ArcSingleReadoutKey {
    const fn index(self) -> usize {
        match self {
            Self::Hue270 => 0,
            Self::Saturation270 => 1,
            Self::Lightness270 => 2,
        }
    }
}

pub struct StoryColorArcTab {
    color_arc_saturation: Entity<ColorArcState>,
    color_arc_lightness: Entity<ColorArcState>,
    color_arc_saturation_vector_compare: Entity<ColorArcState>,
    color_arc_lightness_vector_compare: Entity<ColorArcState>,
    color_arc_saturation_raster_compare: Entity<ColorArcRasterState>,
    color_arc_lightness_raster_compare: Entity<ColorArcRasterState>,
    color_arc_saturation_xsmall: Entity<ColorArcState>,
    color_arc_lightness_xsmall: Entity<ColorArcState>,
    color_arc_saturation_small: Entity<ColorArcState>,
    color_arc_lightness_small: Entity<ColorArcState>,
    color_arc_saturation_medium: Entity<ColorArcState>,
    color_arc_lightness_medium: Entity<ColorArcState>,
    color_arc_saturation_large: Entity<ColorArcState>,
    color_arc_lightness_large: Entity<ColorArcState>,
    color_arc_saturation_thickness_xsmall: Entity<ColorArcState>,
    color_arc_lightness_thickness_xsmall: Entity<ColorArcState>,
    color_arc_saturation_thickness_small: Entity<ColorArcState>,
    color_arc_lightness_thickness_small: Entity<ColorArcState>,
    color_arc_saturation_thickness_medium: Entity<ColorArcState>,
    color_arc_lightness_thickness_medium: Entity<ColorArcState>,
    color_arc_saturation_thickness_large: Entity<ColorArcState>,
    color_arc_lightness_thickness_large: Entity<ColorArcState>,
    color_arc_hue_270: Entity<ColorArcState>,
    color_arc_saturation_270: Entity<ColorArcState>,
    color_arc_lightness_270: Entity<ColorArcState>,
    pair_readouts: [ArcPairReadout; ARC_PAIR_READOUT_COUNT],
    single_readouts: [ArcPairReadout; ARC_SINGLE_READOUT_COUNT],
    _subscriptions: Vec<Subscription>,
}

impl StoryColorArcTab {
    fn prewarm_arc_square(arc: &Entity<ColorArcState>, side_px: f32, cx: &mut Context<Self>) {
        arc.update(cx, |state, cx| {
            state.prewarm_raster_cache_square_in_place(side_px, cx);
        });
    }

    fn prewarm_raster_arc_caches(&self, cx: &mut Context<Self>) {
        Self::prewarm_arc_square(&self.color_arc_saturation, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_lightness, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_saturation_raster_compare, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_lightness_raster_compare, 220.0, cx);

        Self::prewarm_arc_square(&self.color_arc_saturation_xsmall, 140.0, cx);
        Self::prewarm_arc_square(&self.color_arc_lightness_xsmall, 140.0, cx);
        Self::prewarm_arc_square(&self.color_arc_saturation_small, 180.0, cx);
        Self::prewarm_arc_square(&self.color_arc_lightness_small, 180.0, cx);
        Self::prewarm_arc_square(&self.color_arc_saturation_medium, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_lightness_medium, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_saturation_large, 280.0, cx);
        Self::prewarm_arc_square(&self.color_arc_lightness_large, 280.0, cx);

        Self::prewarm_arc_square(&self.color_arc_saturation_thickness_xsmall, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_lightness_thickness_xsmall, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_saturation_thickness_small, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_lightness_thickness_small, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_saturation_thickness_medium, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_lightness_thickness_medium, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_saturation_thickness_large, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_lightness_thickness_large, 220.0, cx);

        Self::prewarm_arc_square(&self.color_arc_hue_270, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_saturation_270, 220.0, cx);
        Self::prewarm_arc_square(&self.color_arc_lightness_270, 220.0, cx);
    }

    fn update_pair_readout(&mut self, key: ArcPairReadoutKey, event: &ColorArcEvent) {
        let (event_name, event_value) = match event {
            ColorArcEvent::Change(value) => ("Change", *value),
            ColorArcEvent::Release(value) => ("Release", *value),
        };
        self.pair_readouts[key.index()] = ArcPairReadout {
            event_name,
            event_value,
        };
    }

    fn pair_readout(&self, key: ArcPairReadoutKey) -> ArcPairReadout {
        self.pair_readouts[key.index()]
    }

    fn update_single_readout(&mut self, key: ArcSingleReadoutKey, event: &ColorArcEvent) {
        let (event_name, event_value) = match event {
            ColorArcEvent::Change(value) => ("Change", *value),
            ColorArcEvent::Release(value) => ("Release", *value),
        };
        self.single_readouts[key.index()] = ArcPairReadout {
            event_name,
            event_value,
        };
    }

    fn single_readout(&self, key: ArcSingleReadoutKey) -> ArcPairReadout {
        self.single_readouts[key.index()]
    }

    fn subscribe_pair_readout_events(
        subscriptions: &mut Vec<Subscription>,
        key: ArcPairReadoutKey,
        saturation_arc: &Entity<ColorArcState>,
        lightness_arc: &Entity<ColorArcState>,
        cx: &mut Context<Self>,
    ) {
        subscriptions.push(cx.subscribe(saturation_arc, move |this, _, event, cx| {
            this.update_pair_readout(key, event);
            cx.notify();
        }));
        subscriptions.push(cx.subscribe(lightness_arc, move |this, _, event, cx| {
            this.update_pair_readout(key, event);
            cx.notify();
        }));
    }

    fn subscribe_single_readout_event(
        subscriptions: &mut Vec<Subscription>,
        key: ArcSingleReadoutKey,
        arc: &Entity<ColorArcState>,
        cx: &mut Context<Self>,
    ) {
        subscriptions.push(cx.subscribe(arc, move |this, _, event, cx| {
            this.update_single_readout(key, event);
            cx.notify();
        }));
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let color_arc_saturation =
            Self::build_saturation_arc(cx, "color_arc_saturation", gpui_component::Size::Medium);
        let color_arc_lightness =
            Self::build_lightness_arc(cx, "color_arc_lightness", gpui_component::Size::Medium);
        let color_arc_saturation_vector_compare = Self::build_saturation_arc_with_renderer(
            cx,
            "color_arc_saturation_vector_compare",
            gpui_component::Size::Medium,
            ColorArcRenderer::Vector,
        );
        let color_arc_lightness_vector_compare = Self::build_lightness_arc_with_renderer(
            cx,
            "color_arc_lightness_vector_compare",
            gpui_component::Size::Medium,
            ColorArcRenderer::Vector,
        );
        let color_arc_saturation_raster_compare = Self::build_saturation_arc_with_renderer(
            cx,
            "color_arc_saturation_raster_compare",
            gpui_component::Size::Medium,
            ColorArcRenderer::Raster,
        );
        let color_arc_lightness_raster_compare = Self::build_lightness_arc_with_renderer(
            cx,
            "color_arc_lightness_raster_compare",
            gpui_component::Size::Medium,
            ColorArcRenderer::Raster,
        );
        let color_arc_saturation_xsmall = Self::build_saturation_arc(
            cx,
            "color_arc_saturation_xsmall",
            gpui_component::Size::XSmall,
        );
        let color_arc_lightness_xsmall = Self::build_lightness_arc(
            cx,
            "color_arc_lightness_xsmall",
            gpui_component::Size::XSmall,
        );
        let color_arc_saturation_small = Self::build_saturation_arc(
            cx,
            "color_arc_saturation_small",
            gpui_component::Size::Small,
        );
        let color_arc_lightness_small =
            Self::build_lightness_arc(cx, "color_arc_lightness_small", gpui_component::Size::Small);
        let color_arc_saturation_medium = Self::build_saturation_arc(
            cx,
            "color_arc_saturation_medium",
            gpui_component::Size::Medium,
        );
        let color_arc_lightness_medium = Self::build_lightness_arc(
            cx,
            "color_arc_lightness_medium",
            gpui_component::Size::Medium,
        );
        let color_arc_saturation_large = Self::build_saturation_arc(
            cx,
            "color_arc_saturation_large",
            gpui_component::Size::Large,
        );
        let color_arc_lightness_large =
            Self::build_lightness_arc(cx, "color_arc_lightness_large", gpui_component::Size::Large);
        let color_arc_saturation_thickness_xsmall = Self::build_saturation_arc_with_thickness(
            cx,
            "color_arc_saturation_thickness_xsmall",
            gpui_component::Size::Medium,
            gpui_component::Size::XSmall,
        );
        let color_arc_lightness_thickness_xsmall = Self::build_lightness_arc_with_thickness(
            cx,
            "color_arc_lightness_thickness_xsmall",
            gpui_component::Size::Medium,
            gpui_component::Size::XSmall,
        );
        let color_arc_saturation_thickness_small = Self::build_saturation_arc_with_thickness(
            cx,
            "color_arc_saturation_thickness_small",
            gpui_component::Size::Medium,
            gpui_component::Size::Small,
        );
        let color_arc_lightness_thickness_small = Self::build_lightness_arc_with_thickness(
            cx,
            "color_arc_lightness_thickness_small",
            gpui_component::Size::Medium,
            gpui_component::Size::Small,
        );
        let color_arc_saturation_thickness_medium = Self::build_saturation_arc_with_thickness(
            cx,
            "color_arc_saturation_thickness_medium",
            gpui_component::Size::Medium,
            gpui_component::Size::Medium,
        );
        let color_arc_lightness_thickness_medium = Self::build_lightness_arc_with_thickness(
            cx,
            "color_arc_lightness_thickness_medium",
            gpui_component::Size::Medium,
            gpui_component::Size::Medium,
        );
        let color_arc_saturation_thickness_large = Self::build_saturation_arc_with_thickness(
            cx,
            "color_arc_saturation_thickness_large",
            gpui_component::Size::Medium,
            gpui_component::Size::Large,
        );
        let color_arc_lightness_thickness_large = Self::build_lightness_arc_with_thickness(
            cx,
            "color_arc_lightness_thickness_large",
            gpui_component::Size::Medium,
            gpui_component::Size::Large,
        );
        let color_arc_hue_270 = Self::build_hue_arc_at_270(cx, "color_arc_hue_270");
        let color_arc_saturation_270 =
            Self::build_saturation_arc_at_270(cx, "color_arc_saturation_270");
        let color_arc_lightness_270 =
            Self::build_lightness_arc_at_270(cx, "color_arc_lightness_270");

        let mut _subscriptions = Vec::new();
        Self::subscribe_pair_readout_events(
            &mut _subscriptions,
            ArcPairReadoutKey::CompareVector,
            &color_arc_saturation_vector_compare,
            &color_arc_lightness_vector_compare,
            cx,
        );
        Self::subscribe_pair_readout_events(
            &mut _subscriptions,
            ArcPairReadoutKey::CompareRaster,
            &color_arc_saturation_raster_compare,
            &color_arc_lightness_raster_compare,
            cx,
        );
        Self::subscribe_pair_readout_events(
            &mut _subscriptions,
            ArcPairReadoutKey::SizeXSmall,
            &color_arc_saturation_xsmall,
            &color_arc_lightness_xsmall,
            cx,
        );
        Self::subscribe_pair_readout_events(
            &mut _subscriptions,
            ArcPairReadoutKey::SizeSmall,
            &color_arc_saturation_small,
            &color_arc_lightness_small,
            cx,
        );
        Self::subscribe_pair_readout_events(
            &mut _subscriptions,
            ArcPairReadoutKey::SizeMedium,
            &color_arc_saturation_medium,
            &color_arc_lightness_medium,
            cx,
        );
        Self::subscribe_pair_readout_events(
            &mut _subscriptions,
            ArcPairReadoutKey::SizeLarge,
            &color_arc_saturation_large,
            &color_arc_lightness_large,
            cx,
        );
        Self::subscribe_pair_readout_events(
            &mut _subscriptions,
            ArcPairReadoutKey::ThicknessXSmall,
            &color_arc_saturation_thickness_xsmall,
            &color_arc_lightness_thickness_xsmall,
            cx,
        );
        Self::subscribe_pair_readout_events(
            &mut _subscriptions,
            ArcPairReadoutKey::ThicknessSmall,
            &color_arc_saturation_thickness_small,
            &color_arc_lightness_thickness_small,
            cx,
        );
        Self::subscribe_pair_readout_events(
            &mut _subscriptions,
            ArcPairReadoutKey::ThicknessMedium,
            &color_arc_saturation_thickness_medium,
            &color_arc_lightness_thickness_medium,
            cx,
        );
        Self::subscribe_pair_readout_events(
            &mut _subscriptions,
            ArcPairReadoutKey::ThicknessLarge,
            &color_arc_saturation_thickness_large,
            &color_arc_lightness_thickness_large,
            cx,
        );
        Self::subscribe_single_readout_event(
            &mut _subscriptions,
            ArcSingleReadoutKey::Hue270,
            &color_arc_hue_270,
            cx,
        );
        Self::subscribe_single_readout_event(
            &mut _subscriptions,
            ArcSingleReadoutKey::Saturation270,
            &color_arc_saturation_270,
            cx,
        );
        Self::subscribe_single_readout_event(
            &mut _subscriptions,
            ArcSingleReadoutKey::Lightness270,
            &color_arc_lightness_270,
            cx,
        );

        let mut this = Self {
            color_arc_saturation,
            color_arc_lightness,
            color_arc_saturation_vector_compare,
            color_arc_lightness_vector_compare,
            color_arc_saturation_raster_compare,
            color_arc_lightness_raster_compare,
            color_arc_saturation_xsmall,
            color_arc_lightness_xsmall,
            color_arc_saturation_small,
            color_arc_lightness_small,
            color_arc_saturation_medium,
            color_arc_lightness_medium,
            color_arc_saturation_large,
            color_arc_lightness_large,
            color_arc_saturation_thickness_xsmall,
            color_arc_lightness_thickness_xsmall,
            color_arc_saturation_thickness_small,
            color_arc_lightness_thickness_small,
            color_arc_saturation_thickness_medium,
            color_arc_lightness_thickness_medium,
            color_arc_saturation_thickness_large,
            color_arc_lightness_thickness_large,
            color_arc_hue_270,
            color_arc_saturation_270,
            color_arc_lightness_270,
            pair_readouts: [ArcPairReadout::DEFAULT; ARC_PAIR_READOUT_COUNT],
            single_readouts: [ArcPairReadout::DEFAULT; ARC_SINGLE_READOUT_COUNT],
            _subscriptions,
        };

        this.pair_readouts[ArcPairReadoutKey::CompareVector.index()] = ArcPairReadout {
            event_name: "Release",
            event_value: this.color_arc_saturation_vector_compare.read(cx).value,
        };
        this.pair_readouts[ArcPairReadoutKey::CompareRaster.index()] = ArcPairReadout {
            event_name: "Release",
            event_value: this.color_arc_saturation_raster_compare.read(cx).value,
        };
        this.pair_readouts[ArcPairReadoutKey::SizeXSmall.index()] = ArcPairReadout {
            event_name: "Release",
            event_value: this.color_arc_saturation_xsmall.read(cx).value,
        };
        this.pair_readouts[ArcPairReadoutKey::SizeSmall.index()] = ArcPairReadout {
            event_name: "Release",
            event_value: this.color_arc_saturation_small.read(cx).value,
        };
        this.pair_readouts[ArcPairReadoutKey::SizeMedium.index()] = ArcPairReadout {
            event_name: "Release",
            event_value: this.color_arc_saturation_medium.read(cx).value,
        };
        this.pair_readouts[ArcPairReadoutKey::SizeLarge.index()] = ArcPairReadout {
            event_name: "Release",
            event_value: this.color_arc_saturation_large.read(cx).value,
        };
        this.pair_readouts[ArcPairReadoutKey::ThicknessXSmall.index()] = ArcPairReadout {
            event_name: "Release",
            event_value: this.color_arc_saturation_thickness_xsmall.read(cx).value,
        };
        this.pair_readouts[ArcPairReadoutKey::ThicknessSmall.index()] = ArcPairReadout {
            event_name: "Release",
            event_value: this.color_arc_saturation_thickness_small.read(cx).value,
        };
        this.pair_readouts[ArcPairReadoutKey::ThicknessMedium.index()] = ArcPairReadout {
            event_name: "Release",
            event_value: this.color_arc_saturation_thickness_medium.read(cx).value,
        };
        this.pair_readouts[ArcPairReadoutKey::ThicknessLarge.index()] = ArcPairReadout {
            event_name: "Release",
            event_value: this.color_arc_saturation_thickness_large.read(cx).value,
        };
        this.single_readouts[ArcSingleReadoutKey::Hue270.index()] = ArcPairReadout {
            event_name: "Release",
            event_value: this.color_arc_hue_270.read(cx).value,
        };
        this.single_readouts[ArcSingleReadoutKey::Saturation270.index()] = ArcPairReadout {
            event_name: "Release",
            event_value: this.color_arc_saturation_270.read(cx).value,
        };
        this.single_readouts[ArcSingleReadoutKey::Lightness270.index()] = ArcPairReadout {
            event_name: "Release",
            event_value: this.color_arc_lightness_270.read(cx).value,
        };

        this.prewarm_raster_arc_caches(cx);
        this
    }

    fn build_saturation_arc(
        cx: &mut Context<Self>,
        id: &'static str,
        size: gpui_component::Size,
    ) -> Entity<ColorArcState> {
        Self::build_saturation_arc_with_thickness(cx, id, size, gpui_component::Size::Small)
    }

    fn build_saturation_arc_with_renderer(
        cx: &mut Context<Self>,
        id: &'static str,
        size: gpui_component::Size,
        renderer: ColorArcRenderer,
    ) -> Entity<ColorArcState> {
        cx.new(move |cx| {
            ColorArcState::saturation_with_renderer(id, 0.72, 195.0, 1.0, renderer, cx)
                .with_size(size)
                .start_degrees(90.0 + ARC_ROTATION_DEGREES + ARC_GAP_DEGREES * 0.5)
                .sweep_degrees(ARC_SWEEP_DEGREES)
                .arc_thickness_size(gpui_component::Size::Small)
                .thumb_size(14.0)
        })
    }

    fn build_saturation_arc_with_thickness(
        cx: &mut Context<Self>,
        id: &'static str,
        size: gpui_component::Size,
        thickness_size: gpui_component::Size,
    ) -> Entity<ColorArcState> {
        cx.new(move |cx| {
            ColorArcState::saturation_with_renderer(
                id,
                0.72,
                195.0,
                1.0,
                ColorArcRenderer::Raster,
                cx,
            )
            .with_size(size)
            .start_degrees(90.0 + ARC_ROTATION_DEGREES + ARC_GAP_DEGREES * 0.5)
            .sweep_degrees(ARC_SWEEP_DEGREES)
            .arc_thickness_size(thickness_size)
            .thumb_size(match thickness_size {
                gpui_component::Size::XSmall => 12.0,
                gpui_component::Size::Small => 14.0,
                gpui_component::Size::Medium => 16.0,
                gpui_component::Size::Large => 20.0,
                gpui_component::Size::Size(px) => px.as_f32(),
            })
        })
    }

    fn build_lightness_arc(
        cx: &mut Context<Self>,
        id: &'static str,
        size: gpui_component::Size,
    ) -> Entity<ColorArcState> {
        Self::build_lightness_arc_with_thickness(cx, id, size, gpui_component::Size::Small)
    }

    fn build_lightness_arc_with_renderer(
        cx: &mut Context<Self>,
        id: &'static str,
        size: gpui_component::Size,
        renderer: ColorArcRenderer,
    ) -> Entity<ColorArcState> {
        cx.new(move |cx| {
            ColorArcState::lightness_with_renderer(id, 0.45, 195.0, 0.9, renderer, cx)
                .with_size(size)
                .start_degrees(270.0 + ARC_ROTATION_DEGREES + ARC_GAP_DEGREES * 0.5)
                .sweep_degrees(ARC_SWEEP_DEGREES)
                .arc_thickness_size(gpui_component::Size::Small)
                .thumb_size(14.0)
        })
    }

    fn build_lightness_arc_with_thickness(
        cx: &mut Context<Self>,
        id: &'static str,
        size: gpui_component::Size,
        thickness_size: gpui_component::Size,
    ) -> Entity<ColorArcState> {
        cx.new(move |cx| {
            ColorArcState::lightness_with_renderer(
                id,
                0.45,
                195.0,
                0.9,
                ColorArcRenderer::Raster,
                cx,
            )
            .with_size(size)
            .start_degrees(270.0 + ARC_ROTATION_DEGREES + ARC_GAP_DEGREES * 0.5)
            .sweep_degrees(ARC_SWEEP_DEGREES)
            .arc_thickness_size(thickness_size)
            .thumb_size(match thickness_size {
                gpui_component::Size::XSmall => 12.0,
                gpui_component::Size::Small => 14.0,
                gpui_component::Size::Medium => 16.0,
                gpui_component::Size::Large => 20.0,
                gpui_component::Size::Size(px) => px.as_f32(),
            })
        })
    }

    fn build_hue_arc_at_270(cx: &mut Context<Self>, id: &'static str) -> Entity<ColorArcState> {
        cx.new(move |cx| {
            ColorArcState::hue_with_renderer(id, 220.0, 0.9, 0.5, ColorArcRenderer::Raster, cx)
                .with_size(gpui_component::Size::Medium)
                .start_degrees(270.0)
                .sweep_degrees(ARC_SWEEP_270_DEGREES)
                .arc_thickness_size(gpui_component::Size::Small)
                .thumb_size(16.0)
        })
    }

    fn build_saturation_arc_at_270(
        cx: &mut Context<Self>,
        id: &'static str,
    ) -> Entity<ColorArcState> {
        cx.new(move |cx| {
            ColorArcState::saturation_with_renderer(
                id,
                0.72,
                195.0,
                1.0,
                ColorArcRenderer::Raster,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
            .start_degrees(270.0)
            .sweep_degrees(ARC_SWEEP_270_DEGREES)
            .arc_thickness_size(gpui_component::Size::Small)
            .thumb_size(16.0)
        })
    }

    fn build_lightness_arc_at_270(
        cx: &mut Context<Self>,
        id: &'static str,
    ) -> Entity<ColorArcState> {
        cx.new(move |cx| {
            ColorArcState::lightness_with_renderer(
                id,
                0.45,
                195.0,
                0.9,
                ColorArcRenderer::Raster,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
            .start_degrees(270.0)
            .sweep_degrees(ARC_SWEEP_270_DEGREES)
            .arc_thickness_size(gpui_component::Size::Small)
            .thumb_size(16.0)
        })
    }
}

impl Render for StoryColorArcTab {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_6()
            .child(
                section("Implementation Compare").max_w_full().child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .items_start()
                        .gap_6()
                        .child(render_labeled_arc_pair(
                            "Vector",
                            &self.color_arc_saturation_vector_compare,
                            &self.color_arc_lightness_vector_compare,
                            220.0,
                            ARC_HORIZONTAL_OFFSET_PX,
                            self.pair_readout(ArcPairReadoutKey::CompareVector),
                        ))
                        .child(render_labeled_arc_pair(
                            "Raster",
                            &self.color_arc_saturation_raster_compare,
                            &self.color_arc_lightness_raster_compare,
                            220.0,
                            ARC_HORIZONTAL_OFFSET_PX,
                            self.pair_readout(ArcPairReadoutKey::CompareRaster),
                        )),
                ),
            )
            .child(
                section("Arc Pair Sizes").max_w_full().child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .items_start()
                        .gap_6()
                        .child(render_labeled_arc_pair(
                            "XSmall",
                            &self.color_arc_saturation_xsmall,
                            &self.color_arc_lightness_xsmall,
                            140.0,
                            ARC_HORIZONTAL_OFFSET_PX,
                            self.pair_readout(ArcPairReadoutKey::SizeXSmall),
                        ))
                        .child(render_labeled_arc_pair(
                            "Small",
                            &self.color_arc_saturation_small,
                            &self.color_arc_lightness_small,
                            180.0,
                            ARC_HORIZONTAL_OFFSET_PX,
                            self.pair_readout(ArcPairReadoutKey::SizeSmall),
                        ))
                        .child(render_labeled_arc_pair(
                            "Medium",
                            &self.color_arc_saturation_medium,
                            &self.color_arc_lightness_medium,
                            220.0,
                            ARC_HORIZONTAL_OFFSET_PX,
                            self.pair_readout(ArcPairReadoutKey::SizeMedium),
                        ))
                        .child(render_labeled_arc_pair(
                            "Large",
                            &self.color_arc_saturation_large,
                            &self.color_arc_lightness_large,
                            280.0,
                            ARC_HORIZONTAL_OFFSET_PX,
                            self.pair_readout(ArcPairReadoutKey::SizeLarge),
                        )),
                ),
            )
            .child(
                section("Arc Pair Thickness Sizes").max_w_full().child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .items_start()
                        .gap_8()
                        .child(render_labeled_arc_pair(
                            "XSmall",
                            &self.color_arc_saturation_thickness_xsmall,
                            &self.color_arc_lightness_thickness_xsmall,
                            220.0,
                            ARC_HORIZONTAL_OFFSET_THICKNESS_PX,
                            self.pair_readout(ArcPairReadoutKey::ThicknessXSmall),
                        ))
                        .child(render_labeled_arc_pair(
                            "Small",
                            &self.color_arc_saturation_thickness_small,
                            &self.color_arc_lightness_thickness_small,
                            220.0,
                            ARC_HORIZONTAL_OFFSET_THICKNESS_PX,
                            self.pair_readout(ArcPairReadoutKey::ThicknessSmall),
                        ))
                        .child(render_labeled_arc_pair(
                            "Medium",
                            &self.color_arc_saturation_thickness_medium,
                            &self.color_arc_lightness_thickness_medium,
                            220.0,
                            ARC_HORIZONTAL_OFFSET_THICKNESS_PX,
                            self.pair_readout(ArcPairReadoutKey::ThicknessMedium),
                        ))
                        .child(render_labeled_arc_pair(
                            "Large",
                            &self.color_arc_saturation_thickness_large,
                            &self.color_arc_lightness_thickness_large,
                            220.0,
                            ARC_HORIZONTAL_OFFSET_THICKNESS_PX,
                            self.pair_readout(ArcPairReadoutKey::ThicknessLarge),
                        )),
                ),
            )
            .child(
                section("Single Arc Delegates @ 270°").max_w_full().child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .items_start()
                        .gap_6()
                        .child(render_labeled_single_arc(
                            "Hue",
                            &self.color_arc_hue_270,
                            220.0,
                            self.single_readout(ArcSingleReadoutKey::Hue270),
                        ))
                        .child(render_labeled_single_arc(
                            "Saturation",
                            &self.color_arc_saturation_270,
                            220.0,
                            self.single_readout(ArcSingleReadoutKey::Saturation270),
                        ))
                        .child(render_labeled_single_arc(
                            "Lightness",
                            &self.color_arc_lightness_270,
                            220.0,
                            self.single_readout(ArcSingleReadoutKey::Lightness270),
                        )),
                ),
            )
    }
}

fn render_arc_pair(
    saturation_arc: &Entity<ColorArcState>,
    lightness_arc: &Entity<ColorArcState>,
    canvas_size: f32,
    horizontal_offset: f32,
    readout: ArcPairReadout,
) -> impl IntoElement {
    let pair_width = canvas_size + horizontal_offset * 2.0;
    div()
        .relative()
        .w(px(pair_width))
        .h(px(canvas_size))
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .size(px(canvas_size))
                .child(ColorArc::new(saturation_arc)),
        )
        .child(
            div()
                .absolute()
                .left(px(horizontal_offset * 2.0))
                .top_0()
                .size(px(canvas_size))
                .child(ColorArc::new(lightness_arc)),
        )
        .child(
            v_flex()
                .absolute()
                .left_1_2()
                .top_1_2()
                .ml(px(-42.0))
                .mt(px(-16.0))
                .w(px(84.0))
                .h(px(32.0))
                .rounded_sm()
                .bg(black().opacity(0.45))
                .text_xs()
                .text_color(white())
                .items_center()
                .justify_center()
                .child(div().child(readout.event_name))
                .child(div().text_sm().child(format_arc_value(readout.event_value))),
        )
}

fn render_labeled_arc_pair(
    label: &'static str,
    saturation_arc: &Entity<ColorArcState>,
    lightness_arc: &Entity<ColorArcState>,
    canvas_size: f32,
    horizontal_offset: f32,
    readout: ArcPairReadout,
) -> impl IntoElement {
    v_flex()
        .items_center()
        .gap_2()
        .child(div().text_xs().child(label))
        .child(render_arc_pair(
            saturation_arc,
            lightness_arc,
            canvas_size,
            horizontal_offset,
            readout,
        ))
}

fn format_arc_value(value: f32) -> String {
    if value.abs() >= 10.0 {
        format!("{:.0}", value)
    } else {
        format!("{:.2}", value)
    }
}

fn render_labeled_single_arc(
    label: &'static str,
    arc: &Entity<ColorArcState>,
    canvas_size: f32,
    readout: ArcPairReadout,
) -> impl IntoElement {
    v_flex()
        .items_center()
        .gap_2()
        .child(div().text_xs().child(label))
        .child(
            div()
                .relative()
                .size(px(canvas_size))
                .child(ColorArc::new(arc))
                .child(
                    v_flex()
                        .absolute()
                        .left_1_2()
                        .top_1_2()
                        .ml(px(-42.0))
                        .mt(px(-16.0))
                        .w(px(84.0))
                        .h(px(32.0))
                        .rounded_sm()
                        .bg(black().opacity(0.45))
                        .text_xs()
                        .text_color(white())
                        .items_center()
                        .justify_center()
                        .child(div().child(readout.event_name))
                        .child(div().text_sm().child(format_arc_value(readout.event_value))),
                ),
        )
}
