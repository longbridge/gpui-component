//! Retained native DataTable binding with immutable row snapshots and lazy cells.

use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDataValue,
    ComponentDelegateSnapshot, ComponentDescriptor, ComponentElementCallback,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, StateDescriptor, TypeScriptDescriptor,
    anyhow,
    gpui::{
        self, AppContext as _, Entity, IntoElement as _, ParentElement as _, Refineable as _,
        RenderOnce, StyleRefinement, Styled as _,
    },
    gpui_component::table::{Column, DataTable, TableDelegate, TableState},
};
use std::sync::Arc;

struct Delegate {
    columns: Vec<Column>,
    rows: ComponentDelegateSnapshot,
    render_cell: Option<ComponentElementCallback>,
}

impl Delegate {
    fn new(keys: Vec<String>) -> Self {
        Self {
            columns: keys
                .iter()
                .map(|key| Column::new(key.clone(), key.clone()))
                .collect(),
            rows: ComponentDelegateSnapshot::new(Vec::new()),
            render_cell: None,
        }
    }
}

impl TableDelegate for Delegate {
    fn columns_count(&self, _: &gpui::App) -> usize {
        self.columns.len()
    }
    fn rows_count(&self, _: &gpui::App) -> usize {
        self.rows.len()
    }
    fn column(&self, col_ix: usize, _: &gpui::App) -> Column {
        self.columns[col_ix].clone()
    }
    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<TableState<Self>>,
    ) -> impl gpui::IntoElement {
        let result = (|| {
            let callback = self
                .render_cell
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("DataTable cell renderer is unavailable"))?;
            let row = self.rows.row(row_ix)?.clone();
            let key = self
                .columns
                .get(col_ix)
                .ok_or_else(|| anyhow::anyhow!("DataTable column index {col_ix} is out of bounds"))?
                .key
                .to_string();
            callback.build_data_with(&[row, ComponentDataValue::String(key)], window, cx)
        })();
        match result {
            Ok(Some(element)) => {
                #[cfg(test)]
                test_probe::built();
                element
            }
            Ok(None) => gpui::div().into_any_element(),
            Err(error) => {
                #[cfg(test)]
                test_probe::error(error.to_string());
                gpui::div()
                    .child(format!("Failed to render DataTable cell: {error:#}"))
                    .into_any_element()
            }
        }
    }
    fn cell_text(&self, row_ix: usize, col_ix: usize, _: &gpui::App) -> String {
        let Some(column) = self.columns.get(col_ix) else {
            return String::new();
        };
        let Ok(ComponentDataValue::Object(fields)) = self.rows.row(row_ix) else {
            return String::new();
        };
        fields
            .iter()
            .find_map(|(key, value)| {
                (key == column.key.as_ref()).then(|| match value {
                    ComponentDataValue::String(v) => v.clone(),
                    ComponentDataValue::Number(v) => v.to_string(),
                    ComponentDataValue::Boolean(v) => v.to_string(),
                    _ => String::new(),
                })
            })
            .unwrap_or_default()
    }
}

#[derive(Clone)]
struct Payload {
    state: ComponentArgument,
    rows: ComponentArgument,
    cell: ComponentArgument,
}
#[derive(Clone, Copy)]
enum Op {
    Stripe(bool),
    Bordered(bool),
    Scrollbars(bool, bool),
    RowSelectable(bool),
    ColSelectable(bool),
    CellSelectable(bool),
    RowHeader(bool),
    Sortable(bool),
    ColResizable(bool),
    ColMovable(bool),
}

struct Materializer;
impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<Payload>()
            .ok_or_else(|| anyhow::anyhow!("DataTable received an incompatible payload"))?
            .clone();
        anyhow::ensure!(
            request.children_len() == 0,
            "DataTable does not accept children"
        );
        let state =
            request.with_state::<Entity<TableState<Delegate>>, _>(&payload.state, Clone::clone)?;
        let rows = request.resolve_data_callback(&payload.rows)?;
        let cell = request.resolve_element_callback(&payload.cell)?;
        let ops = request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<Op>().copied())
            .collect::<Vec<_>>();
        let style = request.take_style();
        Ok(DataTableHost {
            state,
            rows,
            cell,
            ops,
            style,
        }
        .into_any_element())
    }
}

#[derive(gpui::IntoElement)]
struct DataTableHost {
    state: Entity<TableState<Delegate>>,
    rows: gpui_shell::ComponentDataCallback,
    cell: ComponentElementCallback,
    ops: Vec<Op>,
    style: StyleRefinement,
}

impl RenderOnce for DataTableHost {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let snapshot = match self.rows.snapshot_rows_with(&[], window, cx) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                let message =
                    format!("DataTable rows callback must return an array of rows: {error:#}");
                #[cfg(test)]
                test_probe::error(message.clone());
                return gpui::div().child(message).into_any_element();
            }
        };
        self.state.update(cx, |state, cx| {
            state.delegate_mut().rows = snapshot;
            state.delegate_mut().render_cell = Some(self.cell);
            for op in &self.ops {
                match op {
                    Op::RowSelectable(v) => state.row_selectable = *v,
                    Op::ColSelectable(v) => state.col_selectable = *v,
                    Op::CellSelectable(v) => state.cell_selectable = *v,
                    Op::RowHeader(v) => state.row_header = *v,
                    Op::Sortable(v) => state.sortable = *v,
                    Op::ColResizable(v) => state.col_resizable = *v,
                    Op::ColMovable(v) => state.col_movable = *v,
                    _ => {}
                }
            }
            state.refresh(cx);
        });
        let mut table = DataTable::new(&self.state);
        for op in &self.ops {
            table = match op {
                Op::Stripe(v) => table.stripe(*v),
                Op::Bordered(v) => table.bordered(*v),
                Op::Scrollbars(v, h) => table.scrollbar_visible(*v, *h),
                _ => table,
            };
        }
        let mut host = gpui::div().size_full().child(table);
        host.style().refine(&self.style);
        host.into_any_element()
    }
}

