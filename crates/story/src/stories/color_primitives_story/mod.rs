pub mod color_primitives;
pub mod color_spec {
    pub use super::color_primitives::color_slider::color_spec::*;
}
pub mod delegates {
    pub use super::color_primitives::color_slider::delegates::*;
}
pub mod checkerboard {
    pub use super::color_primitives::color_slider::checkerboard::*;
}
pub mod color_thumb {
    pub use super::color_primitives::color_slider::color_thumb::*;
}
pub use color_primitives::color_arc;
pub use color_primitives::color_field;
pub mod color_readout;
pub use color_primitives::color_ring;
pub mod compositions;
pub use color_primitives::color_slider;

pub use color_primitives::mouse_behavior;

mod color_primitives_story;

pub use compositions::color_control_channels;
pub use compositions::color_plane_controls;

pub mod story_color_arc_tab;
pub mod story_color_field_tab;
pub mod story_color_ring_tab;
pub mod story_color_slider_tab;
pub mod story_compositions_tab;

pub use color_primitives_story::*;
