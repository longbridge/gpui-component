use std::{cell::Cell, rc::Rc, time::Duration};

use gpui::{
    prelude::FluentBuilder, AnyElement, App, ClipboardItem, Element, ElementId, GlobalElementId,
    IntoElement, LayoutId, ParentElement, SharedString, Styled, Window,
};

use crate::{
    button::{Button, ButtonVariants as _},
    h_flex, IconName, Sizable as _,
};

pub struct Clipboard {
    id: ElementId,
    value: SharedString,
    value_fn: Option<Rc<dyn Fn(&mut Window, &mut App) -> SharedString>>,
    content_builder: Option<Box<dyn Fn(&mut Window, &mut App) -> AnyElement>>,
    copied_callback: Option<Rc<dyn Fn(SharedString, &mut Window, &mut App)>>,
}

impl Clipboard {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: SharedString::default(),
            value_fn: None,
            content_builder: None,
            copied_callback: None,
        }
    }

    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = value.into();
        self
    }

    /// Set the value of the clipboard to the result of the given function. Default is None.
    ///
    /// When used this, the copy value will use the result of the function.
    pub fn value_fn(
        mut self,
        value: impl Fn(&mut Window, &mut App) -> SharedString + 'static,
    ) -> Self {
        self.value_fn = Some(Rc::new(value));
        self
    }

    pub fn on_copied<F>(mut self, handler: F) -> Self
    where
        F: Fn(SharedString, &mut Window, &mut App) + 'static,
    {
        self.copied_callback = Some(Rc::new(handler));
        self
    }

    pub fn content<E, F>(mut self, builder: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.content_builder = Some(Box::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        self
    }
}

impl IntoElement for Clipboard {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[derive(Default)]
pub struct ClipboardState {
    copied: Cell<bool>,
}

impl Element for Clipboard {
    type RequestLayoutState = AnyElement;

    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        window.with_element_state::<ClipboardState, _>(global_id.unwrap(), |state, window| {
            let state = state.unwrap_or_default();

            let content_element = self
                .content_builder
                .as_ref()
                .map(|builder| builder(window, cx).into_any_element());
            let value = self.value.clone();
            let clipboard_id = self.id.clone();
            let copied_callback = self.copied_callback.as_ref().map(|c| c.clone());
            let copied = state.copied.clone();
            let copide_value = copied.get();
            let value_fn = self.value_fn.clone();

            let mut element = h_flex()
                .gap_1()
                .items_center()
                .when_some(content_element, |this, element| this.child(element))
                .child(
                    Button::new(clipboard_id)
                        .icon(if copide_value {
                            IconName::Check
                        } else {
                            IconName::Copy
                        })
                        .ghost()
                        .xsmall()
                        .when(!copide_value, |this| {
                            this.on_click(move |_, window, cx| {
                                cx.stop_propagation();
                                let value = value_fn
                                    .as_ref()
                                    .map(|f| f(window, cx))
                                    .unwrap_or_else(|| value.clone());
                                cx.write_to_clipboard(ClipboardItem::new_string(value.to_string()));
                                copied.set(true);

                                let copied = copied.clone();
                                cx.spawn(async move |cx| {
                                    cx.background_executor().timer(Duration::from_secs(2)).await;

                                    copied.set(false);
                                })
                                .detach();

                                if let Some(callback) = &copied_callback {
                                    callback(value.clone(), window, cx);
                                }
                            })
                        }),
                )
                .into_any_element();

            ((element.request_layout(window, cx), element), state)
        })
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        element: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        element: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx)
    }
}
