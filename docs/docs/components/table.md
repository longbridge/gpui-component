---
title: Table
description: High-performance data table with virtual scrolling, sorting, filtering, and column management.
---

# Table

A comprehensive data table component designed for handling large datasets with high performance. Features virtual scrolling, column configuration, sorting, filtering, row selection, and custom cell rendering. Perfect for displaying tabular data with thousands of rows while maintaining smooth performance.

## Import

```rust
use gpui_component::table::{Table, TableState, Column, ColumnSort, ColumnFixed, TableEvent};
```

- [Table]: The table UI element.
- [TableState]: State management for the table to hold selected rows, column widths, scroll positions, etc.

## Usage

### Basic Table

To create a table, you need use [TableState] to manage the state and use [Table] to render the table.

```rust
use std::ops::Range;
use gpui::{App, Context, Window, IntoElement};
use gpui_component::table::{Table, TableState, Column, ColumnSort};

struct User {
    id: usize,
    name: String,
    age: u32,
    email: String,
}

struct UserList {
    table: Entity<TableState>,
    columns: Vec<Column>,
    users: Vec<User>,
}

impl UserList {
    fn new(window: &mut Window, cx: &mut App) -> Self {
        let columns = vec![
            Column::new("id", "ID").width(60.),
            Column::new("name", "Name").width(150.).sortable(),
            Column::new("age", "Age").width(80.).sortable(),
            Column::new("email", "Email").width(200.),
        ];

        let users = vec![
            User { id: 1, name: "John".to_string(), age: 30, email: "john@example.com".to_string() },
            User { id: 2, name: "Jane".to_string(), age: 25, email: "jane@example.com".to_string() },
        ];

        let table = cx.new(|cx| {
            TableState::new(columns.clone, users.len(), window, cx)
                .col_movable(true)
                .sortable(true)
                .row_selectable(true)
                .col_selectable(true)
        });

        Self {
            table,
            columns,
            users:
        }
    }

    fn render_td(&self, row_ix: usize, col_ix: usize, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
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

impl Render for UserList {
    fn render(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .child(
                Table::new(&self.table)
                    .stripe(true)
                    .cell(cx.processor(|this, (row_ix, col_ix), window, cx| {
                        let row = &this.data[row_ix];
                        let col = &this.columns[col_ix];

                        match col.key.as_ref() {
                            "id" => row.id.to_string(),
                            "name" => row.name.clone(),
                            "age" => row.age.to_string(),
                            "email" => row.email.clone(),
                            _ => "".to_string(),
                        }.into_any_element()
                    }))
            )
    }
}
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

### Sorting Implementation

Implement sorting in your delegate:

```rust
Table::new(&self.table)
    .on_sort(cx.processor(|this, (col_ix, sort), window, cx| {
        let col = &this.columns[col_ix];

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
    }))
```

### Custom Row Rendering

Use `row` method for custom row elements:

```rust
Table::new(&self.table)
    .row(cx.processor(|this, row_ix, window, cx| {
        div()
            .id(row_ix)
            .on_click(cx.listener(move |_, ev, _, _| {
                if ev.modifiers().secondary() {
                    println!("Right-clicked row {}", row_ix);
                } else {
                    println!("Selected row {}", row_ix);
                }
            }))
    }))
```

### Context Menu

Use `context_menu` method for row actions to build context menus:

```rust
Table::new(&self.table)
    .context_menu(|(row_ix, menu), _, _| {
        menu.menu(format!("Edit {}", row.name), Box::new(EditRowAction(row_ix)))
            .menu("Delete", Box::new(DeleteRowAction(row_ix)))
            .separator()
            .menu("Duplicate", Box::new(DuplicateRowAction(row_ix)))
    })
}

// Handle table events
cx.subscribe_in(&table, window, |view, table, event, _, cx| {
    match event {
        TableEvent::SelectRow(row_ix) => {
            println!("Row {} selected", row_ix);
        }
        TableEvent::DoubleClickedRow(row_ix) => {
            println!("Row {} double-clicked", row_ix);
            // Open detail view or edit mode
        }
        TableEvent::SelectColumn(col_ix) => {
            println!("Column {} selected", col_ix);
        }
        _ => {}
    }
}).detach();
```

### Custom Cell Rendering

Create rich cell content with custom rendering:

```rust
Table::new(&self.table)
    .stripe(true)
    .cell(cx.processor(|this, (row_ix, col_ix), window, cx| {
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
    }))
```

### Column Resizing and Moving

Enable dynamic column management:

```rust
// Configure table features
let table = cx.new(|cx| {
    TableState::new(columns, rows_count, window, cx)
        .col_resizable(true)  // Allow column resizing
        .col_movable(true)    // Allow column reordering
        .sortable(true)       // Enable sorting
        .col_selectable(true) // Allow column selection
        .row_selectable(true) // Allow row selection
});

// Listen for column changes
cx.subscribe_in(&table, window, |view, table, event, _, cx| {
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
Table::new(&self.table)
    // Set loading state to show skeleton.
    .loading(self.loading)
    .on_load_more(cx.processor(|this, _, window, cx| {
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
    }))
}
```

### Table Styling

Customize table appearance:

```rust
Table::new(&self.table)
    .stripe(true)           // Alternating row colors
    .border(true)           // Border around table
    .scrollbar_visible(true, true) // Vertical, horizontal scrollbars
```

## Accessibility

- Full keyboard navigation support (Tab, Arrow keys, Enter, Space, Escape)
- Screen reader support with proper ARIA labels
- High contrast mode support
- Focus indicators for all interactive elements
- Keyboard shortcuts:
  - `↑/↓` - Navigate rows
  - `←/→` - Navigate columns
  - `Enter/Space` - Select row/column
  - `Escape` - Clear selection
- Loading states announced to screen readers
- Sort order changes announced
- Row/column count announced

[Table]: https://docs.rs/gpui-component/latest/gpui_component/table/struct.Table.html
[TableState]: https://docs.rs/gpui-component/latest/gpui_component/table/struct.TableState.html
