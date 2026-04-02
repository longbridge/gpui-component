use std::collections::{HashMap, HashSet};

use super::copy_format::{CopyFormat, CopyFormatter, TableMetadata};
use super::data_grid::DataGrid;
use db::{ColumnInfo, FieldType};
use gpui::{
    App, AppContext, ClipboardItem, Context, InteractiveElement, IntoElement, ParentElement as _,
    SharedString, StatefulInteractiveElement, Styled, Subscription, WeakEntity, Window, div,
    prelude::FluentBuilder, px,
};
use gpui_component::calendar::Date;
use gpui_component::date_picker::{DatePickerEvent, DatePickerState};
use gpui_component::datetime_picker::{DateTimePickerEvent, DateTimePickerState};
use gpui_component::input::{InputEvent, InputState, MaskPattern};
use gpui_component::menu::{PopupMenu, PopupMenuItem};
use gpui_component::time_picker::{TimePickerEvent, TimePickerState};
use gpui_component::tooltip::Tooltip;
use gpui_component::{ActiveTheme, WindowExt, h_flex};
use one_core::storage::DatabaseType;
use one_ui::edit_table::{
    CellEditor, Column, ColumnSort, EditTableDelegate, EditTableEvent, EditTableState,
    filter_panel::FilterValue,
};
use rust_i18n::t;
use uuid::Uuid;

/// Represents a single cell change with old and new values
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellChange {
    pub col_ix: usize,
    pub col_name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// Represents the status of a row
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowStatus {
    /// Original data, unchanged
    Original,
    /// Newly added row
    New,
    /// Modified row
    Modified,
    /// Marked for deletion
    Deleted,
}

/// Represents a change to a row with detailed tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowChange {
    /// A new row was added
    Added {
        /// Data for the new row
        data: Vec<Option<String>>,
    },
    /// An existing row was updated
    Updated {
        /// Original row data (for generating WHERE clause)
        original_data: Vec<Option<String>>,
        /// Changed cells only
        changes: Vec<CellChange>,
        /// Row ID from database (if available)
        rowid: Option<String>,
    },
    /// A row was marked for deletion
    Deleted {
        /// Original data (for generating WHERE clause)
        original_data: Vec<Option<String>>,
        /// Row ID from database (if available)
        rowid: Option<String>,
    },
}

pub struct EditorTableDelegate {
    pub columns: Vec<Column>,
    /// Column metadata with type information
    pub column_meta: Vec<ColumnInfo>,
    pub rows: Vec<Vec<Option<String>>>,
    /// Original data snapshot for change detection
    original_rows: Vec<Vec<Option<String>>>,
    /// Row IDs from database (parallel to rows)
    rowids: Vec<String>,
    /// Original row IDs (parallel to original_rows)
    original_rowids: Vec<String>,
    /// Track row status: key is current row index
    row_status: HashMap<usize, RowStatus>,
    /// Track modified cells (row_ix, col_ix) -> (old_value, new_value)
    cell_changes: HashMap<(usize, usize), (Option<String>, Option<String>)>,
    /// Track modified cells for UI highlighting
    pub modified_cells: HashSet<(usize, usize)>,
    /// Rows marked for deletion (original row indices)
    deleted_original_rows: HashSet<usize>,
    /// Mapping from current row index to original row index (for tracking)
    row_index_map: HashMap<usize, usize>,
    /// Next row index for new rows (negative conceptually, but we use high numbers)
    next_new_row_id: usize,
    /// New rows data: key is the new_row_id
    new_rows: HashMap<usize, Vec<Option<String>>>,
    /// Active filter columns (for UI indication)
    active_filter_columns: HashSet<usize>,
    /// Filtered row indices (None means no filter, show all rows)
    /// When set, only these row indices from `rows` will be displayed
    filtered_row_indices: Option<Vec<usize>>,
    /// Column filter conditions: col_ix -> selected values
    column_filters: HashMap<usize, HashSet<String>>,
    /// Whether cells are editable
    editable: bool,
    /// Whether the table is currently loading data
    loading: bool,
    /// Database type for handling database-specific behaviors
    database_type: DatabaseType,
    /// Table name for SQL generation
    table_name: SharedString,
    /// Primary key column indices
    primary_key_indices: Vec<usize>,
    /// Data grid handle for context menu actions
    data_grid: Option<WeakEntity<DataGrid>>,
}

fn parse_primary_order_by_clause(order_by_clause: &str) -> Option<(String, ColumnSort)> {
    let primary_clause = order_by_clause.split(',').next()?.trim();
    if primary_clause.is_empty() {
        return None;
    }

    let tokens: Vec<&str> = primary_clause.split_whitespace().collect();
    let Some(last_token) = tokens.last() else {
        return None;
    };

    let upper = last_token.to_ascii_uppercase();
    if matches!(upper.as_str(), "ASC" | "DESC") {
        let identifier = primary_clause[..primary_clause.rfind(last_token)?].trim_end();
        if identifier.is_empty() {
            return None;
        }

        return Some((
            identifier.to_string(),
            if upper == "DESC" {
                ColumnSort::Descending
            } else {
                ColumnSort::Ascending
            },
        ));
    }

    Some((primary_clause.to_string(), ColumnSort::Ascending))
}

fn normalize_sort_identifier(identifier: &str) -> String {
    let identifier = identifier.trim();
    let identifier = identifier.rsplit('.').next().unwrap_or(identifier).trim();

    let unquoted = if let Some(inner) = identifier
        .strip_prefix('`')
        .and_then(|value| value.strip_suffix('`'))
    {
        inner.replace("``", "`")
    } else if let Some(inner) = identifier
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        inner.replace("\"\"", "\"")
    } else if let Some(inner) = identifier
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        inner.replace("]]", "]")
    } else {
        identifier.to_string()
    };

    unquoted.to_ascii_lowercase()
}

impl Clone for EditorTableDelegate {
    fn clone(&self) -> Self {
        Self {
            columns: self.columns.clone(),
            column_meta: self.column_meta.clone(),
            rows: self.rows.clone(),
            original_rows: self.original_rows.clone(),
            rowids: self.rowids.clone(),
            original_rowids: self.original_rowids.clone(),
            row_status: self.row_status.clone(),
            cell_changes: self.cell_changes.clone(),
            modified_cells: self.modified_cells.clone(),
            deleted_original_rows: self.deleted_original_rows.clone(),
            row_index_map: self.row_index_map.clone(),
            next_new_row_id: self.next_new_row_id,
            new_rows: self.new_rows.clone(),
            active_filter_columns: self.active_filter_columns.clone(),
            filtered_row_indices: self.filtered_row_indices.clone(),
            column_filters: self.column_filters.clone(),
            editable: self.editable,
            loading: self.loading,
            database_type: self.database_type,
            table_name: self.table_name.clone(),
            primary_key_indices: self.primary_key_indices.clone(),
            data_grid: self.data_grid.clone(),
        }
    }
}

impl EditorTableDelegate {
    pub fn new(
        columns: Vec<Column>,
        rows: Vec<Vec<Option<String>>>,
        editable: bool,
        database_type: DatabaseType,
        _window: &mut Window,
        _cx: &mut Context<EditTableState<Self>>,
    ) -> Self {
        let row_count = rows.len();
        let row_index_map: HashMap<usize, usize> = (0..row_count).map(|i| (i, i)).collect();
        Self {
            columns,
            column_meta: Vec::new(),
            original_rows: rows.clone(),
            rows,
            rowids: Vec::new(),
            original_rowids: Vec::new(),
            row_status: HashMap::new(),
            cell_changes: HashMap::new(),
            modified_cells: HashSet::new(),
            deleted_original_rows: HashSet::new(),
            row_index_map,
            next_new_row_id: 1_000_000,
            new_rows: HashMap::new(),
            active_filter_columns: HashSet::new(),
            filtered_row_indices: None,
            column_filters: HashMap::new(),
            editable,
            loading: false,
            database_type,
            table_name: SharedString::default(),
            primary_key_indices: Vec::new(),
            data_grid: None,
        }
    }

    pub fn set_data_grid(&mut self, data_grid: WeakEntity<DataGrid>) {
        self.data_grid = Some(data_grid);
    }

    pub fn set_editable(&mut self, editable: bool) {
        self.editable = editable;
    }

