use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, IntoElement, ParentElement, Pixels, SharedString, Styled, Window,
    div, relative,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, StyledExt as _, WindowExt as _,
    dialog::{Dialog, DialogButtonProps},
    h_flex, v_flex,
};

type RenderButtonFn = Box<dyn FnOnce(&mut Window, &mut App) -> AnyElement>;
type FooterFn =
    Box<dyn Fn(RenderButtonFn, RenderButtonFn, &mut Window, &mut App) -> Vec<AnyElement>>;

/// AlertDialog is a modal dialog that interrupts the user with important content
/// and expects a response.
///
/// It is built on top of the Dialog component with opinionated defaults:
/// - Footer buttons are center-aligned (vs right-aligned in Dialog)
/// - Icon is optional (disabled by default, enable with `.show_icon(true)`)
/// - Simplified API for common alert scenarios
///
/// # Examples
///
/// ```ignore
/// use gpui_component::{AlertDialog, alert::AlertVariant};
///
/// // Using WindowExt trait
/// window.open_alert_dialog(cx, |alert, _, _| {
///     alert.warning()
///         .title("Unsaved Changes")
///         .description("You have unsaved changes. Are you sure you want to leave?")
///         .show_cancel(true)
///         .show_icon(true)  // Optional: show icon based on variant
/// });
/// ```
pub struct AlertDialog {
    base: Dialog,
    icon: Option<AnyElement>,
    title: Option<AnyElement>,
    description: Option<AnyElement>,
    button_props: DialogButtonProps,
    show_cancel: bool,
    footer: Option<FooterFn>,

    pub(crate) on_action: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static>>,
    pub(crate) on_cancel: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static>>,
}

impl AlertDialog {
    /// Create a new alert dialog.
    pub(crate) fn new(base: Dialog) -> Self {
        Self {
            base,
            title: None,
            description: None,
            button_props: DialogButtonProps::default(),
            show_cancel: false,
            icon: None,
            footer: None,
            on_action: None,
            on_cancel: None,
        }
    }

    /// Sets the icon of the alert dialog, default is None.
    pub fn icon(mut self, icon: impl IntoElement) -> Self {
        self.icon = Some(icon.into_any_element());
        self
    }

