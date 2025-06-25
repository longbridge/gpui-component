use std::{panic::Location, rc::Rc, sync::Arc};

use gpui::{
    App, Bounds, BoxShadow, Corners, DispatchPhase, Element, ElementInputHandler, Entity,
    FocusHandle, GlobalElementId, Hitbox, InspectorElementId, InteractiveElement, Interactivity,
    IntoElement, LayoutId, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, RenderImage, Size,
    StyleRefinement, Styled, Window, hsla, point, px, size,
};
use wef::{Browser, LogicalUnit, Rect};

use crate::{WebView, utils::*};

pub(crate) struct WebViewElement {
    webview: Entity<WebView>,
    focus_handle: FocusHandle,
    browser: Rc<Browser>,
    interactivity: Interactivity,
    view_image: Arc<RenderImage>,
    popup_image: Option<(Rect<LogicalUnit<i32>>, Arc<RenderImage>)>,
}

impl WebViewElement {
    pub(crate) fn new(
        webview: Entity<WebView>,
        focus_handle: FocusHandle,
        browser: Rc<Browser>,
        view_image: Arc<RenderImage>,
        popup_image: Option<(Rect<LogicalUnit<i32>>, Arc<RenderImage>)>,
    ) -> Self {
        Self {
            webview,
            focus_handle,
            browser,
            interactivity: Interactivity::default(),
            view_image,
            popup_image,
        }
    }
}

impl IntoElement for WebViewElement {
    type Element = WebViewElement;

    #[inline]
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Styled for WebViewElement {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.interactivity.base_style
    }
}

impl InteractiveElement for WebViewElement {
    fn interactivity(&mut self) -> &mut Interactivity {
        &mut self.interactivity
    }
}

impl Element for WebViewElement {
    type RequestLayoutState = ();
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<gpui::ElementId> {
        self.interactivity.element_id.clone()
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout_id = self.interactivity.request_layout(
            global_id,
            inspector_id,
            window,
            cx,
            |style, window, cx| window.request_layout(style, None, cx),
        );
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.webview.update(cx, |webview, _| {
            webview.bounds = bounds;
        });

        self.interactivity.prepaint(
            global_id,
            inspector_id,
            bounds,
            bounds.size,
            window,
            cx,
            |_, _, hit_box, _, _| hit_box,
        )
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.handle_input(
            &self.focus_handle,
            ElementInputHandler::new(bounds, self.webview.clone()),
            cx,
        );

        self.interactivity.paint(
            global_id,
            inspector_id,
            bounds,
            hitbox.as_ref(),
            window,
            cx,
            |_, window, _cx| {
                let scale_factor = window.scale_factor();
                self.browser.resize(wef::Size::new(
                    wef::PhysicalUnit((bounds.size.width.0 * scale_factor) as i32),
                    wef::PhysicalUnit((bounds.size.height.0 * scale_factor) as i32),
                ));

                let image_size = self.view_image.size(0);
                _ = window.paint_image(
                    Bounds::new(
                        bounds.origin,
                        Size::new(
                            px(image_size.width.0 as f32 / scale_factor),
                            px(image_size.height.0 as f32 / scale_factor),
                        ),
                    ),
                    Corners::all(px(0.0)),
                    self.view_image.clone(),
                    0,
                    false,
                );

                if let Some((rect, image)) = &self.popup_image {
                    let bounds = Bounds::new(
                        point(px(rect.x.0 as f32), px(rect.y.0 as f32)),
                        size(px(rect.width.0 as f32), px(rect.height.0 as f32)),
                    ) + bounds.origin;

                    let shadows = &[
                        BoxShadow {
                            color: hsla(0., 0., 0., 0.1),
                            offset: point(px(0.), px(10.)),
                            blur_radius: px(15.),
                            spread_radius: px(-3.),
                        },
                        BoxShadow {
                            color: hsla(0., 0., 0., 0.1),
                            offset: point(px(0.), px(4.)),
                            blur_radius: px(6.),
                            spread_radius: px(-4.),
                        },
                    ];
                    window.paint_shadows(bounds, Corners::all(px(0.0)), shadows);

                    _ = window.paint_image(bounds, Corners::all(px(0.0)), image.clone(), 0, false);
                }
            },
        );

        let cursor_style = self.webview.read(cx).cursor;
        window.set_cursor_style(cursor_style, hitbox.as_ref().unwrap());

        window.on_mouse_event({
            let entity = self.webview.clone();
            move |event: &MouseMoveEvent, phase, _, cx| {
                let webview = entity.read(cx);
                if phase == DispatchPhase::Bubble
                    && (event.dragging() || bounds.contains(&event.position))
                {
                    let position = event.position - bounds.origin;
                    webview.browser().send_mouse_move_event(
                        wef::Point::new(
                            wef::LogicalUnit(position.x.0 as i32),
                            wef::LogicalUnit(position.y.0 as i32),
                        ),
                        to_wef_key_modifiers(&event.modifiers),
                    );
                }
            }
        });

        window.on_mouse_event({
            let entity = self.webview.clone();
            move |event: &MouseDownEvent, phase, _, cx| {
                let webview = entity.read(cx);
                if phase == DispatchPhase::Bubble && bounds.contains(&event.position) {
                    if let Some(mouse_button) = to_wef_mouse_button(event.button) {
                        let modifiers = to_wef_key_modifiers(&event.modifiers);
                        webview.browser().send_mouse_click_event(
                            mouse_button,
                            false,
                            event.click_count,
                            modifiers,
                        );
                        webview.browser().set_focus(true);
                    }
                }
            }
        });

        window.on_mouse_event({
            let entity = self.webview.clone();
            move |event: &MouseUpEvent, phase, _, cx| {
                let webview = entity.read(cx);
                if phase == DispatchPhase::Bubble && bounds.contains(&event.position) {
                    if let Some(mouse_button) = to_wef_mouse_button(event.button) {
                        let modifiers = to_wef_key_modifiers(&event.modifiers);
                        webview.browser().send_mouse_click_event(
                            mouse_button,
                            true,
                            event.click_count,
                            modifiers,
                        );
                    }
                }
            }
        });
    }
}
