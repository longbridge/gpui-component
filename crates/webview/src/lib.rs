use std::{ops::Deref, rc::Rc};

#[cfg(target_os = "macos")]
use block2::RcBlock;
#[cfg(target_os = "windows")]
use std::cell::Cell;
use wry::{
    Rect,
    dpi::{self, LogicalSize},
};

#[cfg(target_os = "macos")]
use objc2::{rc::Retained, runtime::AnyObject};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSEvent, NSEventMask};
#[cfg(target_os = "windows")]
use webview2_com::Microsoft::Web::WebView2::Win32::*;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::POINT;

use gpui::{
    App, Bounds, ContentMask, DismissEvent, DispatchPhase, Element, ElementId, Entity,
    EventEmitter, FocusHandle, Focusable, GlobalElementId, Hitbox, InteractiveElement, IntoElement,
    LayoutId, MouseDownEvent, ParentElement as _, Pixels, Render, Size, Style, Styled as _, Window,
    canvas, div,
};
#[cfg(target_os = "windows")]
use gpui::{
    MouseButton, MouseExitEvent, MouseMoveEvent, MouseUpEvent, ScrollDelta, ScrollWheelEvent,
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
    #[cfg(target_os = "windows")]
    native_surface: Option<Rc<dyn gpui::PlatformNativeSurface>>,
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
    /// Builds a child WebView using the platform's native-surface integration.
    pub fn build_as_child(
        builder: wry::WebViewBuilder<'_>,
        parent: &impl wry::raw_window_handle::HasWindowHandle,
        window: &Window,
        cx: &mut App,
    ) -> anyhow::Result<Self> {
        #[cfg(target_os = "windows")]
        {
            use wry::WebViewBuilderExtWindows as _;

            let native_surface = window.create_native_surface()?;
            let platform_handle = native_surface.platform_handle();
            let root_visual = platform_handle
                .downcast::<windows_core::IUnknown>()
                .map_err(|_| anyhow::anyhow!("GPUI returned an invalid Windows portal handle"))?;
            let webview = builder
                .with_composition_root_visual(*root_visual)
                .build_as_child(parent)?;
            return Ok(Self::new_with_native_surface(
                webview,
                native_surface,
                window,
                cx,
            ));
        }

        #[cfg(not(target_os = "windows"))]
        {
            Ok(Self::new(builder.build_as_child(parent)?, window, cx))
        }
    }

    /// Create a new WebView from a wry WebView.
    pub fn new(webview: wry::WebView, _window: &Window, cx: &mut App) -> Self {
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
            #[cfg(target_os = "windows")]
            native_surface: None,
        }
    }

    #[cfg(target_os = "windows")]
    fn new_with_native_surface(
        webview: wry::WebView,
        native_surface: Rc<dyn gpui::PlatformNativeSurface>,
        _window: &Window,
        cx: &mut App,
    ) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            visible: true,
            bounds: Bounds::default(),
            webview: Rc::new(webview),
            native_surface: Some(native_surface),
        }
    }

    /// Show the webview.
    pub fn show(&mut self) {
        let _ = self.webview.set_visible(true);
        #[cfg(target_os = "windows")]
        if let Some(native_surface) = &self.native_surface {
            let _ = native_surface.set_visible(true);
        }
        self.visible = true;
    }

    /// Hide the webview.
    pub fn hide(&mut self) {
        #[cfg(target_os = "windows")]
        focus_parent(&self.webview);
        #[cfg(not(target_os = "windows"))]
        {
            _ = self.webview.focus_parent();
        }
        _ = self.webview.set_visible(false);
        #[cfg(target_os = "windows")]
        if let Some(native_surface) = &self.native_surface {
            let _ = native_surface.set_visible(false);
        }
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

        #[cfg(target_os = "windows")]
        if let Some(native_surface) = &self.parent.read(cx).native_surface {
            let _ = native_surface.set_bounds(bounds.to_device_pixels(window.scale_factor()));
        }

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
            #[cfg(target_os = "windows")]
            {
                let webview = self.view.clone();
                let was_hovered = Rc::new(Cell::new(false));
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, _| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    if bounds.contains(&event.position) {
                        was_hovered.set(true);
                        send_mouse_input(&webview, MouseInput::Move(event), bounds, window);
                    } else if was_hovered.replace(false) {
                        send_mouse_leave(&webview);
                    }
                });

                let webview = self.view.clone();
                let down_hitbox = hitbox.clone();
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, _| {
                    // GPUI has updated its hit test before dispatching the
                    // event. Use it here rather than only checking bounds so
                    // an overlay control above the WebView remains the real
                    // target.
                    let hovered = down_hitbox
                        .as_ref()
                        .is_some_and(|hitbox| hitbox.is_hovered(window));
                    if phase == DispatchPhase::Capture {
                        if hovered {
                            // Move native focus before any bubble listener can
                            // stop propagation. Composition WebView keyboard
                            // input is delivered through the controller's
                            // focused native window, not through GPUI's input
                            // handler.
                            window.blur();
                            let _ = webview.focus();
                        } else {
                            // Overlay controls may stop propagation during
                            // bubble, so hand native focus back during capture.
                            focus_parent(&webview);
                        }
                        return;
                    }
                    if phase != DispatchPhase::Bubble || !hovered {
                        return;
                    }

                    {
                        if let Some(hitbox) = down_hitbox.as_ref() {
                            window.capture_pointer(hitbox.id);
                        }
                        send_mouse_input(&webview, MouseInput::Down(event), bounds, window);
                    }
                });

                let webview = self.view.clone();
                let up_hitbox = hitbox.clone();
                window.on_mouse_event(move |event: &MouseUpEvent, phase, window, _| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    if up_hitbox
                        .as_ref()
                        .is_some_and(|hitbox| hitbox.is_hovered(window))
                    {
                        send_mouse_input(&webview, MouseInput::Up(event), bounds, window);
                    }
                });

                let webview = self.view.clone();
                let wheel_hitbox = hitbox.clone();
                window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, _| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    if wheel_hitbox
                        .as_ref()
                        .is_some_and(|hitbox| hitbox.should_handle_scroll(window))
                    {
                        send_mouse_input(&webview, MouseInput::Wheel(event), bounds, window);
                    }
                });

                let webview = self.view.clone();
                window.on_mouse_event(move |_: &MouseExitEvent, phase, _, _| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    send_mouse_leave(&webview);
                });
            }

            #[cfg(not(target_os = "windows"))]
            {
                let webview = self.view.clone();
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, _| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
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
            }
        });
    }
}

