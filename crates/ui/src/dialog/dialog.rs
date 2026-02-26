use std::{rc::Rc, sync::LazyLock, time::Duration};

use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Bounds, BoxShadow, ClickEvent, Edges,
    FocusHandle, Hsla, InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement,
    Pixels, Point, RenderOnce, SharedString, StyleRefinement, Styled, Window, WindowControlArea,
    actions, anchored, div, hsla, point, prelude::FluentBuilder, px,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, FocusTrapElement as _, IconName, Root, Sizable as _, StyledExt,
    TITLE_BAR_HEIGHT, WindowExt as _,
    animation::cubic_bezier,
    button::{Button, ButtonVariant, ButtonVariants as _},
    dialog::{DialogContent, DialogTitle},
    scroll::ScrollableElement as _,
    v_flex,
};

pub static ANIMATION_DURATION: LazyLock<Duration> = LazyLock::new(|| Duration::from_secs_f64(0.25));
const CONTEXT: &str = "Dialog";

actions!(dialog, [CancelDialog, ConfirmDialog]);

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", CancelDialog, Some(CONTEXT)),
        KeyBinding::new("enter", ConfirmDialog, Some(CONTEXT)),
    ]);
}

/// Dialog button props.
#[derive(Clone)]
pub struct DialogButtonProps {
    pub(crate) ok_text: Option<SharedString>,
    pub(crate) ok_variant: ButtonVariant,
    pub(crate) cancel_text: Option<SharedString>,
    pub(crate) cancel_variant: ButtonVariant,
    pub(crate) show_cancel: bool,
    pub(crate) on_ok: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static>,
    pub(crate) on_cancel: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static>,
    pub(crate) on_close: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
}

impl Default for DialogButtonProps {
    fn default() -> Self {
        Self {
            ok_text: None,
            ok_variant: ButtonVariant::Primary,
            cancel_text: None,
            cancel_variant: ButtonVariant::default(),
            show_cancel: false,
            on_ok: Rc::new(|_, _, _| true),
            on_cancel: Rc::new(|_, _, _| true),
            on_close: Rc::new(|_, _, _| {}),
        }
    }
}

impl DialogButtonProps {
    /// Sets the text of the OK button. Default is `OK`.
    pub fn ok_text(mut self, ok_text: impl Into<SharedString>) -> Self {
        self.ok_text = Some(ok_text.into());
        self
    }

    /// Sets the variant of the OK button. Default is `ButtonVariant::Primary`.
    pub fn ok_variant(mut self, ok_variant: ButtonVariant) -> Self {
        self.ok_variant = ok_variant;
        self
    }

    /// Sets the text of the Cancel button. Default is `Cancel`.
    pub fn cancel_text(mut self, cancel_text: impl Into<SharedString>) -> Self {
        self.cancel_text = Some(cancel_text.into());
        self
    }

    /// Sets the variant of the Cancel button. Default is `ButtonVariant::default()`.
    pub fn cancel_variant(mut self, cancel_variant: ButtonVariant) -> Self {
        self.cancel_variant = cancel_variant;
        self
    }

    /// Sets whether to show the Cancel button. Default is `false`.
    pub fn show_cancel(mut self, show_cancel: bool) -> Self {
        self.show_cancel = show_cancel;
        self
    }

    /// Sets the callback for when the dialog is has been confirmed.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_ok(
        mut self,
        on_ok: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.on_ok = Rc::new(on_ok);
        self
    }

    /// Sets the callback for when the dialog is has been canceled.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_cancel(
        mut self,
        on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.on_cancel = Rc::new(on_cancel);
        self
    }

    pub(crate) fn render_ok(&self, _: &mut Window, _: &mut App) -> AnyElement {
        let on_ok = self.on_ok.clone();
        let on_close = self.on_close.clone();

        let ok_text = self.ok_text.clone().unwrap_or_else(|| t!("Dialog.ok").into());
        let ok_variant = self.ok_variant;

        Button::new("ok")
            .label(ok_text)
            .with_variant(ok_variant)
            .on_click({
                let on_ok = on_ok.clone();
                let on_close = on_close.clone();

                move |_, window, cx| {
                    if on_ok(&ClickEvent::default(), window, cx) {
                        window.close_dialog(cx);
                        on_close(&ClickEvent::default(), window, cx);
                    }
                }
            })
            .into_any_element()
    }

    pub(crate) fn render_cancel(&self, _: &mut Window, _: &mut App) -> AnyElement {
        let on_cancel = self.on_cancel.clone();
        let on_close = self.on_close.clone();
        let cancel_text = self.cancel_text.clone().unwrap_or_else(|| t!("Dialog.cancel").into());
        let cancel_variant = self.cancel_variant;

        Button::new("cancel")
            .label(cancel_text)
            .with_variant(cancel_variant)
            .on_click({
                let on_cancel = on_cancel.clone();
                let on_close = on_close.clone();
                move |_, window, cx| {
                    if !on_cancel(&ClickEvent::default(), window, cx) {
                        return;
                    }

                    window.close_dialog(cx);
                    on_close(&ClickEvent::default(), window, cx);
                }
            })
            .into_any_element()
    }
}

