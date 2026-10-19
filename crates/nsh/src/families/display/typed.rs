//! Props and events are the native registry's single source of JS declarations.
use gpui_kit::component::{
    self, Disableable, Icon, Selectable, Sizable, Size,
    button::{ButtonRounded, ButtonVariants},
    progress::ProgressCircle,
    spinner::Spinner,
};
use gpui_kit::{
    AnyElement, App, FocusHandle, IntoElement, ParentElement, SharedString, Window,
    prelude::FluentBuilder, px,
};
use nsh_core::ui::{Children, Either, Event, JsObject, PropEnum, Props};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::Value;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum ComponentSizeName {
    Xs,
    Sm,
    Md,
    Lg,
}
crate::registration::named_value! { pub(crate) struct ComponentSize(Either<ComponentSizeName, f64>); }
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum ButtonRadius {
    None,
    Small,
    Medium,
    Large,
}
pub(crate) fn size(value: &Option<ComponentSize>) -> Size {
    match value.as_ref().map(ComponentSize::value) {
        Some(Either::Left(ComponentSizeName::Xs)) => Size::XSmall,
        Some(Either::Left(ComponentSizeName::Sm)) => Size::Small,
        Some(Either::Left(ComponentSizeName::Lg)) => Size::Large,
        Some(Either::Right(v)) if *v > 0. => Size::Size(px(*v as f32)),
        _ => Size::Medium,
    }
}

fn text(value: &Option<String>) -> Option<SharedString> {
    value.clone().map(Into::into)
}

