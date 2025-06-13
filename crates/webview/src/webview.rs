use std::{ops::Range, rc::Rc};

use gpui::{
    App, Bounds, ClipboardItem, CursorStyle, Empty, Entity, EntityInputHandler, EventEmitter,
    FocusHandle, Focusable, KeyDownEvent, KeyUpEvent, MouseDownEvent, Pixels, ScrollWheelEvent,
    Subscription, UTF16Selection, WeakEntity, Window, anchored, deferred, div, point, prelude::*,
    px,
};
use wef::{Browser, FuncRegistry, LogicalUnit, Point, Rect};

use crate::{
    browser_handler::WebViewHandler,
    context_menu::{ContextMenuAction, ContextMenuInfo},
    element::WebViewElement,
    events::*,
    frame_view::FrameView,
    utils::*,
};

/// A web view based on the Chromium Embedded Framework (CEF).
pub struct WebView {
    pub(crate) main: FrameView,
    pub(crate) popup: Option<FrameView>,
    pub(crate) popup_rect: Option<Rect<LogicalUnit<i32>>>,
    pub(crate) cursor: CursorStyle,
    pub(crate) context_menu: Option<ContextMenuInfo>,
    pub(crate) bounds: Bounds<Pixels>,
    focus_handle: FocusHandle,
    browser: Rc<Browser>,
    _subscriptions: Vec<Subscription>,
}

impl WebView {
    /// Creates a new `WebView` instance with the given URL.
    pub fn new(url: &str, window: &mut Window, cx: &mut App) -> Entity<Self> {
        Self::with_func_registry(url, FuncRegistry::default(), window, cx)
    }

    /// Creates a new `WebView` instance with the given URL and function
    /// registry.
    pub fn with_func_registry(
        url: &str,
        function_registry: FuncRegistry,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let window_handle = window.window_handle();
        let entity = cx.new(|cx| {
            let entity = cx.entity();

            let browser = Rc::new(
                Browser::builder()
                    .parent(
                        raw_window_handle::HasWindowHandle::window_handle(window)
                            .ok()
                            .map(|handle| handle.as_raw()),
                    )
                    .device_scale_factor(window.scale_factor())
                    .url(url)
                    .handler(WebViewHandler::new(
                        window_handle,
                        entity.downgrade(),
                        cx.to_async(),
                    ))
                    .func_registry(function_registry)
                    .build(),
            );

            let focus_handle = cx.focus_handle();

            let _subscriptions = vec![
                cx.on_focus(&focus_handle, window, Self::on_focus),
                cx.on_blur(&focus_handle, window, Self::on_blur),
            ];

            Self {
                focus_handle,
                main: FrameView::default(),
                popup: None,
                popup_rect: None,
                browser,
                cursor: CursorStyle::Arrow,
                context_menu: None,
                bounds: Bounds::default(),
                _subscriptions,
            }
        });

        cx.observe_release(&entity, |webview, cx| {
            for window in cx.windows() {
                _ = cx.update_window(window, |_, window, _cx| {
                    webview.main.clear(window);
                    if let Some(popup_frame) = &mut webview.popup {
                        popup_frame.clear(window);
                    }
                });
            }
        })
        .detach();

        entity
    }

    /// Returns the browser instance.
    #[inline]
    pub fn browser(&self) -> &Rc<Browser> {
        &self.browser
    }

    fn scroll_wheel_handler(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        let (delta_x, delta_y) = match event.delta {
            gpui::ScrollDelta::Pixels(point) => (point.x.0, point.y.0),
            gpui::ScrollDelta::Lines(point) => (point.x * 20.0, point.y * 20.0),
        };
        self.browser().send_mouse_wheel_event(Point::new(
            LogicalUnit(delta_x as i32),
            LogicalUnit(delta_y as i32),
        ));
    }