type ContentBuilderFn = Rc<dyn Fn(DialogContent, &mut Window, &mut App) -> DialogContent + 'static>;

#[derive(Clone)]
pub(crate) struct DialogProps {
    width: Pixels,
    max_width: Option<Pixels>,
    margin_top: Option<Pixels>,
    close_button: bool,

    overlay: bool,
    overlay_closable: bool,
    pub(crate) overlay_visible: bool,
    keyboard: bool,
}

impl Default for DialogProps {
    fn default() -> Self {
        Self {
            margin_top: None,
            width: px(448.),
            max_width: None,
            overlay: true,
            keyboard: true,
            overlay_visible: false,
            close_button: true,
            overlay_closable: true,
        }
    }
}

/// A modal to display content in a dialog box.
#[derive(IntoElement)]
pub struct Dialog {
    pub(crate) style: StyleRefinement,
    children: Vec<AnyElement>,
    trigger: Option<AnyElement>,
    title: Option<AnyElement>,
    pub(crate) header: Option<AnyElement>,
    pub(crate) footer: Option<AnyElement>,
    pub(crate) content_builder: Option<ContentBuilderFn>,
    pub(crate) props: DialogProps,

    button_props: DialogButtonProps,

    /// This will be change when open the dialog, the focus handle is create when open the dialog.
    pub(crate) focus_handle: FocusHandle,
    pub(crate) layer_ix: usize,
}

pub(crate) fn overlay_color(overlay: bool, cx: &App) -> Hsla {
    if !overlay {
        return hsla(0., 0., 0., 0.);
    }

    cx.theme().overlay
}

impl Dialog {
    /// Create a new dialog.
    pub fn new(cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            style: StyleRefinement::default(),
            trigger: None,
            title: None,
            header: None,
            footer: None,
            content_builder: None,
            props: DialogProps::default(),
            children: Vec::new(),
            layer_ix: 0,
            button_props: DialogButtonProps::default(),
        }
    }

    /// Sets the trigger element for the dialog.
    ///
    /// When a trigger is set, the dialog will render as a trigger button that opens the dialog when clicked.
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// Sets the content of the dialog.
    pub fn content<F>(mut self, builder: F) -> Self
    where
        F: Fn(DialogContent, &mut Window, &mut App) -> DialogContent + 'static,
    {
        self.content_builder = Some(Rc::new(builder));
        self
    }

    /// Sets the title of the dialog.
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self
    }

    /// Sets the footer of the dialog, the footer will render at the bottom of the dialog, usually for action buttons.
    ///
    /// When you set the footer, the `button_props` will be ignored, you need to render the action buttons by yourself.
    pub(crate) fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }

    /// Sets the footer of the dialog, the footer will render at the bottom of the dialog, usually for action buttons.
    ///
    /// When you set the footer, the `button_props` will be ignored, you need to render the action buttons by yourself.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Set the button props of the dialog.
    pub fn button_props(mut self, button_props: DialogButtonProps) -> Self {
        self.button_props = button_props;
        self
    }

    /// Sets the callback for when the dialog is closed.
    ///
    /// Called after [`Self::on_ok`] or [`Self::on_cancel`] callback.
    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.button_props.on_close = Rc::new(on_close);
        self
    }

    /// Sets the callback for when the dialog is has been confirmed.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_ok(
        mut self,
        on_ok: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.button_props = self.button_props.on_ok(on_ok);
        self
    }

    /// Sets the callback for when the dialog is has been canceled.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_cancel(
        mut self,
        on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.button_props = self.button_props.on_cancel(on_cancel);
        self
    }

    /// Sets the false to hide close icon, default: true
    pub fn close_button(mut self, close_button: bool) -> Self {
        self.props.close_button = close_button;
        self
    }

    /// Set the top offset of the dialog, defaults to None, will use the 1/10 of the viewport height.
    pub fn margin_top(mut self, margin_top: impl Into<Pixels>) -> Self {
        self.props.margin_top = Some(margin_top.into());
        self
    }

    /// Sets the width of the dialog, defaults to 448px.
    ///
    /// See also [`Self::width`]
    pub fn w(mut self, width: impl Into<Pixels>) -> Self {
        self.props.width = width.into();
        self
    }

    /// Sets the width of the dialog, defaults to 448px.
    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.props.width = width.into();
        self
    }

    /// Set the maximum width of the dialog, defaults to `None`.
    pub fn max_w(mut self, max_width: impl Into<Pixels>) -> Self {
        self.props.max_width = Some(max_width.into());
        self
    }

    /// Set the overlay of the dialog, defaults to `true`.
    pub fn overlay(mut self, overlay: bool) -> Self {
        self.props.overlay = overlay;
        self
    }

    /// Set the overlay closable of the dialog, defaults to `true`.
    ///
    /// When the overlay is clicked, the dialog will be closed.
    pub fn overlay_closable(mut self, overlay_closable: bool) -> Self {
        self.props.overlay_closable = overlay_closable;
        self
    }

    /// Set whether to support keyboard esc to close the dialog, defaults to `true`.
    pub fn keyboard(mut self, keyboard: bool) -> Self {
        self.props.keyboard = keyboard;
        self
    }

    pub(crate) fn has_overlay(&self) -> bool {
        self.props.overlay
    }

    pub(crate) fn with_props(mut self, props: DialogProps) -> Self {
        self.props = props;
        self
    }

    fn defer_close_dialog(window: &mut Window, cx: &mut App) {
        Root::update(window, cx, |root, window, cx| {
            root.defer_close_dialog(window, cx);
        });
    }
}

