---
title: Table
description: High-performance data table with virtual scrolling, sorting, filtering, and column management.
---

# Table

A comprehensive data table component designed for handling large datasets with high performance. Features virtual scrolling, column configuration, sorting, filtering, row selection, and custom cell rendering. Perfect for displaying tabular data with thousands of rows while maintaining smooth performance.

## Import

```rust
use gpui_component::table::{Table, TableState, TableDelegate, Column, ColumnSort, ColumnFixed, TableEvent};
```

## Usage

### Basic Table

To create a table, you need to implement the `TableDelegate` trait and provide column definitions, and use `TableState` to manage the table state.

```rust
use std::ops::Range;
use gpui::{App, Context, Window, IntoElement};
use gpui_component::table::{Table, TableDelegate, Column, ColumnSort};

struct MyData {
    id: usize,
    name: String,
    age: u32,
    email: String,
}

struct MyTableDelegate {
    data: Vec<MyData>,
    columns: Vec<Column>,
}

impl MyTableDelegate {
    fn new() -> Self {
        Self {
            data: vec![
                MyData { id: 1, name: "John".to_string(), age: 30, email: "john@example.com".to_string() },
                MyData { id: 2, name: "Jane".to_string(), age: 25, email: "jane@example.com".to_string() },
            ],
            columns: vec![
                Column::new("id", "ID").width(60.),
                Column::new("name", "Name").width(150.).sortable(),
                Column::new("age", "Age").width(80.).sortable(),
                Column::new("email", "Email").width(200.),
            ],
        }
    }
}

impl TableDelegate for MyTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.data.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> &Column {
        &self.columns[col_ix]
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, _: &mut Context<TableState<Self>>) -> impl IntoElement {
        let row = &self.data[row_ix];
        let col = &self.columns[col_ix];

        match col.key.as_ref() {
            "id" => row.id.to_string(),
            "name" => row.name.clone(),
            "age" => row.age.to_string(),
            "email" => row.email.clone(),
            _ => "".to_string(),
        }
    }
}

// Create the table
let delegate = MyTableDelegate::new();
let state = cx.new(|cx| TableState::new(delegate, window, cx));
```

### Column Configuration

Columns provide extensive configuration options:

```rust
// Basic column
Column::new("id", "ID")

// Sortable column
Column::new("name", "Name")
    .sortable()
    .width(150.)

// Right-aligned column
Column::new("price", "Price")
    .text_right()
    .sortable()

// Fixed column (pinned to left)
Column::new("actions", "Actions")
    .fixed(ColumnFixed::Left)
    .resizable(false)
    .movable(false)

// Column with custom padding
Column::new("description", "Description")
    .width(200.)
    .paddings(px(8.))

// Non-resizable column
Column::new("status", "Status")
    .width(100.)
    .resizable(false)

// Custom sort orders
Column::new("created", "Created")
    .ascending() // Default ascending
// or
Column::new("modified", "Modified")
    .descending() // Default descending
```

### Virtual Scrolling for Large Datasets

The table automatically handles virtual scrolling for optimal performance:

```rust
struct LargeDataDelegate {
    data: Vec<Record>, // Could be 10,000+ items
    columns: Vec<Column>,
}

impl TableDelegate for LargeDataDelegate {
    fn rows_count(&self, _: &App) -> usize {
        self.data.len() // No performance impact regardless of size
    }

    // Only visible rows are rendered
    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, _: &mut Context<TableState<Self>>) -> impl IntoElement {
        // This is only called for visible rows
        // Efficiently render cell content
        let row = &self.data[row_ix];
        format_cell_data(row, col_ix)
    }

    // Track visible range for optimizations
    fn visible_rows_changed(&mut self, visible_range: Range<usize>, _: &mut Window, _: &mut Context<TableState<Self>>) {
        // Only update data for visible rows if needed
        // This is called when user scrolls
    }
}
```

