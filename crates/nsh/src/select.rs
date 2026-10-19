use gpui_kit::{
    AnyElement, App, AppContext, Entity, FocusHandle, Focusable, IntoElement, SharedString,
    Subscription, Window,
    component::select::{SearchableVec, Select, SelectEvent, SelectItem, SelectState},
    prelude::FluentBuilder,
};
use nsh_core::ui::{Event, JsObject, PropError, Props};
use nsh_platform_gpui::{ComponentContext, TypedComponentContext, TypedNativeComponent};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct SelectOption {
    value: String,
    label: String,
    #[prop(default)]
    disabled: bool,
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct SelectProps {
    items: Vec<SelectOption>,
    value: Option<String>,
    #[prop(default)]
    searchable: bool,
    #[prop(default)]
    disabled: bool,
    #[prop(default)]
    cleanable: bool,
    #[prop(default)]
    accessibility_label: String,
    placeholder: Option<String>,
}
#[derive(Event)]
pub(crate) enum SelectEvents {
    Change(Option<String>),
    #[allow(
        dead_code,
        reason = "The typed host emits validation errors through this declared event."
    )]
    Error(String),
}
pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeSelect>("gpui-component.Select", "Select", NativeSelect::new);
    registry
        .metadata()
        .native_controlled("gpui-component.Select", &[("change", &["value"])]);
}
#[derive(Clone)]
struct Item {
    value: SharedString,
    label: SharedString,
    disabled: bool,
    context: ComponentContext,
}
impl SelectItem for Item {
    type Value = SharedString;
    fn title(&self) -> SharedString {
        self.label.clone()
    }
    fn value(&self) -> &SharedString {
        &self.value
    }
    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        crate::bridge::slot(&self.context, &format!("item:{}", self.value))
            .unwrap_or_else(|| self.label.clone().into_any_element())
    }
    fn display_title(&self) -> Option<AnyElement> {
        crate::bridge::slot(&self.context, &format!("value:{}", self.value))
    }
    fn disabled(&self) -> bool {
        self.disabled
    }
}
type State = SelectState<SearchableVec<Item>>;
pub(super) struct NativeSelect {
    context: TypedComponentContext<SelectEvents>,
    state: Option<Entity<State>>,
    events: Option<Subscription>,
    focus: Option<FocusHandle>,
    props: SelectProps,
}
impl NativeSelect {
    fn new(context: TypedComponentContext<SelectEvents>) -> Self {
        Self {
            context,
            state: None,
            events: None,
            focus: None,
            props: SelectProps::initial(),
        }
    }
    fn state(&mut self, window: &mut Window, cx: &mut App) -> Entity<State> {
        if let Some(state) = &self.state {
            return state.clone();
        }
        let state =
            cx.new(|cx| SelectState::new(SearchableVec::new(Vec::<Item>::new()), None, window, cx));
        let context = self.context.clone();
        self.events = Some(window.subscribe(&state, cx, move |_, event, _, _| {
            let SelectEvent::Confirm(value) = event;
            context.emit(SelectEvents::Change(
                value.as_ref().map(ToString::to_string),
            ));
        }));
        self.state = Some(state.clone());
        state
    }
    fn sync_focus(&mut self, state: &Entity<State>, window: &mut Window, cx: &mut App) {
        let focus = state.focus_handle(cx);
        if self
            .focus
            .as_ref()
            .is_some_and(|previous| previous != &focus && previous.is_focused(window))
        {
            window.focus(&focus, cx);
        }
        self.focus = Some(focus);
    }
}
impl TypedNativeComponent for NativeSelect {
    type Props = SelectProps;
    type Event = SelectEvents;
    fn validate_props(&self, props: &SelectProps) -> Result<(), PropError> {
        let mut identities = HashSet::new();
        if props
            .items
            .iter()
            .any(|item| !identities.insert(&item.value))
        {
            return Err(PropError("Select items must have unique values".into()));
        }
        Ok(())
    }
    fn render(
        &mut self,
        props: &SelectProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let state = self.state(window, cx);
        self.sync_focus(&state, window, cx);
        Some(
            Select::new(&state)
                .accessibility_label(props.accessibility_label.clone())
                .when_some(props.placeholder.clone(), |select, placeholder| {
                    select.placeholder(placeholder)
                })
                .cleanable(props.cleanable)
                .disabled(props.disabled)
                .into_any_element(),
        )
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone().filter(|_| !self.props.disabled)
    }
    fn props_changed(
        &mut self,
        props: &SelectProps,
        changed: &[String],
        window: &mut Window,
        cx: &mut App,
    ) {
        let items_changed = self.props.items != props.items;
        let value_changed =
            self.props.value != props.value || changed.iter().any(|field| field == "value");
        self.props = props.clone();
        let state = self.state(window, cx);
        state.update(cx, |state, cx| {
            state.set_searchable(props.searchable, cx);
            if items_changed {
                state.set_items(
                    SearchableVec::new(
                        props
                            .items
                            .iter()
                            .map(|item| Item {
                                value: item.value.clone().into(),
                                label: item.label.clone().into(),
                                disabled: item.disabled,
                                context: self.context.raw().clone(),
                            })
                            .collect::<Vec<_>>(),
                    ),
                    window,
                    cx,
                );
            }
            let value = props
                .value
                .as_deref()
                .map(|value| SharedString::from(value.to_owned()));
            if (items_changed || value_changed) && state.selected_value() != value.as_ref() {
                match value {
                    Some(value) => state.set_selected_value(&value, window, cx),
                    None => state.set_selected_index(None, window, cx),
                }
            }
        });
        self.sync_focus(&state, window, cx);
    }
    fn command(
        &mut self,
        props: &SelectProps,
        name: &str,
        _: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        if name == "focus" && !props.disabled {
            let focus = self.state(window, cx).focus_handle(cx);
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
        }
    }
}
