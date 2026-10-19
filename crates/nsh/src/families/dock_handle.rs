//! Standalone native resize edge. The NSH owner applies reported deltas.
use super::types::{ResizeHandleEvent, ResizeHandleProps};
use gpui_kit::StatefulInteractiveElement as _;
use gpui_kit::component::resizable::{self, HandleEdge};
use gpui_kit::{
    AnyElement, App, Axis, Context, DispatchPhase, Empty, IntoElement, MouseMoveEvent,
    MouseUpEvent, ParentElement as _, Render, Styled as _, Window, canvas, div,
};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use std::{cell::RefCell, rc::Rc};

pub(super) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeHandle>(
        "gpui-component.ResizeHandle",
        "ResizeHandle",
        NativeHandle::new,
    );
}
struct DragView;
impl Render for DragView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}
#[derive(Default)]
struct DragState {
    start: Option<f32>,
    delta: f32,
}
struct NativeHandle {
    context: TypedComponentContext<ResizeHandleEvent>,
    state: Rc<RefCell<DragState>>,
    vertical: bool,
    trailing: bool,
    disabled: bool,
    label: String,
}
impl NativeHandle {
    fn new(context: TypedComponentContext<ResizeHandleEvent>) -> Self {
        Self {
            context,
            state: Rc::default(),
            vertical: false,
            trailing: false,
            disabled: false,
            label: "Resize edge".into(),
        }
    }
}
impl TypedNativeComponent for NativeHandle {
    type Props = ResizeHandleProps;
    type Event = ResizeHandleEvent;
    fn render(&mut self, _: &ResizeHandleProps, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        if self.disabled {
            self.state.borrow_mut().start = None;
            return None;
        }
        let axis = if self.vertical {
            Axis::Vertical
        } else {
            Axis::Horizontal
        };
        let vertical = self.vertical;
        let state = self.state.clone();
        let context = self.context.clone();
        let handle = resizable::resize_handle::<(), DragView>("standalone-resize", axis)
            .role(gpui_kit::Role::Splitter)
            .aria_label(self.label.clone())
            .inside(if self.trailing {
                HandleEdge::Trailing
            } else {
                HandleEdge::Leading
            })
            .on_drag((), move |_, _, window, cx| {
                let position = window.mouse_position();
                let mut state = state.borrow_mut();
                state.start = Some(if vertical {
                    position.y.into()
                } else {
                    position.x.into()
                });
                state.delta = 0.;
                context.emit(ResizeHandleEvent::Start { delta: 0. });
                use gpui_kit::AppContext as _;
                cx.new(|_| DragView)
            });
        let state = self.state.clone();
        let context = self.context.clone();
        let up_state = state.clone();
        let up_context = context.clone();
        let observer = canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                let state = state.clone();
                let context = context.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                    if phase != DispatchPhase::Capture {
                        return;
                    }
                    let mut state = state.borrow_mut();
                    let Some(start) = state.start else { return };
                    if !cx.has_active_drag() {
                        state.start = None;
                        context.emit(ResizeHandleEvent::End { delta: state.delta });
                        return;
                    }
                    state.delta = f32::from(if vertical {
                        event.position.y
                    } else {
                        event.position.x
                    }) - start;
                    context.emit(ResizeHandleEvent::Resize { delta: state.delta });
                });
                let state = up_state.clone();
                let context = up_context.clone();
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _, _| {
                    if phase != DispatchPhase::Capture {
                        return;
                    }
                    let mut state = state.borrow_mut();
                    if let Some(start) = state.start.take() {
                        state.delta = f32::from(if vertical {
                            event.position.y
                        } else {
                            event.position.x
                        }) - start;
                        context.emit(ResizeHandleEvent::End { delta: state.delta });
                    }
                });
            },
        );
        Some(
            div()
                .relative()
                .size_full()
                .child(handle)
                .child(observer)
                .into_any_element(),
        )
    }
    fn props_changed(
        &mut self,
        props: &ResizeHandleProps,
        _: &[String],
        _: &mut Window,
        _: &mut App,
    ) {
        self.vertical = props.vertical;
        self.trailing = props.trailing;
        self.disabled = props.disabled;
        self.label = props.accessibility_label.clone();
    }
}
impl Drop for NativeHandle {
    fn drop(&mut self) {
        self.state.borrow_mut().start = None;
    }
}
