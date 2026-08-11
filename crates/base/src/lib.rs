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
pub mod motion;
mod radio;
mod slider;
pub mod slider_state;
mod state_style;
mod switch;
pub mod theme_tokens;
mod toggle;

pub use button::{Button, ButtonStyles};
pub use checkbox::{Checkbox, CheckboxState, CheckboxStyles};
pub use event::InteractiveElementExt;
pub use focus_trap::FocusTrapElement;
#[doc(hidden)]
pub use focus_trap::active_focus_trap;
pub use geometry::*;
pub use link::{Link, LinkStyles};
pub use motion::{Interpolate, Transition, TransitionId, transition};
pub use radio::{Radio, RadioGroupState, RadioStyles};
pub use slider::{Slider, SliderStyles};
pub use state_style::StateStyle;
pub use switch::{Switch, SwitchStyles};
pub use theme_tokens::{
    ColorTokens, RadiusTokens, SemanticThemeTokens, ShadowTokens, SpacingTokens, TextStyleToken,
    TypographyTokens,
};
pub use toggle::{Toggle, ToggleStyles};

use gpui::App;

/// Initializes global infrastructure owned by the base layer.
pub fn init(cx: &mut App) {
    focus_trap::init(cx);
}
