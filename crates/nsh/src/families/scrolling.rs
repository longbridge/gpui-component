//! Native scroll surfaces and the original GPUI Component virtual list.
use crate::bridge::{children, slot};
use gpui_kit::component::{
    VirtualListScrollHandle, h_virtual_list, scroll::ScrollableElement, v_virtual_list,
};
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Window, div, px, size,
};
use nsh_core::ui::{Children, Event, PropError, Props};
use nsh_platform_gpui::{ComponentContext, TypedComponentContext, TypedNativeComponent};
use serde_json::Value;
use std::rc::Rc;
pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    for (name, export, horizontal) in [
        ("gpui-component.VirtualList", "VirtualList", false),
        (
            "gpui-component.HorizontalVirtualList",
            "HorizontalVirtualList",
            true,
        ),
    ] {
        registry.register::<Virtual>(name, export, move |context| Virtual {
            context,
            horizontal,
            view: None,
            handle: VirtualListScrollHandle::new(),
        });
    }
    registry.register::<Scroll>("gpui-component.ScrollArea", "ScrollArea", Scroll::new);
}
crate::registration::native_leaf!(Scroll(ScrollProps, context, cx, p) => {
    let content = div().id("scroll-area").size_full().children(children(context));
    if p.horizontal { content.overflow_x_scrollbar().into_any_element() }
    else { content.overflow_y_scrollbar().into_any_element() }
}; {
    horizontal: bool => skip;
    children: Children => skip;
});
#[derive(Clone, PartialEq, Props)]
pub(crate) struct VirtualListProps {
    items: Vec<String>,
    sizes: Vec<[f64; 2]>,
    children: Children,
}
#[derive(Clone, Event)]
pub(crate) enum VirtualEvent {
    // The shared typed host emits validation failures.
    #[allow(dead_code)]
    Error(String),
}
struct Virtual {
    context: TypedComponentContext<VirtualEvent>,
    horizontal: bool,
    view: Option<Entity<Rows>>,
    handle: VirtualListScrollHandle,
}
struct Rows {
    context: ComponentContext,
    items: Vec<String>,
}
impl Render for Rows {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
impl TypedNativeComponent for Virtual {
    type Props = VirtualListProps;
    type Event = VirtualEvent;
    fn validate_props(&self, props: &VirtualListProps) -> Result<(), PropError> {
        validate_layout(&props.items, &props.sizes).map_err(|error| PropError(error.into()))
    }
    fn render(
        &mut self,
        props: &VirtualListProps,
        _: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let view = self
            .view
            .get_or_insert_with(|| {
                cx.new(|_| Rows {
                    context: self.context.raw().clone(),
                    items: props.items.clone(),
                })
            })
            .clone();
        view.update(cx, |view, _| view.items = props.items.clone());
        let render = |view: &mut Rows,
                      range: std::ops::Range<usize>,
                      _: &mut Window,
                      _: &mut Context<Rows>| {
            range
                .filter_map(|index| view.items.get(index))
                .map(|id| slot(&view.context, id).unwrap_or_else(|| div().into_any_element()))
                .collect::<Vec<_>>()
        };
        let sizes = Rc::new(
            props
                .sizes
                .iter()
                .map(|[w, h]| size(px(*w as f32), px(*h as f32)))
                .collect(),
        );
        Some(if self.horizontal {
            h_virtual_list(view, "virtual-list", sizes, render)
                .track_scroll(&self.handle)
                .into_any_element()
        } else {
            v_virtual_list(view, "virtual-list", sizes, render)
                .track_scroll(&self.handle)
                .into_any_element()
        })
    }
    fn command(
        &mut self,
        props: &VirtualListProps,
        name: &str,
        args: &Value,
        _: &mut Window,
        _: &mut App,
    ) {
        if name == "scrollToItem" {
            if let Some(index) = args
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
                .filter(|index| *index < props.items.len())
            {
                self.handle
                    .scroll_to_item(index, gpui_kit::ScrollStrategy::Top);
                self.context.redraw();
            }
        }
    }
}

fn validate_layout(items: &[String], sizes: &[[f64; 2]]) -> Result<(), &'static str> {
    let mut seen = std::collections::HashSet::new();
    if items.iter().any(|id| id.is_empty() || !seen.insert(id)) {
        return Err("VirtualList item ids must be unique nonempty strings");
    }
    if sizes.iter().any(|pair| {
        pair.iter()
            .any(|value| !value.is_finite() || *value <= 0. || *value > f32::MAX as f64)
    }) {
        return Err("VirtualList dimensions must be positive finite pixel values");
    }
    if items.len() != sizes.len() {
        return Err(
            "VirtualList items and sizes must have the same length; keeping the previous valid layout",
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_identity_and_unrepresentable_sizes() {
        assert!(
            validate_layout(&["same".into(), "same".into()], &[[12., 20.], [12., 20.]]).is_err()
        );
        assert!(validate_layout(&["a".into()], &[[12., 1e100]]).is_err());
        assert!(validate_layout(&["a".into()], &[[12., -1.]]).is_err());
        assert!(validate_layout(&["a".into()], &[]).is_err());
        assert!(validate_layout(&["a".into()], &[[12., 20.]]).is_ok());
    }
}
