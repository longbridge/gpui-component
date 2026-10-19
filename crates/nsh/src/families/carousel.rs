//! Retained carousel selection with live NSH item slots.
use crate::bridge::slot;
use gpui_kit::component::carousel::{
    Carousel, CarouselContent, CarouselEvent, CarouselItem, CarouselNext, CarouselPagination,
    CarouselPaginationItem, CarouselPrevious, CarouselState,
};
use gpui_kit::{
    AnyElement, App, AppContext, Axis, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Styled, Subscription, Window, div, prelude::FluentBuilder,
};
use nsh_core::ui::{Children, Event, PropError, Props};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::Value;

#[derive(Clone, PartialEq, Props)]
pub(crate) struct CarouselProps {
    items: Vec<String>,
    value: Option<usize>,
    #[prop(default)]
    vertical: bool,
    #[prop(default)]
    looping: bool,
    #[prop(default = true)]
    controls: bool,
    #[prop(default = true)]
    pagination: bool,
    accessibility_label: String,
    children: Children,
}
#[derive(Clone, Event)]
pub(crate) enum NativeCarouselEvent {
    Change(usize),
    // The shared typed host emits validation failures.
    #[allow(dead_code)]
    Error(String),
}
pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeCarousel>("gpui-component.Carousel", "Carousel", |context| {
        NativeCarousel {
            context,
            state: None,
            subscription: None,
            focus: None,
        }
    });
}
struct NativeCarousel {
    context: TypedComponentContext<NativeCarouselEvent>,
    state: Option<Entity<CarouselState>>,
    subscription: Option<Subscription>,
    focus: Option<FocusHandle>,
}
impl TypedNativeComponent for NativeCarousel {
    type Props = CarouselProps;
    type Event = NativeCarouselEvent;
    fn validate_props(&self, props: &CarouselProps) -> Result<(), PropError> {
        let mut seen = std::collections::HashSet::new();
        if props
            .items
            .iter()
            .any(|id| id.is_empty() || !seen.insert(id))
        {
            return Err(PropError(
                "Carousel item ids must be unique nonempty strings".into(),
            ));
        }
        Ok(())
    }
    fn render(
        &mut self,
        props: &CarouselProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let state = self
            .state
            .get_or_insert_with(|| {
                let state = cx.new(|_| CarouselState::new(props.items.len()));
                let context = self.context.clone();
                self.subscription =
                    Some(
                        window.subscribe(&state, cx, move |_, event: &CarouselEvent, _, _| {
                            let CarouselEvent::Change(index) = event;
                            context.emit(NativeCarouselEvent::Change(*index));
                            context.redraw();
                        }),
                    );
                self.focus = Some(state.focus_handle(cx));
                state
            })
            .clone();
        state.update(cx, |state, cx| {
            state.set_item_count(props.items.len(), cx);
            state.set_axis(
                if props.vertical {
                    Axis::Vertical
                } else {
                    Axis::Horizontal
                },
                cx,
            );
            state.set_looping(props.looping, cx);
            if let Some(selected) = props.value {
                if state.selected_index() != Some(selected) {
                    state.set_selected_index(selected, cx);
                }
            }
        });
        let content = CarouselContent::new(&state).children(props.items.iter().enumerate().map(
            |(index, id)| {
                CarouselItem::new(id.clone(), index, &state)
                    .child(slot(&self.context, id).unwrap_or_else(|| div().into_any_element()))
            },
        ));
        let mut parts = vec![content.into_any_element()];
        if props.controls {
            parts.push(CarouselPrevious::new(&state).into_any_element());
            parts.push(CarouselNext::new(&state).into_any_element());
        }
        if props.pagination {
            parts.push(
                CarouselPagination::new()
                    .children(
                        props.items.iter().enumerate().map(|(index, id)| {
                            CarouselPaginationItem::new(id.clone(), index, &state)
                        }),
                    )
                    .into_any_element(),
            );
        }
        // Component's standard controls sit one control width plus one
        // spacing step outside its content frame. NSH clips a native node to
        // its allocated rectangle, so reserve that relative-scale gutter
        // inside the node instead of placing hit targets outside its bounds.
        Some(
            div()
                .size_full()
                .when(props.controls, |this| {
                    if props.vertical {
                        this.py_12()
                    } else {
                        this.px_12()
                    }
                })
                .child(
                    Carousel::new("carousel", &state)
                        .w_full()
                        .accessibility_label(props.accessibility_label.clone())
                        .children(parts),
                )
                .into_any_element(),
        )
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone()
    }
    fn command(&mut self, _: &CarouselProps, name: &str, _: &Value, _: &mut Window, cx: &mut App) {
        if let Some(state) = &self.state {
            state.update(cx, |state, cx| match name {
                "next" => {
                    state.select_next(cx);
                }
                "previous" => {
                    state.select_previous(cx);
                }
                _ => {}
            });
        }
    }
}
