//! Behavior and infrastructure foundations for GPUI applications.
//!
//! Primitives deliberately avoid presentation styles. Layout, positioning,
//! colors, sizing, and motion belong to applications or the
//! `gpui-component` façade.

pub mod animation;
mod button;
mod checkbox;
mod event;
mod focus_trap;
mod geometry;
mod link;
pub mod motion;
mod radio;
mod radio_group;
pub mod slider_state;
mod state_style;
mod styled;
mod switch;
pub mod theme_tokens;
mod toggle;
mod toggle_group;

pub use button::{Button, ButtonStyles};
pub use checkbox::{
    Checkbox, CheckboxIndicator, CheckboxIndicatorStyles, CheckboxState, CheckboxStyles,
};
pub use event::InteractiveElementExt;
pub use focus_trap::FocusTrapElement;
#[doc(hidden)]
pub use focus_trap::active_focus_trap;
pub use geometry::*;
pub use link::{Link, LinkStyles};
pub use motion::{Interpolate, Transition, TransitionId, transition};
pub use radio::{Radio, RadioStyles};
pub use radio_group::RadioGroup;
pub use state_style::StateStyle;
pub use styled::{FocusableExt, StyledExt, StyledTheme, box_shadow, h_flex, v_flex};
#[cfg(any(feature = "inspector", debug_assertions))]
pub use styled::styled_ext_reflection_methods;
pub use switch::{Switch, SwitchStyles, SwitchThumb, SwitchThumbStyles};
pub use theme_tokens::{
    ColorTokens, RadiusTokens, SemanticThemeTokens, ShadowTokens, SpacingTokens, TextStyleToken,
    TypographyTokens,
};
pub use toggle::{Toggle, ToggleStyles};
pub use toggle_group::ToggleGroup;

use gpui::App;

/// Initializes global infrastructure owned by the base layer.
pub fn init(cx: &mut App) {
    focus_trap::init(cx);
}
