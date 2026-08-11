//! Behavior and infrastructure foundations for GPUI applications.
//!
//! Primitives deliberately avoid presentation styles. Layout, positioning,
//! colors, sizing, and motion belong to applications or the
//! `gpui-component` façade.

mod accordion;
pub mod actions;
mod alert_dialog;
pub mod animation;
#[doc(hidden)]
pub mod async_util;
mod auto_scroll;
mod avatar;
mod button;
mod checkbox;
mod collapsible;
pub mod component_traits;
mod dialog;
mod element_ext;
mod event;
mod focus_trap;
mod geometry;
mod global_state;
mod history;
mod hover_card;
mod index_path;
mod link;
mod list_settings;
#[cfg(all(target_os = "macos", not(test)))]
mod macos_accessibility;
mod measure;
pub mod motion;
mod popover;
mod popup;
mod progress;
mod radio;
mod radio_group;
mod resizable;
mod scrollbar;
mod sheet;
pub mod slider;
mod state_style;
mod styled;
mod switch;
mod tabs;
mod theme;
pub mod theme_tokens;
mod toggle;
mod toggle_group;
mod tooltip;
mod tree;
mod virtual_list;

pub use accordion::{Accordion, AccordionHeader, AccordionItem, AccordionPanel, AccordionTrigger};
pub use alert_dialog::AlertDialog;
pub use auto_scroll::AutoScroll;
pub use avatar::{Avatar, AvatarFallback, AvatarImage};
pub use button::{Button, ButtonStyles};
pub use checkbox::{
    Checkbox, CheckboxIndicator, CheckboxIndicatorStyles, CheckboxState, CheckboxStyles,
};
pub use collapsible::Collapsible;
pub use component_traits::{Disableable, Selectable};
pub use dialog::{
    AcceptDialog, CancelDialog, ConfirmDialog, Dialog, DialogCallbacks, DialogClose,
    DialogDescription, DialogPlacement, DialogTitle, DialogTrigger, DismissDialog,
};
pub use element_ext::ElementExt;
pub use event::InteractiveElementExt;
pub use focus_trap::FocusTrapElement;
#[doc(hidden)]
pub use focus_trap::active_focus_trap;
pub use geometry::*;
pub use global_state::GlobalState;
pub use history::{History, HistoryItem};
pub use hover_card::{HoverCard, HoverCardState};
pub use index_path::IndexPath;
pub use link::{Link, LinkStyles};
pub use list_settings::ListSettings;
#[cfg(all(target_os = "macos", not(test)))]
#[doc(hidden)]
pub use macos_accessibility::install_window_hit_test_forwarder;
#[doc(hidden)]
pub use measure::measurement_enabled;
pub use measure::{Measure, measure, measure_if};
pub use motion::{Interpolate, Transition, TransitionId, transition};
pub use popover::{Popover, PopoverState};
pub use popup::Popup;
pub use progress::{Progress, ProgressIndicator, ProgressTrack};
pub use radio::{Radio, RadioStyles};
pub use radio_group::RadioGroup;
#[doc(hidden)]
pub use resizable::{PANEL_MIN_SIZE, resize_handle};
pub use resizable::{
    ResizablePanel, ResizablePanelEvent, ResizablePanelGroup, ResizableState, h_resizable,
    resizable_panel, v_resizable,
};
pub use scrollbar::{
    Scrollbar, ScrollbarAxis, ScrollbarHandle, ScrollbarMode, ScrollbarStyles, ScrollbarThumbStyle,
    ScrollbarTrackStyle,
};
pub use sheet::{CancelSheet, Sheet};
pub use slider::{Slider, SliderIndicator, SliderThumb, SliderTrack};
pub use state_style::StateStyle;
#[cfg(any(feature = "inspector", debug_assertions))]
pub use styled::styled_ext_reflection_methods;
pub use styled::{FocusableExt, StyledExt, box_shadow, h_flex, v_flex};
pub use switch::{
    Switch, SwitchStyles, SwitchThumb, SwitchThumbStyles, SwitchTrack, SwitchTrackStyles,
};
pub use tabs::{Tab, TabStyles, Tabs};
pub use theme::{ResizableTheme, ScrollbarTheme, Theme};
pub use theme_tokens::{
    ColorTokens, RadiusTokens, SemanticThemeTokens, ShadowTokens, SpacingTokens, TextStyleToken,
    TypographyTokens,
};
pub use toggle::{Toggle, ToggleStyles};
pub use toggle_group::ToggleGroup;
pub use tooltip::{Tooltip, TooltipOverlay, TooltipPositioner, TooltipRequest, TooltipTransition};
pub use tree::{Tree, TreeEntry, TreeEntryState, TreeEvent, TreeItem, TreeState};
#[doc(hidden)]
pub use tree::{init as init_tree, key_context as tree_key_context};
#[doc(hidden)]
pub use virtual_list::virtual_list;
pub use virtual_list::{VirtualList, VirtualListScrollHandle, h_virtual_list, v_virtual_list};

use gpui::App;

/// Initializes global infrastructure owned by the base layer.
pub fn init(cx: &mut App) {
    let _ = Theme::global_mut(cx);
    GlobalState::init(cx);
    dialog::init(cx);
    focus_trap::init(cx);
    sheet::init(cx);
    popover::init(cx);
    tree::init(cx);
}
