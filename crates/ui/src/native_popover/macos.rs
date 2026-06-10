//! macOS native popover implementation (AppKit `NSPopover` via objc2).
//!
//! Unlike [`crate::native_menu`], whose `NSMenu` runs a blocking tracking loop,
//! `NSPopover` is *modeless*: `show` returns immediately and the popover stays
//! up until a button is clicked or the user clicks outside (transient
//! behavior). We therefore:
//!
//!   - keep the live `NSPopover` (and its content objects) in a thread-local so
//!     they outlive `show`;
//!   - deliver button clicks back to GPUI over a channel (the AppKit target
//!     can't reach `cx` directly);
//!   - poll from a foreground task for either a click or an external dismissal,
//!     then dispatch the action and tear the popover down.

use std::cell::RefCell;
use std::time::Duration;

use gpui::{
    Action, App, AppContext as _, Bounds, Entity, Pixels, Render, SharedString, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, define_class, msg_send, sel};
use objc2_app_kit::{NSButton, NSPopover, NSPopoverBehavior, NSTextField, NSView, NSViewController};
use objc2_foundation::{NSPoint, NSRect, NSRectEdge, NSSize, NSString};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use smol::Timer;
use smol::channel::{Sender, unbounded};

use super::NativePopoverButton;

/// Content layout constants (logical points, AppKit coordinates).
const WIDTH: f64 = 260.0;
const PAD: f64 = 14.0;
const GAP: f64 = 8.0;
const TITLE_H: f64 = 18.0;
const BUTTON_H: f64 = 28.0;
/// Poll interval while the popover is open.
const POLL: Duration = Duration::from_millis(30);

/// Ivars for [`PopoverTarget`]: the channel a click's tag is sent over.
struct PopoverTargetIvars {
    tx: Sender<usize>,
}

define_class!(
    // A throwaway Objective-C object that receives button clicks and forwards
    // the clicked button's tag (its index) over a channel.
    #[unsafe(super(NSObject))]
    #[name = "GPUIComponentNativePopoverTarget"]
    #[ivars = PopoverTargetIvars]
    struct PopoverTarget;

    impl PopoverTarget {
        #[unsafe(method(buttonClicked:))]
        fn button_clicked(&self, sender: &AnyObject) {
            let tag: isize = unsafe { msg_send![sender, tag] };
            let _ = self.ivars().tx.try_send(tag as usize);
        }
    }
);

impl PopoverTarget {
    fn new(tx: Sender<usize>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(PopoverTargetIvars { tx });
        unsafe { msg_send![super(this), init] }
    }
}

/// The live popover plus the objects that must outlive `show`.
struct ActivePopover {
    popover: Retained<NSPopover>,
    /// Present only for the native-content path (button click target).
    _target: Option<Retained<PopoverTarget>>,
    _controller: Retained<NSViewController>,
}

thread_local! {
    static ACTIVE: RefCell<Option<ActivePopover>> = const { RefCell::new(None) };
}

/// Close and release the active popover, if any. Main thread only.
fn close_active() {
    if let Some(active) = ACTIVE.with(|a| a.borrow_mut().take()) {
        if active.popover.isShown() {
            unsafe { active.popover.performClose(None) };
        }
    }
}

/// Whether the active popover is still on screen. Main thread only.
fn active_is_shown() -> bool {
    ACTIVE.with(|a| {
        a.borrow()
            .as_ref()
            .map(|active| active.popover.isShown())
            .unwrap_or(false)
    })
}