fn bool_method(name: &'static str, make: fn(bool) -> Op) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Boolean)],
        move |args| match args {
            [ComponentArgument::Boolean(value)] => Ok(ComponentPayload::new(make(*value))),
            _ => Err(format!("DataTable.{name} expects boolean")),
        },
    )
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register_state(
        StateDescriptor::new(
            "DataTableState",
            "DataTableState",
            vec![ArgumentDescriptor::new(
                "columns",
                ArgumentSchema::Array(Box::new(ArgumentSchema::String)),
            )],
            |args, window, cx| match args {
                [ComponentArgument::Array(columns)] => {
                    let keys = columns
                        .iter()
                        .map(|column| match column {
                            ComponentArgument::String(key) if !key.trim().is_empty() => {
                                Ok(key.clone())
                            }
                            _ => Err("DataTableState columns must be non-empty strings".into()),
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    if keys.is_empty() {
                        return Err("DataTableState requires at least one column".into());
                    }
                    let mut unique = std::collections::HashSet::new();
                    if !keys.iter().all(|key| unique.insert(key.clone())) {
                        return Err("DataTableState column keys must be unique".into());
                    }
                    Ok(Box::new(cx.new(|cx| {
                        TableState::new(Delegate::new(keys), window, cx)
                    })))
                }
                _ => Err("DataTableState expects a string array".into()),
            },
        )
        .documented(
            "Retained native DataTable focus, selection, scrolling, measurement and column state.",
        ),
    )?;
    registry.register(ComponentDescriptor {
        name: "DataTable",
        constructors: vec![ConstructorDescriptor::new("DataTable", vec![
            ArgumentDescriptor::new("state", ArgumentSchema::Entity("DataTableState")),
            ArgumentDescriptor::new("rows", ArgumentSchema::Callback("(cx: Context) => readonly unknown[]")),
            ArgumentDescriptor::new("renderCell", ArgumentSchema::Callback("(row: unknown, column: string, cx: Context) => Element")),
        ], |args| match args { [state @ ComponentArgument::Entity { .. }, rows @ ComponentArgument::Callback(_), cell @ ComponentArgument::Callback(_)] => Ok(ComponentPayload::new(Payload { state: state.clone(), rows: rows.clone(), cell: cell.clone() })), _ => Err("DataTable expects DataTableState, rows callback and cell renderer".into()) })],
        methods: vec![
            bool_method("stripe", Op::Stripe), bool_method("bordered", Op::Bordered),
            MethodDescriptor::new("scrollbarVisible", vec![ArgumentDescriptor::new("vertical", ArgumentSchema::Boolean), ArgumentDescriptor::new("horizontal", ArgumentSchema::Boolean)], |args| match args { [ComponentArgument::Boolean(v), ComponentArgument::Boolean(h)] => Ok(ComponentPayload::new(Op::Scrollbars(*v, *h))), _ => Err("DataTable.scrollbarVisible expects two booleans".into()) }),
            bool_method("rowSelectable", Op::RowSelectable), bool_method("columnSelectable", Op::ColSelectable), bool_method("cellSelectable", Op::CellSelectable), bool_method("rowHeader", Op::RowHeader), bool_method("sortable", Op::Sortable), bool_method("columnResizable", Op::ColResizable), bool_method("columnMovable", Op::ColMovable),
        ],
        typescript: TypeScriptDescriptor::new("A real retained native DataTable. Rows are captured as an immutable plain-data snapshot and visible cells are built lazily from (row, column). Style applies to the full-size table host."),
        materializer: Arc::new(Materializer),
    })?;
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_probe {
    use std::cell::Cell;
    thread_local! { static BUILDS: Cell<usize> = const { Cell::new(0) }; static ERRORS: Cell<usize> = const { Cell::new(0) }; }
    pub(super) fn built() {
        BUILDS.with(|v| v.set(v.get() + 1));
    }
    pub(super) fn error(_: String) {
        ERRORS.with(|v| v.set(v.get() + 1));
    }
    pub(crate) fn reset() {
        BUILDS.with(|v| v.set(0));
        ERRORS.with(|v| v.set(0));
    }
    pub(crate) fn cell_builds() -> usize {
        BUILDS.with(Cell::get)
    }
    pub(crate) fn errors() -> usize {
        ERRORS.with(Cell::get)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_is_retained_data_table_only() {
        let mut registry =
            ComponentRegistry::new(gpui_shell::COMPONENT_REGISTRY_API_VERSION).unwrap();
        register(&mut registry).unwrap();
        assert_eq!(
            registry
                .freeze()
                .unwrap()
                .descriptors()
                .map(|d| d.name)
                .collect::<Vec<_>>(),
            ["DataTable"]
        );
    }
}
