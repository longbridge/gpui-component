//! Liquid Glass, embeds a native `NSGlassEffectView` (macOS 26+) behind the
//! window content.
//!
//! This mirrors how GPUI injects an `NSVisualEffectView` for
//! [`WindowBackgroundAppearance::Blurred`](gpui::WindowBackgroundAppearance):
//! the glass view is added to the window's `contentView`, positioned below
//! the GPUI rendered view, with an autoresizing mask to follow window resize.

use gpui::{Window, WindowBackgroundAppearance};
use objc2::ffi::{NSInteger, NSUInteger};
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2_foundation::NSRect;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// `NSViewWidthSizable | NSViewHeightSizable`
const AUTORESIZE_WIDTH_HEIGHT: NSUInteger = (1 << 1) | (1 << 4);
/// `NSWindowBelow`
const NS_WINDOW_BELOW: NSInteger = -1;

/// Returns the `contentView` of the window's native `NSWindow`.
fn content_view(window: &Window) -> Option<*mut AnyObject> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };

    unsafe {
        let ns_view: *mut AnyObject = handle.ns_view.as_ptr().cast();
        let ns_window: *mut AnyObject = msg_send![ns_view, window];
        if ns_window.is_null() {
            return None;
        }
        let content_view: *mut AnyObject = msg_send![ns_window, contentView];
        if content_view.is_null() {
            return None;
        }

        Some(content_view)
    }
}

pub(crate) fn enable(window: &mut Window) -> bool {
    // The class only exists on macOS 26 (Tahoe) and later, so a runtime
    // lookup doubles as the OS version check.
    let Some(glass_class) = AnyClass::get(c"NSGlassEffectView") else {
        return false;
    };
    let Some(content_view) = content_view(window) else {
        return false;
    };

    // Let GPUI make the window non-opaque and its Metal layer
    // transparent, so the glass below shows through.
    window.set_background_appearance(WindowBackgroundAppearance::Transparent);

    unsafe {
        let bounds: NSRect = msg_send![content_view, bounds];
        let glass: Retained<AnyObject> = msg_send![glass_class, new];
        let _: () = msg_send![&*glass, setFrame: bounds];
        let _: () = msg_send![&*glass, setAutoresizingMask: AUTORESIZE_WIDTH_HEIGHT];
        let _: () = msg_send![
            content_view,
            addSubview: &*glass,
            positioned: NS_WINDOW_BELOW,
            relativeTo: std::ptr::null_mut::<AnyObject>()
        ];
    }

    true
}

/// Removes the injected glass views and restores the opaque background.
pub(crate) fn disable(window: &mut Window) {
    let Some(glass_class) = AnyClass::get(c"NSGlassEffectView") else {
        return;
    };
    let Some(content_view) = content_view(window) else {
        return;
    };

    unsafe {
        // The `subviews` getter returns a copy of the array, so it is
        // safe to remove views while iterating over it.
        let subviews: Retained<AnyObject> = msg_send![content_view, subviews];
        let count: usize = msg_send![&*subviews, count];
        for i in 0..count {
            let view: *mut AnyObject = msg_send![&*subviews, objectAtIndex: i];
            let is_glass: bool = msg_send![view, isKindOfClass: glass_class];
            if is_glass {
                let _: () = msg_send![view, removeFromSuperview];
            }
        }
    }

    window.set_background_appearance(WindowBackgroundAppearance::Opaque);
}