### Sorting Implementation

Implement sorting in your delegate:

```rust
impl TableDelegate for MyTableDelegate {
    fn perform_sort(&mut self, col_ix: usize, sort: ColumnSort, _: &mut Window, _: &mut Context<TableState<Self>>) {
        let col = &self.columns[col_ix];

        match col.key.as_ref() {
            "name" => {
                match sort {
                    ColumnSort::Ascending => self.data.sort_by(|a, b| a.name.cmp(&b.name)),
                    ColumnSort::Descending => self.data.sort_by(|a, b| b.name.cmp(&a.name)),
                    ColumnSort::Default => {
                        // Reset to original order or default sort
                        self.data.sort_by(|a, b| a.id.cmp(&b.id));
                    }
                }
            }
            "age" => {
                match sort {
                    ColumnSort::Ascending => self.data.sort_by(|a, b| a.age.cmp(&b.age)),
                    ColumnSort::Descending => self.data.sort_by(|a, b| b.age.cmp(&a.age)),
                    ColumnSort::Default => self.data.sort_by(|a, b| a.id.cmp(&b.id)),
                }
            }
            _ => {}
        }
    }
}
```

### ContextMenu

```rust
impl TableDelegate for MyTableDelegate {
    // Context menu for right-click
    fn context_menu(&mut self, row_ix: usize, menu: PopupMenu, _: &mut Window, _: &mut Context<TableState<Self>>) -> PopupMenu {
        let row = &self.data[row_ix];
        menu.menu(format!("Edit {}", row.name), Box::new(EditRowAction(row_ix)))
            .menu("Delete", Box::new(DeleteRowAction(row_ix)))
            .separator()
            .menu("Duplicate", Box::new(DuplicateRowAction(row_ix)))
    }
}
```

### Cell Rendering

Create rich cell content with custom rendering:

```rust
impl TableDelegate for MyTableDelegate {
    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, cx: &mut Context<TableState<Self>>) -> impl IntoElement {
        let row = &self.data[row_ix];
        let col = &self.columns[col_ix];

        match col.key.as_ref() {
            "status" => {
                // Custom status badge
                let (color, text) = match row.status {
                    Status::Active => (cx.theme().green, "Active"),
                    Status::Inactive => (cx.theme().red, "Inactive"),
                    Status::Pending => (cx.theme().yellow, "Pending"),
                };

                div()
                    .px_2()
                    .py_1()
                    .rounded(px(4.))
                    .bg(color.opacity(0.1))
                    .text_color(color)
                    .child(text)
            }
            "progress" => {
                // Progress bar
                div()
                    .w_full()
                    .h(px(8.))
                    .bg(cx.theme().muted)
                    .rounded(px(4.))
                    .child(
                        div()
                            .h_full()
                            .w(percentage(row.progress))
                            .bg(cx.theme().primary)
                            .rounded(px(4.))
                    )
            }
            "actions" => {
                // Action buttons
                h_flex()
                    .gap_1()
                    .child(Button::new(format!("edit-{}", row_ix)).text().icon(IconName::Edit))
                    .child(Button::new(format!("delete-{}", row_ix)).text().icon(IconName::Trash))
            }
            "avatar" => {
                // User avatar with image
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(32.))
                            .h(px(32.))
                            .rounded_full()
                            .bg(cx.theme().accent)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(row.name.chars().next().unwrap_or('?').to_string())
                    )
                    .child(row.name.clone())
            }
            _ => row.get_field_value(col.key.as_ref()).into_any_element(),
        }
    }
}
```

### Column Resizing and Moving

Enable dynamic column management:

```rust
// Configure table features
let state = cx.new(|cx| {
    TableState::new(delegate, window, cx)
        .col_resizable(true)  // Allow column resizing
        .col_movable(true)    // Allow column reordering
        .sortable(true)       // Enable sorting
        .col_selectable(true) // Allow column selection
        .row_selectable(true) // Allow row selection
});

// Listen for column changes
cx.subscribe_in(&state, window, |view, table, event, _, cx| {
    match event {
        TableEvent::ColumnWidthsChanged(widths) => {
            // Save column widths to user preferences
            save_column_widths(widths);
        }
        TableEvent::MoveColumn(from_ix, to_ix) => {
            // Save column order
            save_column_order(from_ix, to_ix);
        }
        _ => {}
    }
}).detach();
```

### Infinite Loading / Pagination

Implement loading more data as user scrolls:

```rust
impl TableDelegate for MyTableDelegate {
    fn has_more(&self, _: &App) -> bool {
        self.has_more_data
    }

    fn load_more_threshold(&self) -> usize {
        50 // Load more when 50 rows from bottom
    }

    fn load_more(&mut self, _: &mut Window, cx: &mut Context<TableState<Self>>) {
        if self.loading {
            return; // Prevent multiple loads
        }

        self.loading = true;

        // Spawn async task to load data
        cx.spawn(async move |view, cx| {
            let new_data = fetch_more_data().await;

            cx.update(|cx| {
                view.update(cx, |view, _| {
                    let delegate = view.table.delegate_mut();
                    delegate.data.extend(new_data);
                    delegate.loading = false;
                    delegate.has_more_data = !new_data.is_empty();
                });
            })
        }).detach();
    }

    fn loading(&self, _: &App) -> bool {
        self.loading
    }
}
```

### Table Styling

Customize table appearance:

```rust
let state = cx.new(|cx| {
    TableState::new(delegate, window, cx)
});

// In render
Table::new(&state)
    .stripe(true)           // Alternating row colors
    .bordered(true)           // Border around table
    .scrollbar_visible(true, true) // Vertical, horizontal scrollbars
```

## Examples

### Financial Data Table

```rust
struct StockData {
    symbol: String,
    price: f64,
    change: f64,
    change_percent: f64,
    volume: u64,
}

impl TableDelegate for StockTableDelegate {
    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, cx: &mut Context<TableState<Self>>) -> impl IntoElement {
        let stock = &self.stocks[row_ix];
        let col = &self.columns[col_ix];

        match col.key.as_ref() {
            "symbol" => div().font_weight(FontWeight::BOLD).child(stock.symbol.clone()),
            "price" => div().text_right().child(format!("${:.2}", stock.price)),
            "change" => {
                let color = if stock.change >= 0.0 { cx.theme().green } else { cx.theme().red };
                div()
                    .text_right()
                    .text_color(color)
                    .child(format!("{:+.2}", stock.change))
            }
            "change_percent" => {
                let color = if stock.change_percent >= 0.0 { cx.theme().green } else { cx.theme().red };
                div()
                    .text_right()
                    .text_color(color)
                    .child(format!("{:+.1}%", stock.change_percent * 100.0))
            }
            "volume" => div().text_right().child(format!("{:,}", stock.volume)),
            _ => div(),
        }
    }
}
```

### User Management Table

```rust
struct UserTableDelegate {
    users: Vec<User>,
    columns: Vec<Column>,
}

impl UserTableDelegate {
    fn new() -> Self {
        Self {
            users: Vec::new(),
            columns: vec![
                Column::new("avatar", "").width(50.).resizable(false).movable(false),
                Column::new("name", "Name").width(150.).sortable().fixed_left(),
                Column::new("email", "Email").width(200.).sortable(),
                Column::new("role", "Role").width(100.).sortable(),
                Column::new("status", "Status").width(100.),
                Column::new("last_login", "Last Login").width(120.).sortable(),
                Column::new("actions", "Actions").width(100.).resizable(false),
            ],
        }
    }
}
```

## Keyboard shortcuts

- `↑/↓` - Navigate rows
- `←/→` - Navigate columns
- `Enter/Space` - Select row/column
- `Escape` - Clear selection
