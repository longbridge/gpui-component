use super::arc::ColorArcState;
use super::delegates::{HueArcDelegate, LightnessArcDelegate, SaturationArcDelegate};
use super::raster::{ColorArcRenderer, RasterArcDelegate};
use gpui::{Context, SharedString};

impl ColorArcState {
    #[allow(dead_code)] // Public constructor for renderer selection parity.
    pub fn hue_with_renderer<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        saturation: f32,
        lightness: f32,
        renderer: ColorArcRenderer,
        cx: &mut Context<V>,
    ) -> Self {
        match renderer {
            ColorArcRenderer::Vector => Self::new(
                id,
                value,
                Box::new(HueArcDelegate {
                    saturation,
                    lightness,
                }),
                cx,
            )
            .max(360.0),
            ColorArcRenderer::Raster => Self::hue_raster(id, value, saturation, lightness, cx),
        }
    }

    pub fn saturation_with_renderer<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        hue: f32,
        hsv_value: f32,
        renderer: ColorArcRenderer,
        cx: &mut Context<V>,
    ) -> Self {
        match renderer {
            ColorArcRenderer::Vector => {
                Self::new(id, value, Box::new(SaturationArcDelegate { hue, hsv_value }), cx)
            }
            ColorArcRenderer::Raster => Self::saturation_raster(id, value, hue, hsv_value, cx),
        }
    }

    pub fn lightness_with_renderer<V: 'static>(
        id: impl Into<SharedString>,
        value: f32,
        hue: f32,
        saturation: f32,
        renderer: ColorArcRenderer,
        cx: &mut Context<V>,
    ) -> Self {
        match renderer {
            ColorArcRenderer::Vector => {
                Self::new(id, value, Box::new(LightnessArcDelegate { hue, saturation }), cx)
            }
            ColorArcRenderer::Raster => Self::lightness_raster(id, value, hue, saturation, cx),
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
            Box::new(RasterArcDelegate::hue(saturation, lightness)),
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
            Box::new(RasterArcDelegate::saturation(hue, hsv_value)),
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
            Box::new(RasterArcDelegate::lightness(hue, saturation)),
            cx,
        )
    }
}