    /// Sets the title of the alert dialog.
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self
    }

    /// Sets the description of the alert dialog.
    pub fn description(mut self, description: impl IntoElement) -> Self {
        self.description = Some(description.into_any_element());
        self
    }

    /// Sets the text of the action button. Default is "OK".
    pub fn action_text(mut self, action_text: impl Into<SharedString>) -> Self {
        self.button_props = self.button_props.ok_text(action_text);
        self
    }

    /// Sets the text of the cancel button. Default is "Cancel".
    pub fn cancel_text(mut self, cancel_text: impl Into<SharedString>) -> Self {
        self.button_props = self.button_props.cancel_text(cancel_text);
        self
    }

    /// Set the button props of the alert dialog.
    pub fn button_props(mut self, button_props: DialogButtonProps) -> Self {
        self.button_props = button_props;
        self
    }

    /// Sets the width of the alert dialog, defaults to 420px.
    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.base = self.base.width(width);
        self
    }

    /// Show cancel button. Default is false.
    pub fn show_cancel(mut self, show_cancel: bool) -> Self {
        self.show_cancel = show_cancel;
        self
    }

    /// Set the footer of the alert dialog.
    ///
    /// The `footer` is a function that takes two `RenderButtonFn` and returns a list of elements.
    ///
    /// - First `RenderButtonFn` is the render function for the action button.
    /// - Second `RenderButtonFn` is the render function for the cancel button.
    ///
    /// When you set a custom footer, it will replace the default center-aligned buttons.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// window.open_alert_dialog(cx, |alert, _, _| {
    ///     alert.warning()
    ///         .title("Custom Footer")
    ///         .footer(|action, cancel, window, cx| {
    ///             vec![
    ///                 cancel(window, cx),
    ///                 action(window, cx),
    ///             ]
    ///         })
    /// });
    /// ```
    pub fn footer<E, F>(mut self, footer: F) -> Self
    where
        E: IntoElement,
        F: Fn(RenderButtonFn, RenderButtonFn, &mut Window, &mut App) -> Vec<E> + 'static,
    {
        self.footer = Some(Box::new(move |action, cancel, window, cx| {
            footer(action, cancel, window, cx)
                .into_iter()
                .map(|e| e.into_any_element())
                .collect()
        }));
        self
    }

    /// Set the overlay closable of the alert dialog, defaults to `false`.
    ///
    /// When the overlay is clicked, the dialog will be closed.
    pub fn overlay_closable(mut self, overlay_closable: bool) -> Self {
        self.base = self.base.overlay_closable(overlay_closable);
        self
    }

    /// Set whether to support keyboard esc to close the dialog, defaults to `true`.
    pub fn keyboard(mut self, keyboard: bool) -> Self {
        self.base = self.base.keyboard(keyboard);
        self
    }

    /// Sets the callback for when the alert dialog is closed.
    ///
    /// Called after [`Self::on_action`] or [`Self::on_cancel`] callback.
    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.base = self.base.on_close(on_close);
        self
    }

    /// Sets the callback for when the action button is clicked.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_action(
        mut self,
        on_action: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.on_action = Some(Rc::new(on_action));
        self
    }

    /// Sets the callback for when the alert dialog has been canceled.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_cancel(
        mut self,
        on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.on_cancel = Some(Rc::new(on_cancel));
        self
    }

    /// Convert AlertDialog into a configured Dialog.
    pub(crate) fn into_dialog(self, _window: &mut Window, cx: &mut App) -> Dialog {
        use crate::button::{Button, ButtonVariants as _};

        let action_text = self
            .button_props
            .ok_text
            .unwrap_or_else(|| t!("Dialog.ok").into());
        let cancel_text = self
            .button_props
            .cancel_text
            .unwrap_or_else(|| t!("Dialog.cancel").into());

        let action_variant = self.button_props.ok_variant;
        let cancel_variant = self.button_props.cancel_variant;

        // Create render button closures
        let render_action: RenderButtonFn = Box::new({
            let on_action = self.on_action.clone();
            let action_text = action_text.clone();
            move |_, _| {
                Button::new("action")
                    .label(action_text)
                    .with_variant(action_variant)
                    .on_click({
                        let on_action = on_action.clone();
                        move |_, window, cx| {
                            let should_close = if let Some(on_action) = &on_action {
                                on_action(&gpui::ClickEvent::default(), window, cx)
                            } else {
                                true
                            };

                            if should_close {
                                window.close_dialog(cx);
                            }
                        }
                    })
                    .into_any_element()
            }
        });

        let render_cancel: RenderButtonFn = Box::new({
            let on_cancel = self.on_cancel.clone();
            let cancel_text = cancel_text.clone();
            move |_, _| {
                Button::new("cancel")
                    .label(cancel_text)
                    .with_variant(cancel_variant)
                    .on_click({
                        let on_cancel = on_cancel.clone();
                        move |_, window, cx| {
                            let should_close = if let Some(on_cancel) = &on_cancel {
                                on_cancel(&gpui::ClickEvent::default(), window, cx)
                            } else {
                                true
                            };

                            if should_close {
                                window.close_dialog(cx);
                            }
                        }
                    })
                    .into_any_element()
            }
        });

        let mut title_desc = v_flex().gap_2().items_center();
        if let Some(title) = self.title {
            title_desc = title_desc.child(
                div()
                    .font_semibold()
                    .text_lg()
                    .line_height(relative(1.4))
                    .text_center()
                    .child(title),
            );
        }
        if let Some(desc) = self.description {
            title_desc = title_desc.child(
                div()
                    .text_sm()
                    .line_height(relative(1.6))
                    .text_color(cx.theme().muted_foreground)
                    .text_center()
                    .child(desc),
            );
        }

        let mut main_content = v_flex().gap_4().items_center();
        if let Some(icon) = self.icon {
            main_content = main_content.child(icon);
        }
        main_content = main_content.child(title_desc);

        // Use custom footer if provided, otherwise use default center-aligned layout
        let footer_content = if let Some(footer_fn) = self.footer {
            h_flex()
                .gap_2()
                .justify_center()
                .line_height(relative(1.))
                .children(footer_fn(render_action, render_cancel, _window, cx))
        } else {
            let mut footer = h_flex().gap_2().justify_center().line_height(relative(1.));
            if self.show_cancel {
                footer = footer.child(render_cancel(_window, cx));
            }
            footer = footer.child(render_action(_window, cx));
            footer
        };

        let content = v_flex()
            .gap_6()
            .items_center()
            .child(main_content)
            .child(footer_content);

        self.base.close_button(false).child(content)
    }
}
