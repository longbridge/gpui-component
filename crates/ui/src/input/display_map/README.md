# Display Mapping System

The display mapping system provides unified coordinate conversion and code folding for Editor/Input.

## Architecture

Based on a layered projection model:

```
Buffer (Rope)              Logical text
    ↓
WrapMap                    Soft-wrapping layer
    ↓ (wrap_row)
FoldMap                    Fold projection layer
    ↓ (display_row)
DisplayMap (Facade)        Unified public API
```

### Coordinate Systems

1. **BufferPos** `{ line, col }` - Buffer logical coordinates
   - line: Logical line number (split by `\n`)
   - col: Column (byte offset)

2. **WrapPos** `{ row, col }` - Post-soft-wrap coordinates (internal)
   - row: wrap_row (visual row after soft-wrapping)
   - col: Visual column

3. **DisplayPos** `{ row, col }` - Final display coordinates (public API)
   - row: display_row (visible row after folding)
   - col: Display column

## Module Responsibilities

### DisplayMap (`display_map.rs`)

**Unified public interface (Facade)**

Main features:
- BufferPos ↔ DisplayPos conversion
- Fold control (set_folded, toggle_fold, clear_folds)
- Row count queries (display_row_count, buffer_line_count)
- Text updates (on_text_changed, on_layout_changed)

Temporary accessors (for gradual migration):
- `wrap_map()` - Access the WrapMap layer
- `fold_map()` - Access the FoldMap layer

### WrapMap (`wrap_map.rs`)

**Soft-wrapping mapping layer**

Built on TextWrapper, provides:
- Buffer ↔ Wrap coordinate conversion
- wrap_row ↔ buffer_line queries
- Prefix sum cache for O(1) lookups

Core methods:
- `buffer_pos_to_wrap_pos(pos)` - Buffer → Wrap
- `wrap_pos_to_buffer_pos(pos)` - Wrap → Buffer
- `buffer_line_to_first_wrap_row(line)` - Line number → first wrap_row
- `wrap_row_to_buffer_line(row)` - wrap_row → line number

### FoldMap (`fold_map.rs`)

**Fold projection layer**

Implements folding by filtering wrap rows:
- Maintains a list of visible wrap rows
- Bidirectional Wrap ↔ Display mapping
- Handles fold state changes

Data structures:
- `visible_wrap_rows: Vec<usize>` - display_row → wrap_row
- `wrap_row_to_display_row: Vec<Option<usize>>` - wrap_row → display_row
- `candidates: Vec<FoldRange>` - Fold candidates
- `folded: Vec<FoldRange>` - Currently folded ranges

### Types (`types.rs`)

Core coordinate types:
- `BufferPos` - Buffer position (public)
- `WrapPos` - Soft-wrap position (internal)
- `DisplayPos` - Display position (public)
- `FoldRange` - Fold range (public)

## Usage Examples

### Basic Coordinate Conversion

```rust
use crate::input::{DisplayMap, BufferPos, DisplayPos};

let mut display_map = DisplayMap::new(font, font_size, wrap_width);
display_map.set_text(&rope, cx);

// Buffer → Display
let buffer_pos = BufferPos::new(10, 5);
let display_pos = display_map.buffer_pos_to_display_pos(buffer_pos);

// Display → Buffer
let buffer_pos = display_map.display_pos_to_buffer_pos(display_pos);
```

### Code Folding

```rust
use crate::input::FoldRange;

// Set fold candidates (from tree-sitter/LSP)
let candidates = vec![
    FoldRange::new(10, 15),  // Fold lines 10-15
    FoldRange::new(20, 25),  // Fold lines 20-25
];
display_map.set_fold_candidates(candidates);

// Toggle fold state
display_map.toggle_fold(10);  // Fold/unfold line 10

// Query fold state
if display_map.is_folded_at(10) {
    println!("Line 10 is folded");
}

// Clear all folds
display_map.clear_folds();
```

### Text Updates

```rust
// Text changed
display_map.on_text_changed(&changed_text, &range, &new_text, cx);

// Layout changed (wrap width changed)
display_map.on_layout_changed(Some(new_width), cx);

// Font changed
display_map.set_font(font, font_size, cx);
```

### Accessing Underlying Layers (during gradual migration)

```rust
// Access TextWrapper (for existing rendering code)
let wrapper = display_map.wrap_map().wrapper();
let lines = wrapper.lines;
let longest_row = wrapper.longest_row;

// Access WrapMap
let wrap_count = display_map.wrap_map().wrap_row_count();

// Access FoldMap
let folded_ranges = display_map.fold_map().folded_ranges();
```

## Performance Characteristics

### O(1) Operations
- `display_row_count()` - Precomputed cache
- `buffer_line_to_first_wrap_row()` - Prefix sum array
- `wrap_row_to_display_row()` - Direct array lookup

### O(log n) Operations
- `wrap_row_to_buffer_line()` - Binary search

### Incremental Updates
- Text changes: Only recomputes affected lines (provided by TextWrapper)
- Fold changes: Rebuilds FoldMap mapping (typically fast)

## Design Principles

1. **Separation of Concerns**
   - WrapMap: Only handles soft-wrapping
   - FoldMap: Only handles folding
   - DisplayMap: Unified public interface

2. **Unidirectional Dependencies**
   ```
   FoldMap → WrapMap → TextWrapper
   (Upper layers depend on lower layers; lower layers are unaware of upper layers)
   ```

3. **Internal Detail Hiding**
   - WrapPos is not exposed publicly
   - External code only needs BufferPos and DisplayPos

4. **Gradual Migration Support**
   - Provides wrap_map()/fold_map() accessors
   - Allows existing code to transition smoothly

## Future Extensions

The architecture is designed for extensibility:

- **Inlay Hints** - Can be added as a new mapping layer
- **Block Decorations** - Insert virtual rows
- **Tab Expansion** - Tab character expansion
- **Diff Mapping** - Line mapping for diff views

Extension approach: Insert new layers between WrapMap and FoldMap; DisplayMap remains unchanged.