impl ParentElement for Dialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Dialog {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl Dialog {
    fn render_trigger(self, trigger: AnyElement, _: &mut Window, _: &mut App) -> AnyElement {
        let content_builder = self.content_builder.clone();
        let style = self.style.clone();
        let props = self.props.clone();
        let button_props = self.button_props.clone();

        div()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                let content_builder = content_builder.clone();
                let style = style.clone();
                let props = props.clone();
                let button_props = button_props.clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .refine_style(&style)
                        .button_props(button_props.clone())
                        .with_props(props.clone())
                        .content({
                            let content_builder = content_builder.clone();
                            move |content, window, cx| {
                                if let Some(builder) = content_builder.clone() {
                                    builder(content, window, cx)
                                } else {
                                    content
                                }
                            }
                        })
                });
                cx.stop_propagation();
            })
            .child(trigger)
            .into_any_element()
    }
}

impl RenderOnce for Dialog {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(trigger) = self.trigger.take() {
            return self.render_trigger(trigger, window, cx);
        }

        let layer_ix = self.layer_ix;
        let on_close = self.button_props.on_close.clone();
        let on_ok = self.button_props.on_ok.clone();
        let on_cancel = self.button_props.on_cancel.clone();

        let window_paddings = crate::window_border::window_paddings(window);
        let view_size = window.viewport_size()
            - gpui::size(
                window_paddings.left + window_paddings.right,
                window_paddings.top + window_paddings.bottom,
            );
        let bounds = Bounds { origin: Point::default(), size: view_size };
        let offset_top = px(layer_ix as f32 * 16.);
        let y = self.props.margin_top.unwrap_or(view_size.height / 10.) + offset_top;
        let x = bounds.center().x - self.props.width / 2.;

        let base_size = window.text_style().font_size;
        let rem_size = window.rem_size();

        let mut paddings = Edges::all(px(16.));
        if let Some(pl) = self.style.padding.left {
            paddings.left = pl.to_pixels(base_size, rem_size);
        }
        if let Some(pr) = self.style.padding.right {
            paddings.right = pr.to_pixels(base_size, rem_size);
        }
        if let Some(pt) = self.style.padding.top {
            paddings.top = pt.to_pixels(base_size, rem_size);
        }
        if let Some(pb) = self.style.padding.bottom {
            paddings.bottom = pb.to_pixels(base_size, rem_size);
        }

        let animation =
            Animation::new(*ANIMATION_DURATION).with_easing(cubic_bezier(0.32, 0.72, 0., 1.));

