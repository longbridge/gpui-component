use super::schema::{
    DataTableEvent as BindingEvent, DataTableProps, SortDirection, TableSelection as Selection,
};
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement, SharedString, Styled, Subscription, Window,
    component::table::{
        Column, ColumnSort, DataTable, Table, TableBody, TableCaption, TableCell, TableDelegate,
        TableEvent, TableHead, TableHeader, TableRow, TableSelection, TableState,
    },
    div,
    prelude::FluentBuilder,
    px,
};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
type ComponentContext = TypedComponentContext<BindingEvent>;
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
#[derive(Clone)]
struct ColumnData {
    key: String,
    label: SharedString,
    width: f32,
    sortable: bool,
    resizable: bool,
    movable: bool,
    align: String,
}
#[derive(Clone)]
struct Row {
    id: String,
    cells: HashMap<String, Value>,
    text: HashMap<String, SharedString>,
}
fn cell_text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(value)) => value.clone(),
        Some(value) => value.to_string(),
    }
}
struct Delegate {
    context: ComponentContext,
    columns: Vec<ColumnData>,
    rows: Vec<Row>,
    loading: bool,
    more: bool,
    original_order: Vec<String>,
    sort: Option<(String, ColumnSort)>,
    local_sort: bool,
    selection: Rc<RefCell<Option<Selection>>>,
}
impl Delegate {
    fn row_id(&self, ix: usize) -> Option<String> {
        self.rows.get(ix).map(|row| row.id.clone())
    }
    fn column_id(&self, ix: usize) -> Option<String> {
        self.columns.get(ix).map(|column| column.key.clone())
    }
    fn cell(&self, row: usize, col: usize) -> AnyElement {
        let Some(row) = self.rows.get(row) else {
            return div().into_any_element();
        };
        let Some(column) = self.columns.get(col) else {
            return div().into_any_element();
        };
        crate::bridge::slot(&self.context, &format!("cell:{}:{}", row.id, column.key))
            .unwrap_or_else(|| {
                row.text
                    .get(&column.key)
                    .cloned()
                    .unwrap_or_default()
                    .into_any_element()
            })
    }
    fn sort_rows(&mut self) {
        if !self.local_sort {
            return;
        }
        let Some((key, sort)) = self.sort.clone() else {
            return;
        };
        if sort != ColumnSort::Default {
            self.rows.sort_by(|a, b| {
                let left = a.cells.get(&key);
                let right = b.cells.get(&key);
                let order = match (left.and_then(Value::as_f64), right.and_then(Value::as_f64)) {
                    (Some(left), Some(right)) => left.total_cmp(&right),
                    _ => cell_text(left).cmp(&cell_text(right)),
                };
                if sort == ColumnSort::Descending {
                    order.reverse()
                } else {
                    order
                }
            });
        }
        if sort == ColumnSort::Default {
            let order: HashMap<&str, usize> = self
                .original_order
                .iter()
                .enumerate()
                .map(|(ix, id)| (id.as_str(), ix))
                .collect();
            self.rows
                .sort_by_key(|row| order.get(row.id.as_str()).copied().unwrap_or(usize::MAX));
        }
    }
    fn selection_value(&self, selection: TableSelection) -> Option<Selection> {
        match selection {
            TableSelection::Row(row) => Selection::from_parts(self.row_id(row), None),
            TableSelection::Column(column) => Selection::from_parts(None, self.column_id(column)),
            TableSelection::Cell(row, column) => {
                Selection::from_parts(self.row_id(row), self.column_id(column))
            }
            TableSelection::None => None,
        }
    }
    fn resolve_selection(&self, selection: &Option<Selection>) -> TableSelection {
        let row = selection
            .as_ref()
            .and_then(|selection| selection.row())
            .and_then(|id| self.rows.iter().position(|row| &row.id == id));
        let column = selection
            .as_ref()
            .and_then(|selection| selection.column())
            .and_then(|key| self.columns.iter().position(|column| &column.key == key));
        match (row, column) {
            (Some(row), Some(column)) => TableSelection::Cell(row, column),
            (Some(row), _) => TableSelection::Row(row),
            (_, Some(column)) => TableSelection::Column(column),
            _ => TableSelection::None,
        }
    }
    fn event(&self, event: &TableEvent) -> BindingEvent {
        match event {
            TableEvent::SelectRow(row) => {
                BindingEvent::SelectionChange(self.selection_value(TableSelection::Row(*row)))
            }
            TableEvent::SelectColumn(column) => {
                BindingEvent::SelectionChange(self.selection_value(TableSelection::Column(*column)))
            }
            TableEvent::SelectCell(row, column) => BindingEvent::SelectionChange(
                self.selection_value(TableSelection::Cell(*row, *column)),
            ),
            TableEvent::ClearSelection => BindingEvent::SelectionChange(None),
            TableEvent::DoubleClickedRow(row) => {
                BindingEvent::Confirm(self.selection_value(TableSelection::Row(*row)))
            }
            TableEvent::DoubleClickedCell(row, column) => {
                BindingEvent::Confirm(self.selection_value(TableSelection::Cell(*row, *column)))
            }
            TableEvent::RightClickedRow(row) => BindingEvent::ContextMenu {
                row: row.and_then(|row| self.row_id(row)),
                column: None,
            },
            TableEvent::RightClickedCell(row, column) => BindingEvent::ContextMenu {
                row: self.row_id(*row),
                column: self.column_id(*column),
            },
            TableEvent::ColumnWidthsChanged(widths) => BindingEvent::ColumnWidthsChange(
                widths
                    .iter()
                    .map(|width| f32::from(*width) as f64)
                    .collect(),
            ),
            TableEvent::MoveColumn(from, to) => BindingEvent::ColumnMove {
                from: *from,
                to: *to,
                keys: self
                    .columns
                    .iter()
                    .map(|column| column.key.clone())
                    .collect(),
            },
        }
    }
}
impl TableDelegate for Delegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }
    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        let data = &self.columns[ix];
        Column::new(data.key.clone(), data.label.clone())
            .width(px(data.width))
            .resizable(data.resizable)
            .movable(data.movable)
            .when(data.sortable, |this| {
                this.sort(
                    self.sort
                        .as_ref()
                        .filter(|(key, _)| key == &data.key)
                        .map_or(ColumnSort::Default, |(_, sort)| *sort),
                )
            })
            .when(data.align == "right", |this| this.text_right())
            .when(data.align == "center", |this| this.text_center())
    }
    fn perform_sort(
        &mut self,
        ix: usize,
        sort: ColumnSort,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        let Some(column) = self.columns.get(ix) else {
            return;
        };
        let key = column.key.clone();
        self.sort = Some((key.clone(), sort));
        self.sort_rows();
        let selection = self.selection.borrow().clone();
        let selected = self.resolve_selection(&selection);
        cx.defer_in(window, move |state, _, cx| {
            if state.selection() != selected {
                state.set_selection(selected, cx);
            }
        });
        let direction = match sort {
            ColumnSort::Ascending => SortDirection::Ascending,
            ColumnSort::Descending => SortDirection::Descending,
            ColumnSort::Default => SortDirection::Default,
        };
        self.context.emit(BindingEvent::Sort {
            column: key,
            direction,
            row_ids: self.rows.iter().map(|row| row.id.clone()).collect(),
        });
        cx.notify();
    }
    fn render_th(
        &mut self,
        col: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let column = &self.columns[col];
        crate::bridge::slot(&self.context, &format!("header:{}", column.key))
            .unwrap_or_else(|| column.label.clone().into_any_element())
    }
    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        self.cell(row, col)
    }
    fn move_column(
        &mut self,
        col: usize,
        to: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        let column = self.columns.remove(col);
        self.columns.insert(to, column);
    }
    fn cell_text(&self, row: usize, col: usize, _: &App) -> String {
        self.rows
            .get(row)
            .and_then(|row| {
                self.columns
                    .get(col)
                    .map(|column| cell_text(row.cells.get(&column.key)))
            })
            .unwrap_or_default()
    }
    fn loading(&self, _: &App) -> bool {
        self.loading
    }
    fn has_more(&self, _: &App) -> bool {
        self.more && !self.loading
    }
    fn load_more(&mut self, _: &mut Window, _: &mut Context<TableState<Self>>) {
        self.context.emit(BindingEvent::LoadMore);
    }
    fn render_empty(
        &mut self,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        crate::bridge::slot(&self.context, "empty").unwrap_or_else(|| div().into_any_element())
    }
    fn visible_rows_changed(
        &mut self,
        range: std::ops::Range<usize>,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        self.context.emit(BindingEvent::VisibleRowsChange {
            start: range.start,
            end: range.end,
        });
    }
}
type State = TableState<Delegate>;
pub(super) struct NativeTable {
    context: ComponentContext,
    virtualized: bool,
    state: Option<Entity<State>>,
    events: Option<Subscription>,
    focus: Option<FocusHandle>,
    columns: Option<Vec<ColumnData>>,
    rows: Option<Vec<Row>>,
    selection: Option<Option<Selection>>,
    reported: Rc<RefCell<Option<Selection>>>,
    stripe: bool,
    bordered: bool,
    loading: bool,
    more: bool,
    caption: Option<SharedString>,
    cell_selection: bool,
    local_sort: bool,
}
impl NativeTable {
    pub(super) fn new(context: ComponentContext, virtualized: bool) -> Self {
        Self {
            context,
            virtualized,
            state: None,
            events: None,
            focus: None,
            columns: None,
            rows: None,
            selection: None,
            reported: Rc::new(RefCell::new(None)),
            stripe: false,
            bordered: true,
            loading: false,
            more: false,
            caption: None,
            cell_selection: false,
            local_sort: true,
        }
    }
    fn sync(&mut self, window: &mut Window, cx: &mut App) -> Entity<State> {
        let state = self
            .state
            .get_or_insert_with(|| {
                let state = cx.new(|cx| {
                    TableState::new(
                        Delegate {
                            context: self.context.clone(),
                            columns: Vec::new(),
                            rows: Vec::new(),
                            loading: false,
                            more: false,
                            original_order: Vec::new(),
                            sort: None,
                            local_sort: self.local_sort,
                            selection: self.reported.clone(),
                        },
                        window,
                        cx,
                    )
                    .cell_selectable(self.cell_selection)
                });
                let context = self.context.clone();
                let reported = self.reported.clone();
                self.events = Some(window.subscribe(&state, cx, move |state, event, _, cx| {
                    if let TableEvent::ColumnWidthsChanged(widths) = event {
                        state.update(cx, |state, _| {
                            for (column, width) in
                                state.delegate_mut().columns.iter_mut().zip(widths)
                            {
                                column.width = f32::from(*width);
                            }
                        });
                    }
                    let delegate = state.read(cx).delegate();
                    let event = delegate.event(event);
                    if let BindingEvent::SelectionChange(value) = &event {
                        if &*reported.borrow() == value {
                            return;
                        }
                        *reported.borrow_mut() = value.clone();
                    }
                    context.emit(event);
                }));
                state
            })
            .clone();
        state.update(cx, |state, cx| {
            let changed = self.columns.is_some() || self.rows.is_some();
            let old = state.delegate().selection_value(state.selection());
            if let Some(columns) = self.columns.take() {
                state.delegate_mut().columns = columns;
            }
            state.delegate_mut().local_sort = self.local_sort;
            if let Some(rows) = self.rows.take() {
                state.delegate_mut().original_order =
                    rows.iter().map(|row| row.id.clone()).collect();
                state.delegate_mut().rows = rows;
                state.delegate_mut().sort_rows();
            }
            state.set_cell_selectable(self.cell_selection, cx);
            if state.delegate().loading != self.loading || state.delegate().more != self.more {
                state.delegate_mut().loading = self.loading;
                state.delegate_mut().more = self.more;
                cx.notify();
            }
            if changed {
                state.refresh(cx);
            }
            if let Some(selection) = self.selection.take().or_else(|| changed.then_some(old)) {
                let selection = state.delegate().resolve_selection(&selection);
                *self.reported.borrow_mut() = state.delegate().selection_value(selection);
                if state.selection() != selection {
                    state.set_selection(selection, cx);
                }
            }
        });
        self.focus = Some(state.focus_handle(cx));
        state
    }
}
impl TypedNativeComponent for NativeTable {
    type Props = DataTableProps;
    type Event = BindingEvent;
    fn render(
        &mut self,
        _: &DataTableProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let children = crate::bridge::children(&self.context);
        if !self.virtualized && !children.is_empty() {
            return Some(
                Table::new()
                    .children(children.into_iter().map(super::parts::Child))
                    .into_any_element(),
            );
        }
        let state = self.sync(window, cx);
        if self.virtualized {
            Some(
                DataTable::new(&state)
                    .stripe(self.stripe)
                    .bordered(self.bordered)
                    .into_any_element(),
            )
        } else {
            let delegate = state.read(cx).delegate();
            Some(
                Table::new()
                    .child(TableHeader::new().child(TableRow::new().children(
                        delegate.columns.iter().map(|column| {
                            TableHead::new().w(px(column.width)).child(
                                crate::bridge::slot(
                                    &self.context,
                                    &format!("header:{}", column.key),
                                )
                                .unwrap_or_else(|| column.label.clone().into_any_element()),
                            )
                        }),
                    )))
                    .child(
                        TableBody::new().children(delegate.rows.iter().enumerate().map(
                            |(row_ix, _)| {
                                TableRow::new().children(
                                    (0..delegate.columns.len()).map(|col| {
                                        TableCell::new().child(delegate.cell(row_ix, col))
                                    }),
                                )
                            },
                        )),
                    )
                    .when_some(self.caption.clone(), |this, caption| {
                        this.child(TableCaption::new().child(caption))
                    })
                    .into_any_element(),
            )
        }
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone().filter(|_| self.virtualized)
    }
    fn validate_props(&self, p: &DataTableProps) -> Result<(), nsh_core::ui::PropError> {
        let mut ids = HashSet::new();
        if p.columns.iter().any(|column| {
            !ids.insert(&column.key)
                || !column.width.is_finite()
                || column.width <= 0.
                || column.width > f32::MAX as f64
        }) {
            return Err(nsh_core::ui::PropError(
                "Table column keys must be unique and widths positive finite numbers".into(),
            ));
        }
        ids.clear();
        if p.rows.iter().any(|row| !ids.insert(&row.id)) {
            return Err(nsh_core::ui::PropError(
                "Table row ids must be unique".into(),
            ));
        }
        Ok(())
    }
    fn props_changed(
        &mut self,
        p: &DataTableProps,
        changed: &[String],
        _: &mut Window,
        _: &mut App,
    ) {
        if changed.iter().any(|f| f == "columns") {
            self.columns = Some(
                p.columns
                    .iter()
                    .map(|column| ColumnData {
                        key: column.key.clone(),
                        label: column.label.clone().into(),
                        width: column.width as f32,
                        sortable: column.sortable,
                        resizable: column.resizable,
                        movable: column.movable,
                        align: column.align.text().to_owned(),
                    })
                    .collect(),
            );
        }
        if changed.iter().any(|f| f == "rows") {
            self.rows = Some(
                p.rows
                    .iter()
                    .map(|row| Row {
                        id: row.id.clone(),
                        cells: row.cells.clone().into_iter().collect(),
                        text: row
                            .cells
                            .iter()
                            .map(|(key, value)| (key.clone(), cell_text(Some(value)).into()))
                            .collect(),
                    })
                    .collect(),
            );
        }
        if changed.iter().any(|f| f == "selection") {
            self.selection = Some(p.selection.clone());
        }
        self.stripe = p.stripe;
        self.bordered = p.bordered;
        self.loading = p.loading;
        self.more = p.has_more;
        self.cell_selection = p.cell_selectable;
        self.caption = p.caption.clone().map(Into::into);
        self.local_sort = matches!(p.sort_mode, super::schema::SortMode::Local);
    }
    fn command(
        &mut self,
        _: &DataTableProps,
        name: &str,
        payload: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = self.sync(window, cx);
        match name {
            "focus" => window.focus(&state.focus_handle(cx), cx),
            "scrollToRow" => {
                if let Some(id) = payload.as_str() {
                    state.update(cx, |state, cx| {
                        if let Some(ix) = state.delegate().rows.iter().position(|row| row.id == id)
                        {
                            state.scroll_to_row(ix, cx);
                        }
                    });
                }
            }
            "scrollToColumn" => {
                if let Some(key) = payload.as_str() {
                    state.update(cx, |state, cx| {
                        if let Some(ix) = state
                            .delegate()
                            .columns
                            .iter()
                            .position(|column| column.key == key)
                        {
                            state.scroll_to_col(ix, cx);
                        }
                    });
                }
            }
            "export" => {
                let (headers, rows) = state.read(cx).dump(cx);
                self.context.emit(BindingEvent::Export {
                    headers: headers.into_iter().map(|value| value.to_string()).collect(),
                    rows: rows
                        .into_iter()
                        .map(|row| row.into_iter().map(|value| value.to_string()).collect())
                        .collect(),
                });
            }
            _ => {}
        }
    }
}

