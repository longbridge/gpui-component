use gpui::*;
use gpui_component::{
    highlighter::{Diagnostic, DiagnosticSeverity},
    input::{Input, InputEvent, InputState, Position},
    button::Button,
    h_flex, v_flex, ActiveTheme, Root,
};

pub struct Example {
    editor_state: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl Example {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Create a code editor with syntax error highlighting support
        let editor_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("rust")
                .line_number(true)
                .placeholder("Enter your code here...")
                .default_value(
                    r#"fn main() {
    let x = 10;
    println!("Hello, world!");
    let y = undefinedVariable;
    let z = x + y
}"#,
                )
        });

        // Add some initial syntax errors
        editor_state.update(cx, |state, cx| {
            if let Some(diagnostics) = state.diagnostics_mut() {
                // Clear any existing diagnostics
                diagnostics.clear();

                // Add error: undefined variable
                diagnostics.push(
                    Diagnostic::new(
                        Position::new(3, 12)..Position::new(3, 29),
                        "cannot find value `undefinedVariable` in this scope",
                    )
                    .with_severity(DiagnosticSeverity::Error)
                    .with_code("E0425"),
                );

                // Add error: missing semicolon
                diagnostics.push(
                    Diagnostic::new(
                        Position::new(4, 18)..Position::new(4, 19),
                        "expected `;`, found `}`",
                    )
                    .with_severity(DiagnosticSeverity::Error)
                    .with_code("E0308"),
                );

                // Add warning: unused variable
                diagnostics.push(
                    Diagnostic::new(
                        Position::new(1, 8)..Position::new(1, 9),
                        "unused variable: `x`",
                    )
                    .with_severity(DiagnosticSeverity::Warning)
                    .with_code("unused_variables"),
                );
            }
            cx.notify();
        });

        let _subscriptions = vec![cx.subscribe_in(&editor_state, window, {
            let editor_state = editor_state.clone();
            move |_this, _, ev: &InputEvent, _window, _cx| match ev {
                InputEvent::Change => {
                    // You could perform syntax checking here and update diagnostics
                    let _value = editor_state.read(_cx).value();
                    // For this example, we keep the existing diagnostics
                }
                _ => {}
            }
        })];

        Self {
            editor_state,
            _subscriptions,
        }
    }

    fn clear_errors(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor_state.update(cx, |state, cx| {
            if let Some(diagnostics) = state.diagnostics_mut() {
                diagnostics.clear();
            }
            cx.notify();
        });
    }

    fn add_sample_errors(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor_state.update(cx, |state, cx| {
            if let Some(diagnostics) = state.diagnostics_mut() {
                diagnostics.clear();

                // Add a new error at a different location
                diagnostics.push(
                    Diagnostic::new(
                        Position::new(2, 4)..Position::new(2, 11),
                        "mismatched types: expected `i32`, found `&str`",
                    )
                    .with_severity(DiagnosticSeverity::Error)
                    .with_code("E0308"),
                );

                // Add a hint
                diagnostics.push(
                    Diagnostic::new(
                        Position::new(0, 0)..Position::new(0, 8),
                        "function `main` is never used",
                    )
                    .with_severity(DiagnosticSeverity::Hint),
                );
            }
            cx.notify();
        });
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .p_5()
            .gap_4()
            .size_full()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("clear-errors")
                            .label("Clear Errors")
                            .on_click(cx.listener(Self::clear_errors)),
                    )
                    .child(
                        Button::new("add-errors")
                            .label("Add Sample Errors")
                            .on_click(cx.listener(Self::add_sample_errors)),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .overflow_hidden()
                    .child(Input::new(&self.editor_state).h_full()),
            )
            .child(
                v_flex()
                    .gap_1()
                    .p_3()
                    .bg(cx.theme().muted)
                    .rounded(cx.theme().radius)
                    .text_sm()
                    .child("Syntax Error Display Demo")
                    .child("• Errors are shown with red wavy underlines")
                    .child("• Warnings are shown with yellow wavy underlines")
                    .child("• Hints are shown with blue wavy underlines")
                    .child("• Hover over underlined text to see error details")
                    .child("• Use the buttons above to add or clear errors"),
            )
    }
}

fn main() {
    let app = Application::new();

    app.run(move |cx| {
        // Initialize the components
        gpui_component::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(900.), px(700.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| Example::new(window, cx));
                cx.new(|cx| Root::new(view.into(), window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