#[cfg(target_os = "windows")]
enum MouseInput<'a> {
    Move(&'a MouseMoveEvent),
    Down(&'a MouseDownEvent),
    Up(&'a MouseUpEvent),
    Wheel(&'a ScrollWheelEvent),
}

#[cfg(target_os = "windows")]
fn composition_controller(webview: &wry::WebView) -> Option<ICoreWebView2CompositionController> {
    use wry::WebViewExtWindows as _;
    webview.composition_controller()
}

#[cfg(target_os = "windows")]
fn focus_parent(webview: &wry::WebView) {
    let _ = webview.focus_parent();
}

#[cfg(target_os = "windows")]
fn send_mouse_leave(webview: &wry::WebView) {
    if let Some(controller) = composition_controller(webview) {
        let _ = unsafe {
            controller.SendMouseInput(
                COREWEBVIEW2_MOUSE_EVENT_KIND_LEAVE,
                COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS_NONE,
                0,
                POINT { x: 0, y: 0 },
            )
        };
    }
}

#[cfg(target_os = "windows")]
fn send_mouse_input(
    webview: &wry::WebView,
    input: MouseInput<'_>,
    bounds: Bounds<Pixels>,
    window: &Window,
) {
    let Some(controller) = composition_controller(webview) else {
        return;
    };

    let (position, kind, virtual_keys, mouse_data) = match input {
        MouseInput::Move(event) => (
            event.position,
            COREWEBVIEW2_MOUSE_EVENT_KIND_MOVE,
            virtual_keys(event.modifiers, event.pressed_button),
            0,
        ),
        MouseInput::Down(event) => (
            event.position,
            button_kind(event.button, true, event.click_count),
            virtual_keys(event.modifiers, Some(event.button)),
            0,
        ),
        MouseInput::Up(event) => (
            event.position,
            button_kind(event.button, false, event.click_count),
            virtual_keys(event.modifiers, None),
            0,
        ),
        MouseInput::Wheel(event) => {
            let (kind, delta) = match event.delta {
                ScrollDelta::Pixels(delta) => {
                    if delta.x.as_f32() != 0. {
                        (
                            COREWEBVIEW2_MOUSE_EVENT_KIND_HORIZONTAL_WHEEL,
                            delta.x.as_f32() as i32,
                        )
                    } else {
                        (COREWEBVIEW2_MOUSE_EVENT_KIND_WHEEL, delta.y.as_f32() as i32)
                    }
                }
                ScrollDelta::Lines(delta) => {
                    if delta.x != 0. {
                        (
                            COREWEBVIEW2_MOUSE_EVENT_KIND_HORIZONTAL_WHEEL,
                            (delta.x * 120.) as i32,
                        )
                    } else {
                        (COREWEBVIEW2_MOUSE_EVENT_KIND_WHEEL, (delta.y * 120.) as i32)
                    }
                }
            };
            (
                event.position,
                kind,
                virtual_keys(event.modifiers, None),
                delta,
            )
        }
    };

    let scale_factor = window.scale_factor();
    let point = POINT {
        x: ((position.x - bounds.origin.x).as_f32() * scale_factor) as i32,
        y: ((position.y - bounds.origin.y).as_f32() * scale_factor) as i32,
    };
    let _ = unsafe { controller.SendMouseInput(kind, virtual_keys, mouse_data as u32, point) };
}

#[cfg(target_os = "windows")]
fn virtual_keys(
    modifiers: gpui::Modifiers,
    pressed_button: Option<MouseButton>,
) -> COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS {
    let mut keys = COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS_NONE;
    if modifiers.control {
        keys = keys | COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS_CONTROL;
    }
    if modifiers.shift {
        keys = keys | COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS_SHIFT;
    }
    keys | match pressed_button {
        Some(MouseButton::Left) => COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS_LEFT_BUTTON,
        Some(MouseButton::Right) => COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS_RIGHT_BUTTON,
        Some(MouseButton::Middle) => COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS_MIDDLE_BUTTON,
        _ => COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS_NONE,
    }
}

#[cfg(target_os = "windows")]
fn button_kind(
    button: MouseButton,
    down: bool,
    click_count: usize,
) -> COREWEBVIEW2_MOUSE_EVENT_KIND {
    match (button, down, click_count > 1) {
        (MouseButton::Left, true, true) => COREWEBVIEW2_MOUSE_EVENT_KIND_LEFT_BUTTON_DOUBLE_CLICK,
        (MouseButton::Left, true, false) => COREWEBVIEW2_MOUSE_EVENT_KIND_LEFT_BUTTON_DOWN,
        (MouseButton::Left, false, _) => COREWEBVIEW2_MOUSE_EVENT_KIND_LEFT_BUTTON_UP,
        (MouseButton::Right, true, true) => COREWEBVIEW2_MOUSE_EVENT_KIND_RIGHT_BUTTON_DOUBLE_CLICK,
        (MouseButton::Right, true, false) => COREWEBVIEW2_MOUSE_EVENT_KIND_RIGHT_BUTTON_DOWN,
        (MouseButton::Right, false, _) => COREWEBVIEW2_MOUSE_EVENT_KIND_RIGHT_BUTTON_UP,
        (MouseButton::Middle, true, true) => {
            COREWEBVIEW2_MOUSE_EVENT_KIND_MIDDLE_BUTTON_DOUBLE_CLICK
        }
        (MouseButton::Middle, true, false) => COREWEBVIEW2_MOUSE_EVENT_KIND_MIDDLE_BUTTON_DOWN,
        (MouseButton::Middle, false, _) => COREWEBVIEW2_MOUSE_EVENT_KIND_MIDDLE_BUTTON_UP,
        _ => COREWEBVIEW2_MOUSE_EVENT_KIND_MOVE,
    }
}