pub(super) struct NativeBasicTable(NativeTable);
impl NativeBasicTable {
    pub(super) fn new(context: TypedComponentContext<super::schema::TableEvent>) -> Self {
        Self(NativeTable::new(context.raw().clone().into(), false))
    }
}
impl TypedNativeComponent for NativeBasicTable {
    type Props = super::schema::TableProps;
    type Event = super::schema::TableEvent;
    fn render(&mut self, _: &Self::Props, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        self.0.render(
            &<DataTableProps as nsh_core::ui::Props>::initial(),
            window,
            cx,
        )
    }
    fn validate_props(&self, p: &Self::Props) -> Result<(), nsh_core::ui::PropError> {
        let props = DataTableProps {
            children: p.children,
            columns: p.columns.clone(),
            rows: p.rows.clone(),
            caption: p.caption.clone(),
            ..<DataTableProps as nsh_core::ui::Props>::initial()
        };
        self.0.validate_props(&props)
    }
    fn props_changed(
        &mut self,
        p: &Self::Props,
        changed: &[String],
        window: &mut Window,
        cx: &mut App,
    ) {
        let props = DataTableProps {
            children: p.children,
            columns: p.columns.clone(),
            rows: p.rows.clone(),
            caption: p.caption.clone(),
            ..<DataTableProps as nsh_core::ui::Props>::initial()
        };
        self.0.props_changed(&props, changed, window, cx);
    }
}
