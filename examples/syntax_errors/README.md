# Syntax Error Display Example

This example demonstrates how to use the syntax error highlighting feature in the code editor.

## Features

- Display syntax errors with colored wavy underlines
- Support for different severity levels (Error, Warning, Info, Hint)
- Add and clear error ranges dynamically
- Hover over errors to see diagnostic messages

## Error Severity Colors

The code editor displays errors with different colored wavy underlines based on their severity:

- **Error**: Red wavy underline
- **Warning**: Yellow/Orange wavy underline
- **Info**: Blue wavy underline
- **Hint**: Light blue wavy underline

## Important Note

**This feature is only available in Code Editor mode.** Single-line and basic multi-line input modes do not support diagnostics. You must use `InputState::code_editor(language)` to enable this functionality.

## Usage

### Creating a Code Editor with Diagnostics

```rust
use gpui_component::input::InputState;

let editor_state = cx.new(|cx| {
    InputState::new(window, cx)
        .code_editor("rust")  // Enable code editor mode
        .line_number(true)
        .placeholder("Enter your code here...")
});
```

### Adding Syntax Errors

```rust
use gpui_component::highlighter::{Diagnostic, DiagnosticSeverity};
use gpui_component::input::Position;

editor_state.update(cx, |state, cx| {
    if let Some(diagnostics) = state.diagnostics_mut() {
        // Add an error
        diagnostics.push(
            Diagnostic::new(
                Position::new(3, 12)..Position::new(3, 29),
                "cannot find value `undefinedVariable` in this scope",
            )
            .with_severity(DiagnosticSeverity::Error)
            .with_code("E0425"),
        );
        
        // Add a warning
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
```

### Clearing Errors

```rust
editor_state.update(cx, |state, cx| {
    if let Some(diagnostics) = state.diagnostics_mut() {
        diagnostics.clear();
    }
    cx.notify();
});
```

### Extending with Multiple Errors

```rust
editor_state.update(cx, |state, cx| {
    if let Some(diagnostics) = state.diagnostics_mut() {
        let errors = vec![
            Diagnostic::new(
                Position::new(0, 0)..Position::new(0, 10),
                "First error",
            ).with_severity(DiagnosticSeverity::Error),
            
            Diagnostic::new(
                Position::new(1, 5)..Position::new(1, 15),
                "Second error",
            ).with_severity(DiagnosticSeverity::Warning),
        ];
        
        diagnostics.extend(errors);
    }
    cx.notify();
});
```

## API Reference

### Diagnostic

Create a new diagnostic with a range and message:

```rust
Diagnostic::new(
    start_position..end_position,
    "Error message"
)
```

Builder methods:
- `.with_severity(DiagnosticSeverity)` - Set the severity level
- `.with_code(impl Into<SharedString>)` - Set the error code (e.g., "E0425")
- `.with_source(impl Into<SharedString>)` - Set the source of the diagnostic (e.g., "rustc")

### DiagnosticSeverity

Available severity levels:
- `DiagnosticSeverity::Error` - Red wavy underline
- `DiagnosticSeverity::Warning` - Yellow wavy underline
- `DiagnosticSeverity::Info` - Blue wavy underline
- `DiagnosticSeverity::Hint` - Light blue wavy underline

### Position

Create a position with line and character (both 0-indexed):

```rust
Position::new(line, character)
```

## Running this Example

```bash
cargo run -p syntax_errors
```
