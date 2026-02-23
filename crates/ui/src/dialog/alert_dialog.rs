use gpui::{
    AnyElement, App, ClickEvent, InteractiveElement as _, IntoElement, MouseButton, ParentElement,
    Pixels, RenderOnce, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    dialog::{
        Dialog, DialogButtonProps, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
    },
};

/// AlertDialog is a modal dialog that interrupts the user with important content
/// and expects a response.
///
/// It is built on top of the Dialog component with opinionated defaults:
/// - Footer buttons are center-aligned (vs right-aligned in Dialog)
/// - Icon is optional (disabled by default, enable with `.show_icon(true)`)
/// - Simplified API for common alert scenarios
/// - Uses declarative DialogHeader, DialogTitle, DialogDescription, and DialogFooter components
/// - Supports both imperative and declarative API styles
///
/// # Examples
///
/// ## Imperative API (using WindowExt)
///
/// ```ignore
/// use gpui_component::{AlertDialog, alert::AlertVariant};
///
/// // Using WindowExt trait
/// window.open_alert_dialog(cx, |alert, _, _| {
///     alert
///         .title("Unsaved Changes")
///         .description("You have unsaved changes. Are you sure you want to leave?")
///         .show_cancel(true)
/// });
/// ```
///
/// ## Declarative API (using trigger and content)
///
/// ```ignore
/// use gpui_component::{AlertDialog, DialogHeader, DialogTitle, DialogDescription, DialogFooter};
///
/// AlertDialog::new(cx)
///     .trigger(Button::new("delete").label("Delete"))
///     .content(|content, _, cx| {
///         content
///             .child(
///                 DialogHeader::new()
///                     .items_center()
///                     .child(DialogTitle::new().child("Delete File"))
///                     .child(DialogDescription::new().child("Are you sure?"))
///             )
///             .child(
///                 DialogFooter::new()
///                     .justify_center()
///                     .child(Button::new("cancel").label("Cancel"))
///                     .child(Button::new("confirm").label("Delete"))
///             )
///     })
/// ```
#[derive(IntoElement)]
pub struct AlertDialog {
    base: Dialog,
    trigger: Option<AnyElement>,
    icon: Option<AnyElement>,
    title: Option<AnyElement>,
    description: Option<AnyElement>,
    button_props: DialogButtonProps,
    children: Vec<AnyElement>,
}

impl AlertDialog {
    /// Create a new AlertDialog.
    ///
    /// By default, the dialog is not overlay closable. You can change this with `.overlay_closable(true)`.
    pub fn new(cx: &mut App) -> Self {
        Self {
            base: Dialog::new(cx).overlay_closable(false).close_button(false),
            trigger: None,
            icon: None,
            title: None,
            description: None,
            button_props: DialogButtonProps::default(),
            children: Vec::new(),
        }
    }

    /// Sets the trigger element for the alert dialog.
    ///
    /// When a trigger is set, the dialog will render as a trigger element that opens the dialog when clicked.
    ///
    /// **Note**: When using `.trigger()`, you should also use `.content()` to define the dialog content
    /// declaratively instead of using `.title()`, `.description()`, etc.
    ///
    /// The `title`, `description`, `icon`, and `button_props` will be ignored when used together with `.trigger()`.
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// Sets the content builder for declarative API.
    ///
    /// When using this method, you define the dialog content using declarative components like
    /// `DialogHeader`, `DialogTitle`, `DialogDescription`, and `DialogFooter`.
    ///
    /// This method is typically used together with `.trigger()` for a fully declarative API.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// AlertDialog::new(cx)
    ///     .trigger(Button::new("delete").label("Delete"))
    ///     .content(|content, _, cx| {
    ///         content
    ///             .child(DialogHeader::new().child(DialogTitle::new().child("Confirm")))
    ///             .child(DialogFooter::new().child(Button::new("ok").label("OK")))
    ///     })
    /// ```
    pub fn content<F>(mut self, builder: F) -> Self
    where
        F: Fn(crate::dialog::DialogContent, &mut Window, &mut App) -> crate::dialog::DialogContent
            + 'static,
    {
        self.base = self.base.content(builder);
        self
    }

    #[track_caller]
    fn debug_assert_no_trigger(&self) {
        debug_assert!(
            self.trigger.is_none() && self.base.content_builder.is_none(),
            "Cannot set this property when trigger is used. Use content() to define dialog content instead."
        );
    }

    /// Sets the icon of the alert dialog, default is None.
    #[track_caller]
    pub fn icon(mut self, icon: impl IntoElement) -> Self {
        self.debug_assert_no_trigger();
        self.icon = Some(icon.into_any_element());
        self
    }