    fn values_equal(a: &Option<String>, b: &Option<String>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(s1), Some(s2)) => s1 == s2,
            _ => false,
        }
    }

    /// Compare datetime values, ignoring milliseconds
    fn datetime_values_equal(a: &Option<String>, b: &Option<String>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(s1), Some(s2)) => {
                // 截取到秒级别（前19个字符: YYYY-MM-DD HH:MM:SS）
                let a_normalized = if s1.len() >= 19 {
                    &s1[..19]
                } else {
                    s1.as_str()
                };
                let b_normalized = if s2.len() >= 19 {
                    &s2[..19]
                } else {
                    s2.as_str()
                };
                a_normalized == b_normalized
            }
            _ => false,
        }
    }

    /// Set column metadata
    pub fn set_column_meta(&mut self, meta: Vec<ColumnInfo>) {
        // 自动检测主键列
        self.primary_key_indices = meta
            .iter()
            .enumerate()
            .filter(|(_, col)| col.is_primary_key)
            .map(|(i, _)| i)
            .collect();
        self.column_meta = meta;
    }

    /// Set table name for SQL generation
    pub fn set_table_name(&mut self, name: impl Into<SharedString>) {
        self.table_name = name.into();
    }

    /// Set primary key column indices manually
    pub fn set_primary_key_indices(&mut self, indices: Vec<usize>) {
        self.primary_key_indices = indices;
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    /// Get loading state
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Check if this delegate has rowids
    pub fn has_rowids(&self) -> bool {
        !self.rowids.is_empty()
    }

    /// Get column metadata
    pub fn column_meta(&self) -> &[ColumnInfo] {
        &self.column_meta
    }

    /// Get field type for a column
    pub fn get_field_type(&self, col_ix: usize) -> FieldType {
        self.column_meta
            .get(col_ix)
            .map(|m| {
                let field_type = FieldType::from_db_type(&*m.data_type);
                // Oracle DATE contains both date and time
                if field_type == FieldType::Date && self.database_type == DatabaseType::Oracle {
                    FieldType::DateTime
                } else {
                    field_type
                }
            })
            .unwrap_or(FieldType::Unknown)
    }
    /// Record a cell change (used by external editors like large text editor)
    ///
    /// This method handles tracking cell changes and updating row status.
    /// It's similar to `on_cell_edited` but can be called directly from external code.
    pub fn record_cell_change(&mut self, row_ix: usize, col_ix: usize, new_value: String) -> bool {
        // 空字符串转换为 None (NULL)
        let new_opt_value: Option<String> = if new_value.is_empty() {
            None
        } else {
            Some(new_value)
        };

        self.record_cell_change_value(row_ix, col_ix, new_opt_value)
    }

    fn record_cell_change_value(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        new_opt_value: Option<String>,
    ) -> bool {
        // Get old value from current rows
        let Some(row) = self.rows.get_mut(row_ix) else {
            return false;
        };
        let Some(cell) = row.get_mut(col_ix) else {
            return false;
        };

        // If value hasn't changed, don't record
        if Self::values_equal(cell, &new_opt_value) {
            self.modified_cells
                .retain(|&(r, c)| r != row_ix || c != col_ix);
            return false;
        }

        let old_value = cell.clone();
        *cell = new_opt_value.clone();

        // Mark cell as modified for UI
        self.modified_cells.insert((row_ix, col_ix));

        // Track the change only if not a new row
        if self.is_new_row(row_ix) {
            // For new rows, update the new_rows data
            if let Some(new_row_id) = self.find_new_row_id(row_ix) {
                if let Some(new_row_data) = self.new_rows.get_mut(&new_row_id) {
                    if let Some(cell) = new_row_data.get_mut(col_ix) {
                        *cell = new_opt_value;
                    }
                }
            }
        } else {
            // For existing rows, track the cell change
            self.cell_changes
                .entry((row_ix, col_ix))
                .and_modify(|(_, new)| *new = new_opt_value.clone())
                .or_insert((old_value, new_opt_value));

            // Update row status
            self.row_status.insert(row_ix, RowStatus::Modified);
        }

        true
    }

    pub fn clone_row(&mut self, display_row_ix: usize) -> Option<usize> {
        let actual_row_ix = self.resolve_display_row(display_row_ix)?;
        if self.is_deleted_row(actual_row_ix) {
            return None;
        }

        let row_data = self.rows.get(actual_row_ix).cloned()?;
        let new_row_ix = self.rows.len();
        self.rows.push(row_data.clone());

        let new_row_id = self.next_new_row_id;
        self.next_new_row_id += 1;
        self.new_rows.insert(new_row_id, row_data);
        self.row_status.insert(new_row_ix, RowStatus::New);
        self.row_index_map.insert(new_row_ix, new_row_id);

        Some(new_row_ix)
    }

    pub fn update_data(
        &mut self,
        columns: Vec<Column>,
        rows: Vec<Vec<Option<String>>>,
        rowids: Vec<String>,
        _cx: &mut App,
    ) {
        const MIN_WIDTH: usize = 80;
        const MAX_WIDTH: usize = 400;

        // Set column widths based on data type and content
        self.columns = columns
            .into_iter()
            .enumerate()
            .map(|(ix, mut col)| {
                let width = self.calculate_column_width(ix, &col, &rows, MIN_WIDTH, MAX_WIDTH);
                col.width = px(width as f32);
                col = col.sortable();
                col
            })
            .collect();

        let row_count = rows.len();
        self.original_rows = rows.clone();
        self.rows = rows.clone();
        self.rowids = rowids.clone();
        self.original_rowids = rowids;
        self.row_index_map = (0..row_count).map(|i| (i, i)).collect();

        // Clear all change tracking
        self.clear_changes();
    }

    pub fn apply_order_by_clause(&mut self, order_by_clause: &str) {
        for column in &mut self.columns {
            if column.sort.is_some() {
                column.sort = Some(ColumnSort::Default);
            }
        }

        let Some((column_name, sort)) = parse_primary_order_by_clause(order_by_clause) else {
            return;
        };

        let target = normalize_sort_identifier(&column_name);
        if let Some(column) = self.columns.iter_mut().find(|column| {
            normalize_sort_identifier(column.key.as_ref()) == target
                || normalize_sort_identifier(column.name.as_ref()) == target
        }) {
            column.sort = Some(sort);
        }
    }

    fn calculate_column_width(
        &self,
        col_ix: usize,
        col: &Column,
        rows: &[Vec<Option<String>>],
        min_width: usize,
        max_width: usize,
    ) -> usize {
        let header_width = col.name.len() * 10 + 50;

        let content_width = rows
            .iter()
            .take(100)
            .filter_map(|row| row.get(col_ix))
            .map(|cell| cell.as_ref().map(|s| s.len()).unwrap_or(4))
            .max()
            .unwrap_or(6);

        let type_based_width = self
            .column_meta
            .get(col_ix)
            .map(|meta| {
                let db_type = meta.data_type.to_uppercase();
                let is_oracle_date =
                    db_type == "DATE" && self.database_type == DatabaseType::Oracle;

                if is_oracle_date || db_type.contains("TIMESTAMP") || db_type.contains("DATETIME") {
                    220
                } else if db_type.contains("DATE") {
                    120
                } else if db_type.contains("TIME") {
                    100
                } else if db_type.contains("BOOL") || db_type.contains("BIT") {
                    70
                } else if db_type.contains("TEXT")
                    || db_type.contains("CLOB")
                    || db_type.contains("BLOB")
                    || db_type.contains("JSON")
                    || db_type.contains("XML")
                {
                    200
                } else if db_type.contains("UUID") || db_type.contains("GUID") {
                    300
                } else {
                    0
                }
            })
            .unwrap_or(0);

        let base_width = if type_based_width > 0 {
            type_based_width
        } else {
            content_width * 9 + 40
        };

        base_width.max(header_width).clamp(min_width, max_width)
    }

    /// Get all pending changes for saving to database
    pub fn get_changes(&self) -> Vec<RowChange> {
        let mut changes = Vec::new();

        // Collect deleted rows
        for &original_ix in &self.deleted_original_rows {
            if let Some(original_data) = self.original_rows.get(original_ix) {
                let rowid = self.original_rowids.get(original_ix).cloned();
                changes.push(RowChange::Deleted {
                    original_data: original_data.clone(),
                    rowid,
                });
            }
        }

        // Collect modified rows
        let mut modified_rows: HashMap<usize, Vec<CellChange>> = HashMap::new();
        for (&(row_ix, col_ix), (old_val, new_val)) in &self.cell_changes {
            // Skip if this row is deleted
            if let Some(&original_ix) = self.row_index_map.get(&row_ix) {
                if self.deleted_original_rows.contains(&original_ix) {
                    continue;
                }
            }

            let col_name = self
                .columns
                .get(col_ix)
                .map(|c| c.name.to_string())
                .unwrap_or_default();

            modified_rows.entry(row_ix).or_default().push(CellChange {
                col_ix,
                col_name,
                old_value: old_val.clone(),
                new_value: new_val.clone(),
            });
        }

        for (row_ix, cell_changes) in modified_rows {
            if let Some(&original_ix) = self.row_index_map.get(&row_ix) {
                if let Some(original_data) = self.original_rows.get(original_ix) {
                    let rowid = self.original_rowids.get(original_ix).cloned();
                    changes.push(RowChange::Updated {
                        original_data: original_data.clone(),
                        changes: cell_changes,
                        rowid,
                    });
                }
            }
        }

        // Collect new rows
        for (_, data) in &self.new_rows {
            changes.push(RowChange::Added { data: data.clone() });
        }

        changes
    }

    /// Clear all pending changes
    pub fn clear_changes(&mut self) {
        self.row_status.clear();
        self.cell_changes.clear();
        self.modified_cells.clear();
        self.deleted_original_rows.clear();
        self.new_rows.clear();
    }

    /// Revert all changes and restore to original state
    ///
    /// This method:
    /// 1. Restores row data to original values
    /// 2. Removes all newly added rows
    /// 3. Restores deleted rows
    /// 4. Clears all change tracking
    /// 5. Recalculates filter results
    pub fn revert_all_changes(&mut self) {
        // Restore rows to original state
        self.rows = self.original_rows.clone();

        // Restore row_index_map for restored rows
        let row_count = self.rows.len();
        self.row_index_map = (0..row_count).map(|i| (i, i)).collect();

        // Clear all change tracking
        self.clear_changes();

        // Recalculate filter results with restored data
        if !self.column_filters.is_empty() {
            self.recalculate_filtered_indices();
        }
    }

    /// Check if there are any pending changes
    pub fn has_changes(&self) -> bool {
        !self.cell_changes.is_empty()
            || !self.deleted_original_rows.is_empty()
            || !self.new_rows.is_empty()
    }

    /// Get the count of pending changes
    pub fn changes_count(&self) -> usize {
        let modified_rows: HashSet<usize> = self.cell_changes.keys().map(|(r, _)| *r).collect();
        modified_rows.len() + self.deleted_original_rows.len() + self.new_rows.len()
    }

    /// Get column names
    pub fn column_names(&self) -> Vec<String> {
        self.columns.iter().map(|c| c.name.to_string()).collect()
    }

    /// Check if a row is newly added
    pub fn is_new_row(&self, row_ix: usize) -> bool {
        self.row_status.get(&row_ix) == Some(&RowStatus::New)
    }

    /// Check if a row is marked for deletion
    pub fn is_deleted_row(&self, row_ix: usize) -> bool {
        self.row_status.get(&row_ix) == Some(&RowStatus::Deleted)
    }

    /// Set active filter columns for UI indication
    pub fn set_active_filter_columns(&mut self, columns: HashSet<usize>) {
        self.active_filter_columns = columns;
    }

    /// Set filtered row indices for display
    ///
    /// When set to Some(indices), only these rows will be displayed in the table.
    /// When set to None, all rows will be displayed.
    ///
    /// Requirements: 5.1, 5.5
    pub fn set_filtered_indices(&mut self, indices: Option<Vec<usize>>) {
        self.filtered_row_indices = indices;
    }

    /// Get the actual row index from the display row index
    ///
    /// When filtering is active, the display row index (0, 1, 2...) needs to be
    /// mapped to the actual row index in the full dataset.
    fn map_display_to_actual_row(&self, display_row_ix: usize) -> usize {
        if let Some(ref indices) = self.filtered_row_indices {
            indices
                .get(display_row_ix)
                .copied()
                .unwrap_or(display_row_ix)
        } else {
            display_row_ix
        }
    }

    /// Get the filtered row count (for display)
    pub fn filtered_row_count(&self) -> usize {
        if let Some(ref indices) = self.filtered_row_indices {
            indices.len()
        } else {
            self.rows.len()
        }
    }

    pub fn resolve_display_row(&self, display_row_ix: usize) -> Option<usize> {
        if let Some(ref indices) = self.filtered_row_indices {
            indices.get(display_row_ix).copied()
        } else if display_row_ix < self.rows.len() {
            Some(display_row_ix)
        } else {
            None
        }
    }

    // ============================================================================
    // Column Filter Methods (to be called from external code)
    // ============================================================================

    /// 应用筛选到数据（支持多列筛选）
    pub fn apply_filter(&mut self, col_ix: usize, selected_values: HashSet<String>) {
        // 存储该列的筛选条件
        self.column_filters.insert(col_ix, selected_values);
        self.active_filter_columns.insert(col_ix);

        // 重新计算所有筛选条件的组合结果
        self.recalculate_filtered_indices();
    }

    /// 清除单列筛选
    pub fn clear_column_filter(&mut self, col_ix: usize) {
        self.column_filters.remove(&col_ix);
        self.active_filter_columns.remove(&col_ix);

        // 重新计算筛选结果
        self.recalculate_filtered_indices();
    }

    /// 清除所有筛选
    pub fn clear_all_filters(&mut self) {
        self.column_filters.clear();
        self.active_filter_columns.clear();
        self.filtered_row_indices = None;
    }

    /// 重新计算筛选后的行索引（多列 AND 组合）
    fn recalculate_filtered_indices(&mut self) {
        if self.column_filters.is_empty() {
            self.filtered_row_indices = None;
            return;
        }

        let filtered_indices: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                // 所有筛选条件都必须满足（AND）
                self.column_filters
                    .iter()
                    .all(|(&col_ix, selected_values)| {
                        let cell_value = row
                            .get(col_ix)
                            .and_then(|opt| opt.as_ref())
                            .map(|s| s.as_str())
                            .unwrap_or("NULL");
                        selected_values.contains(cell_value)
                    })
            })
            .map(|(ix, _)| ix)
            .collect();

        // 如果筛选后的行数等于总行数，说明没有实际筛选效果
        if filtered_indices.len() == self.rows.len() {
            self.filtered_row_indices = None;
        } else {
            self.filtered_row_indices = Some(filtered_indices);
        }
    }
}

