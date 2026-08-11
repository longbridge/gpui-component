//! Unstyled behavior and infrastructure foundations for GPUI applications.
//!
//! This crate deliberately avoids application-specific visual language. Styled
//! component implementations belong to applications or registry templates.

pub mod animation;
mod button;
mod checkbox;
mod event;
mod focus_trap;
mod geometry;
mod link;
mod radio;
mod slider;
pub mod slider_state;
mod switch;
pub mod theme_tokens;
mod toggle;

pub use button::Button;
pub use checkbox::{Checkbox, CheckboxState};
pub use event::InteractiveElementExt;
pub use focus_trap::FocusTrapElement;
#[doc(hidden)]
pub use focus_trap::active_focus_trap;
pub use geometry::*;
pub use link::Link;
pub use radio::{Radio, RadioGroupState};
pub use slider::Slider;
pub use switch::Switch;
pub use theme_tokens::{
    ColorTokens, RadiusTokens, SemanticThemeTokens, ShadowTokens, SpacingTokens, TextStyleToken,
    TypographyTokens,
};
pub use toggle::Toggle;

use gpui::App;

/// Initializes global infrastructure owned by the base layer.
pub fn init(cx: &mut App) {
    focus_trap::init(cx);
}