    fn keydown_handler(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        let modifiers = to_wef_key_modifiers(&event.keystroke.modifiers);
        if let Some(key_code) = to_wef_key_code(&event.keystroke.key) {
            self.browser().send_key_event(true, key_code, modifiers);
        };
    }

    fn keyup_handler(&mut self, event: &KeyUpEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        let modifiers = to_wef_key_modifiers(&event.keystroke.modifiers);
        let Some(key_code) = to_wef_key_code(&event.keystroke.key) else {
            return;
        };
        self.browser().send_key_event(false, key_code, modifiers);
    }

    fn on_focus(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.browser().set_focus(true);
    }

    fn on_blur(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.browser().set_focus(false);
    }

    fn on_context_menu_action(
        &mut self,
        action: &ContextMenuAction,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use ContextMenuAction::*;

        if let Some(info) = self.context_menu.take() {
            let action = *action;

            cx.spawn(async move |webview: WeakEntity<WebView>, cx| {
                let Ok(browser) = webview.read_with(cx, |webview, _cx| webview.browser().clone())
                else {
                    return;
                };
                match action {
                    CopyLinkAddress => {
                        if let Some(link_url) = &info.link_url {
                            _ = cx.update(|cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(link_url.clone()))
                            });
                        }
                    }
                    Undo => info.frame.undo(),
                    Redo => info.frame.redo(),
                    Cut => info.frame.cut(),
                    Copy => info.frame.copy(),
                    Paste => info.frame.paste(),
                    ParseAsPlainText => info.frame.paste_and_match_style(),
                    SelectAll => info.frame.select_all(),
                    GoBack => browser.back(),
                    GoForward => browser.forward(),
                    Reload => browser.reload(),
                }
            })
            .detach();
        }
    }

    fn context_menu_mousedown_out_handler(
        &mut self,
        _event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.context_menu = None;
        cx.notify();
    }
}

impl Focusable for WebView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for WebView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(image) = self.main.render(window) else {
            return Empty.into_any_element();
        };

        let mut root = div().size_full().child(
            WebViewElement::new(
                cx.entity(),
                self.focus_handle.clone(),
                self.browser.clone(),
                image.clone(),
                self.popup_rect
                    .zip(self.popup.as_mut().and_then(|f| f.render(window))),
            )
            .size_full()
            .track_focus(&self.focus_handle)
            .on_scroll_wheel(cx.listener(Self::scroll_wheel_handler))
            .on_key_down(cx.listener(Self::keydown_handler))
            .on_key_up(cx.listener(Self::keyup_handler)),
        );

        if let Some(info) = &self.context_menu {
            root = root.child(deferred(
                anchored()
                    .position(point(px(info.crood.x.0 as f32), px(info.crood.y.0 as f32)))
                    .child(
                        div()
                            .child(info.menu.clone())
                            .shadow_2xl()
                            .on_mouse_down_out(
                                cx.listener(Self::context_menu_mousedown_out_handler),
                            ),
                    ),
            ));
        }

        root.on_action(cx.listener(Self::on_context_menu_action))
            .into_any_element()
    }
}

impl EntityInputHandler for WebView {
    fn text_for_range(
        &mut self,
        _range: Range<usize>,
        _adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        None
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        None
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn replace_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.browser().ime_commit(text);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        let new_selected_range = new_selected_range.unwrap_or_default();
        self.browser().ime_set_composition(
            new_text,
            new_selected_range.start,
            new_selected_range.end,
        );
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        None
    }

    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

macro_rules! impl_emiter {
    ($($ty:ty),*) => {
        $(
            impl EventEmitter<$ty> for WebView {}
        )*
    };
}

impl_emiter!(
    LoadingProgressChangedEvent,
    CreatedEvent,
    AddressChangedEvent,
    TitleChangedEvent,
    TooltipEvent,
    StatusMessageEvent,
    ConsoleMessageEvent,
    BeforePopupEvent,
    LoadingStateChangedEvent,
    LoadStartEvent,
    LoadEndEvent,
    LoadErrorEvent
);
