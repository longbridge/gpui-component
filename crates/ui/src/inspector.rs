use std::{cell::OnceCell, collections::HashMap, fmt::Write as _, sync::OnceLock};

use gpui::{
    actions, div, inspector_reflection::FunctionReflection, prelude::FluentBuilder, px, AnyElement,
    App, AppContext, Context, DivInspectorState, Entity, Inspector, InspectorElementId,
    InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Refineable as _, Render,
    SharedString, StyleRefinement, Styled, Subscription, Window,
};

use crate::{
    alert::Alert,
    button::{Button, ButtonVariants},
    clipboard::Clipboard,
    description_list::DescriptionList,
    dropdown::{Dropdown, DropdownState, SearchableVec},
    h_flex,
    input::{InputEvent, InputState, TabSize, TextInput},
    link::Link,
    v_flex, ActiveTheme, IconName, Selectable, Sizable, TITLE_BAR_HEIGHT,
};

actions!(inspector, [ToggleInspector]);

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

struct EditorState {
    /// The input state for the editor.
    state: Entity<InputState>,
    /// Error to display from parsing the input, or if serialization errors somehow occur.
    error: Option<SharedString>,
    /// Whether the editor is currently being edited.
    editing: bool,
}

pub struct DivInspector {
    inspector_id: Option<InspectorElementId>,
    inspector_state: Option<DivInspectorState>,
    rust_dropdown: Entity<DropdownState<SearchableVec<SharedString>>>,
    rust_state: EditorState,
    json_state: EditorState,
    /// Initial style before any edits
    initial_style: StyleRefinement,
    /// Part of the initial style that could not be converted to Rust code
    unconvertible_style: StyleRefinement,
    _subscriptions: Vec<Subscription>,
}

