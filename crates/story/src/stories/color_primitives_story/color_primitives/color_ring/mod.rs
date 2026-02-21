pub(crate) mod common;
pub mod delegates;
pub mod factory;
pub mod raster;
pub mod ring;

pub use delegates::HueRingDelegate;
#[allow(unused_imports)]
pub use delegates::LightnessRingDelegate;
#[allow(unused_imports)]
pub use delegates::SaturationRingDelegate;
#[allow(unused_imports)]
pub use raster::{ColorRingRasterEvent, ColorRingRasterState, ColorRingRenderer};
#[allow(unused_imports)]
pub use ring::{
    ColorRing, ColorRingDelegate, ColorRingEvent, ColorRingMouseBehavior, ColorRingMouseContext,
    ColorRingMousePreset, ColorRingState,
};

use gpui::{App, Hsla};
use gpui_component::ActiveTheme as _;

fn theme_ring_border_color(cx: &App) -> Hsla {
    cx.theme().border
}

pub(crate) const COLOR_RING_BORDER_COLOR: fn(&App) -> Hsla = theme_ring_border_color;
