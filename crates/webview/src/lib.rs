use std::{ops::Deref, rc::Rc};

#[cfg(target_os = "macos")]
use block2::RcBlock;
use wry::{
    Rect,
    dpi::{self, LogicalSize},
};

#[cfg(target_os = "macos")]
use objc2::{rc::Retained, runtime::AnyObject};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSEvent, NSEventMask};

use gpui::{
    App, Bounds, ContentMask, DismissEvent, Element, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable, GlobalElementId, Hitbox, InteractiveElement, IntoElement, LayoutId, MouseDownEvent,
    ParentElement as _, Pixels, Render, Size, Style, Styled as _, Window, canvas, div,
};

/// A webview based on wry WebView.
///
/// [experimental]
pub struct WebView {
    focus_handle: FocusHandle,
    webview: Rc<wry::WebView>,
    visible: bool,
    bounds: Bounds<Pixels>,
    #[cfg(target_os = "macos")]
    event_monitor: Option<Retained<AnyObject>>,
}

impl Drop for WebView {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(event_monitor) = self.event_monitor.take() {
            // SAFETY: The token was returned by addLocalMonitor and is removed
            // exactly once while the application is still running.
            unsafe { NSEvent::removeMonitor(&event_monitor) };
        }
        self.hide();
    }
}

impl WebView {
    /// Create a new WebView from a wry WebView.
    pub fn new(webview: wry::WebView, _window: &mut Window, cx: &mut App) -> Self {
        let _ = webview.set_bounds(Rect::default());

        #[cfg(target_os = "macos")]
        _window
            .enable_scene_overlay()
            .expect("macOS WebView requires GPUI layered scene support");

        #[cfg(target_os = "macos")]
        let event_monitor = install_focus_monitor(&webview, _window, cx);

        Self {
            focus_handle: cx.focus_handle(),
            visible: true,
            bounds: Bounds::default(),
            webview: Rc::new(webview),
            #[cfg(target_os = "macos")]
            event_monitor,
        }
    }

    /// Show the webview.
    pub fn show(&mut self) {
        let _ = self.webview.set_visible(true);
        self.visible = true;
    }

    /// Hide the webview.
    pub fn hide(&mut self) {
        _ = self.webview.focus_parent();
        _ = self.webview.set_visible(false);
        self.visible = false;
    }

    /// Get whether the webview is visible.
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// Get the current bounds of the webview.
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    /// Go back in the webview history.
    pub fn back(&mut self) -> anyhow::Result<()> {
        Ok(self.webview.evaluate_script("history.back();")?)
    }

    /// Load a URL in the webview.
    pub fn load_url(&mut self, url: &str) {
        let _ = self.webview.load_url(url);
    }

    /// Get the raw wry webview.
    pub fn raw(&self) -> &wry::WebView {
        &self.webview
    }
}

#[cfg(target_os = "macos")]
fn install_focus_monitor(
    webview: &wry::WebView,
    window: &Window,
    cx: &App,
) -> Option<Retained<AnyObject>> {
    use wry::WebViewExtMacOS as _;

    let native_webview = webview.webview();
    let native_window = webview.ns_window();
    let async_window = window.to_async(cx);
    let foreground_executor = cx.foreground_executor().clone();
    let handler = RcBlock::new(move |event: std::ptr::NonNull<NSEvent>| {
        let event = unsafe { event.as_ref() };
        let clicked_webview = event
            .window(objc2::MainThreadMarker::new().expect("NSEvent runs on the main thread"))
            .filter(|event_window| std::ptr::eq(&**event_window, &*native_window))
            .and_then(|event_window| event_window.contentView())
            .and_then(|content_view| {
                let point = content_view.convertPoint_fromView(event.locationInWindow(), None);
                content_view.hitTest(point)
            })
            .is_some_and(|hit_view| {
                std::ptr::eq(&*hit_view, &***native_webview)
                    || hit_view.isDescendantOf(&native_webview)
            });

        if clicked_webview {
            let mut async_window = async_window.clone();
            foreground_executor
                .spawn(async move {
                    let _ = async_window.update(|window, _| window.blur());
                })
                .detach();
        }

        event as *const NSEvent as *mut NSEvent
    });

    // SAFETY: The block returns the same live NSEvent it receives. The retained
    // monitor token is stored on WebView and removed in Drop.
    unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(
            NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown | NSEventMask::OtherMouseDown,
            &handler,
        )
    }
}

impl Deref for WebView {
    type Target = wry::WebView;

    fn deref(&self) -> &Self::Target {
        &self.webview
    }
}

impl Focusable for WebView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for WebView {}

impl Render for WebView {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().clone();

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .child({
                let view = cx.entity().clone();
                canvas(
                    move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
            })
            .child(WebViewElement::new(self.webview.clone(), view, window, cx))
    }
}

/// A webview element can display a wry webview.
pub struct WebViewElement {
    parent: Entity<WebView>,
    view: Rc<wry::WebView>,
}

impl WebViewElement {
    /// Create a new webview element from a wry WebView.
    pub fn new(
        view: Rc<wry::WebView>,
        parent: Entity<WebView>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self {
        Self { view, parent }
    }
}

impl IntoElement for WebViewElement {
    type Element = WebViewElement;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for WebViewElement {
    type RequestLayoutState = ();
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: Size::full(),
            flex_shrink: 1.,
            ..Default::default()
        };

        // If the parent view is no longer visible, we don't need to layout the webview
        let id = window.request_layout(style, [], cx);
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if !self.parent.read(cx).visible() {
            return None;
        }

        let _ = self.view.set_bounds(Rect {
            size: dpi::Size::Logical(LogicalSize {
                width: bounds.size.width.into(),
                height: bounds.size.height.into(),
            }),
            position: dpi::Position::Logical(dpi::LogicalPosition::new(
                bounds.origin.x.into(),
                bounds.origin.y.into(),
            )),
        });

        // Create a hitbox to handle mouse event
        Some(window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal))
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let bounds = hitbox.clone().map(|h| h.bounds).unwrap_or(bounds);
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            let webview = self.view.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, _, window, _| {
                if bounds.contains(&event.position) {
                    // Native WebView focus is outside GPUI's focus tree. Clear
                    // GPUI focus so text inputs stop showing a stale caret.
                    window.blur();
                } else {
                    // Return native focus to the GPUI parent when clicking
                    // elsewhere in the GPUI window.
                    let _ = webview.focus_parent();
                }
            });
        });
    }
}