impl DivInspector {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let json_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("json")
                .line_number(false)
        });

        let rust_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("rust")
                .line_number(false)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
        });

        let rust_dropdown = cx.new(|cx| {
            DropdownState::new(
                SearchableVec::new({
                    let mut methods: Vec<_> = StyleMethods::get()
                        .table
                        .iter()
                        .map(|(_, method)| method.name.into())
                        .collect();
                    methods.sort();
                    methods
                }),
                None,
                window,
                cx,
            )
        });

        let _subscriptions = vec![
            cx.subscribe_in(
                &json_input_state,
                window,
                |this: &mut DivInspector, _, event: &InputEvent, window, cx| match event {
                    InputEvent::Change(new_style) => {
                        this.edit_json(new_style, window, cx);
                    }
                    _ => {}
                },
            ),
            cx.subscribe_in(
                &rust_input_state,
                window,
                |this: &mut DivInspector, _, event: &InputEvent, window, cx| match event {
                    InputEvent::Change(new_style) => {
                        this.edit_rust(new_style, window, cx);
                    }
                    _ => {}
                },
            ),
        ];

        let rust_state = EditorState {
            state: rust_input_state,
            error: None,
            editing: false,
        };

        let json_state = EditorState {
            state: json_input_state,
            error: None,
            editing: false,
        };

        Self {
            inspector_id: None,
            inspector_state: None,
            rust_dropdown,
            rust_state,
            json_state,
            initial_style: Default::default(),
            unconvertible_style: Default::default(),
            _subscriptions,
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

        let initial_style = state.base_style.as_ref();
        self.initial_style = initial_style.clone();
        self.json_state.editing = false;
        self.update_json_from_style(initial_style, window, cx);
        self.rust_state.editing = false;
        let rust_style = self.update_rust_from_style(initial_style, window, cx);
        self.unconvertible_style = initial_style.subtract(&rust_style);
        self.inspector_id = Some(inspector_id);
        self.inspector_state = Some(state);
        cx.notify();
    }

    fn edit_json(&mut self, code: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.json_state.editing {
            self.json_state.editing = true;
            return;
        }

        match serde_json::from_str::<StyleRefinement>(code) {
            Ok(new_style) => {
                self.json_state.error = None;
                self.rust_state.error = None;
                self.rust_state.editing = false;
                let rust_style = self.update_rust_from_style(&new_style, window, cx);
                self.unconvertible_style = new_style.subtract(&rust_style);
                self.update_element_style(new_style, window, cx);
            }
            Err(e) => {
                self.json_state.error = Some(e.to_string().trim_end().to_string().into());
                window.refresh();
            }
        }
    }

    fn edit_rust(&mut self, code: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.rust_state.editing {
            self.rust_state.editing = true;
            return;
        }

        let (new_style, err) = rust_to_style(self.unconvertible_style.clone(), code);
        self.rust_state.error = err;
        self.json_state.error = None;
        self.json_state.editing = false;
        self.update_json_from_style(&new_style, window, cx);
        self.update_element_style(new_style, window, cx);
    }

    fn update_element_style(
        &self,
        style: StyleRefinement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.with_inspector_state::<DivInspectorState, _>(
            self.inspector_id.as_ref(),
            cx,
            |state, _window| {
                if let Some(state) = state {
                    *state.base_style = style;
                }
            },
        );
        window.refresh();
    }

    fn reset_style(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.rust_state.editing = false;
        let rust_style = self.update_rust_from_style(&self.initial_style, window, cx);
        self.unconvertible_style = self.initial_style.subtract(&rust_style);
        self.json_state.editing = false;
        self.update_json_from_style(&self.initial_style, window, cx);
        if let Some(state) = self.inspector_state.as_mut() {
            *state.base_style = self.initial_style.clone();
        }
    }

    fn update_json_from_style(
        &self,
        style: &StyleRefinement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.json_state.state.update(cx, |state, cx| {
            state.set_value(style_to_json(style), window, cx);
        });
    }

    fn update_rust_from_style(
        &self,
        style: &StyleRefinement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> StyleRefinement {
        self.rust_state.state.update(cx, |state, cx| {
            let (rust_code, rust_style) = style_to_rust(style);
            state.set_value(rust_code, window, cx);
            rust_style
        })
    }

    fn rust_add_style(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(method) = self.rust_dropdown.read(cx).selected_value() {
            let code = self.rust_state.state.read(cx).value();
            let new_code = format!("        .{method}()\n");
            let Some(insert_pos) = code.rfind('}') else {
                self.rust_state.error = Some("Failed to add method: Could not find `}`".into());
                return;
            };
            let code = format!("{}{}{}", &code[..insert_pos], new_code, &code[insert_pos..]);

            self.rust_state.editing = true;
            self.rust_state.state.update(cx, |state, cx| {
                state.set_value(code, window, cx);
                // an edit event will be triggered, the style will be updated there
            });
        }
    }
}

fn style_to_json(style: &StyleRefinement) -> String {
    serde_json::to_string_pretty(style).unwrap_or_else(|e| format!("{{ \"error\": \"{}\" }}", e))
}

struct StyleMethods {
    table: Vec<(Box<StyleRefinement>, FunctionReflection<StyleRefinement>)>,
    map: HashMap<&'static str, FunctionReflection<StyleRefinement>>,
}

impl StyleMethods {
    fn get() -> &'static Self {
        static STYLE_METHODS: OnceLock<StyleMethods> = OnceLock::new();
        STYLE_METHODS.get_or_init(|| {
            let table: Vec<_> = [
                crate::styled_ext_reflection::methods::<StyleRefinement>(),
                gpui::styled_reflection::methods::<StyleRefinement>(),
            ]
            .into_iter()
            .flatten()
            .map(|method| (Box::new(method.invoke(StyleRefinement::default())), method))
            .collect();
            let map = table
                .iter()
                .map(|(_, method)| (method.name, method.clone()))
                .collect();

            Self { table, map }
        })
    }
}

fn style_to_rust(input_style: &StyleRefinement) -> (String, StyleRefinement) {
    let methods: Vec<_> = StyleMethods::get()
        .table
        .iter()
        .filter_map(|(style, method)| {
            if input_style.is_superset_of(style) {
                Some(method)
            } else {
                None
            }
        })
        .collect();
    let mut code = "fn build() -> Div {\n    div()\n".to_string();
    let mut style = StyleRefinement::default();
    for method in methods {
        let before_invoke = style.clone();
        style = method.invoke(style);
        if style != before_invoke {
            _ = write!(code, "        .{}()\n", method.name);
        }
    }
    code.push_str("}");
    (code, style)
}

