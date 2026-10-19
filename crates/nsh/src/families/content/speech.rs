//! Native speech controls share a retained session within their window.
use std::collections::HashMap;

use gpui_kit::component::{
    Disableable,
    speech::{SpeechButton, SpeechEvent as NativeSpeechEvent, SpeechState, SpeechWaveform},
};
use gpui_kit::{AnyElement, App, AppContext, Entity, IntoElement, Subscription, Window};
use nsh_core::ui::{Event, Props};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::Value;

pub(super) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<Speech>("gpui-component.SpeechButton", "SpeechButton", |context| {
        Speech::new(SpeechControl::Button, context)
    });
    registry.register::<Speech>(
        "gpui-component.SpeechWaveform",
        "SpeechWaveform",
        |context| Speech::new(SpeechControl::Waveform, context),
    );
}

/// Both controls accept the same options; button presentation options have no
/// effect on the waveform, preserving the existing speech-control contract.
#[derive(Clone, PartialEq, Props)]
pub(crate) struct SpeechProps {
    #[prop(default)]
    session: String,
    #[prop(default)]
    disabled: bool,
    #[prop(default)]
    show_when_unsupported: bool,
}

#[derive(Event)]
pub(crate) enum SpeechEvents {
    Started,
    Partial(String),
    Final(String),
    Cancelled,
    Error(String),
}

#[derive(Default)]
struct SpeechSessions(HashMap<(gpui_kit::WindowId, String), gpui_kit::WeakEntity<SpeechState>>);
impl gpui_kit::Global for SpeechSessions {}

enum SpeechControl {
    Button,
    Waveform,
}

struct Speech {
    control: SpeechControl,
    context: TypedComponentContext<SpeechEvents>,
    state: Option<Entity<SpeechState>>,
    subscription: Option<Subscription>,
    session: String,
}

impl Speech {
    fn new(control: SpeechControl, context: TypedComponentContext<SpeechEvents>) -> Self {
        Self {
            control,
            context,
            state: None,
            subscription: None,
            session: String::new(),
        }
    }
}

impl TypedNativeComponent for Speech {
    type Props = SpeechProps;
    type Event = SpeechEvents;

    fn render(
        &mut self,
        props: &SpeechProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        if self.state.is_none() {
            let key = (window.window_handle().window_id(), props.session.clone());
            cx.default_global::<SpeechSessions>()
                .0
                .retain(|_, state| state.upgrade().is_some());
            let shared = if props.session.is_empty() {
                None
            } else {
                cx.default_global::<SpeechSessions>()
                    .0
                    .get(&key)
                    .and_then(gpui_kit::WeakEntity::upgrade)
            };
            let state = shared.unwrap_or_else(|| cx.new(SpeechState::new));
            if !props.session.is_empty() {
                cx.default_global::<SpeechSessions>()
                    .0
                    .insert(key, state.downgrade());
            }
            let context = self.context.clone();
            self.subscription = Some(window.subscribe(
                &state,
                cx,
                move |_, event: &NativeSpeechEvent, _, _| {
                    context.emit(match event {
                        NativeSpeechEvent::Started => SpeechEvents::Started,
                        NativeSpeechEvent::Partial(text) => SpeechEvents::Partial(text.to_string()),
                        NativeSpeechEvent::Final(text) => SpeechEvents::Final(text.to_string()),
                        NativeSpeechEvent::Cancelled => SpeechEvents::Cancelled,
                        NativeSpeechEvent::Error(error) => SpeechEvents::Error(error.to_string()),
                    });
                    context.redraw();
                },
            ));
            self.state = Some(state);
        }
        let state = self.state.as_ref().expect("speech state initialized above");
        Some(match self.control {
            SpeechControl::Waveform => SpeechWaveform::new(state).into_any_element(),
            SpeechControl::Button => SpeechButton::new(state)
                .disabled(props.disabled)
                .show_when_unsupported(props.show_when_unsupported)
                .into_any_element(),
        })
    }

    fn props_changed(&mut self, props: &SpeechProps, _: &[String], _: &mut Window, _: &mut App) {
        if self.session != props.session {
            self.session.clone_from(&props.session);
            self.subscription = None;
            self.state = None;
        }
    }

    fn command(&mut self, _: &SpeechProps, name: &str, _: &Value, _: &mut Window, cx: &mut App) {
        if let Some(state) = &self.state {
            state.update(cx, |state, cx| match name {
                "start" => state.start(cx),
                "stop" => state.stop(cx),
                "cancel" => state.cancel(cx),
                "toggle" => state.toggle(cx),
                _ => {}
            });
        }
    }
}