pub(super) fn show(
    title: Option<SharedString>,
    buttons: Vec<NativePopoverButton>,
    anchor: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(view_ptr) = ns_view_ptr(window) else {
        return;
    };
    // Inherent `Window::window_handle` (GPUI's `AnyWindowHandle`), not the
    // `raw_window_handle` trait method in scope below.
    let handle = Window::window_handle(window);

    // Replace any previous popover.
    close_active();

    let (tx, rx) = unbounded::<usize>();
    let target = PopoverTarget::new(tx);

    // SAFETY: `view_ptr` came from the window's AppKit handle, and the window
    // outlives this synchronous build.
    let view: &NSView = unsafe { &*(view_ptr as *const NSView) };

    // --- Build the content view (AppKit bottom-left coordinates: larger y is
    // higher on screen) ---
    let n = buttons.len();
    let title_block = if title.is_some() { TITLE_H + GAP } else { 0.0 };
    let buttons_block = if n > 0 {
        n as f64 * BUTTON_H + (n as f64 - 1.0) * GAP
    } else {
        0.0
    };
    let content_h = PAD + title_block + buttons_block + PAD;
    let inner_w = WIDTH - 2.0 * PAD;

    let content = NSView::new(mtm);
    content.setFrameSize(NSSize::new(WIDTH, content_h));

    if let Some(title) = &title {
        let label = NSTextField::labelWithString(&NSString::from_str(title), mtm);
        label.setFrameSize(NSSize::new(inner_w, TITLE_H));
        label.setFrameOrigin(NSPoint::new(PAD, content_h - PAD - TITLE_H));
        content.addSubview(&label);
    }

    // Buttons stacked downward from just under the title.
    let buttons_top = content_h - PAD - title_block;
    let mut actions: Vec<Box<dyn Action>> = Vec::with_capacity(n);
    for (i, button) in buttons.into_iter().enumerate() {
        let ns_button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(&button.label),
                Some(&*target as &AnyObject),
                Some(sel!(buttonClicked:)),
                mtm,
            )
        };
        ns_button.setTag(i as isize);
        ns_button.setFrameSize(NSSize::new(inner_w, BUTTON_H));
        let y = buttons_top - (i as f64 + 1.0) * BUTTON_H - i as f64 * GAP;
        ns_button.setFrameOrigin(NSPoint::new(PAD, y));
        content.addSubview(&ns_button);
        actions.push(button.action);
    }

    // --- Controller + popover ---
    let controller = NSViewController::new(mtm);
    controller.setView(&content);

    let popover = NSPopover::new(mtm);
    popover.setBehavior(NSPopoverBehavior::Transient);
    popover.setAnimates(true);
    popover.setContentViewController(Some(&controller));
    popover.setContentSize(NSSize::new(WIDTH, content_h));

    // Positioning rect: window coordinates (top-left origin, GPUI) -> view
    // coordinates (bottom-left origin, non-flipped GPUI content view).
    let view_h = view.bounds().size.height;
    let ax = f32::from(anchor.origin.x) as f64;
    let ay = f32::from(anchor.origin.y) as f64;
    let aw = f32::from(anchor.size.width) as f64;
    let ah = f32::from(anchor.size.height) as f64;
    let rect = NSRect::new(NSPoint::new(ax, view_h - (ay + ah)), NSSize::new(aw, ah));

    // Prefer anchoring to the bottom edge so the popover appears below the
    // trigger (arrow pointing up at it).
    popover.showRelativeToRect_ofView_preferredEdge(rect, view, NSRectEdge::MinY);

    ACTIVE.with(|a| {
        *a.borrow_mut() = Some(ActivePopover {
            popover,
            _target: Some(target),
            _controller: controller,
        });
    });

    // The whole task runs on the foreground (main) thread, so the thread-local
    // and AppKit calls below are main-thread safe.
    cx.spawn(async move |cx| {
        loop {
            if let Ok(tag) = rx.try_recv() {
                let _ = cx.update(|app| {
                    let _ = handle.update(app, |_, window, app| {
                        if let Some(action) = actions.get(tag) {
                            window.dispatch_action(action.boxed_clone(), app);
                        }
                        window.refresh();
                    });
                });
                close_active();
                break;
            }

            Timer::after(POLL).await;

            // Externally dismissed (clicked outside / transient close).
            if !active_is_shown() {
                close_active();
                break;
            }
        }
    })
    .detach();
}

/// SPIKE: show arbitrary GPUI content inside a native `NSPopover`.
///
/// Strategy: open a hidden GPUI `PopUp` window that renders `build(...)`, then
/// *reparent* its AppKit `NSView` (which carries the Metal layer and the input
/// responders) into the `NSPopover`'s content view controller. GPUI keeps
/// driving rendering/input through that view; the popover provides the native
/// shell. The source GPUI window is kept alive (it owns the GPUI window state)
/// and torn down when the popover closes.
pub(super) fn show_view<V: 'static + Render>(
    anchor: Bounds<Pixels>,
    size: Size<Pixels>,
    window: &mut Window,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V> + 'static,
) {
    let Some(main_view_ptr) = ns_view_ptr(window) else {
        return;
    };

    // Run open + reparent + show from a foreground task so we don't re-enter
    // GPUI's window borrow while the caller's event is still dispatching
    // (opening a window mid-borrow triggers "RefCell already borrowed").
    cx.spawn(async move |cx| {
        let Some(child) =
            cx.update(|app| open_reparent_show(main_view_ptr, anchor, size, app, build))
        else {
            return;
        };

        // Poll for external (transient) dismissal, then tear everything down.
        loop {
            Timer::after(POLL).await;
            if cx.update(|_| active_is_shown()) {
                continue;
            }
            let _ = cx.update(|app| {
                let _ = child.update(app, |_, w, _| w.remove_window());
                close_active();
            });
            break;
        }
    })
    .detach();
}

