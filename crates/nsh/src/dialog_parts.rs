use crate::{bridge::children, registration::native_leaf};
use gpui_kit::{
    AnyElement, App, ClickEvent, FocusHandle, IntoElement, ParentElement, SharedString, Window,
    component::{
        button::Button,
        dialog::{
            Confirm, DialogAction, DialogClose, DialogContent, DialogDescription, DialogFooter,
            DialogHeader, DialogTitle,
        },
    },
};
use nsh_core::ui::{Children, NoEvent, Props};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::Value;
use std::marker::PhantomData;

native_leaf!(NativeDialogContent(DialogContentProps, context, cx, p) =>
DialogContent::new().child(p.text.clone()).children(children(context)); {
    text: String => skip; children: Children => skip;
});
native_leaf!(NativeDialogDescription(DialogDescriptionProps, context, cx, p) =>
DialogDescription::new().child(p.text.clone()).children(children(context)); {
    text: String => skip; children: Children => skip;
});
native_leaf!(NativeDialogHeader(DialogHeaderProps, context, cx, p) =>
DialogHeader::new().child(p.text.clone()).children(children(context)); {
    text: String => skip; children: Children => skip;
});
native_leaf!(NativeDialogFooter(DialogFooterProps, context, cx, p) =>
DialogFooter::new().child(p.text.clone()).children(children(context)); {
    text: String => skip; children: Children => skip;
});
native_leaf!(NativeDialogTitle(DialogTitleProps, context, cx, p) =>
DialogTitle::new().child(p.text.clone()).children(children(context)); {
    text: String => skip; children: Children => skip;
});

trait DialogControlProps: Props {
    const ACTION: bool;
    fn text(&self) -> &str;
}
macro_rules! dialog_control_props {
    ($props:ident, $action:literal) => {
        #[derive(Clone, PartialEq, Props)]
        struct $props {
            #[prop(default)]
            text: String,
            children: Children,
        }
        impl DialogControlProps for $props {
            const ACTION: bool = $action;
            fn text(&self) -> &str {
                &self.text
            }
        }
    };
}
dialog_control_props!(DialogCloseProps, false);
dialog_control_props!(DialogActionProps, true);

pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeDialogContent>(
        "gpui-component.DialogContent",
        "DialogContent",
        NativeDialogContent::new,
    );
    registry.register::<NativeDialogDescription>(
        "gpui-component.DialogDescription",
        "DialogDescription",
        NativeDialogDescription::new,
    );
    registry.register::<NativeDialogHeader>(
        "gpui-component.DialogHeader",
        "DialogHeader",
        NativeDialogHeader::new,
    );
    registry.register::<NativeDialogFooter>(
        "gpui-component.DialogFooter",
        "DialogFooter",
        NativeDialogFooter::new,
    );
    registry.register::<NativeDialogTitle>(
        "gpui-component.DialogTitle",
        "DialogTitle",
        NativeDialogTitle::new,
    );
    registry.register::<DialogControl<DialogCloseProps>>(
        "gpui-component.DialogClose",
        "DialogClose",
        DialogControl::new,
    );
    registry.register::<DialogControl<DialogActionProps>>(
        "gpui-component.DialogAction",
        "DialogAction",
        DialogControl::new,
    );
}

struct DialogControl<P: DialogControlProps> {
    context: TypedComponentContext<NoEvent>,
    focus: Option<FocusHandle>,
    text_only: bool,
    props: PhantomData<P>,
}
impl<P: DialogControlProps> DialogControl<P> {
    fn new(context: TypedComponentContext<NoEvent>) -> Self {
        Self {
            context,
            focus: None,
            text_only: false,
            props: PhantomData,
        }
    }
}
impl<P: DialogControlProps> TypedNativeComponent for DialogControl<P> {
    type Props = P;
    type Event = NoEvent;
    fn render(&mut self, props: &P, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let children = children(&self.context);
        let text: SharedString = props.text().to_owned().into();
        self.text_only = children.is_empty() && !text.is_empty();
        if !self.text_only {
            return Some(if P::ACTION {
                DialogAction::new()
                    .child(text)
                    .children(children)
                    .into_any_element()
            } else {
                DialogClose::new()
                    .child(text)
                    .children(children)
                    .into_any_element()
            });
        }
        let focus = self.focus.get_or_insert_with(|| cx.focus_handle()).clone();
        Some(if P::ACTION {
            DialogAction::new()
                .child(
                    Button::new("dialog-action-button")
                        .track_focus(&focus)
                        .label(text)
                        .on_click(move |event, window, cx| {
                            // GPUI keyboard clicks activate only the focused Button;
                            // pointer clicks bubble to the native DialogAction anchor.
                            if matches!(event, ClickEvent::Keyboard(_))
                                && focus.contains_focused(window, cx)
                            {
                                window.dispatch_action(Box::new(Confirm { secondary: false }), cx);
                            }
                        }),
                )
                .into_any_element()
        } else {
            DialogClose::new()
                .trigger(|button| {
                    button
                        .track_focus(&focus)
                        .accessibility_label(text.clone())
                        .label(text)
                })
                .into_any_element()
        })
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.text_only.then(|| self.focus.clone()).flatten()
    }
    fn command(&mut self, _: &P, name: &str, _: &Value, window: &mut Window, cx: &mut App) {
        if name == "focus"
            && let Some(handle) = self.focus_handle()
        {
            window.focus(&handle, cx);
        }
    }
}
