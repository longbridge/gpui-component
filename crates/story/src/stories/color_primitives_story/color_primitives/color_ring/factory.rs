use super::delegates::{HueRingDelegate, LightnessRingDelegate, SaturationRingDelegate};
use super::raster::{ColorRingRenderer, RasterRingDelegate};
use super::ring::ColorRingState;
use gpui::{Context, SharedString};

impl ColorRingState {
    pub fn hue_with_renderer<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        saturation: f32,
        lightness: f32,
        renderer: ColorRingRenderer,
        cx: &mut Context<V>,
    ) -> Self {
        match renderer {
            ColorRingRenderer::Vector => Self::new(
                id,
                value,
                Box::new(HueRingDelegate {
                    saturation,
                    lightness,
                }),
                cx,
            )
            .max(360.0),
            ColorRingRenderer::Raster => Self::hue_raster(id, value, saturation, lightness, cx),
        }
    }

    pub fn saturation_with_renderer<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        hue: f32,
        hsv_value: f32,
        renderer: ColorRingRenderer,
        cx: &mut Context<V>,
    ) -> Self {
        match renderer {
            ColorRingRenderer::Vector => {
                Self::new(id, value, Box::new(SaturationRingDelegate { hue, hsv_value }), cx)
            }
            ColorRingRenderer::Raster => Self::saturation_raster(id, value, hue, hsv_value, cx),
        }
    }

    pub fn lightness_with_renderer<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        hue: f32,
        saturation: f32,
        renderer: ColorRingRenderer,
        cx: &mut Context<V>,
    ) -> Self {
        match renderer {
            ColorRingRenderer::Vector => {
                Self::new(id, value, Box::new(LightnessRingDelegate { hue, saturation }), cx)
            }
            ColorRingRenderer::Raster => Self::lightness_raster(id, value, hue, saturation, cx),
        }
    }

    pub fn hue_raster<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        saturation: f32,
        lightness: f32,
        cx: &mut Context<V>,
    ) -> Self {
        Self::new(
            id,
            value,
            Box::new(RasterRingDelegate::hue(saturation, lightness)),
            cx,
        )
        .max(360.0)
    }

    pub fn saturation_raster<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        hue: f32,
        hsv_value: f32,
        cx: &mut Context<V>,
    ) -> Self {
        Self::new(
            id,
            value,
            Box::new(RasterRingDelegate::saturation(hue, hsv_value)),
            cx,
        )
    }

    pub fn lightness_raster<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        hue: f32,
        saturation: f32,
        cx: &mut Context<V>,
    ) -> Self {
        Self::new(
            id,
            value,
            Box::new(RasterRingDelegate::lightness(hue, saturation)),
            cx,
        )
    }
}
