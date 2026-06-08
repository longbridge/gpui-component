//! macOS native menu implementation (AppKit `NSMenu` via objc2).

use std::cell::Cell;

use gpui::{App, Pixels, Point, Window};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, define_class, msg_send, sel};
use objc2_app_kit::{NSMenu, NSMenuItem, NSView};
use objc2_foundation::{NSPoint, NSString};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use super::NativeMenuItem;

/// Ivars for [`MenuTarget`]: the tag of the selected item, or `-1` if none.
struct MenuTargetIvars {
    selected: Cell<isize>,
}

define_class!(
    // A throwaway Objective-C object that receives the menu item action and
    // records which item (by tag) was clicked.
    #[unsafe(super(NSObject))]
    #[name = "GPUIComponentNativeMenuTarget"]
    #[ivars = MenuTargetIvars]
    struct MenuTarget;

    impl MenuTarget {
        #[unsafe(method(menuItemClicked:))]
        fn menu_item_clicked(&self, sender: &NSMenuItem) {
            let tag = sender.tag();
            self.ivars().selected.set(tag);
        }
    }
);

impl MenuTarget {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(MenuTargetIvars {
            selected: Cell::new(-1),
        });
        unsafe { msg_send![super(this), init] }
    }
}

/// Show a native popup menu and dispatch the selected item's action.
///
/// The AppKit tracking loop is run from a foreground task so that GPUI is not
/// borrowed while the menu is open — otherwise re-entrant events delivered
/// during tracking would hit an already-borrowed `RefCell`.
pub(super) fn popup(
    items: Vec<NativeMenuItem>,
    position: Point<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(view_ptr) = ns_view_ptr(window) else {
        return;
    };
    // Inherent `Window::window_handle` (GPUI's `AnyWindowHandle`), not the
    // `raw_window_handle::HasWindowHandle` trait method in scope below.
    let handle = Window::window_handle(window);

    cx.spawn(async move |cx| {
        let Some(index) = run_menu(view_ptr, &items, position) else {
            return;
        };
        let Some(NativeMenuItem::Item {
            action: Some(action),
            ..
        }) = items.get(index)
        else {
            return;
        };
        let action = action.boxed_clone();

        cx.update(move |app| {
            let _ = handle.update(app, move |_, window, app| {
                window.dispatch_action(action, app);
            });
        });
    })
    .detach();
}

/// Build and synchronously run the `NSMenu`, returning the selected item index.
fn run_menu(view_ptr: usize, items: &[NativeMenuItem], position: Point<Pixels>) -> Option<usize> {
    let mtm = MainThreadMarker::new()?;
    // SAFETY: `view_ptr` came from the window's AppKit handle, and the window
    // outlives this synchronous call.
    let view: &NSView = unsafe { &*(view_ptr as *const NSView) };

    let target = MenuTarget::new();
    let ns_menu = NSMenu::new(mtm);
    // Items are configured explicitly, so disable AppKit's automatic enabling.
    ns_menu.setAutoenablesItems(false);

    for (index, item) in items.iter().enumerate() {
        let ns_item = match item {
            NativeMenuItem::Separator => NSMenuItem::separatorItem(mtm),
            NativeMenuItem::Item {
                label,
                disabled,
                checked,
                ..
            } => {
                let ns_item = NSMenuItem::new(mtm);
                let title = NSString::from_str(label);
                unsafe {
                    ns_item.setTitle(&title);
                    ns_item.setTag(index as isize);
                    ns_item.setEnabled(!*disabled);
                    if *checked {
                        // `NSControlStateValueOn`
                        ns_item.setState(1);
                    }
                    if !*disabled {
                        ns_item.setTarget(Some(&*target as &AnyObject));
                        ns_item.setAction(Some(sel!(menuItemClicked:)));
                    }
                }
                ns_item
            }
        };
        ns_menu.addItem(&ns_item);
    }

    // `position` is window-relative, logical pixels, origin top-left (GPUI).
    // AppKit view coordinates have their origin at the bottom-left with the y
    // axis pointing up, so flip y against the view height.
    let height = view.bounds().size.height;
    let location = NSPoint::new(
        f32::from(position.x) as f64,
        height - f32::from(position.y) as f64,
    );
    ns_menu.popUpMenuPositioningItem_atLocation_inView(None, location, Some(view));

    match target.ivars().selected.get() {
        index if index >= 0 => Some(index as usize),
        _ => None,
    }
}

/// Extract the AppKit `NSView` pointer from the window's raw handle.
fn ns_view_ptr(window: &Window) -> Option<usize> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    Some(handle.ns_view.as_ptr() as usize)
}
