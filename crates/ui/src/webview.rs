use std::{ops::Deref, rc::Rc};

use wry::{
    dpi::{self, LogicalSize},
    Rect,
};

use gpui::{Window, AppContext, Model, 
    canvas, div, Bounds, ContentMask, DismissEvent, Element, ElementId, EventEmitter, FocusHandle,
    FocusableView, GlobalElementId, Hitbox, InteractiveElement, IntoElement, LayoutId,
    MouseDownEvent, ParentElement as _, Pixels, Render, Size, Style, Styled as _, 
    
};

pub struct WebView {
    focus_handle: FocusHandle,
    webview: Rc<wry::WebView>,
    visible: bool,
    bounds: Bounds<Pixels>,
}

impl Drop for WebView {
    fn drop(&mut self) {
        self.hide();
    }
}

impl WebView {
    pub fn new(cx: &mut WindowContext, webview: wry::WebView) -> Self {
        let _ = webview.set_bounds(Rect::default());

        Self {
            focus_handle: cx.focus_handle(),
            visible: true,
            bounds: Bounds::default(),
            webview: Rc::new(webview),
        }
    }

    pub fn show(&mut self) {
        let _ = self.webview.set_visible(true);
    }

    pub fn hide(&mut self) {
        _ = self.webview.focus_parent();
        _ = self.webview.set_visible(false);
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    /// Go back in the webview history.
    pub fn back(&mut self) -> anyhow::Result<()> {
        Ok(self.webview.evaluate_script("history.back();")?)
    }

    pub fn load_url(&mut self, url: &str) {
        self.webview.load_url(url).unwrap();
    }
}

impl Deref for WebView {
    type Target = wry::WebView;

    fn deref(&self) -> &Self::Target {
        &self.webview
    }
}

impl FocusableView for WebView {
    fn focus_handle(&self, _cx: &gpui::AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for WebView {}

impl Render for WebView {
    fn render(&mut self, cx: &mut gpui::ViewContext<Self>) -> impl IntoElement {
        let view = cx.view().clone();

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .child({
                let view = cx.view().clone();
                canvas(
                    move |bounds, window, cx| view.update(cx, |r, _| r.bounds = bounds),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
            })
            .child(WebViewElement::new(self.webview.clone(), view, cx))
    }
}

/// A webview element can display a wry webview.
pub struct WebViewElement {
    parent: View<WebView>,
    view: Rc<wry::WebView>,
}

impl WebViewElement {
    /// Create a new webview element from a wry WebView.
    pub fn new(view: Rc<wry::WebView>, parent: View<WebView>, _: &mut WindowContext) -> Self {
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

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        cx: &mut WindowContext,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.flex_grow = 0.0;
        style.flex_shrink = 1.;
        style.size = Size::full();
        // If the parent view is no longer visible, we don't need to layout the webview

        let id = cx.request_layout(style, []);
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        cx: &mut WindowContext,
    ) -> Self::PrepaintState {
        if !self.parent.read(cx).visible() {
            return None;
        }

        self.view
            .set_bounds(Rect {
                size: dpi::Size::Logical(LogicalSize {
                    width: (bounds.size.width.0).into(),
                    height: (bounds.size.height.0).into(),
                }),
                position: dpi::Position::Logical(dpi::LogicalPosition::new(
                    bounds.origin.x.into(),
                    bounds.origin.y.into(),
                )),
            })
            .unwrap();

        // Create a hitbox to handle mouse event
        Some(cx.insert_hitbox(bounds, false))
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        cx: &mut WindowContext,
    ) {
        let bounds = hitbox.clone().map(|h| h.bounds).unwrap_or(bounds);
        cx.with_content_mask(Some(ContentMask { bounds }), |cx| {
            let webview = self.view.clone();
            cx.on_mouse_event(move |event: &MouseDownEvent, _, _| {
                if !bounds.contains(&event.position) {
                    // Click white space to blur the input focus
                    let _ = webview.focus_parent();
                }
            });
        });
    }
}
