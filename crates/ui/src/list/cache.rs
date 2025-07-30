use std::rc::Rc;

use gpui::App;

use crate::IndexPath;

#[derive(Default, Clone)]
pub(crate) struct RowsCache {
    pub(crate) flatten_rows: Rc<Vec<IndexPath>>,
    pub(crate) sections: Rc<Vec<usize>>,
}

impl RowsCache {
    pub(crate) fn get(&self, flatten_ix: usize) -> Option<IndexPath> {
        self.flatten_rows.get(flatten_ix).cloned()
    }

    /// Returns the number of flattened rows.
    pub(crate) fn len(&self) -> usize {
        self.flatten_rows.len()
    }

    /// Returns the index of the given path in the flattened rows.
    pub(crate) fn position_of(&self, path: &IndexPath) -> Option<usize> {
        self.flatten_rows.iter().position(|p| p == path)
    }

    pub(crate) fn prepare_if_needed<F>(&mut self, sections_count: usize, cx: &App, rows_count_f: F)
    where
        F: Fn(usize, &App) -> usize,
    {
        let mut new_sections = vec![];
        for section_ix in 0..sections_count {
            new_sections.push(rows_count_f(section_ix, cx));
        }
        if new_sections == *self.sections {
            return;
        }

        self.sections = Rc::new(new_sections);
        self.flatten_rows = Rc::new(
            self.sections
                .iter()
                .enumerate()
                .flat_map(|(section_ix, items_count)| {
                    (0..*items_count)
                        .map(move |row_ix| IndexPath::default().section(section_ix).row(row_ix))
                        .collect::<Vec<_>>()
                })
                .collect(),
        );
    }
}