        anchored()
            .position(point(window_paddings.left, window_paddings.top))
            .snap_to_window()
            .child(
                div()
                    .id("dialog")
                    .occlude()
                    .w(view_size.width)
                    .h(view_size.height)
                    .when(self.props.overlay_visible, |this| {
                        this.bg(overlay_color(self.props.overlay, cx))
                    })
                    .when(self.props.overlay, |this| {
                        // Only the last dialog owns the `mouse down - close dialog` event.
                        if (self.layer_ix + 1) != Root::read(window, cx).active_dialogs.len() {
                            return this;
                        }

                        this.window_control_area(WindowControlArea::Drag).on_any_mouse_down({
                            let on_cancel = on_cancel.clone();
                            let on_close = on_close.clone();
                            move |event, window, cx| {
                                if event.position.y < TITLE_BAR_HEIGHT {
                                    return;
                                }

                                cx.stop_propagation();
                                if self.props.overlay_closable && event.button == MouseButton::Left
                                {
                                    if on_cancel(&ClickEvent::default(), window, cx) {
                                        on_close(&ClickEvent::default(), window, cx);
                                        window.close_dialog(cx);
                                    }
                                }
                            }
                        })
                    })
                    .child(
                        v_flex()
                            .id(layer_ix)
                            .track_focus(&self.focus_handle)
                            .focus_trap(format!("dialog-{}", layer_ix), &self.focus_handle)
                            .bg(cx.theme().background)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius_lg)
                            .min_h_24()
                            .pt(paddings.top)
                            .pb(paddings.bottom)
                            .gap(paddings.top.max(px(8.)))
                            .refine_style(&self.style)
                            .px_0()
                            .key_context(CONTEXT)
                            .when(self.props.keyboard, |this| {
                                this.on_action({
                                    let on_cancel = on_cancel.clone();
                                    let on_close = on_close.clone();
                                    move |_: &CancelDialog, window, cx| {
                                        // FIXME:
                                        //
                                        // Here some Dialog have no focus_handle, so it will not work will Escape key.
                                        // But by now, we `cx.close_dialog()` going to close the last active model,
                                        // so the Escape is unexpected to work.
                                        if on_cancel(&ClickEvent::default(), window, cx) {
                                            window.close_dialog(cx);
                                            on_close(&ClickEvent::default(), window, cx);
                                        }
                                    }
                                })
                                .on_action({
                                    let on_ok = on_ok.clone();
                                    let on_close = on_close.clone();
                                    move |_: &ConfirmDialog, window, cx| {
                                        if on_ok(&ClickEvent::default(), window, cx) {
                                            Self::defer_close_dialog(window, cx);
                                            on_close(&ClickEvent::default(), window, cx);
                                        }
                                    }
                                })
                            })
                            // There style is high priority, can't be overridden.
                            .absolute()
                            .occlude()
                            .relative()
                            .left(x)
                            .top(y)
                            .w(self.props.width)
                            .when_some(self.props.max_width, |this, w| this.max_w(w))
                            .when_some(self.header, |this, header| {
                                this.child(div().pl(paddings.left).pr(paddings.right).child(header))
                            })
                            .when_some(self.title, |this, title| {
                                this.child(
                                    DialogTitle::new()
                                        .pl(paddings.left)
                                        .pr(paddings.right)
                                        .child(title),
                                )
                            })
                            .when_some(self.content_builder, |this, builder| {
                                this.child(builder(
                                    DialogContent::new()
                                        .gap(paddings.bottom)
                                        .pl(paddings.left)
                                        .pr(paddings.right),
                                    window,
                                    cx,
                                ))
                            })
                            .when(!self.children.is_empty(), |this| {
                                this.child(
                                    div().flex_1().overflow_hidden().child(
                                        // Body
                                        v_flex()
                                            .size_full()
                                            .overflow_y_scrollbar()
                                            .pl(paddings.left)
                                            .pr(paddings.right)
                                            .children(self.children),
                                    ),
                                )
                            })
                            .when_some(self.footer, |this, footer| {
                                this.child(div().pl(paddings.left).pr(paddings.right).child(footer))
                            })
                            .children(self.props.close_button.then(|| {
                                let top = (paddings.top - px(10.)).max(px(8.));
                                let right = (paddings.right - px(10.)).max(px(8.));

                                Button::new("close")
                                    .absolute()
                                    .top(top)
                                    .right(right)
                                    .small()
                                    .ghost()
                                    .icon(IconName::Close)
                                    .on_click({
                                        let on_cancel = self.button_props.on_cancel.clone();
                                        let on_close = self.button_props.on_close.clone();
                                        move |_, window, cx| {
                                            window.close_dialog(cx);
                                            on_cancel(&ClickEvent::default(), window, cx);
                                            on_close(&ClickEvent::default(), window, cx);
                                        }
                                    })
                            }))
                            .on_any_mouse_down({
                                |_, _, cx| {
                                    cx.stop_propagation();
                                }
                            })
                            .with_animation("slide-down", animation.clone(), move |this, delta| {
                                // This is equivalent to `shadow_xl` with an extra opacity.
                                let shadow = vec![
                                    BoxShadow {
                                        color: hsla(0., 0., 0., 0.1 * delta),
                                        offset: point(px(0.), px(20.)),
                                        blur_radius: px(25.),
                                        spread_radius: px(-5.),
                                    },
                                    BoxShadow {
                                        color: hsla(0., 0., 0., 0.1 * delta),
                                        offset: point(px(0.), px(8.)),
                                        blur_radius: px(10.),
                                        spread_radius: px(-6.),
                                    },
                                ];
                                this.top(y * delta).shadow(shadow)
                            }),
                    )
                    .with_animation("fade-in", animation, move |this, delta| this.opacity(delta)),
            )
            .into_any_element()
    }
}