impl EditTableDelegate for EditorTableDelegate {
    fn cell_edit_enabled(&self, _cx: &App) -> bool {
        true
    }
    fn single_click_to_edit(&self, _cx: &App) -> bool {
        true
    }
    fn row_number_enabled(&self, _cx: &App) -> bool {
        true
    }
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        // Return filtered row count if filtering is active
        self.filtered_row_count()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) {
        let Some(column_name) = self
            .columns
            .get(col_ix)
            .map(|column| column.name.to_string())
        else {
            return;
        };
        let Some(data_grid) = self.data_grid.clone() else {
            return;
        };

        // `EditTableState::perform_sort` 会在当前表格实体的 update 闭包中调用 delegate。
        // 如果这里同步触发 `DataGrid::apply_column_sort`，后者会再次更新同一个表格实体，
        // 从而命中 GPUI 的重入更新保护并 panic。
        window.defer(cx, move |window, cx| {
            if let Err(error) = data_grid.update(cx, |grid, cx| {
                grid.apply_column_sort(&column_name, sort, window, cx);
            }) {
                tracing::error!("Failed to apply column sort: {}", error);
            }
        });
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _window: &mut Window,
        _: &mut Context<EditTableState<Self>>,
    ) -> impl IntoElement {
        let col_name = self
            .columns
            .get(col_ix)
            .map(|c| c.name.clone())
            .unwrap_or_default();

        let tooltip_text = self
            .column_meta
            .get(col_ix)
            .map(|meta| {
                let mut text = meta.data_type.to_lowercase().clone();
                if let Some(comment) = &meta.comment {
                    if !comment.is_empty() {
                        text.push('\n');
                        text.push_str(&*comment);
                    }
                }
                text
            })
            .unwrap_or_default();

        h_flex()
            .id(SharedString::from(format!("col-{}", col_ix)))
            .size_full()
            .items_center()
            .justify_between()
            .gap_1()
            .when(!tooltip_text.is_empty(), |this| {
                this.tooltip(move |window, cx| Tooltip::new(tooltip_text.clone()).build(window, cx))
            })
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(col_name),
            )
    }

    fn context_menu(
        &mut self,
        row_ix: usize,
        menu: PopupMenu,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> PopupMenu {
        let table = cx.entity().clone();
        let editable = self.editable;
        let loading = self.loading;
        let edit_disabled = !editable || loading;
        let data_grid = self.data_grid.clone();

        // 创建复制格式的辅助闭包
        fn copy_with_format(
            table: &gpui::Entity<EditTableState<EditorTableDelegate>>,
            format: CopyFormat,
            cx: &mut App,
        ) {
            table.update(cx, |state, cx| {
                let Some(data) = state.get_selection_data(cx) else {
                    return;
                };
                let columns = state.get_selection_columns(cx);
                let metadata = state.delegate().get_table_metadata();
                let text = CopyFormatter::format(format, &data, &columns, &metadata);
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            });
        }

        fn collect_selected_cells(
            state: &EditTableState<EditorTableDelegate>,
            row_number_offset: usize,
        ) -> Vec<(usize, usize)> {
            let selection = state.selection();
            let mut cells = selection.all_cells();
            if cells.is_empty() {
                if let Some(active) = selection.active.or(state.selected_cell()) {
                    cells.push(active);
                }
            }
            cells
                .into_iter()
                .filter_map(|(row, col)| col.checked_sub(row_number_offset).map(|col| (row, col)))
                .collect()
        }

        fn apply_value_to_selection<F>(
            state: &mut EditTableState<EditorTableDelegate>,
            mut value_for_cell: F,
            cx: &mut Context<EditTableState<EditorTableDelegate>>,
        ) -> bool
        where
            F: FnMut() -> Option<String>,
        {
            let row_number_offset = if state.delegate().row_number_enabled(cx) {
                1
            } else {
                0
            };
            let selected_cells = collect_selected_cells(state, row_number_offset);
            if selected_cells.is_empty() {
                return false;
            }

            let mut changed = false;
            let delegate = state.delegate_mut();
            for (row_ix, col_ix) in selected_cells {
                let Some(actual_row_ix) = delegate.resolve_display_row(row_ix) else {
                    continue;
                };
                if delegate.is_deleted_row(actual_row_ix) {
                    continue;
                }
                if actual_row_ix >= delegate.rows.len() || col_ix >= delegate.columns.len() {
                    continue;
                }
                let new_value = value_for_cell();
                changed |= delegate.record_cell_change_value(actual_row_ix, col_ix, new_value);
            }

            if changed {
                state.refresh(cx);
            }
            changed
        }

        fn paste_from_clipboard(
            state: &mut EditTableState<EditorTableDelegate>,
            window: &mut Window,
            cx: &mut Context<EditTableState<EditorTableDelegate>>,
        ) {
            let start = {
                let selection = state.selection();
                selection.active.or(state.selected_cell())
            };
            let Some(start) = start else {
                window.push_notification(t!("TableData.select_cell").to_string(), cx);
                return;
            };

            let Some(clipboard) = cx.read_from_clipboard() else {
                return;
            };
            let Some(text) = clipboard.text() else {
                return;
            };

            let data: Vec<Vec<String>> = text
                .lines()
                .map(|line| line.split('\t').map(|value| value.to_string()).collect())
                .collect();
            if data.is_empty() {
                return;
            }

            let row_number_offset = if state.delegate().row_number_enabled(cx) {
                1
            } else {
                0
            };
            let mut changes: Vec<(usize, usize, String)> = Vec::new();
            for (row_offset, row_data) in data.iter().enumerate() {
                for (col_offset, value) in row_data.iter().enumerate() {
                    let target_row = start.0 + row_offset;
                    let target_col = start.1 + col_offset;
                    let delegate_col = target_col.saturating_sub(row_number_offset);
                    changes.push((target_row, delegate_col, value.clone()));
                }
            }

            if state.delegate_mut().set_cell_values(changes, window, cx) {
                state
                    .delegate_mut()
                    .on_paste(data.clone(), start, window, cx);
                cx.emit(EditTableEvent::PasteData { data, start });
                state.refresh(cx);
            }
        }

        let table_tsv = table.clone();
        let table_csv = table.clone();
        let table_json = table.clone();
        let table_md = table.clone();
        let table_insert = table.clone();
        let table_update = table.clone();
        let table_delete = table.clone();
        let table_in = table.clone();
        let table_empty = table.clone();
        let table_null = table.clone();
        let table_uuid = table.clone();
        let table_uuid_simple = table.clone();
        let table_paste = table.clone();
        let table_copy_columns = table.clone();
        let table_clone_row = table.clone();

        let uuid_menu = PopupMenu::build(window, cx, {
            let table_uuid = table_uuid.clone();
            let table_uuid_simple = table_uuid_simple.clone();
            move |submenu, _window, _cx| {
                submenu
                    .item(
                        PopupMenuItem::new(t!("TableData.uuid_standard").to_string()).on_click({
                            let table = table_uuid.clone();
                            move |_, _window, cx| {
                                table.update(cx, |state, cx| {
                                    apply_value_to_selection(
                                        state,
                                        || Some(Uuid::new_v4().to_string()),
                                        cx,
                                    );
                                });
                            }
                        }),
                    )
                    .item(
                        PopupMenuItem::new(t!("TableData.uuid_simple").to_string()).on_click({
                            let table = table_uuid_simple.clone();
                            move |_, _window, cx| {
                                table.update(cx, |state, cx| {
                                    apply_value_to_selection(
                                        state,
                                        || Some(Uuid::new_v4().simple().to_string()),
                                        cx,
                                    );
                                });
                            }
                        }),
                    )
            }
        });

        let copy_sql_menu = PopupMenu::build(window, cx, {
            let table_insert = table_insert.clone();
            let table_update = table_update.clone();
            let table_delete = table_delete.clone();
            let table_in = table_in.clone();
            move |submenu, _window, _cx| {
                submenu
                    .item(PopupMenuItem::new("CSV").on_click({
                        let t = table_csv.clone();
                        move |_, _window, cx| {
                            copy_with_format(&t, CopyFormat::Csv, cx);
                        }
                    }))
                    .item(PopupMenuItem::new("JSON").on_click({
                        let t = table_json.clone();
                        move |_, _window, cx| {
                            copy_with_format(&t, CopyFormat::Json, cx);
                        }
                    }))
                    .item(PopupMenuItem::new("Markdown").on_click({
                        let t = table_md.clone();
                        move |_, _window, cx| {
                            copy_with_format(&t, CopyFormat::Markdown, cx);
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new("INSERT").on_click({
                        let t = table_insert.clone();
                        move |_, _window, cx| {
                            copy_with_format(&t, CopyFormat::SqlInsert, cx);
                        }
                    }))
                    .item(PopupMenuItem::new("UPDATE").on_click({
                        let t = table_update.clone();
                        move |_, _window, cx| {
                            copy_with_format(&t, CopyFormat::SqlUpdate, cx);
                        }
                    }))
                    .item(PopupMenuItem::new("DELETE").on_click({
                        let t = table_delete.clone();
                        move |_, _window, cx| {
                            copy_with_format(&t, CopyFormat::SqlDelete, cx);
                        }
                    }))
                    .item(
                        PopupMenuItem::new(t!("TableData.sql_in_clause").to_string()).on_click({
                            let t = table_in.clone();
                            move |_, _window, cx| {
                                copy_with_format(&t, CopyFormat::SqlIn, cx);
                            }
                        }),
                    )
            }
        });

        menu.item(
            PopupMenuItem::new(t!("TableData.set_empty_string").to_string())
                .disabled(edit_disabled)
                .on_click({
                    let table = table_empty.clone();
                    move |_, _window, cx| {
                        table.update(cx, |state, cx| {
                            apply_value_to_selection(state, || Some(String::new()), cx);
                        });
                    }
                }),
        )
        .item(
            PopupMenuItem::new(t!("TableData.set_null").to_string())
                .disabled(edit_disabled)
                .on_click({
                    let table = table_null.clone();
                    move |_, _window, cx| {
                        table.update(cx, |state, cx| {
                            apply_value_to_selection(state, || None, cx);
                        });
                    }
                }),
        )
        .item(
            PopupMenuItem::submenu(t!("TableData.generate_uuid").to_string(), uuid_menu)
                .disabled(edit_disabled),
        )
        .item(
            PopupMenuItem::new(t!("TableData.edit_in_cell_editor").to_string())
                .disabled(edit_disabled)
                .on_click({
                    let data_grid = data_grid.clone();
                    move |_, window, cx| {
                        let Some(data_grid) = data_grid.clone() else {
                            window.push_notification(
                                t!("TableData.open_cell_editor_failed").to_string(),
                                cx,
                            );
                            return;
                        };
                        if let Err(error) = data_grid.update(cx, |grid, cx| {
                            grid.open_large_text_editor(window, cx);
                        }) {
                            tracing::error!("Failed to open cell editor: {}", error);
                            window.push_notification(
                                t!("TableData.open_cell_editor_failed").to_string(),
                                cx,
                            );
                        }
                    }
                }),
        )
        .separator()
        .item(
            PopupMenuItem::new(t!("TableData.clone_row").to_string())
                .disabled(edit_disabled)
                .on_click({
                    let table = table_clone_row.clone();
                    move |_, window, cx| {
                        let cloned = table.update(cx, |state, cx| {
                            let new_row_ix = state.delegate_mut().clone_row(row_ix);
                            if let Some(new_row_ix) = new_row_ix {
                                state.scroll_to_row(new_row_ix, cx);
                                state.refresh(cx);
                                true
                            } else {
                                false
                            }
                        });
                        if !cloned {
                            window.push_notification(
                                t!("TableData.clone_row_failed").to_string(),
                                cx,
                            );
                        }
                    }
                }),
        )
        .item(
            PopupMenuItem::new(t!("TableData.delete_record").to_string())
                .disabled(edit_disabled)
                .on_click({
                    let table = table_delete.clone();
                    move |_, window, cx| {
                        table.update(cx, |state, cx| {
                            state.delete_row(row_ix, window, cx);
                        });
                    }
                }),
        )
        .separator()
        .item(
            PopupMenuItem::new(t!("TableData.paste").to_string())
                .disabled(edit_disabled)
                .on_click({
                    let table = table_paste.clone();
                    move |_, window, cx| {
                        table.update(cx, |state, cx| {
                            paste_from_clipboard(state, window, cx);
                        });
                    }
                }),
        )
        .separator()
        .item(
            PopupMenuItem::new(t!("TableData.copy").to_string()).on_click({
                let t = table_tsv.clone();
                move |_, _window, cx| {
                    copy_with_format(&t, CopyFormat::Tsv, cx);
                }
            }),
        )
        .item(
            PopupMenuItem::new(t!("TableData.copy_column_name").to_string()).on_click({
                let table = table_copy_columns.clone();
                move |_, _window, cx| {
                    table.update(cx, |state, cx| {
                        let mut columns = state.get_selection_columns(cx);
                        if columns.is_empty() {
                            if let Some((_, col_ix)) =
                                state.selection().active.or(state.selected_cell())
                            {
                                let row_number_offset = if state.delegate().row_number_enabled(cx) {
                                    1
                                } else {
                                    0
                                };
                                if let Some(delegate_col) = col_ix.checked_sub(row_number_offset) {
                                    columns
                                        .push(state.delegate().get_column_name(delegate_col, cx));
                                }
                            }
                        }

                        if columns.is_empty() {
                            return;
                        }

                        let text = columns
                            .iter()
                            .map(|name| name.as_ref())
                            .collect::<Vec<_>>()
                            .join("\t");
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                    });
                }
            }),
        )
        .separator()
        .item(PopupMenuItem::submenu(
            t!("TableData.copy_as").to_string(),
            copy_sql_menu,
        ))
        .separator()
        .item(
            PopupMenuItem::new(t!("TableData.save_data_as").to_string()).on_click({
                let data_grid = data_grid.clone();
                move |_, window, cx| {
                    let Some(data_grid) = data_grid.clone() else {
                        window
                            .push_notification(t!("TableData.export_data_failed").to_string(), cx);
                        return;
                    };
                    if let Err(error) = data_grid.update(cx, |grid, cx| {
                        grid.open_export_view(window, cx);
                    }) {
                        tracing::error!("Failed to open export view: {}", error);
                        window
                            .push_notification(t!("TableData.export_data_failed").to_string(), cx);
                    }
                }
            }),
        )
        .separator()
        .item(
            PopupMenuItem::new(t!("TableData.refresh").to_string()).on_click({
                let data_grid = data_grid.clone();
                move |_, window, cx| {
                    let Some(data_grid) = data_grid.clone() else {
                        window
                            .push_notification(t!("TableData.refresh_data_failed").to_string(), cx);
                        return;
                    };
                    if let Err(error) = data_grid.update(cx, |grid, cx| {
                        grid.refresh_data(cx);
                    }) {
                        tracing::error!("Failed to refresh data grid: {}", error);
                        window
                            .push_notification(t!("TableData.refresh_data_failed").to_string(), cx);
                    }
                }
            }),
        )
    }

    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> impl IntoElement {
        // Map display row index to actual row index
        let actual_row = self.map_display_to_actual_row(row);

        let value = self
            .rows
            .get(actual_row)
            .and_then(|r| r.get(col))
            .cloned()
            .unwrap_or(None);

        match value {
            None => div()
                .text_color(cx.theme().muted_foreground.opacity(0.5))
                .italic()
                .child("NULL"),
            Some(s) => div().child(s),
        }
    }

    fn loading(&self, _cx: &App) -> bool {
        self.loading
    }

    fn build_input(
        &self,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> Option<(CellEditor, Vec<Subscription>)> {
        // If not editable, return None to disable editing
        if !self.editable {
            return None;
        }

        // Map display row index to actual row index
        let actual_row = self.map_display_to_actual_row(row_ix);

        if self.is_deleted_row(actual_row) {
            return None;
        }

        let value = self
            .rows
            .get(actual_row)
            .and_then(|r| r.get(col_ix))
            .cloned()
            .unwrap_or(None);
        // NULL 值编辑时显示为空
        let edit_value = value.unwrap_or_default();

        // 根据字段类型创建不同配置的输入组件
        let field_type = self.get_field_type(col_ix);

        match field_type {
            FieldType::Date => {
                let input = cx.new(|cx| {
                    let mut state = InputState::new(window, cx);
                    state.set_value(edit_value.clone(), window, cx);
                    state.focus(window, cx);
                    state
                });

                let picker = cx.new(|cx| {
                    let mut state = DatePickerState::new(window, cx).date_format("%Y-%m-%d");
                    if let Ok(date) = chrono::NaiveDate::parse_from_str(&edit_value, "%Y-%m-%d") {
                        state.set_date(date, window, cx);
                    } else if !edit_value.is_empty() {
                        tracing::warn!("Failed to parse date value: '{}'", edit_value);
                        state.set_date(chrono::Local::now().date_naive(), window, cx);
                    }
                    state.set_open(true, cx);
                    state
                });

                let input_subscription = {
                    let input_handle = input.clone();
                    let picker_handle = picker.clone();
                    cx.subscribe_in(
                        &input,
                        window,
                        move |table, _, evt: &InputEvent, window, cx| match evt {
                            InputEvent::Change => {
                                let text = input_handle.read(cx).text().to_string();
                                let trimmed = text.trim();
                                if trimmed.is_empty() {
                                    picker_handle.update(cx, |state, cx| {
                                        state.set_date(Date::Single(None), window, cx);
                                    });
                                    return;
                                }
                                if let Ok(date) =
                                    chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
                                {
                                    picker_handle.update(cx, |state, cx| {
                                        state.set_date(date, window, cx);
                                    });
                                }
                            }
                            InputEvent::PressEnter { .. } => {
                                table.commit_cell_edit(window, cx);
                            }
                            _ => {}
                        },
                    )
                };

                let picker_subscription = {
                    let input_handle = input.clone();
                    cx.subscribe_in(
                        &picker,
                        window,
                        move |_, _, evt: &DatePickerEvent, window, cx| match evt {
                            DatePickerEvent::Change(date) => {
                                let new_value = date
                                    .format("%Y-%m-%d")
                                    .map(|value| value.to_string())
                                    .unwrap_or_default();
                                input_handle.update(cx, |state, cx| {
                                    state.set_value(new_value, window, cx);
                                    state.focus(window, cx);
                                });
                            }
                        },
                    )
                };

                Some((
                    CellEditor::DatePickerInput { input, picker },
                    vec![input_subscription, picker_subscription],
                ))
            }
            FieldType::DateTime => {
                let edit_value_trimmed = edit_value.trim().to_string();
                let (initial_value, initial_datetime) = if edit_value_trimmed.is_empty() {
                    let now = chrono::Local::now().naive_local();
                    (now.format("%Y-%m-%d %H:%M:%S").to_string(), Some(now))
                } else {
                    (edit_value.clone(), None)
                };
                let input = cx.new(|cx| {
                    let mut state = InputState::new(window, cx);
                    state.set_value(initial_value, window, cx);
                    state.focus(window, cx);
                    state
                });

                let picker = cx.new(|cx| {
                    let mut state =
                        DateTimePickerState::new(window, cx).datetime_format("%Y-%m-%d %H:%M:%S");
                    if let Some(datetime) = initial_datetime {
                        state.set_datetime(Some(datetime), window, cx);
                        state.set_open(true, window, cx);
                        return state;
                    }
                    let datetime = chrono::NaiveDateTime::parse_from_str(
                        &edit_value_trimmed,
                        "%Y-%m-%d %H:%M:%S",
                    )
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(
                            &edit_value_trimmed,
                            "%Y-%m-%dT%H:%M:%S",
                        )
                    })
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(
                            &edit_value_trimmed,
                            "%Y-%m-%d %H:%M:%S%.f",
                        )
                    })
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(
                            &edit_value_trimmed,
                            "%Y-%m-%dT%H:%M:%S%.f",
                        )
                    })
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(
                            &edit_value_trimmed,
                            "%Y/%m/%d %H:%M:%S",
                        )
                    })
                    .ok();

                    if let Some(dt) = datetime {
                        state.set_datetime(Some(dt), window, cx);
                    } else if !edit_value_trimmed.is_empty() {
                        tracing::warn!("Failed to parse datetime value: '{}'", edit_value_trimmed);
                        state.set_datetime(Some(chrono::Local::now().naive_local()), window, cx);
                    }
                    state.set_open(true, window, cx);
                    state
                });

                let input_subscription = {
                    let input_handle = input.clone();
                    let picker_handle = picker.clone();
                    cx.subscribe_in(
                        &input,
                        window,
                        move |table, _, evt: &InputEvent, window, cx| match evt {
                            InputEvent::Change => {
                                let text = input_handle.read(cx).text().to_string();
                                let trimmed = text.trim();
                                if trimmed.is_empty() {
                                    picker_handle.update(cx, |state, cx| {
                                        state.set_datetime(None, window, cx);
                                    });
                                    return;
                                }
                                let datetime = chrono::NaiveDateTime::parse_from_str(
                                    trimmed,
                                    "%Y-%m-%d %H:%M:%S",
                                )
                                .or_else(|_| {
                                    chrono::NaiveDateTime::parse_from_str(
                                        trimmed,
                                        "%Y-%m-%dT%H:%M:%S",
                                    )
                                })
                                .or_else(|_| {
                                    chrono::NaiveDateTime::parse_from_str(
                                        trimmed,
                                        "%Y-%m-%d %H:%M:%S%.f",
                                    )
                                })
                                .or_else(|_| {
                                    chrono::NaiveDateTime::parse_from_str(
                                        trimmed,
                                        "%Y-%m-%dT%H:%M:%S%.f",
                                    )
                                })
                                .or_else(|_| {
                                    chrono::NaiveDateTime::parse_from_str(
                                        trimmed,
                                        "%Y/%m/%d %H:%M:%S",
                                    )
                                })
                                .ok();
                                if let Some(dt) = datetime {
                                    picker_handle.update(cx, |state, cx| {
                                        state.set_datetime(Some(dt), window, cx);
                                    });
                                }
                            }
                            InputEvent::PressEnter { .. } => {
                                table.commit_cell_edit(window, cx);
                            }
                            _ => {}
                        },
                    )
                };

                let picker_subscription = {
                    let input_handle = input.clone();
                    cx.subscribe_in(
                        &picker,
                        window,
                        move |_, _, evt: &DateTimePickerEvent, window, cx| match evt {
                            DateTimePickerEvent::Change(datetime) => {
                                let new_value = datetime
                                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                    .unwrap_or_default();
                                input_handle.update(cx, |state, cx| {
                                    state.set_value(new_value, window, cx);
                                    state.focus(window, cx);
                                });
                            }
                        },
                    )
                };

                Some((
                    CellEditor::DateTimePickerInput { input, picker },
                    vec![input_subscription, picker_subscription],
                ))
            }
            FieldType::Time => {
                let edit_value_trimmed = edit_value.trim().to_string();
                let (initial_value, initial_time) = if edit_value_trimmed.is_empty() {
                    let now = chrono::Local::now().time();
                    (now.format("%H:%M:%S").to_string(), Some(now))
                } else {
                    (edit_value.clone(), None)
                };
                let input = cx.new(|cx| {
                    let mut state = InputState::new(window, cx);

                    state.set_value(initial_value, window, cx);
                    state.focus(window, cx);
                    state
                });

                let picker = cx.new(|cx| {
                    let mut state = TimePickerState::new(window, cx).time_format("%H:%M:%S");
                    if let Some(time) = initial_time {
                        state.set_time(Some(time), window, cx);
                        state.set_open(true, window, cx);
                        return state;
                    }
                    let time = chrono::NaiveTime::parse_from_str(&edit_value_trimmed, "%H:%M:%S")
                        .or_else(|_| {
                            chrono::NaiveTime::parse_from_str(&edit_value_trimmed, "%H:%M:%S%.f")
                        })
                        .or_else(|_| {
                            chrono::NaiveTime::parse_from_str(&edit_value_trimmed, "%H:%M")
                        })
                        .ok();

                    if let Some(t) = time {
                        state.set_time(Some(t), window, cx);
                    } else if !edit_value_trimmed.is_empty() {
                        tracing::warn!("Failed to parse time value: '{}'", edit_value_trimmed);
                        state.set_time(Some(chrono::Local::now().time()), window, cx);
                    }
                    state.set_open(true, window, cx);
                    state
                });

                let input_subscription = {
                    let input_handle = input.clone();
                    let picker_handle = picker.clone();
                    cx.subscribe_in(
                        &input,
                        window,
                        move |table, _, evt: &InputEvent, window, cx| match evt {
                            InputEvent::Change => {
                                let text = input_handle.read(cx).text().to_string();
                                let trimmed = text.trim();
                                if trimmed.is_empty() {
                                    picker_handle.update(cx, |state, cx| {
                                        state.set_time(None, window, cx);
                                    });
                                    return;
                                }
                                let time = chrono::NaiveTime::parse_from_str(trimmed, "%H:%M:%S")
                                    .or_else(|_| {
                                        chrono::NaiveTime::parse_from_str(trimmed, "%H:%M:%S%.f")
                                    })
                                    .or_else(|_| {
                                        chrono::NaiveTime::parse_from_str(trimmed, "%H:%M")
                                    })
                                    .ok();
                                if let Some(t) = time {
                                    picker_handle.update(cx, |state, cx| {
                                        state.set_time(Some(t), window, cx);
                                    });
                                }
                            }
                            InputEvent::PressEnter { .. } => {
                                table.commit_cell_edit(window, cx);
                            }
                            _ => {}
                        },
                    )
                };

                let picker_subscription = {
                    let input_handle = input.clone();
                    cx.subscribe_in(
                        &picker,
                        window,
                        move |_, _, evt: &TimePickerEvent, window, cx| match evt {
                            TimePickerEvent::Change(time) => {
                                let new_value = time
                                    .map(|t| t.format("%H:%M:%S").to_string())
                                    .unwrap_or_default();
                                input_handle.update(cx, |state, cx| {
                                    state.set_value(new_value, window, cx);
                                    state.focus(window, cx);
                                });
                            }
                        },
                    )
                };

                Some((
                    CellEditor::TimePickerInput { input, picker },
                    vec![input_subscription, picker_subscription],
                ))
            }
            _ => {
                // 使用 Input 组件
                let input = cx.new(|cx| {
                    let mut state = match field_type {
                        FieldType::Integer | FieldType::Decimal => {
                            InputState::new(window, cx).mask_pattern(MaskPattern::number(None))
                        }
                        _ => InputState::new(window, cx).multi_line(true).rows(1),
                    };
                    state.set_value(edit_value, window, cx);
                    state.focus(window, cx);
                    state
                });

                let input_subscription = cx.subscribe_in(
                    &input,
                    window,
                    move |table, _, evt: &InputEvent, window, cx| match evt {
                        InputEvent::Blur => {
                            tracing::debug!("Input blur event received, committing cell edit");
                            table.commit_cell_edit(window, cx);
                        }
                        InputEvent::PressEnter { .. } => {
                            table.commit_cell_edit(window, cx);
                        }
                        _ => {}
                    },
                );

                Some((CellEditor::Input(input), vec![input_subscription]))
            }
        }
    }

    fn on_cell_edited(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        new_value: String,
        _window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> bool {
        // Map display row index to actual row index
        let actual_row = self.map_display_to_actual_row(row_ix);
        // 空字符串转换为 None (NULL)
        let new_opt_value: Option<String> = if new_value.is_empty() {
            None
        } else {
            Some(new_value.clone())
        };

        tracing::debug!(
            "on_cell_edited: row={}, col={}, new_value='{}', new_opt_value={:?}",
            actual_row,
            col_ix,
            new_value,
            new_opt_value
        );

        // 根据字段类型选择比较方法
        let field_type = self.get_field_type(col_ix);
        let values_eq: fn(&Option<String>, &Option<String>) -> bool = match field_type {
            FieldType::DateTime => Self::datetime_values_equal,
            _ => Self::values_equal,
        };

        // Check if cell is already modified
        if self.is_cell_modified(row_ix, col_ix, cx) {
            tracing::debug!(
                "on_cell_edited: cell is already modified, actual_row={}, col_ix={}",
                actual_row,
                col_ix
            );
            tracing::debug!(
                "on_cell_edited: original_rows.len()={}, is_new_row={}",
                self.original_rows.len(),
                self.is_new_row(actual_row)
            );

            // Check if user reverted to original value (only for existing rows, not new rows)
            if !self.is_new_row(actual_row) {
                if let Some(row) = self.original_rows.get(actual_row) {
                    if let Some(original_cell) = row.get(col_ix) {
                        tracing::debug!("on_cell_edited: original_cell={:?}", original_cell);
                        if values_eq(original_cell, &new_opt_value) {
                            tracing::debug!(
                                "on_cell_edited: user reverted to original value, clearing modification"
                            );
                            // User reverted to original value - clear modification markers
                            self.modified_cells
                                .retain(|&(r, c)| r != actual_row || c != col_ix);
                            self.cell_changes.remove(&(actual_row, col_ix));

                            // Update the cell value
                            if let Some(current_row) = self.rows.get_mut(actual_row) {
                                if let Some(cell) = current_row.get_mut(col_ix) {
                                    *cell = original_cell.clone();
                                }
                            }

                            // Check if row still has any modifications
                            let row_has_changes = (0..self.columns.len())
                                .any(|c| self.modified_cells.contains(&(actual_row, c)));

                            if !row_has_changes && !self.is_new_row(actual_row) {
                                self.row_status.insert(actual_row, RowStatus::Original);
                            }

                            return true; // Still need to refresh UI
                        }
                    }
                }
            }

            // Cell is modified and new value is different from original
            // Update the cell value
            if let Some(current_row) = self.rows.get_mut(actual_row) {
                if let Some(cell) = current_row.get_mut(col_ix) {
                    tracing::debug!("on_cell_edited: current cell={:?}", cell);
                    if values_eq(cell, &new_opt_value) {
                        // No actual change
                        tracing::debug!("on_cell_edited: no actual change from current value");
                        return false;
                    }

                    let old_value = cell.clone();
                    *cell = new_opt_value.clone();
                    tracing::debug!(
                        "on_cell_edited: updated cell from {:?} to {:?}",
                        old_value,
                        new_opt_value
                    );

                    // Update cell_changes or new_rows depending on row type
                    if self.is_new_row(actual_row) {
                        // For new rows, update the new_rows data
                        if let Some(new_row_id) = self.find_new_row_id(actual_row) {
                            if let Some(new_row_data) = self.new_rows.get_mut(&new_row_id) {
                                if let Some(cell) = new_row_data.get_mut(col_ix) {
                                    tracing::debug!("on_cell_edited: updating new_rows data");
                                    *cell = new_opt_value.clone();
                                }
                            }
                        }
                    } else {
                        tracing::debug!(
                            "on_cell_edited: cell_changes.contains_key={}",
                            self.cell_changes.contains_key(&(actual_row, col_ix))
                        );
                        self.cell_changes
                            .entry((actual_row, col_ix))
                            .and_modify(|(_, new)| {
                                tracing::debug!(
                                    "on_cell_edited: updating cell_changes new value to {:?}",
                                    new_opt_value
                                );
                                *new = new_opt_value.clone()
                            })
                            .or_insert_with(|| {
                                tracing::debug!("on_cell_edited: inserting new cell_changes entry");
                                (old_value.clone(), new_opt_value.clone())
                            });
                    }
                }
            }

            return true;
        }

        tracing::debug!("on_cell_edited: cell is not modified yet (initial edit)");
        // Cell not yet modified - initial edit
        if let Some(row) = self.rows.get_mut(actual_row) {
            if let Some(cell) = row.get_mut(col_ix) {
                // Only mark as modified if value actually changed
                if values_eq(cell, &new_opt_value) {
                    return false;
                }

                let old_value = cell.clone();
                *cell = new_opt_value.clone();

                // Mark cell as modified for UI (use actual row index)
                self.modified_cells.insert((actual_row, col_ix));

                // Track the change with old and new values
                // If this is a new row, we don't need to track cell changes
                if self.is_new_row(actual_row) {
                    // Just update the new_rows data
                    if let Some(new_row_id) = self.find_new_row_id(actual_row) {
                        if let Some(new_row_data) = self.new_rows.get_mut(&new_row_id) {
                            if let Some(cell) = new_row_data.get_mut(col_ix) {
                                *cell = new_opt_value;
                            }
                        }
                    }
                } else {
                    // For existing rows, track the cell change
                    // If we already have a change for this cell, keep the original old_value
                    self.cell_changes
                        .entry((actual_row, col_ix))
                        .and_modify(|(_, new)| *new = new_opt_value.clone())
                        .or_insert((old_value, new_opt_value));

                    // Update row status
                    self.row_status.insert(actual_row, RowStatus::Modified);
                }

                return true;
            }
        }
        false
    }

    fn is_cell_modified(&self, row_ix: usize, col_ix: usize, _cx: &App) -> bool {
        // Map display row index to actual row index
        let actual_row = self.map_display_to_actual_row(row_ix);
        self.modified_cells.contains(&(actual_row, col_ix))
    }

    fn is_row_deleted(&self, row_ix: usize, _cx: &App) -> bool {
        // Map display row index to actual row index
        let actual_row = self.map_display_to_actual_row(row_ix);
        self.is_deleted_row(actual_row)
    }

    fn is_row_added(&self, row_ix: usize, _cx: &App) -> bool {
        // Map display row index to actual row index
        let actual_row = self.map_display_to_actual_row(row_ix);
        self.is_new_row(actual_row)
    }

    fn on_row_added(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<EditTableState<Self>>,
    ) -> usize {
        // Add a new empty row (None represents NULL/empty value)
        let new_row: Vec<Option<String>> = vec![None; self.columns.len()];
        let row_ix = self.rows.len();
        self.rows.push(new_row.clone());

        // Track as new row
        let new_row_id = self.next_new_row_id;
        self.next_new_row_id += 1;
        self.new_rows.insert(new_row_id, new_row);
        self.row_status.insert(row_ix, RowStatus::New);

        // Map the new row index to the new_row_id (using high number as marker)
        self.row_index_map.insert(row_ix, new_row_id);

        self.next_new_row_id
    }

    fn on_row_deleted(
        &mut self,
        row_ix: usize,
        _window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) {
        if row_ix >= self.rows.len() {
            return;
        }

        // Check if this is a new row (not yet saved to DB)
        if self.is_new_row(row_ix) {
            // For new rows, remove them immediately since they don't exist in DB
            if let Some(new_row_id) = self.find_new_row_id(row_ix) {
                self.new_rows.remove(&new_row_id);
            }
            self.rows.remove(row_ix);
            self.row_status.remove(&row_ix);
            self.row_index_map.remove(&row_ix);

            // Re-index rows after deletion
            self.reindex_after_deletion(row_ix);
        } else {
            // For existing rows (from DB), only mark as deleted but keep the row visible
            // This allows users to see deleted rows with special styling and undo the deletion
            if let Some(&original_ix) = self.row_index_map.get(&row_ix) {
                self.deleted_original_rows.insert(original_ix);
            }
            self.row_status.insert(row_ix, RowStatus::Deleted);

            // It will be visually marked as deleted (e.g., strikethrough)
            // and will only be truly removed when changes are submitted
        }

        // Clean up cell changes for deleted row
        self.cell_changes.retain(|&(r, _), _| r != row_ix);
        self.modified_cells.retain(|&(r, _)| r != row_ix);

        cx.notify();
    }

    fn column_filter_enabled(&self, _cx: &App) -> bool {
        true
    }

    fn get_column_filter_values(&self, col_ix: usize, _cx: &App) -> Vec<FilterValue> {
        use std::collections::HashMap;

        let mut value_counts: HashMap<String, usize> = HashMap::new();

        // 获取其他列的筛选条件（排除当前列）
        let other_filters: HashMap<usize, &HashSet<String>> = self
            .column_filters
            .iter()
            .filter(|(c, _)| **c != col_ix)
            .map(|(c, v)| (*c, v))
            .collect();

        for (_row_ix, row) in self.rows.iter().enumerate() {
            // 检查该行是否满足其他列的筛选条件
            let passes_other_filters = other_filters.iter().all(|(&other_col, selected_values)| {
                let cell_value = row
                    .get(other_col)
                    .and_then(|opt| opt.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("NULL");
                selected_values.contains(cell_value)
            });

            if passes_other_filters {
                let value = row
                    .get(col_ix)
                    .and_then(|opt| opt.clone())
                    .unwrap_or_else(|| "NULL".to_string());
                *value_counts.entry(value).or_insert(0) += 1;
            }
        }

        let mut result: Vec<_> = value_counts
            .into_iter()
            .map(|(value, count)| FilterValue::new(value, count))
            .collect();
        result.sort_by(|a, b| a.value.cmp(&b.value));
        result
    }

    fn is_column_filtered(&self, col_ix: usize, _cx: &App) -> bool {
        self.active_filter_columns.contains(&col_ix)
    }

    fn on_column_filter_changed(
        &mut self,
        col_ix: usize,
        selected_values: HashSet<String>,
        _window: &mut Window,
        _cx: &mut Context<EditTableState<Self>>,
    ) {
        self.apply_filter(col_ix, selected_values);
    }

    // ============================================================================
    // 多选和复制粘贴实现
    // ============================================================================

    fn on_column_filter_cleared(
        &mut self,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<EditTableState<Self>>,
    ) {
        self.clear_column_filter(col_ix);
    }

    fn multi_select_enabled(&self, _cx: &App) -> bool {
        true
    }

    fn get_cell_value(&self, row_ix: usize, col_ix: usize, _cx: &App) -> String {
        // Map display row index to actual row index
        let actual_row = self.map_display_to_actual_row(row_ix);

        self.rows
            .get(actual_row)
            .and_then(|r| r.get(col_ix))
            .and_then(|opt| opt.clone())
            .unwrap_or_else(|| "NULL".to_string())
    }

    fn set_cell_values(
        &mut self,
        changes: Vec<(usize, usize, String)>,
        _window: &mut Window,
        _cx: &mut Context<EditTableState<Self>>,
    ) -> bool {
        if !self.editable {
            return false;
        }

        for (row_ix, col_ix, value) in changes {
            // Map display row index to actual row index
            let actual_row = self.map_display_to_actual_row(row_ix);

            // Skip if row is deleted
            if self.is_deleted_row(actual_row) {
                continue;
            }

            // Skip if out of bounds
            if actual_row >= self.rows.len() || col_ix >= self.columns.len() {
                continue;
            }

            // 使用现有的 record_cell_change 方法
            self.record_cell_change(actual_row, col_ix, value);
        }

        true
    }

    fn on_copy(
        &mut self,
        _data: Vec<Vec<String>>,
        _window: &mut Window,
        _cx: &mut Context<EditTableState<Self>>,
    ) {
        // 可以在这里添加复制成功的通知等
        tracing::debug!("Copied {} rows of data", _data.len());
    }

    fn on_paste(
        &mut self,
        data: Vec<Vec<String>>,
        start: (usize, usize),
        _window: &mut Window,
        _cx: &mut Context<EditTableState<Self>>,
    ) {
        tracing::debug!(
            "Pasted {} rows starting at ({}, {})",
            data.len(),
            start.0,
            start.1
        );
    }
}

impl EditorTableDelegate {
    /// 获取表格元数据（用于生成 SQL 语句）
    pub fn get_table_metadata(&self) -> TableMetadata {
        TableMetadata {
            table_name: self.table_name.clone(),
            column_names: self.columns.iter().map(|c| c.name.clone()).collect(),
            primary_key_indices: self.primary_key_indices.clone(),
        }
    }

    /// Find the new_row_id for a given row index
    fn find_new_row_id(&self, row_ix: usize) -> Option<usize> {
        self.row_index_map
            .get(&row_ix)
            .copied()
            .filter(|&id| id >= 1_000_000)
    }

    /// Re-index rows after a deletion
    fn reindex_after_deletion(&mut self, deleted_ix: usize) {
        // Update row_index_map: shift all indices after deleted_ix
        let mut new_map = HashMap::new();
        for (&row_ix, &original_ix) in &self.row_index_map {
            if row_ix > deleted_ix {
                new_map.insert(row_ix - 1, original_ix);
            } else if row_ix < deleted_ix {
                new_map.insert(row_ix, original_ix);
            }
            // Skip the deleted row
        }
        self.row_index_map = new_map;

        // Update row_status
        let mut new_status = HashMap::new();
        for (&row_ix, &status) in &self.row_status {
            if row_ix > deleted_ix {
                new_status.insert(row_ix - 1, status);
            } else if row_ix < deleted_ix {
                new_status.insert(row_ix, status);
            }
        }
        self.row_status = new_status;

        // Update cell_changes
        let mut new_changes = HashMap::new();
        for (&(row_ix, col_ix), change) in &self.cell_changes {
            if row_ix > deleted_ix {
                new_changes.insert((row_ix - 1, col_ix), change.clone());
            } else if row_ix < deleted_ix {
                new_changes.insert((row_ix, col_ix), change.clone());
            }
        }
        self.cell_changes = new_changes;

        // Update modified_cells
        let mut new_modified = HashSet::new();
        for &(row_ix, col_ix) in &self.modified_cells {
            if row_ix > deleted_ix {
                new_modified.insert((row_ix - 1, col_ix));
            } else if row_ix < deleted_ix {
                new_modified.insert((row_ix, col_ix));
            }
        }
        self.modified_cells = new_modified;
    }

    // ============================================================================
    // SQL 语句生成方法
    // ============================================================================

    /// 获取选中行的数据
    ///
    /// # Arguments
    /// * `row_indices` - 行索引列表
    ///
    /// # Returns
    /// 选中行的数据
    pub fn get_rows_data(&self, row_indices: &[usize]) -> Vec<Vec<Option<String>>> {
        row_indices
            .iter()
            .filter_map(|&row_ix| self.rows.get(row_ix).cloned())
            .collect()
    }

    /// 获取原始行数据（用于生成 WHERE 子句）
    pub fn get_original_rows_data(&self, row_indices: &[usize]) -> Vec<Vec<Option<String>>> {
        row_indices
            .iter()
            .filter_map(|&row_ix| {
                // 尝试获取原始数据
                self.row_index_map
                    .get(&row_ix)
                    .and_then(|&original_ix| {
                        if original_ix < 1_000_000 {
                            self.original_rows.get(original_ix).cloned()
                        } else {
                            // 新行没有原始数据，使用当前数据
                            self.rows.get(row_ix).cloned()
                        }
                    })
                    .or_else(|| self.rows.get(row_ix).cloned())
            })
            .collect()
    }

    /// 获取数据库类型
    pub fn database_type(&self) -> DatabaseType {
        self.database_type
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_sort_identifier, parse_primary_order_by_clause};
    use one_ui::edit_table::ColumnSort;

    #[test]
    fn parse_primary_order_by_clause_uses_first_segment() {
        let parsed = parse_primary_order_by_clause("`order` DESC, `id` ASC");

        assert_eq!(
            parsed,
            Some(("`order`".to_string(), ColumnSort::Descending))
        );
    }

    #[test]
    fn parse_primary_order_by_clause_defaults_to_ascending() {
        let parsed = parse_primary_order_by_clause("\"created_at\"");

        assert_eq!(
            parsed,
            Some(("\"created_at\"".to_string(), ColumnSort::Ascending))
        );
    }

    #[test]
    fn normalize_sort_identifier_handles_common_quote_styles() {
        assert_eq!(normalize_sort_identifier("`order`"), "order");
        assert_eq!(normalize_sort_identifier("\"created_at\""), "created_at");
        assert_eq!(normalize_sort_identifier("[Order Detail]"), "order detail");
        assert_eq!(normalize_sort_identifier("t.\"user_id\""), "user_id");
    }
}