#[derive(Clone, Copy, Default, PartialEq, PropEnum)]
pub(crate) enum ButtonVariant {
    #[default]
    Default,
    Primary,
    Secondary,
    Danger,
    Success,
    Warning,
    Info,
    Ghost,
    Link,
    Text,
}
impl ButtonVariant {
    fn resolve(self) -> component::button::ButtonVariant {
        use component::button::ButtonVariant as V;
        match self {
            Self::Default => V::Default,
            Self::Primary => V::Primary,
            Self::Secondary => V::Secondary,
            Self::Danger => V::Danger,
            Self::Success => V::Success,
            Self::Warning => V::Warning,
            Self::Info => V::Info,
            Self::Ghost => V::Ghost,
            Self::Link => V::Link,
            Self::Text => V::Text,
        }
    }
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum SpinnerIconStyle {
    Spinner,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum ProgressIconStyle {
    Progress,
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct SpinnerIconOptions {
    kind: SpinnerIconStyle,
    path: Option<String>,
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct ProgressIconOptions {
    kind: ProgressIconStyle,
    value: Option<f64>,
    #[prop(default)]
    loading: bool,
}
type ButtonIcon = Either<String, Either<SpinnerIconOptions, ProgressIconOptions>>;

fn apply_icon(
    button: component::button::Button,
    value: Option<&ButtonIcon>,
    component_size: &Option<ComponentSize>,
) -> component::button::Button {
    button.when_some(value, |button, icon| match icon {
        Either::Left(path) => button.icon(Icon::default().path(path.clone())),
        Either::Right(Either::Left(icon)) => button.icon(
            Spinner::new()
                .with_size(size(component_size))
                .when_some(icon.path.clone(), |s, path| {
                    s.icon(Icon::default().path(path))
                }),
        ),
        Either::Right(Either::Right(icon)) => button.icon(
            ProgressCircle::new("button-progress")
                .with_size(size(component_size))
                .value(icon.value.unwrap_or(0.) as f32)
                .loading(icon.loading),
        ),
    })
}

/// A native GPUI Component Button. Pointer activation preserves keyboard focus.
#[derive(Clone, PartialEq, Props)]
pub(crate) struct ButtonProps {
    children: Children,
    label: Option<String>,
    #[prop(default)]
    variant: ButtonVariant,
    icon: Option<ButtonIcon>,
    loading_icon: Option<String>,
    #[prop(default)]
    loading: bool,
    #[prop(default)]
    selected: bool,
    toggled: Option<bool>,
    #[prop(default)]
    outline: bool,
    #[prop(default)]
    compact: bool,
    #[prop(default)]
    dropdown_caret: bool,
    rounded: Option<Either<ButtonRadius, f64>>,
    /// Milliseconds before showing a tooltip.
    tooltip_delay: Option<u64>,
    size: Option<ComponentSize>,
    #[prop(default)]
    disabled: bool,
    #[prop(default = true)]
    tab_stop: bool,
    #[prop(default)]
    tab_index: i64,
    accessibility_label: Option<String>,
    tooltip: Option<String>,
}
#[derive(Clone, PartialEq, Event)]
pub(crate) enum ButtonEvent {
    Press,
    Hover(bool),
}

pub(crate) struct Button {
    context: TypedComponentContext<ButtonEvent>,
    focus: Option<FocusHandle>,
    enabled: bool,
    tab_stop: bool,
    tab_index: isize,
}
impl TypedNativeComponent for Button {
    type Props = ButtonProps;
    type Event = ButtonEvent;
    fn render(&mut self, p: &ButtonProps, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let focus = self.focus.get_or_insert_with(|| cx.focus_handle()).clone();
        let press = self.context.clone();
        let hover = self.context.clone();
        Some(
            component::button::Button::new("control")
                .track_focus(&focus)
                .when_some(text(&p.label), |b, v| b.label(v))
                .disabled(p.disabled)
                .with_size(size(&p.size))
                .with_variant(p.variant.resolve())
                .loading(p.loading)
                .selected(p.selected)
                .when_some(p.toggled, |b, v| b.toggled(v))
                .tab_stop(p.tab_stop && p.tab_index >= 0 && !p.disabled && !p.loading)
                .tab_index(p.tab_index as isize)
                .dropdown_caret(p.dropdown_caret)
                .when(p.outline, |b| b.outline())
                .when(p.compact, |b| b.compact())
                .when_some(p.rounded.as_ref(), |b, v| match v {
                    Either::Right(v) => b.rounded(px(*v as f32)),
                    Either::Left(v) => b.rounded(match v {
                        ButtonRadius::None => ButtonRounded::None,
                        ButtonRadius::Small => ButtonRounded::Small,
                        ButtonRadius::Large => ButtonRounded::Large,
                        ButtonRadius::Medium => ButtonRounded::default(),
                    }),
                })
                .map(|button| apply_icon(button, p.icon.as_ref(), &p.size))
                .when_some(p.loading_icon.clone(), |b, path| {
                    b.loading_icon(Icon::default().path(path))
                })
                .when_some(text(&p.tooltip), |b, v| b.tooltip(v))
                .when_some(p.tooltip_delay, |b, v| {
                    b.tooltip_show_delay(Duration::from_millis(v))
                })
                .when_some(text(&p.accessibility_label), |b, v| {
                    b.accessibility_label(v)
                })
                .children(crate::bridge::children(self.context.raw()))
                .on_hover(move |v, _, _| hover.emit(ButtonEvent::Hover(*v)))
                .on_click(move |_, _, _| press.emit(ButtonEvent::Press))
                .into_any_element(),
        )
    }
    fn props_changed(&mut self, p: &ButtonProps, _: &[String], _: &mut Window, _: &mut App) {
        self.enabled = !p.disabled && !p.loading;
        self.tab_stop = p.tab_stop && p.tab_index >= 0;
        self.tab_index = p.tab_index as isize;
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.enabled.then(|| self.focus.clone()).flatten()
    }
    fn tab_index(&self) -> isize {
        self.tab_index
    }
    fn focus_handles(&self) -> Vec<FocusHandle> {
        self.focus_handle()
            .filter(|_| self.tab_stop)
            .into_iter()
            .collect()
    }
    fn command(
        &mut self,
        p: &ButtonProps,
        name: &str,
        _: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        if name == "focus" && !p.disabled && !p.loading {
            let focus = self.focus.get_or_insert_with(|| cx.focus_handle());
            window.focus(focus, cx);
        }
    }
}

use crate::registration::native_leaf;

native_leaf!(
    /// A native avatar with an optional image or icon placeholder.
    #[derive(JsObject)]
    Avatar(AvatarProps, context, cx) => component::avatar::Avatar::new(); {
    name: Option<String> => optional;
    src: Option<String> => optional;
    placeholder: Option<String> => mapped(|path|Icon::default().path(path));
    size: Option<ComponentSize> => size(size);
});
native_leaf!(
    /// An animated native loading placeholder.
    Skeleton(SkeletonProps, context, cx) => component::skeleton::Skeleton::new(); {
    secondary: bool => flag;
});
#[path = "remaining.rs"]
mod remaining;

pub(crate) fn register(registry: &mut impl crate::registration::BindingRegistry) {
    remaining::register(registry);
    registry.register::<Button>("gpui-component.Button", "Button", |context| Button {
        context,
        focus: None,
        enabled: true,
        tab_stop: true,
        tab_index: 0,
    });
    registry.register::<Avatar>("gpui-component.Avatar", "Avatar", Avatar::new);
    registry.register::<Skeleton>("gpui-component.Skeleton", "Skeleton", Skeleton::new);
}
