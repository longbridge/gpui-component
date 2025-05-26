use std::cell::OnceCell;

use gpui::{
    actions, div, prelude::FluentBuilder, px, AnyElement, App, AppContext as _, Context,
    DivInspectorState, Entity, Inspector, InspectorElementId, InteractiveElement as _, IntoElement,
    KeyBinding, ParentElement as _, Render, SharedString, Styled, Window,
};

use crate::{
    button::{Button, ButtonVariants},
    description_list::DescriptionList,
    h_flex,
    highlighter::HighlightTheme,
    input::{InputState, TextInput},
    v_flex, ActiveTheme, IconName, Selectable, Sizable, TITLE_BAR_HEIGHT,
};

actions!(app, [ToggleInspector]);

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
            this.set_inspector_state(id, state.clone(), cx);
            this.render(window, cx).into_any_element()
        })
    });

    cx.set_inspector_renderer(Box::new(render_inspector));
}

pub struct DivInspector {
    inspector_id: Option<InspectorElementId>,
    inspector_state: Option<DivInspectorState>,
    input_state: Entity<InputState>,
}

impl DivInspector {
    pub fn new(window: &mut Window, cx: &mut App) -> Self {
        let theme = if cx.theme().is_dark() {
            HighlightTheme::default_dark()
        } else {
            HighlightTheme::default_light()
        };

        let input_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor(Some("json"), theme)
                .line_number(false);
            state.set_disabled(true, window, cx);
            state
        });

        Self {
            inspector_id: None,
            inspector_state: None,
            input_state,
        }
    }

    pub fn set_inspector_state(
        &mut self,
        inspector_id: InspectorElementId,
        state: DivInspectorState,
        cx: &mut Context<Self>,
    ) {
        self.inspector_id = Some(inspector_id);
        self.inspector_state = Some(state);
        cx.notify();
    }
}

impl Render for DivInspector {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let input_state = self.input_state.clone();
        let last_styles = input_state.read(cx).value().clone();

        v_flex().size_full().gap_1().text_sm().when_some(
            self.inspector_state.clone(),
            |this, state| {
                let styles = serde_json::to_string_pretty(&state.base_style);
                let styles: SharedString = match styles {
                    Ok(json) => json,
                    Err(e) => format!("{{ \"error\": \"{}\" }}", e),
                }
                .into();

                if styles != last_styles {
                    input_state.update(cx, |s, cx| s.set_value(styles.clone(), window, cx));
                }

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
                    v_flex().w_full().flex_1().gap_1().child("Styles").child(
                        div()
                            .flex_1()
                            .w_full()
                            .font_family("Monaco")
                            .text_size(px(12.))
                            .child(TextInput::new(&input_state).h_full()),
                    ),
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
    v_flex()
        .id("inspector")
        .size_full()
        .bg(cx.theme().background)
        .border_l_1()
        .border_color(cx.theme().border)
        .child(
            h_flex()
                .w_full()
                .gap_2()
                .h(TITLE_BAR_HEIGHT)
                .line_height(TITLE_BAR_HEIGHT)
                .overflow_hidden()
                .px_2()
                .border_b_1()
                .border_color(cx.theme().border)
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
            div()
                .flex_1()
                .p_2()
                .children(inspector.render_inspector_states(window, cx)),
        )
        .into_any_element()
}