/// Open the source GPUI window, reparent its view into a fresh `NSPopover`, show
/// it, and return the source window handle (kept alive until the popover closes).
fn open_reparent_show<V: 'static + Render>(
    main_view_ptr: usize,
    anchor: Bounds<Pixels>,
    size: Size<Pixels>,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> Option<WindowHandle<crate::Root>> {
    let mtm = MainThreadMarker::new()?;
    close_active();

    // Open a GPUI PopUp window rendering the arbitrary content. It is shown (so
    // the display link runs) but its view is immediately reparented away.
    let child = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: anchor.origin,
                    size,
                })),
                titlebar: None,
                kind: WindowKind::PopUp,
                focus: false,
                show: true,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                window_background: WindowBackgroundAppearance::Transparent,
                ..Default::default()
            },
            // The child window's root must be a `Root` (like any GPUI Component
            // window) — content such as `Input` calls `Root::read`/`update`.
            |w, cx| {
                let content = build(w, cx);
                cx.new(|cx| crate::Root::new(content, w, cx))
            },
        )
        .ok()?;

    let child_view_ptr = match child.update(cx, |_, w, _| ns_view_ptr(w)) {
        Ok(Some(ptr)) => ptr,
        _ => {
            let _ = child.update(cx, |_, w, _| w.remove_window());
            return None;
        }
    };

    // SAFETY: both pointers come from live windows' AppKit handles.
    let child_view: &NSView = unsafe { &*(child_view_ptr as *const NSView) };
    let main_view: &NSView = unsafe { &*(main_view_ptr as *const NSView) };

    // Hide the source window visually but keep it "visible" to AppKit, so its
    // CVDisplayLink keeps driving frames and its input/responder routing stays
    // intact (ordering it out breaks event delivery entirely).
    let child_window: *mut AnyObject = unsafe { msg_send![child_view, window] };
    if !child_window.is_null() {
        unsafe {
            let _: () = msg_send![child_window, setAlphaValue: 0.0f64];
        }
    }

    child_view.removeFromSuperview();

    let controller = NSViewController::new(mtm);
    controller.setView(child_view);

    // Round the GPUI content layer to match the NSPopover's rounded shell — the
    // square Metal layer would otherwise show hard corners over the popover's
    // curves on a transparent/vibrant background.
    let layer: *mut AnyObject = unsafe { msg_send![child_view, layer] };
    if !layer.is_null() {
        unsafe {
            let _: () = msg_send![layer, setCornerRadius: 10.0f64];
            let _: () = msg_send![layer, setMasksToBounds: true];
        }
    }

    let popover = NSPopover::new(mtm);
    popover.setBehavior(NSPopoverBehavior::Transient);
    popover.setAnimates(true);
    popover.setContentViewController(Some(&controller));
    popover.setContentSize(NSSize::new(
        f32::from(size.width) as f64,
        f32::from(size.height) as f64,
    ));

    let view_h = main_view.bounds().size.height;
    let ax = f32::from(anchor.origin.x) as f64;
    let ay = f32::from(anchor.origin.y) as f64;
    let aw = f32::from(anchor.size.width) as f64;
    let ah = f32::from(anchor.size.height) as f64;
    let rect = NSRect::new(NSPoint::new(ax, view_h - (ay + ah)), NSSize::new(aw, ah));
    popover.showRelativeToRect_ofView_preferredEdge(rect, main_view, NSRectEdge::MinY);

    // Make the reparented GPUI view the first responder of the popover's window
    // so keyboard events (e.g. typing into an `Input`) reach GPUI.
    let popover_window: *mut AnyObject = unsafe { msg_send![child_view, window] };
    if !popover_window.is_null() {
        unsafe {
            let _: bool = msg_send![popover_window, makeFirstResponder: child_view];
        }
    }

    ACTIVE.with(|a| {
        *a.borrow_mut() = Some(ActivePopover {
            popover,
            _target: None,
            _controller: controller,
        });
    });

    Some(child)
}

/// Extract the AppKit `NSView` pointer from the window's raw handle.
fn ns_view_ptr(window: &Window) -> Option<usize> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    Some(handle.ns_view.as_ptr() as usize)
}
