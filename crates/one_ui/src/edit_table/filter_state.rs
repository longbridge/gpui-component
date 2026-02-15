use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct ColumnFilter {
    pub selected_values: HashSet<String>,
    pub is_active: bool,
}

impl ColumnFilter {
    pub fn new(selected_values: HashSet<String>, is_active: bool) -> Self {
        Self {
            selected_values,
            is_active,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FilterState {
    filters: HashMap<usize, ColumnFilter>,
    filtered_row_indices: Vec<usize>,
}

impl FilterState {
    pub fn new() -> Self {
        Self {
            filters: HashMap::new(),
            filtered_row_indices: Vec::new(),
        }
    }

    pub fn set_filter(&mut self, col_ix: usize, selected_values: HashSet<String>) {
        let is_active = !selected_values.is_empty();
        self.filters
            .insert(col_ix, ColumnFilter::new(selected_values, is_active));
    }

    pub fn set_filter_with_all_values(
        &mut self,
        col_ix: usize,
        selected_values: HashSet<String>,
        all_values: HashSet<String>,
    ) {
        let is_active = !selected_values.is_empty() && selected_values != all_values;
        self.filters
            .insert(col_ix, ColumnFilter::new(selected_values, is_active));
    }

    pub fn clear_filter(&mut self, col_ix: usize) {
        self.filters.remove(&col_ix);
    }

    pub fn clear_all(&mut self) {
        self.filters.clear();
        self.filtered_row_indices.clear();
    }

    pub fn is_column_filtered(&self, col_ix: usize) -> bool {
        self.filters
            .get(&col_ix)
            .map(|f| f.is_active)
            .unwrap_or(false)
    }

    pub fn get_filter(&self, col_ix: usize) -> Option<&ColumnFilter> {
        self.filters.get(&col_ix)
    }

    pub fn apply_filters(&mut self, rows: &[Vec<String>]) -> Vec<usize> {
        if self.filters.is_empty() {
            self.filtered_row_indices = (0..rows.len()).collect();
            return self.filtered_row_indices.clone();
        }

        self.filtered_row_indices = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                self.filters.iter().all(|(col_ix, filter)| {
                    if !filter.is_active {
                        return true;
                    }

                    let cell_value = row.get(*col_ix).map(|s| s.as_str()).unwrap_or("NULL");
                    filter.selected_values.contains(cell_value)
                })
            })
            .map(|(ix, _)| ix)
            .collect();

        self.filtered_row_indices.clone()
    }

    pub fn filtered_count(&self) -> usize {
        self.filtered_row_indices.len()
    }

    pub fn filtered_indices(&self) -> &[usize] {
        &self.filtered_row_indices
    }

    pub fn active_filter_columns(&self) -> HashSet<usize> {
        self.filters
            .iter()
            .filter_map(|(col_ix, filter)| {
                if filter.is_active {
                    Some(*col_ix)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for FilterState {
    fn default() -> Self {
        Self::new()
    }
}
