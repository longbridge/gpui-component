use std::{rc::Rc, sync::Arc};

use gpui::{AnyWindowHandle, AppContext, AsyncApp, ParentElement, RenderImage, Styled, WeakEntity};
use gpui_component::{
    ContextModal,
    input::{InputState, TextInput},
    v_flex,
};
use wef::{
    BrowserHandler, ContextMenuParams, CursorInfo, CursorType, DirtyRects, Frame, ImageBuffer,
    JsDialogCallback, JsDialogType, LogSeverity, LogicalUnit, PaintElementType, Point, Rect,
};

use crate::{
    WebView,
    context_menu::{ContextMenuInfo, build_context_menu},
    events::*,
    frame_view::FrameView,
    utils::from_wef_cursor_type,
};

/// A Handler implementation for the WebView.
pub(crate) struct WebViewHandler {
    window_handle: AnyWindowHandle,
    entity: WeakEntity<WebView>,
    cx: AsyncApp,
}

impl WebViewHandler {
    pub(crate) fn new(
        window_handle: AnyWindowHandle,
        entity: WeakEntity<WebView>,
        cx: AsyncApp,
    ) -> Self {
        Self {
            window_handle,
            entity,
            cx,
        }
    }
}

impl BrowserHandler for WebViewHandler {
    fn on_popup_show(&mut self, show: bool) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |webview, _cx| {
                webview.popup = show.then(FrameView::default);
            });
        }
    }

    fn on_popup_position(&mut self, rect: Rect<LogicalUnit<i32>>) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |webview, _cx| {
                webview.popup_rect = Some(rect);
            });
        }
    }

    fn on_created(&mut self) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |_webview, cx| {
                cx.emit(CreatedEvent);
            });
        }
    }

    fn on_paint(
        &mut self,
        type_: PaintElementType,
        _dirty_rects: &DirtyRects,
        image_buffer: ImageBuffer,
    ) {
        let image = Arc::new(RenderImage::new([image::Frame::new(
            image::ImageBuffer::from_vec(
                image_buffer.width(),
                image_buffer.height(),
                image_buffer.to_vec(),
            )
            .unwrap(),
        )]));

        _ = self.entity.update(&mut self.cx, |webview, cx| {
            match type_ {
                PaintElementType::View => webview.main.update(image),
                PaintElementType::Popup => {
                    if let Some(popup_frame) = &mut webview.popup {
                        popup_frame.update(image);
                    }
                }
            }
            cx.notify();
        });
    }

    fn on_address_changed(&mut self, frame: Frame, url: &str) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |_, cx| {
                cx.emit(AddressChangedEvent {
                    frame,
                    url: url.to_string(),
                });
            });
        }
    }

    fn on_title_changed(&mut self, title: &str) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |_, cx| {
                cx.emit(TitleChangedEvent {
                    title: title.to_string(),
                });
            });
        }
    }

    fn on_tooltip(&mut self, text: &str) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |_, cx| {
                cx.emit(TooltipEvent {
                    text: text.to_string(),
                });
            });
        }
    }

    fn on_status_message(&mut self, text: &str) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |_, cx| {
                cx.emit(StatusMessageEvent {
                    text: text.to_string(),
                });
            });
        }
    }

    fn on_console_message(
        &mut self,
        message: &str,
        level: LogSeverity,
        source: &str,
        line_number: i32,
    ) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |_, cx| {
                cx.emit(ConsoleMessageEvent {
                    message: message.to_string(),
                    level,
                    source: source.to_string(),
                    line_number,
                });
            });
        }
    }

    fn on_before_popup(&mut self, url: &str) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |_, cx| {
                cx.emit(BeforePopupEvent {
                    url: url.to_string(),
                });
            });
        }
    }

    fn on_loading_progress_changed(&mut self, progress: f32) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |_, cx| {
                cx.emit(LoadingProgressChangedEvent { progress });
            });
        }
    }

    fn on_loading_state_changed(
        &mut self,
        is_loading: bool,
        can_go_back: bool,
        can_go_forward: bool,
    ) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |_, cx| {
                cx.emit(LoadingStateChangedEvent {
                    is_loading,
                    can_go_back,
                    can_go_forward,
                });
            });
        }
    }

    fn on_load_start(&mut self, frame: Frame) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |_, cx| {
                cx.emit(LoadStartEvent { frame });
            });
        }
    }

    fn on_load_end(&mut self, frame: Frame) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |_, cx| {
                cx.emit(LoadEndEvent { frame });
            });
        }
    }

    fn on_load_error(&mut self, frame: Frame, error_text: &str, failed_url: &str) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |_, cx| {
                cx.emit(LoadErrorEvent {
                    frame,
                    error_text: error_text.to_string(),
                    failed_url: failed_url.to_string(),
                });
            });
        }
    }

    fn on_cursor_changed(
        &mut self,
        cursor_type: CursorType,
        _cursor_info: Option<CursorInfo>,
    ) -> bool {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_entity(&entity, |webview, cx| {
                webview.cursor = from_wef_cursor_type(cursor_type);
                cx.notify();
            });
        }
        true
    }

    fn on_context_menu(&mut self, frame: Frame, params: ContextMenuParams) {
        if let Some(entity) = self.entity.upgrade() {
            _ = self.cx.update_window(self.window_handle, |_, window, cx| {
                cx.update_entity(&entity, |webview, cx| {
                    webview.context_menu = Some(ContextMenuInfo {
                        crood: Point::new(
                            LogicalUnit(params.crood.x.0 + webview.bounds.origin.x.0 as i32),
                            LogicalUnit(params.crood.y.0 + webview.bounds.origin.y.0 as i32),
                        ),
                        frame,
                        menu: build_context_menu(webview, &params, window, cx),
                        link_url: params.link_url.map(ToString::to_string),
                    });
                    cx.notify();
                })
            });
        }
    }

    fn on_js_dialog(
        &mut self,
        type_: JsDialogType,
        message_text: &str,
        callback: JsDialogCallback,
    ) -> bool {
        _ = self.cx.update_window(self.window_handle, |_, window, cx| {
            let message_text = message_text.to_string();
            let callback = Rc::new(callback);

            match type_ {
                JsDialogType::Alert => {
                    window.open_modal(cx, move |modal, _, _| {
                        modal
                            .footer(|ok, _, window, cx| vec![ok(window, cx)])
                            .child(message_text.clone())
                            .on_ok({
                                let callback = callback.clone();
                                move |_, _, _| {
                                    callback.continue_(true, None);
                                    true
                                }
                            })
                    });
                }
                JsDialogType::Confirm => {
                    window.open_modal(cx, move |modal, _, _| {
                        modal
                            .footer(|ok, cancel, window, cx| {
                                vec![ok(window, cx), cancel(window, cx)]
                            })
                            .child(message_text.clone())
                            .on_ok({
                                let callback = callback.clone();
                                move |_, _, _| {
                                    callback.continue_(true, None);
                                    true
                                }
                            })
                            .on_cancel({
                                let callback = callback.clone();
                                move |_, _, _| {
                                    callback.continue_(false, None);
                                    true
                                }
                            })
                    });
                }
                JsDialogType::Prompt {
                    default_prompt_text,
                } => {
                    let default_prompt_text = default_prompt_text.to_string();
                    let input_state =
                        cx.new(|cx| InputState::new(window, cx).default_value(default_prompt_text));
                    window.open_modal(cx, move |modal, _, _| {
                        modal
                            .footer(move |ok, cancel, window, cx| {
                                vec![ok(window, cx), cancel(window, cx)]
                            })
                            .child(
                                v_flex()
                                    .gap_3()
                                    .child(message_text.clone())
                                    .child(TextInput::new(&input_state)),
                            )
                            .on_ok({
                                let callback = callback.clone();
                                let input_state = input_state.clone();
                                move |_, _, cx| {
                                    callback.continue_(true, Some(&input_state.read(cx).value()));
                                    true
                                }
                            })
                            .on_cancel({
                                let callback = callback.clone();
                                move |_, _, _| {
                                    callback.continue_(false, None);
                                    true
                                }
                            })
                    });
                }
            }
        });

        true
    }
}
