use nsh_core::ui::{Children, Either, Event, JsObject, PropEnum, Props};
use serde_json::{Map, Value};
#[derive(Clone, Copy, PartialEq, PropEnum, Default)]
pub(super) enum CollectionSize {
    Xs,
    Sm,
    #[default]
    Md,
    Lg,
}
impl CollectionSize {
    pub(super) fn resolve(self) -> gpui_kit::component::Size {
        match self {
            Self::Xs => gpui_kit::component::Size::XSmall,
            Self::Sm => gpui_kit::component::Size::Small,
            Self::Md => gpui_kit::component::Size::Medium,
            Self::Lg => gpui_kit::component::Size::Large,
        }
    }
}
#[derive(Clone, Copy, PartialEq, PropEnum, Default)]
pub(super) enum TableAlignment {
    #[default]
    Left,
    Center,
    Right,
}
impl TableAlignment {
    pub(super) fn text(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }
}
#[derive(Clone, PartialEq, JsObject)]
pub(super) struct ListItem {
    pub(super) id: String,
    pub(super) label: String,
    #[prop(default)]
    pub(super) disabled: bool,
    #[prop(default)]
    pub(super) section: String,
}
#[derive(Clone, PartialEq, Props)]
pub(super) struct ListProps {
    pub(super) children: Children,
    #[prop(default)]
    pub(super) items: Vec<ListItem>,
    pub(super) selected_id: Option<String>,
    pub(super) query: Option<String>,
    #[prop(default)]
    pub(super) searchable: bool,
    #[prop(default = true)]
    pub(super) selectable: bool,
    #[prop(default)]
    pub(super) loading: bool,
    #[prop(default)]
    pub(super) has_more: bool,
    pub(super) placeholder: Option<String>,
}
#[derive(Clone, Event)]
pub(super) enum ListEvent {
    SelectionChange(Option<String>),
    QueryChange(String),
    Confirm { id: String, secondary: bool },
    Cancel,
    LoadMore,
    Error(String),
}
#[derive(Clone, PartialEq, JsObject)]
pub(super) struct PickerItem {
    pub(super) value: String,
    pub(super) label: String,
    #[prop(default)]
    pub(super) disabled: bool,
}
#[derive(Clone, PartialEq, JsObject)]
pub(super) struct PickerGroup {
    pub(super) label: String,
    pub(super) items: Vec<PickerItem>,
}
#[derive(Clone, PartialEq, Props)]
pub(super) struct ComboboxProps {
    pub(super) children: Children,
    #[prop(default)]
    pub(super) items: Vec<PickerItem>,
    pub(super) groups: Option<Vec<PickerGroup>>,
    pub(super) values: Option<Vec<String>>,
    pub(super) query: Option<String>,
    #[prop(default)]
    pub(super) multiple: bool,
    #[prop(default = true)]
    pub(super) searchable: bool,
    #[prop(default)]
    pub(super) disabled: bool,
    #[prop(default)]
    pub(super) cleanable: bool,
    pub(super) placeholder: Option<String>,
    pub(super) search_placeholder: Option<String>,
}
#[derive(Clone, Event)]
pub(super) enum ComboboxEvent {
    Change(Vec<String>),
    Confirm(Vec<String>),
    Error(String),
}
#[derive(Clone, PartialEq, JsObject)]
pub(super) struct TreeItem {
    pub(super) id: String,
    pub(super) label: String,
    #[prop(default)]
    pub(super) disabled: bool,
    #[prop(default)]
    pub(super) expanded: bool,
    #[prop(default)]
    pub(super) children: Vec<TreeItem>,
}
#[derive(Clone, PartialEq, Props)]
pub(super) struct TreeProps {
    pub(super) children: Children,
    #[prop(default)]
    pub(super) items: Vec<TreeItem>,
    pub(super) selected_id: Option<String>,
    pub(super) expanded_ids: Option<Vec<String>>,
}
#[derive(Clone, Event)]
pub(super) enum TreeEvent {
    SelectionChange(Option<String>),
    ExpandedChange { id: String, expanded: bool },
    Confirm(String),
    Error(String),
}
#[derive(Clone, PartialEq, JsObject)]
pub(super) struct CommandItem {
    pub(super) id: String,
    pub(super) label: String,
    #[prop(default)]
    pub(super) group: String,
    #[prop(default)]
    pub(super) disabled: bool,
    #[prop(default)]
    pub(super) checked: bool,
    #[prop(default)]
    pub(super) keywords: Vec<String>,
}
#[derive(Clone, PartialEq, Props)]
pub(super) struct CommandProps {
    pub(super) children: Children,
    #[prop(default)]
    pub(super) items: Vec<CommandItem>,
    pub(super) selected_id: Option<String>,
    pub(super) query: Option<String>,
    #[prop(default = true)]
    pub(super) searchable: bool,
    #[prop(default = true)]
    pub(super) filterable: bool,
    #[prop(default)]
    pub(super) loading: bool,
    pub(super) placeholder: Option<String>,
}
#[derive(Clone, Event)]
pub(super) enum CommandEvent {
    QueryChange(String),
    SelectionChange(Option<String>),
    Confirm(Option<String>),
    Cancel,
    Error(String),
}
#[derive(Clone, PartialEq, JsObject)]
pub(super) struct TableColumn {
    pub(super) key: String,
    pub(super) label: String,
    #[prop(default = 120.0)]
    pub(super) width: f64,
    #[prop(default)]
    pub(super) sortable: bool,
    #[prop(default = true)]
    pub(super) resizable: bool,
    #[prop(default = true)]
    pub(super) movable: bool,
    #[prop(default)]
    pub(super) align: TableAlignment,
}
#[derive(Clone, PartialEq, JsObject)]
pub(super) struct TableRow {
    pub(super) id: String,
    pub(super) cells: Map<String, Value>,
}
#[derive(Clone, PartialEq, JsObject)]
pub(super) struct TableRowSelection {
    row: String,
    column: Option<String>,
}
#[derive(Clone, PartialEq, JsObject)]
pub(super) struct TableColumnSelection {
    row: Option<String>,
    column: String,
}
crate::registration::named_value! {pub(super) struct TableSelection(Option<Either<TableRowSelection,TableColumnSelection>>);}
impl TableSelection {
    pub(super) fn from_parts(row: Option<String>, column: Option<String>) -> Option<Self> {
        match (row, column) {
            (Some(row), column) => Some(Self::new(Some(Either::Left(TableRowSelection {
                row,
                column,
            })))),
            (None, Some(column)) => Some(Self::new(Some(Either::Right(TableColumnSelection {
                row: None,
                column,
            })))),
            _ => None,
        }
    }
    pub(super) fn row(&self) -> Option<&String> {
        match self.value() {
            Some(Either::Left(value)) => Some(&value.row),
            Some(Either::Right(value)) => value.row.as_ref(),
            None => None,
        }
    }
    pub(super) fn column(&self) -> Option<&String> {
        match self.value() {
            Some(Either::Left(value)) => value.column.as_ref(),
            Some(Either::Right(value)) => Some(&value.column),
            None => None,
        }
    }
}
#[derive(Clone, Copy, PartialEq, PropEnum, Default)]
pub(super) enum SortMode {
    #[default]
    Local,
    Manual,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(super) enum SortDirection {
    Ascending,
    Descending,
    Default,
}
#[derive(Clone, PartialEq, Props)]
pub(super) struct DataTableProps {
    pub(super) children: Children,
    #[prop(default)]
    pub(super) columns: Vec<TableColumn>,
    #[prop(default)]
    pub(super) rows: Vec<TableRow>,
    pub(super) selection: Option<TableSelection>,
    #[prop(default)]
    pub(super) stripe: bool,
    #[prop(default = true)]
    pub(super) bordered: bool,
    #[prop(default)]
    pub(super) loading: bool,
    #[prop(default)]
    pub(super) has_more: bool,
    #[prop(default)]
    pub(super) cell_selectable: bool,
    pub(super) caption: Option<String>,
    #[prop(default)]
    pub(super) sort_mode: SortMode,
}
#[derive(Clone, PartialEq, Props)]
pub(super) struct TableProps {
    pub(super) children: Children,
    #[prop(default)]
    pub(super) columns: Vec<TableColumn>,
    #[prop(default)]
    pub(super) rows: Vec<TableRow>,
    pub(super) caption: Option<String>,
}
#[derive(Clone, Event)]
pub(super) enum DataTableEvent {
    SelectionChange(Option<TableSelection>),
    Confirm(Option<TableSelection>),
    ContextMenu {
        row: Option<String>,
        column: Option<String>,
    },
    Sort {
        column: String,
        direction: SortDirection,
        row_ids: Vec<String>,
    },
    ColumnWidthsChange(Vec<f64>),
    ColumnMove {
        from: usize,
        to: usize,
        keys: Vec<String>,
    },
    VisibleRowsChange {
        start: usize,
        end: usize,
    },
    LoadMore,
    Export {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Error(String),
}
#[derive(Clone, Event)]
pub(super) enum TableEvent {
    Error(String),
}
#[derive(Clone, PartialEq, Props)]
pub(super) struct PaginationProps {
    #[prop(default = 1)]
    pub(super) page: usize,
    #[prop(default = 1)]
    pub(super) total_pages: usize,
    #[prop(default = 5)]
    pub(super) visible_pages: usize,
    #[prop(default)]
    pub(super) compact: bool,
    #[prop(default)]
    pub(super) disabled: bool,
    #[prop(default)]
    pub(super) size: CollectionSize,
}
#[derive(Clone, Event)]
pub(super) enum PaginationEvent {
    Change(usize),
    Error(String),
}

impl From<String> for ListEvent {
    fn from(value: String) -> Self {
        Self::Error(value)
    }
}
impl From<String> for ComboboxEvent {
    fn from(value: String) -> Self {
        Self::Error(value)
    }
}
impl From<String> for TreeEvent {
    fn from(value: String) -> Self {
        Self::Error(value)
    }
}
impl From<String> for CommandEvent {
    fn from(value: String) -> Self {
        Self::Error(value)
    }
}
impl From<String> for DataTableEvent {
    fn from(value: String) -> Self {
        Self::Error(value)
    }
}
impl From<String> for TableEvent {
    fn from(value: String) -> Self {
        Self::Error(value)
    }
}
impl From<String> for PaginationEvent {
    fn from(value: String) -> Self {
        Self::Error(value)
    }
}
