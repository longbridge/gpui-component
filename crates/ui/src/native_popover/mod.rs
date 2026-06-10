//! A popover rendered natively by the operating system.
//!
//! Unlike [`crate::popover::Popover`], which is drawn by GPUI inside the window
//! (and clipped to it), [`NativePopover`] is a real OS popover. On macOS it is
//! an `NSPopover`: it has the system arrow, corner radius, vibrant (frosted)
//! background, show/dismiss animation, and transient behavior (clicking outside
//! dismisses it) — and it can extend beyond the window bounds.
//!
//! The trade-off versus [`crate::popover::Popover`]: because the content is
//! rendered by AppKit (not GPUI), it is limited to native controls. A
//! [`NativePopover`] is therefore described declaratively — a title and a set of
//! buttons, each carrying a GPUI [`Action`] dispatched via
//! [`Window::dispatch_action`] when clicked.
//!
//! ```ignore
//! use gpui_component::native_popover::NativePopover;
//!
//! NativePopover::new()
//!     .title("Delete this item?")
//!     .button("Delete", Box::new(Delete))
//!     .button("Cancel", Box::new(Cancel))
//!     .show(trigger_bounds, window, cx);
//! ```
//!
//! Platform support: macOS (native `NSPopover`). Other platforms currently do
//! nothing; a GPUI [`crate::popover::Popover`] fallback is planned.

use gpui::{Action, App, Bounds, Entity, Pixels, Render, SharedString, Size, Window};

#[cfg(target_os = "macos")]
mod macos;

/// SPIKE (macOS only): show arbitrary GPUI content inside a native `NSPopover`
/// by reparenting a hidden GPUI window's view into the popover. Verifies whether
/// "native shell + arbitrary GPUI content" is viable. No-op off macOS.
///
/// `anchor` is the trigger's window-relative bounds; `size` is the content size.
pub fn show_view<V: 'static + Render>(
    anchor: Bounds<Pixels>,
    size: Size<Pixels>,
    window: &mut Window,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V> + 'static,
) {
    #[cfg(target_os = "macos")]
    macos::show_view(anchor, size, window, cx, build);

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (anchor, size, window, cx, build);
    }
}

/// A single actionable button in a [`NativePopover`].
struct NativePopoverButton {
    label: SharedString,
    /// Action dispatched when the button is clicked.
    action: Box<dyn Action>,
}

/// A popover rendered by the operating system.
///
/// Build it with [`NativePopover::title`] / [`NativePopover::button`], then call
/// [`NativePopover::show`] anchored to a trigger's bounds.
#[derive(Default)]
pub struct NativePopover {
    title: Option<SharedString>,
    buttons: Vec<NativePopoverButton>,
}

impl NativePopover {
    /// Create an empty native popover.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the (single-line) title shown at the top of the popover.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Append a button that dispatches `action` when clicked.
    pub fn button(mut self, label: impl Into<SharedString>, action: Box<dyn Action>) -> Self {
        self.buttons.push(NativePopoverButton {
            label: label.into(),
            action,
        });
        self
    }

    /// Whether the popover has no title and no buttons.
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.buttons.is_empty()
    }

    /// Show the popover anchored to `anchor` (the trigger's window-relative
    /// bounds, in logical pixels). On macOS the system positions it adjacent to
    /// that rect with an arrow, and dismisses it when the user clicks outside.
    pub fn show(self, anchor: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        if self.is_empty() {
            return;
        }

        #[cfg(target_os = "macos")]
        macos::show(self.title, self.buttons, anchor, window, cx);

        #[cfg(not(target_os = "macos"))]
        {
            // TODO: fall back to a GPUI-drawn `Popover` on non-macOS platforms.
            let _ = (anchor, window, cx);
        }
    }
}
