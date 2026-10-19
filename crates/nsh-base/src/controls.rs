//! Base owns native behavior; NSH owns content, state and visual composition.
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::{
    AnyElement, App, AppContext, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Styled, Subscription, Window,
    base::{
        self,
        input::{InputEvent as NativeInputEvent, InputState},
    },
    prelude::FluentBuilder,
};
use nsh_core::ui::{Children, Event, Props};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::Value;
use std::{cell::RefCell, ops::Range, rc::Rc};

#[derive(Clone, PartialEq, Props)]
pub(crate) struct ButtonProps {
    children: Children,
    #[prop(default)]
    disabled: bool,
    accessibility_label: Option<String>,
    #[prop(default = true)]
    tab_stop: bool,
    #[prop(default)]
    tab_index: i64,
}
#[derive(Clone, PartialEq, Event)]
pub(crate) enum ButtonEvent {
    Press,
    Focus,
    Blur,
    #[allow(dead_code)] // The shared typed decoder reports invalid props.
    Error(String),
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct CheckboxProps {
    children: Children,
    checked: bool,
    #[prop(default)]
    indeterminate: bool,
    #[prop(default)]
    disabled: bool,
    accessibility_label: Option<String>,
    #[prop(default = true)]
    tab_stop: bool,
    #[prop(default)]
    tab_index: i64,
}
#[derive(Clone, PartialEq, Event)]
pub(crate) enum CheckboxEvent {
    Change(bool),
    Focus,
    Blur,
    #[allow(dead_code)] // The shared typed decoder reports invalid props.
    Error(String),
}

/// A retained focus handle and real window focus notifications, without a skin.
#[derive(Default)]
struct ControlFocus {
    handle: Option<FocusHandle>,
    subscriptions: Vec<Subscription>,
    disabled: bool,
    skip_tab: bool,
    tab_index: isize,
}
impl ControlFocus {
    fn ensure<E: nsh_core::ui::Event + Clone>(
        &mut self,
        context: &TypedComponentContext<E>,
        focused: E,
        blurred: E,
        window: &mut Window,
        cx: &mut App,
    ) -> FocusHandle {
        self.handle
            .get_or_insert_with(|| {
                let handle = cx.focus_handle();
                let focus_context = context.clone();
                let blur_context = context.clone();
                self.subscriptions = vec![
                    window
                        .on_focus_in(&handle, cx, move |_, _| focus_context.emit(focused.clone())),
                    window.on_focus_out(&handle, cx, move |_, _, _| {
                        blur_context.emit(blurred.clone())
                    }),
                ];
                handle
            })
            .clone()
    }
    fn configure(&mut self, disabled: bool, tab_stop: bool, tab_index: i64) {
        self.disabled = disabled;
        self.tab_index = tab_index as isize;
        self.skip_tab = !tab_stop || tab_index < 0;
    }
    fn entry(&self) -> Option<FocusHandle> {
        self.handle
            .clone()
            .filter(|_| !self.disabled && !self.skip_tab)
    }
}

pub(crate) struct Button {
    context: TypedComponentContext<ButtonEvent>,
    focus: ControlFocus,
}
impl TypedNativeComponent for Button {
    type Props = ButtonProps;
    type Event = ButtonEvent;
    fn props_changed(&mut self, p: &ButtonProps, _: &[String], _: &mut Window, _: &mut App) {
        self.focus.configure(p.disabled, p.tab_stop, p.tab_index);
    }
    fn render(&mut self, p: &ButtonProps, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let focus = self.focus.ensure(
            &self.context,
            ButtonEvent::Focus,
            ButtonEvent::Blur,
            window,
            cx,
        );
        let context = self.context.clone();
        Some(
            base::Button::new("control")
                .track_focus(&focus)
                .disabled(p.disabled)
                .tab_stop(p.tab_stop && p.tab_index >= 0 && !p.disabled)
                .tab_index(p.tab_index as isize)
                .when_some(p.accessibility_label.clone(), |button, label| {
                    button.accessibility_label(label)
                })
                // Fill the host rectangle. The NSH subtree supplies its own layout.
                .size_full()
                .children(crate::bridge::children(self.context.raw()))
                .on_click(move |_, _, _| context.emit(ButtonEvent::Press))
                .into_any_element(),
        )
    }
    fn tab_index(&self) -> isize {
        self.focus.tab_index
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.handle.clone().filter(|_| !self.focus.disabled)
    }
    fn focus_handles(&self) -> Vec<FocusHandle> {
        self.focus.entry().into_iter().collect()
    }
    fn command(
        &mut self,
        p: &ButtonProps,
        name: &str,
        _: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        if name == "focus" && !p.disabled {
            let focus = self.focus.ensure(
                &self.context,
                ButtonEvent::Focus,
                ButtonEvent::Blur,
                window,
                cx,
            );
            window.focus(&focus, cx);
        }
    }
}

pub(crate) struct Checkbox {
    context: TypedComponentContext<CheckboxEvent>,
    focus: ControlFocus,
}
impl TypedNativeComponent for Checkbox {
    type Props = CheckboxProps;
    type Event = CheckboxEvent;
    fn props_changed(&mut self, p: &CheckboxProps, _: &[String], _: &mut Window, _: &mut App) {
        self.focus.configure(p.disabled, p.tab_stop, p.tab_index);
    }
    fn render(
        &mut self,
        p: &CheckboxProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let focus = self.focus.ensure(
            &self.context,
            CheckboxEvent::Focus,
            CheckboxEvent::Blur,
            window,
            cx,
        );
        let context = self.context.clone();
        Some(
            base::Checkbox::new("control")
                .track_focus(&focus)
                .checked(p.checked)
                .indeterminate(p.indeterminate)
                .disabled(p.disabled)
                .tab_stop(p.tab_stop && p.tab_index >= 0 && !p.disabled)
                .tab_index(p.tab_index as isize)
                .when_some(p.accessibility_label.clone(), |checkbox, label| {
                    checkbox.accessibility_label(label)
                })
                .size_full()
                .children(crate::bridge::children(self.context.raw()))
                .on_change(move |state, _, _, _| {
                    context.emit(CheckboxEvent::Change(state == base::CheckboxState::Checked))
                })
                .into_any_element(),
        )
    }
    fn tab_index(&self) -> isize {
        self.focus.tab_index
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.handle.clone().filter(|_| !self.focus.disabled)
    }
    fn focus_handles(&self) -> Vec<FocusHandle> {
        self.focus.entry().into_iter().collect()
    }
    fn command(
        &mut self,
        p: &CheckboxProps,
        name: &str,
        _: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        if name == "focus" && !p.disabled {
            let focus = self.focus.ensure(
                &self.context,
                CheckboxEvent::Focus,
                CheckboxEvent::Blur,
                window,
                cx,
            );
            window.focus(&focus, cx);
        }
    }
}

/// A controlled single-line Base editor. Equal echoes preserve native selection.
#[derive(Clone, PartialEq, Props)]
pub(crate) struct InputProps {
    value: String,
    #[prop(default)]
    placeholder: String,
    #[prop(default)]
    disabled: bool,
    #[prop(default)]
    readonly: bool,
    accessibility_label: Option<String>,
    #[prop(default = true)]
    tab_stop: bool,
    #[prop(default)]
    tab_index: i64,
}
#[derive(Clone, PartialEq, Event)]
pub(crate) enum InputEvent {
    Change(String),
    Focus,
    Blur,
    Submit {
        secondary: bool,
        shift: bool,
    },
    #[allow(dead_code)] // The shared typed decoder reports invalid props.
    Error(String),
}
#[derive(Default, Clone)]
struct EditSnapshot {
    value: String,
    selection: Range<usize>,
}
pub(crate) struct Input {
    context: TypedComponentContext<InputEvent>,
    state: Option<Entity<InputState>>,
    subscriptions: Vec<Subscription>,
    focus: ControlFocus,
    snapshot: Rc<RefCell<EditSnapshot>>,
    pending: Rc<RefCell<Option<EditSnapshot>>>,
}
impl Input {
    fn state(&mut self, p: &InputProps, window: &mut Window, cx: &mut App) -> Entity<InputState> {
        if let Some(state) = &self.state {
            return state.clone();
        }
        let state = cx.new(|cx| InputState::new(window, cx));
        state.update(cx, |state, cx| {
            state.set_value(p.value.clone(), window, cx);
            state.set_placeholder(p.placeholder.clone(), window, cx);
            state.set_disabled(p.disabled, cx);
            state.set_readonly(p.readonly, cx);
        });
        self.focus.handle = Some(state.focus_handle(cx));
        *self.snapshot.borrow_mut() = EditSnapshot {
            value: p.value.clone(),
            selection: state.read(cx).selected_range(),
        };
        let context = self.context.clone();
        let snapshot = self.snapshot.clone();
        let pending = self.pending.clone();
        let listener = window.subscribe(&state, cx, move |state, event, _, cx| match event {
            NativeInputEvent::Change => {
                *pending.borrow_mut() = Some(snapshot.borrow().clone());
                context.emit(InputEvent::Change(state.read(cx).value().to_string()));
            }
            NativeInputEvent::Focus => context.emit(InputEvent::Focus),
            NativeInputEvent::Blur => context.emit(InputEvent::Blur),
            NativeInputEvent::PressEnter { secondary, shift } => context.emit(InputEvent::Submit {
                secondary: *secondary,
                shift: *shift,
            }),
        });
        let snapshot = self.snapshot.clone();
        let observer = cx.observe(&state, move |state, cx| {
            let state = state.read(cx);
            *snapshot.borrow_mut() = EditSnapshot {
                value: state.value().to_string(),
                selection: state.selected_range(),
            };
        });
        self.subscriptions = vec![listener, observer];
        self.state = Some(state.clone());
        state
    }
}
impl TypedNativeComponent for Input {
    type Props = InputProps;
    type Event = InputEvent;
    fn props_changed(
        &mut self,
        p: &InputProps,
        changed: &[String],
        window: &mut Window,
        cx: &mut App,
    ) {
        self.focus.configure(p.disabled, p.tab_stop, p.tab_index);
        let Some(state) = self.state.clone() else {
            return;
        };
        state.update(cx, |state, cx| {
            if changed.iter().any(|name| name == "value") {
                let pending = self.pending.borrow_mut().take();
                if state.value().as_str() != p.value {
                    state.set_value(p.value.clone(), window, cx);
                    if let Some(before) = pending.filter(|before| before.value == p.value) {
                        state.set_selected_range(before.selection, cx);
                    }
                }
            }
            if changed.iter().any(|name| name == "placeholder") {
                state.set_placeholder(p.placeholder.clone(), window, cx);
            }
            state.set_disabled(p.disabled, cx);
            state.set_readonly(p.readonly, cx);
        });
    }
    fn render(&mut self, p: &InputProps, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let state = self.state(p, window, cx);
        let focus = state
            .focus_handle(cx)
            .tab_stop(p.tab_stop && p.tab_index >= 0 && !p.disabled)
            .tab_index(p.tab_index as isize);
        Some(
            self.context
                .inherit_content_insets(
                    self.context.inherit_text_style(
                        base::InputBase::new("control")
                            .track_focus(&focus)
                            .disabled(p.disabled)
                            .when_some(p.accessibility_label.clone(), |input, label| {
                                input.accessibility_label(label)
                            })
                            .aria_value(state.read(cx).value())
                            .size_full()
                            .border_0()
                            .child(base::Input::new(&state)),
                    ),
                )
                .into_any_element(),
        )
    }
    fn tab_index(&self) -> isize {
        self.focus.tab_index
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.handle.clone().filter(|_| !self.focus.disabled)
    }
    fn focus_handles(&self) -> Vec<FocusHandle> {
        self.focus.entry().into_iter().collect()
    }
    fn command(
        &mut self,
        p: &InputProps,
        name: &str,
        _: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        if p.disabled {
            return;
        }
        let state = self.state(p, window, cx);
        state.update(cx, |state, cx| match name {
            "focus" => state.focus(window, cx),
            "selectAll" => state.select_all(window, cx),
            _ => {}
        });
    }
}

pub(crate) fn register(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<Button>("gpui-base.Button", "Button", |context| Button {
        context,
        focus: ControlFocus::default(),
    });
    registry.register::<Checkbox>("gpui-base.Checkbox", "Checkbox", |context| Checkbox {
        context,
        focus: ControlFocus::default(),
    });
    registry.register::<Input>("gpui-base.Input", "Input", |context| Input {
        context,
        state: None,
        subscriptions: vec![],
        focus: ControlFocus::default(),
        snapshot: Rc::default(),
        pending: Rc::default(),
    });
    registry.metadata().native_controlled(
        "gpui-base.Checkbox",
        &[("change", &["checked", "indeterminate"])],
    );
    registry
        .metadata()
        .native_controlled("gpui-base.Input", &[("change", &["value"])]);
}