    /// Sets the title of the alert dialog.
    #[track_caller]
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.debug_assert_no_trigger();
        self.title = Some(title.into_any_element());
        self
    }

    /// Sets the description of the alert dialog.
    #[track_caller]
    pub fn description(mut self, description: impl IntoElement) -> Self {
        self.debug_assert_no_trigger();
        self.description = Some(description.into_any_element());
        self
    }

    /// Set the button props of the alert dialog.
    ///
    /// Use this to configure button text, variants, and visibility.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// alert.button_props(
    ///     DialogButtonProps::default()
    ///         .ok_text("Delete")
    ///         .ok_variant(ButtonVariant::Danger)
    ///         .cancel_text("Keep")
    ///         .show_cancel(true)
    /// )
    /// ```
    #[track_caller]
    pub fn button_props(mut self, button_props: DialogButtonProps) -> Self {
        self.debug_assert_no_trigger();
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
        self.button_props = self.button_props.show_cancel(show_cancel);
        self
    }

    /// Set the overlay closable of the alert dialog, defaults to `false`.
    ///
    /// When the overlay is clicked, the dialog will be closed.
    pub fn overlay_closable(mut self, overlay_closable: bool) -> Self {
        self.base = self.base.overlay_closable(overlay_closable);
        self
    }

    /// Set the close button of the alert dialog, defaults to `false`.
    pub fn close_button(mut self, close_button: bool) -> Self {
        self.base = self.base.close_button(close_button);
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

    /// Sets the callback for when the OK/action button is clicked.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_ok(
        mut self,
        on_ok: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.button_props = self.button_props.on_ok(on_ok);
        self
    }

    /// Sets the callback for when the alert dialog has been canceled.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_cancel(
        mut self,
        on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.button_props = self.button_props.on_cancel(on_cancel);
        self
    }

    /// Convert AlertDialog into a configured Dialog.
    pub(crate) fn into_dialog(self, _window: &mut Window, _cx: &mut App) -> Dialog {
        let button_props = self.button_props.clone();
        let action_text = self.button_props.ok_text.unwrap_or_else(|| t!("Dialog.ok").into());
        let cancel_text =
            self.button_props.cancel_text.unwrap_or_else(|| t!("Dialog.cancel").into());

        let action_variant = self.button_props.ok_variant;
        let cancel_variant = self.button_props.cancel_variant;

        self.base
            .button_props(button_props)
            .child(
                DialogHeader::new()
                    .when_some(self.icon, |this, icon| this.child(icon))
                    .when_some(self.title, |this, title| {
                        this.child(DialogTitle::new().child(title))
                    })
                    .when_some(self.description, |this, desc| {
                        this.child(DialogDescription::new().child(desc))
                    }),
            )
            .children(self.children)
            .child(
                DialogFooter::new()
                    .when(self.button_props.show_cancel, |this| {
                        this.child({
                            let on_cancel = self.button_props.on_cancel.clone();
                            Button::new("cancel")
                                .label(cancel_text)
                                .with_variant(cancel_variant)
                                .on_click({
                                    move |_, window, cx| {
                                        if on_cancel(&ClickEvent::default(), window, cx) {
                                            window.close_dialog(cx);
                                        }
                                    }
                                })
                        })
                    })
                    .child({
                        let on_action = self.button_props.on_ok.clone();
                        Button::new("action")
                            .label(action_text)
                            .with_variant(action_variant)
                            .on_click({
                                move |_, window, cx| {
                                    let should_close = if let Some(on_action) = &on_action {
                                        on_action(&ClickEvent::default(), window, cx)
                                    } else {
                                        true
                                    };

                                    if should_close {
                                        window.close_dialog(cx);
                                    }
                                }
                            })
                    }),
            )
    }
}

impl Styled for AlertDialog {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.base.style
    }
}

impl ParentElement for AlertDialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl AlertDialog {
    fn render_trigger(self, trigger: AnyElement, _: &mut Window, _: &mut App) -> AnyElement {
        let content_builder = self.base.content_builder.clone();
        let style = self.base.style.clone();
        let props = self.base.props.clone();

        div()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                let content_builder = content_builder.clone();
                let style = style.clone();
                let props = props.clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog.refine_style(&style).with_props(props.clone()).when_some(
                        content_builder.clone(),
                        |this, content_builder| {
                            this.content(move |content, window, cx| {
                                content_builder(content, window, cx)
                            })
                        },
                    )
                });
                cx.stop_propagation();
            })
            .child(trigger)
            .into_any_element()
    }
}

impl RenderOnce for AlertDialog {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(trigger) = self.trigger.take() {
            // If a trigger is provided, render the trigger element that opens the dialog
            self.render_trigger(trigger, window, cx)
        } else {
            // Otherwise, render the dialog content directly
            self.into_dialog(window, cx).into_any_element()
        }
    }
}
