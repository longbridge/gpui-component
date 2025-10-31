# Syntax Error Display Feature

## Overview

The code editor (input module) now supports displaying syntax errors with colored wavy underlines. This feature is already implemented in the codebase and is fully functional through the public API.

## Key Components

### 1. Diagnostic System
- **Location**: `crates/ui/src/highlighter/diagnostics.rs`
- **Key Types**:
  - `Diagnostic`: Represents a single diagnostic message with range, severity, code, and message
  - `DiagnosticSeverity`: Error, Warning, Info, Hint
  - `DiagnosticSet`: Container for managing multiple diagnostics

### 2. Code Editor Integration
- Diagnostics are integrated into `InputMode::CodeEditor` variant
- Accessible via `InputState::diagnostics()` and `InputState::diagnostics_mut()`
- Automatically rendered with wavy underlines in the editor

### 3. Visual Rendering
- **Location**: `crates/ui/src/input/element.rs`
- **Implementation**: Lines 683-691
- Diagnostic styles are combined with syntax highlighting styles
- Wavy underlines are rendered with configurable colors based on severity

## API Usage

### Public API

All necessary types are exported through `gpui_component`:

```rust
use gpui_component::highlighter::{Diagnostic, DiagnosticSeverity};
use gpui_component::input::{InputState, Position};
```

### Creating a Code Editor

```rust
let editor_state = cx.new(|cx| {
    InputState::new(window, cx)
        .code_editor("rust")
        .line_number(true)
});
```

### Adding Diagnostics

```rust
editor_state.update(cx, |state, cx| {
    if let Some(diagnostics) = state.diagnostics_mut() {
        diagnostics.push(
            Diagnostic::new(
                Position::new(line, col)..Position::new(line, end_col),
                "Error message"
            )
            .with_severity(DiagnosticSeverity::Error)
            .with_code("E0425")
        );
    }
    cx.notify();
});
```

### Clearing Diagnostics

```rust
editor_state.update(cx, |state, cx| {
    if let Some(diagnostics) = state.diagnostics_mut() {
        diagnostics.clear();
    }
    cx.notify();
});
```

## Styling

### Wavy Underlines
- **Implementation**: `DiagnosticSeverity::highlight_style()`
- **Style**: `wavy: true`, `thickness: px(1.)`
- **Colors**: Theme-based colors for each severity level

### Color Mapping
- `Error` → Red (theme.style.status.error)
- `Warning` → Yellow/Orange (theme.style.status.warning)
- `Info` → Blue (theme.style.status.info)
- `Hint` → Light blue (theme.style.status.hint)

## Example

A complete working example is available at:
- **Path**: `examples/syntax_errors/`
- **Run**: `cargo run -p syntax_errors`
- **Documentation**: `examples/syntax_errors/README.md`

## Features Supported

✅ Add error ranges with Position-based coordinates
✅ Clear all diagnostics
✅ Multiple severity levels (Error, Warning, Info, Hint)
✅ Red wavy underlines for errors (and appropriate colors for other severities)
✅ Hover support to show diagnostic messages
✅ Integration with LSP-based diagnostics
✅ Code actions associated with diagnostics

## Implementation Notes

1. **Only available in Code Editor mode**: Diagnostics are only supported in `InputMode::CodeEditor`, not in single-line or basic multi-line modes.

2. **Position coordinates**: Both line and character are 0-indexed (first line is 0, first character is 0).

3. **Thread-safe**: The `DiagnosticSet` uses a `SumTree` for efficient range queries and updates.

4. **Rendering optimization**: Only visible diagnostics are rendered, improving performance for large files.

5. **Theme integration**: Colors are pulled from the active theme's status colors, ensuring consistency with the application's appearance.

## Testing

The feature has been verified to:
- Compile successfully
- Export all necessary types in the public API
- Support the documented API usage patterns
- Integrate with the existing LSP infrastructure (see `crates/story/examples/editor.rs`)
