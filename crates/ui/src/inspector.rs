use std::cell::OnceCell;

use gpui::{
    actions, div, prelude::FluentBuilder, px, AnyElement, App, AppContext as _, Context,
    DivInspectorState, Entity, Inspector, InspectorElementId, InteractiveElement as _, IntoElement,
    KeyBinding, ParentElement as _, Render, SharedString, StyleRefinement, Styled, Window,
};

use crate::{
    alert::Alert,
    button::{Button, ButtonVariants},
    clipboard::Clipboard,
    description_list::DescriptionList,
    h_flex,
    input::{InputEvent, InputState, TextInput},
    link::Link,
    v_flex, ActiveTheme, IconName, Selectable, Sizable, TITLE_BAR_HEIGHT,
};

actions!(inspector, [ToggleInspector, ResetStyle]);

/// Initialize the inspector and register the action to toggle it.
pub fn init(cx: &mut App) {
    cx.bind_keys(vec![
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-alt-i", ToggleInspector, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-i", ToggleInspector, None),
    ]);

    cx.on_action(|_: &ToggleInspector, cx| {
        let Some(active_window) = cx.active_window() else {
            return;
        };

        cx.defer(move |cx| {
            _ = active_window.update(cx, |_, window, cx| {
                window.toggle_inspector(cx);
            });
        });
    });

    let inspector_el = OnceCell::new();
    cx.register_inspector_element(move |id, state: &DivInspectorState, window, cx| {
        let el = inspector_el.get_or_init(|| cx.new(|cx| DivInspector::new(window, cx)));
        el.update(cx, |this, cx| {
            this.update_inspected_element(id, state.clone(), window, cx);
            this.render(window, cx).into_any_element()
        })
    });

    cx.set_inspector_renderer(Box::new(render_inspector));
}

pub struct DivInspector {
    inspector_id: Option<InspectorElementId>,
    inspector_state: Option<DivInspectorState>,
    input_state: Entity<InputState>,
    /// Error message for JSON style parsing
    json_err: Option<SharedString>,
    /// Initial style before any edits
    initial_style: StyleRefinement,
}

impl DivInspector {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("json")
                .line_number(false)
        });

        cx.subscribe_in(
            &input_state,
            window,
            |this: &mut DivInspector, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change(new_style) => {
                    this.edit_json(new_style, window, cx);
                }
                _ => {}
            },
        )
        .detach();

        Self {
            inspector_id: None,
            inspector_state: None,
            input_state,
            json_err: None,
            initial_style: Default::default(),
        }
    }

    pub fn update_inspected_element(
        &mut self,
        inspector_id: InspectorElementId,
        state: DivInspectorState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Skip updating if the inspector ID hasn't changed
        if self.inspector_id.as_ref() == Some(&inspector_id) {
            return;
        }

        self.initial_style = state.base_style.as_ref().clone();
        self.input_state.update(cx, |input_state, cx| {
            input_state.set_value(style_to_json(&self.initial_style), window, cx);
        });
        self.inspector_id = Some(inspector_id.clone());
        self.inspector_state = Some(state);
        cx.notify();
    }

    fn edit_json(&mut self, new_style: &str, window: &mut Window, cx: &mut Context<Self>) {
        match serde_json::from_str::<StyleRefinement>(new_style) {
            Ok(style) => {
                self.json_err = None;
                window.with_inspector_state::<DivInspectorState, _>(
                    self.inspector_id.as_ref(),
                    cx,
                    |state, _window| {
                        if let Some(state) = state {
                            *state.base_style = style;
                        }
                    },
                );
            }
            Err(e) => {
                let e = format!("{}", e);
                self.json_err = Some(e.into());
            }
        }
        window.refresh();
    }

    fn reset_style(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input_state.update(cx, |input_state, cx| {
            input_state.set_value(style_to_json(&self.initial_style), window, cx);
        });
        if let Some(state) = self.inspector_state.as_mut() {
            *state.base_style = self.initial_style.clone();
        }
    }
}

fn style_to_json(style: &StyleRefinement) -> String {
    serde_json::to_string_pretty(style).unwrap_or_else(|e| format!("{{ \"error\": \"{}\" }}", e))
}

impl Render for DivInspector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().size_full().gap_3().text_sm().when_some(
            self.inspector_state.as_ref(),
            |this, state| {
                this.child(
                    DescriptionList::new()
                        .columns(1)
                        .label_width(px(110.))
                        .bordered(false)
                        .child("Origin", format!("{}", state.bounds.origin), 1)
                        .child("Size", format!("{}", state.bounds.size), 1)
                        .child("Content Size", format!("{}", state.content_size), 1),
                )
                .child(
                    v_flex()
                        .w_full()
                        .flex_1()
                        .gap_1()
                        .text_sm()
                        .text_color(cx.theme().description_list_label_foreground)
                        .child(
                            h_flex()
                                .gap_1()
                                .child(div().flex_1().child("Styles"))
                                .child({
                                    Button::new("reset-style")
                                        .label("Reset")
                                        .small()
                                        .ghost()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.reset_style(window, cx);
                                        }))
                                }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .w_full()
                                .font_family("Monaco")
                                .text_size(px(12.))
                                .child(TextInput::new(&self.input_state).h_full()),
                        )
                        .when_some(self.json_err.clone(), |this, err| {
                            this.child(Alert::error("inspector-error", err))
                        }),
                )
            },
        )
    }
}

fn render_inspector(
    inspector: &mut Inspector,
    window: &mut Window,
    cx: &mut Context<Inspector>,
) -> AnyElement {
    let inspector_element_id = inspector.active_element_id();
    let source_location =
        inspector_element_id.map(|id| SharedString::new(format!("{}", id.path.source_location)));
    let element_global_id = inspector_element_id.map(|id| format!("{}", id.path.global_id));

    v_flex()
        .id("inspector")
        .size_full()
        .bg(cx.theme().background)
        .border_l_1()
        .border_color(cx.theme().border)
        .text_color(cx.theme().foreground)
        .child(
            h_flex()
                .w_full()
                .justify_between()
                .gap_2()
                .h(TITLE_BAR_HEIGHT)
                .line_height(TITLE_BAR_HEIGHT)
                .overflow_hidden()
                .px_2()
                .border_b_1()
                .border_color(cx.theme().title_bar_border)
                .bg(cx.theme().title_bar)
                .child(
                    h_flex()
                        .gap_2()
                        .text_sm()
                        .child(
                            Button::new("inspect")
                                .icon(IconName::Inspector)
                                .selected(inspector.is_picking())
                                .small()
                                .ghost()
                                .on_click(cx.listener(|this, _, window, _| {
                                    this.start_picking();
                                    window.refresh();
                                })),
                        )
                        .child("Inspector"),
                )
                .child(
                    Button::new("close")
                        .icon(IconName::Close)
                        .small()
                        .ghost()
                        .on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(ToggleInspector), cx);
                        }),
                ),
        )
        .child(
            v_flex()
                .flex_1()
                .p_3()
                .gap_3()
                .text_sm()
                .when_some(source_location, |this, source_location| {
                    this.child(
                        h_flex()
                            .gap_1()
                            .text_sm()
                            .child(
                                Link::new("source-location")
                                    .href(format!("file://{}", source_location))
                                    .child(source_location.clone()),
                            )
                            .child(Clipboard::new("copy-source-location").value(source_location)),
                    )
                })
                .children(element_global_id)
                .children(inspector.render_inspector_states(window, cx)),
        )
        .into_any_element()
}