fn rust_to_style(
    mut style: StyleRefinement,
    rust_code: &str,
) -> (StyleRefinement, Option<SharedString>) {
    // remove line comments
    let rust_code = rust_code
        .lines()
        .map(|line| line.find("//").map_or(line, |i| &line[..i]).trim())
        .collect::<Vec<_>>()
        .concat();

    let Some(begin) = rust_code.find("div()").map(|i| i + "div()".len()) else {
        return (style, Some("Expected `div()`".into()));
    };

    let mut err = String::new();
    let methods = rust_code[begin..]
        .split(&['.', '(', ')', '{', '}'])
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let style_methods = StyleMethods::get();
    for method in methods {
        match style_methods.map.get(method) {
            Some(method_reflection) => style = method_reflection.invoke(style),
            None => _ = writeln!(err, "Unknown method: {method}"),
        }
    }

    let err = if err.is_empty() {
        None
    } else {
        Some(err.trim_end().to_string().into())
    };
    (style, err)
}

impl Render for DivInspector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().size_full().gap_y_4().text_sm().when_some(
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
                        .flex_1()
                        .gap_y_3()
                        .child(
                            v_flex().gap_y_2().child("Rust Styles").child(
                                h_flex()
                                    .gap_x_2()
                                    .child(
                                        Dropdown::new(&self.rust_dropdown)
                                            .icon(IconName::Search)
                                            .small()
                                            .cleanable()
                                            .flex_1(),
                                    )
                                    .child(Button::new("rust-add").label("Add").small().on_click(
                                        cx.listener(|this, _, window, cx| {
                                            this.rust_add_style(window, cx);
                                        }),
                                    ))
                                    .child(
                                        Button::new("rust-reset").label("Reset").small().on_click(
                                            cx.listener(|this, _, window, cx| {
                                                this.reset_style(window, cx);
                                            }),
                                        ),
                                    ),
                            ),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .gap_y_1()
                                .font_family("Monaco")
                                .text_size(px(12.))
                                .child(TextInput::new(&self.rust_state.state).h_full())
                                .when_some(self.rust_state.error.clone(), |this, err| {
                                    this.child(Alert::error("rust-error", err).text_xs())
                                }),
                        ),
                )
                .child(
                    v_flex()
                        .gap_y_3()
                        .h_3_5()
                        .flex_shrink_0()
                        .child(
                            h_flex()
                                .gap_x_2()
                                .child(div().flex_1().child("JSON Styles"))
                                .child(Button::new("json-reset").label("Reset").small().on_click(
                                    cx.listener(|this, _, window, cx| {
                                        this.reset_style(window, cx);
                                    }),
                                )),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .gap_y_1()
                                .font_family("Monaco")
                                .text_size(px(12.))
                                .child(TextInput::new(&self.json_state.state).h_full())
                                .when_some(self.json_state.error.clone(), |this, err| {
                                    this.child(Alert::error("json-error", err).text_xs())
                                }),
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
    let inspector_element_id = inspector.active_element_id();
    let source_location =
        inspector_element_id.map(|id| SharedString::new(format!("{}", id.path.source_location)));
    let element_global_id = inspector_element_id.map(|id| format!("{}", id.path.global_id));

    v_flex()
        .id("inspector")
        .font_family(".SystemUIFont")
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
                .overflow_x_hidden()
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
                .gap_y_3()
                .text_sm()
                .when_some(source_location, |this, source_location| {
                    this.child(
                        h_flex()
                            .gap_x_2()
                            .text_sm()
                            .child(
                                Link::new("source-location")
                                    .href(format!("file://{}", source_location))
                                    .child(source_location.clone())
                                    .flex_1()
                                    .overflow_x_hidden(),
                            )
                            .child(Clipboard::new("copy-source-location").value(source_location)),
                    )
                })
                .children(element_global_id)
                .children(inspector.render_inspector_states(window, cx)),
        )
        .into_any_element()
}
