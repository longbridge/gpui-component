//! Retained questionnaire behavior, validation and keyboard navigation.
use crate::bridge::{children, slot};
use gpui_kit::component::questionnaire::{
    QuestionnaireAnswer as ComponentQuestionnaireAnswer,
    QuestionnaireChoice as ComponentQuestionnaireChoice,
    QuestionnaireItem as ComponentQuestionnaireItem, *,
};
use gpui_kit::{
    AnyElement, App, AppContext, Entity, FocusHandle, IntoElement, ParentElement, Subscription,
    Window, div,
};
use nsh_core::ui::{Children, Event, JsObject, PropError, Props};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct QuestionnaireChoice {
    value: String,
    label: String,
    #[prop(default)]
    disabled: bool,
    #[prop(default)]
    description: String,
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct QuestionnaireItem {
    id: String,
    label: String,
    #[prop(default)]
    description: String,
    #[prop(default)]
    required: bool,
    #[prop(default)]
    multiple: bool,
    #[prop(default)]
    disabled: bool,
    #[prop(default)]
    choices: Vec<QuestionnaireChoice>,
    #[prop(default)]
    input: Option<QuestionnaireInputProps>,
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct QuestionnaireInputProps {
    label: String,
    #[prop(default)]
    placeholder: String,
    #[prop(default)]
    disabled: bool,
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct QuestionnaireProps {
    items: Vec<QuestionnaireItem>,
    children: Children,
}
// Answers always carry the freeform key, including its explicit null value.
type QuestionnaireFreeform = Option<String>;
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct QuestionnaireAnswer {
    choices: Vec<String>,
    freeform: QuestionnaireFreeform,
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct QuestionnaireAnswerChange {
    item: String,
    answer: QuestionnaireAnswer,
}
crate::registration::named_value! { pub(crate) struct QuestionnaireAnswers(BTreeMap<String, QuestionnaireAnswer>); }
#[derive(Clone, Event)]
pub(crate) enum NativeQuestionnaireEvent {
    Change(QuestionnaireAnswerChange),
    CurrentChange(Option<String>),
    Complete(QuestionnaireAnswers),
    Submit(QuestionnaireAnswers),
    Error(String),
}
pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeQuestionnaire>(
        "gpui-component.Questionnaire",
        "Questionnaire",
        |context| NativeQuestionnaire {
            context,
            questions: vec![],
            pending: None,
            state: None,
            focus: None,
            subscription: None,
        },
    );
}
struct NativeQuestionnaire {
    context: TypedComponentContext<NativeQuestionnaireEvent>,
    questions: Vec<QuestionnaireItem>,
    pending: Option<Vec<QuestionnaireItem>>,
    state: Option<Entity<QuestionnaireState>>,
    focus: Option<FocusHandle>,
    subscription: Option<Subscription>,
}
fn validate(questions: &[QuestionnaireItem]) -> Result<(), String> {
    let mut names = HashSet::new();
    for question in questions {
        if question.id.is_empty() || !names.insert(&question.id) {
            return Err("Question ids must be nonempty and unique".into());
        }
        let mut values = HashSet::new();
        for choice in &question.choices {
            if choice.value.is_empty() || !values.insert(&choice.value) {
                return Err(format!(
                    "Question {} has empty or duplicate choice values",
                    question.id
                ));
            }
        }
    }
    Ok(())
}
fn answer_value(answer: &ComponentQuestionnaireAnswer) -> QuestionnaireAnswer {
    QuestionnaireAnswer {
        choices: answer.choices().iter().map(ToString::to_string).collect(),
        freeform: answer.freeform().map(ToString::to_string),
    }
}
fn submission_value(submission: &QuestionnaireSubmission) -> QuestionnaireAnswers {
    submission
        .items()
        .iter()
        .map(|item| (item.name().to_string(), answer_value(item.answer())))
        .collect::<BTreeMap<_, _>>()
        .into()
}
impl TypedNativeComponent for NativeQuestionnaire {
    type Props = QuestionnaireProps;
    type Event = NativeQuestionnaireEvent;
    fn validate_props(&self, props: &QuestionnaireProps) -> Result<(), PropError> {
        validate(&props.items).map_err(PropError)
    }
    fn props_changed(
        &mut self,
        props: &QuestionnaireProps,
        _: &[String],
        _: &mut Window,
        _: &mut App,
    ) {
        self.pending = if props.items != self.questions || self.state.is_none() {
            Some(props.items.clone())
        } else {
            None
        };
    }
    fn render(
        &mut self,
        _: &QuestionnaireProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        if let Some(questions) = self.pending.take() {
            let definitions = questions
                .iter()
                .map(|question| {
                    let definition = QuestionnaireItemDefinition::new(
                        question.id.clone(),
                        question.label.clone(),
                    )
                    .with_description(question.description.clone())
                    .with_required(question.required)
                    .with_multiple(question.multiple)
                    .with_disabled(question.disabled)
                    .with_choices(question.choices.iter().map(|choice| {
                        QuestionnaireChoiceDefinition::new(
                            choice.value.clone(),
                            choice.label.clone(),
                        )
                        .with_description(choice.description.clone())
                        .with_disabled(choice.disabled)
                    }));
                    if let Some(input) = &question.input {
                        let input_state = cx.new(|cx| {
                            gpui_kit::component::input::InputState::new(window, cx)
                                .placeholder(input.placeholder.clone())
                        });
                        definition.with_input(
                            QuestionnaireInputDefinition::new(input_state, input.label.clone())
                                .with_disabled(input.disabled),
                        )
                    } else {
                        definition
                    }
                })
                .collect();
            let state = cx.new(|cx| {
                QuestionnaireState::new(definitions, cx)
                    .expect("questionnaire schema validated before creating state")
            });
            let context = self.context.clone();
            self.subscription =
                Some(
                    window.subscribe(&state, cx, move |_, event: &QuestionnaireEvent, _, _| {
                        match event {
                            QuestionnaireEvent::AnswerChanged(change) => context.emit(
                                NativeQuestionnaireEvent::Change(QuestionnaireAnswerChange {
                                    item: change.item().to_string(),
                                    answer: answer_value(change.answer()),
                                }),
                            ),
                            QuestionnaireEvent::CurrentItemChanged { current, .. } => {
                                context.emit(NativeQuestionnaireEvent::CurrentChange(
                                    current.as_ref().map(ToString::to_string),
                                ))
                            }
                            QuestionnaireEvent::Completed(submission) => context.emit(
                                NativeQuestionnaireEvent::Complete(submission_value(submission)),
                            ),
                            QuestionnaireEvent::Submit(submission) => context.emit(
                                NativeQuestionnaireEvent::Submit(submission_value(submission)),
                            ),
                            _ => {}
                        }
                        context.redraw();
                    }),
                );
            self.questions = questions;
            self.focus = Some(state.read(cx).focus_handle().clone());
            self.state = Some(state);
        }
        let Some(state) = &self.state else {
            return Some(div().into_any_element());
        };
        let parts = self
            .questions
            .iter()
            .map(|question| {
                ComponentQuestionnaireItem::new(state, question.id.clone())
                    .child(QuestionnaireTitle::new(state, question.id.clone()))
                    .child(QuestionnaireDescription::new(state, question.id.clone()))
                    .child(
                        QuestionnaireChoices::new(state, question.id.clone()).children(
                            question.choices.iter().map(|choice| {
                                ComponentQuestionnaireChoice::new(
                                    state,
                                    question.id.clone(),
                                    choice.value.clone(),
                                )
                                .children(slot(
                                    &self.context,
                                    &format!("{}:{}", question.id, choice.value),
                                ))
                            }),
                        ),
                    )
                    .children(
                        question
                            .input
                            .as_ref()
                            .map(|_| QuestionnaireInput::new(state, question.id.clone())),
                    )
                    .children(slot(&self.context, &question.id))
                    .child(QuestionnaireError::new(state, question.id.clone()))
            })
            .collect::<Vec<_>>();
        Some(
            Questionnaire::new(state)
                .child(QuestionnaireProgress::new(state))
                .children(parts)
                .children(children(&self.context))
                .child(
                    QuestionnaireActions::new(state)
                        .child(QuestionnairePrevious::new(state))
                        .child(QuestionnaireSkip::new(state))
                        .child(QuestionnaireNext::new(state))
                        .child(QuestionnaireSubmit::new(state)),
                )
                .into_any_element(),
        )
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone()
    }
    fn command(
        &mut self,
        _: &QuestionnaireProps,
        name: &str,
        args: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(state) = &self.state {
            state.update(cx, |state, cx| {
                let result = match name {
                    "reset" => {
                        state.reset(window, cx);
                        Ok(())
                    }
                    "submit" => {
                        state.submit(window, cx);
                        Ok(())
                    }
                    "next" => {
                        state.go_next(window, cx);
                        Ok(())
                    }
                    "previous" => {
                        state.go_previous(window, cx);
                        Ok(())
                    }
                    "skip" => {
                        state.skip_current(window, cx);
                        Ok(())
                    }
                    "confirm" => {
                        state.confirm_current(window, cx);
                        Ok(())
                    }
                    "focus" => {
                        state.focus_current_item(window, cx);
                        Ok(())
                    }
                    "setCurrent" => args
                        .as_str()
                        .ok_or_else(|| "setCurrent requires an item id".to_owned())
                        .and_then(|item| {
                            state
                                .set_current_item(item, window, cx)
                                .map_err(|error| format!("{error:?}"))
                        }),
                    "setAnswer" => parse_answer(args).and_then(|(item, answer)| {
                        state
                            .set_answer(&item, answer, window, cx)
                            .map_err(|error| format!("{error:?}"))
                    }),
                    "setError" => args
                        .get("item")
                        .and_then(Value::as_str)
                        .zip(args.get("message").and_then(Value::as_str))
                        .ok_or_else(|| "setError requires item and message strings".to_owned())
                        .and_then(|(item, message)| {
                            state
                                .set_external_error(item, message, cx)
                                .map_err(|error| format!("{error:?}"))
                        }),
                    "clearError" => args
                        .as_str()
                        .ok_or_else(|| "clearError requires an item id".to_owned())
                        .and_then(|item| {
                            state
                                .clear_external_error(item, cx)
                                .map_err(|error| format!("{error:?}"))
                        }),
                    _ => Ok(()),
                };
                if let Err(error) = result {
                    self.context.emit(NativeQuestionnaireEvent::Error(error));
                }
            });
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicate_choice_identity() {
        let choice = |label: &str| QuestionnaireChoice {
            value: "a".into(),
            label: label.into(),
            disabled: false,
            description: String::new(),
        };
        let items = vec![QuestionnaireItem {
            id: "q".into(),
            label: "Question".into(),
            description: String::new(),
            required: false,
            multiple: false,
            disabled: false,
            choices: vec![choice("A"), choice("B")],
            input: None,
        }];
        assert!(validate(&items).is_err());
    }
}

fn parse_answer(args: &Value) -> Result<(String, ComponentQuestionnaireAnswer), String> {
    let item = args
        .get("item")
        .and_then(Value::as_str)
        .ok_or("setAnswer requires an item id")?
        .to_owned();
    let choices = args
        .get("choices")
        .and_then(Value::as_array)
        .ok_or("setAnswer requires a choices array")?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or("choice values must be strings")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let answer = ComponentQuestionnaireAnswer::new().with_choices(choices);
    let answer = if let Some(freeform) = args.get("freeform").filter(|value| !value.is_null()) {
        answer.with_freeform(
            freeform
                .as_str()
                .ok_or("freeform must be a string or null")?
                .to_owned(),
        )
    } else {
        answer
    };
    Ok((item, answer))
}
