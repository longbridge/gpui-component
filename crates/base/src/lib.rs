//! Behavior and infrastructure foundations for GPUI applications.
//!
//! Primitives deliberately avoid presentation styles. Layout, positioning,
//! colors, sizing, and motion belong to applications or the
//! `gpui-component` façade.

pub mod actions;
pub mod animation;
mod accordion;
mod auto_scroll;
mod button;
mod checkbox;
mod element_ext;
mod event;
mod focus_trap;
mod geometry;
mod history;
mod index_path;
mod link;
pub mod motion;
mod radio;
mod radio_group;
mod scrollbar;
pub mod slider_state;
mod state_style;
mod styled;
mod switch;
mod theme;
pub mod theme_tokens;
mod toggle;
mod toggle_group;
mod virtual_list;

pub use auto_scroll::AutoScroll;
pub use accordion::{Accordion, AccordionHeader, AccordionItem, AccordionPanel, AccordionTrigger};
pub use button::{Button, ButtonStyles};
pub use checkbox::{
    Checkbox, CheckboxIndicator, CheckboxIndicatorStyles, CheckboxState, CheckboxStyles,
};
pub use element_ext::ElementExt;
pub use event::InteractiveElementExt;
pub use focus_trap::FocusTrapElement;
#[doc(hidden)]
pub use focus_trap::active_focus_trap;
pub use geometry::*;
pub use history::{History, HistoryItem};
pub use index_path::IndexPath;
pub use link::{Link, LinkStyles};
pub use motion::{Interpolate, Transition, TransitionId, transition};
pub use radio::{Radio, RadioStyles};
pub use radio_group::RadioGroup;
pub use scrollbar::{
    Scrollbar, ScrollbarAxis, ScrollbarHandle, ScrollbarMode, ScrollbarStyles, ScrollbarThumbStyle,
    ScrollbarTrackStyle,
};
pub use state_style::StateStyle;
#[cfg(any(feature = "inspector", debug_assertions))]
pub use styled::styled_ext_reflection_methods;
pub use styled::{FocusableExt, StyledExt, box_shadow, h_flex, v_flex};
pub use switch::{
    Switch, SwitchStyles, SwitchThumb, SwitchThumbStyles, SwitchTrack, SwitchTrackStyles,
};
pub use theme::{ScrollbarTheme, Theme};
pub use theme_tokens::{
    ColorTokens, RadiusTokens, SemanticThemeTokens, ShadowTokens, SpacingTokens, TextStyleToken,
    TypographyTokens,
};
pub use toggle::{Toggle, ToggleStyles};
pub use toggle_group::ToggleGroup;
#[doc(hidden)]
pub use virtual_list::virtual_list;
pub use virtual_list::{VirtualList, VirtualListScrollHandle, h_virtual_list, v_virtual_list};

use gpui::App;

/// Initializes global infrastructure owned by the base layer.
pub fn init(cx: &mut App) {
    let _ = Theme::global_mut(cx);
    focus_trap::init(cx);
}
