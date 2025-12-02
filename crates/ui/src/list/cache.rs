use std::rc::Rc;

use gpui::{App, Pixels, Size};

use crate::IndexPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowEntry {
    Entry(IndexPath),
    SectionHeader(usize),
    SectionFooter(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct MeasuredEntrySize {
    pub(crate) item_size: Size<Pixels>,
    pub(crate) section_header_size: Size<Pixels>,
    pub(crate) section_footer_size: Size<Pixels>,
}

impl RowEntry {
    #[inline]
    #[allow(unused)]
    pub(crate) fn is_section_header(&self) -> bool {
        matches!(self, RowEntry::SectionHeader(_))
    }

    pub(crate) fn eq_index_path(&self, path: &IndexPath) -> bool {
        match self {
            RowEntry::Entry(index_path) => index_path == path,
            RowEntry::SectionHeader(_) | RowEntry::SectionFooter(_) => false,
        }
    }

    #[allow(unused)]
    pub(crate) fn index(&self) -> IndexPath {
        match self {
            RowEntry::Entry(index_path) => *index_path,
            RowEntry::SectionHeader(ix) => IndexPath::default().section(*ix),
            RowEntry::SectionFooter(ix) => IndexPath::default().section(*ix),
        }
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn is_section_footer(&self) -> bool {
        matches!(self, RowEntry::SectionFooter(_))
    }

    #[inline]
    pub(crate) fn is_entry(&self) -> bool {
        matches!(self, RowEntry::Entry(_))
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn section_ix(&self) -> Option<usize> {
        match self {
            RowEntry::SectionHeader(ix) | RowEntry::SectionFooter(ix) => Some(*ix),
            _ => None,
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct RowsCache {
    pub(crate) entities: Rc<Vec<RowEntry>>,
    pub(crate) items_count: usize,
    /// The sections, the item is number of rows in each section.
    pub(crate) sections: Rc<Vec<usize>>,
    pub(crate) entries_sizes: Rc<Vec<Size<Pixels>>>,
    measured_size: MeasuredEntrySize,
}

impl RowsCache {
    pub(crate) fn get(&self, flatten_ix: usize) -> Option<RowEntry> {
        self.entities.get(flatten_ix).cloned()
    }

    /// Returns the number of flattened rows (Includes header, item, footer).
    pub(crate) fn len(&self) -> usize {
        self.entities.len()
    }

    /// Return the number of items in the cache.
    pub(crate) fn items_count(&self) -> usize {
        self.items_count
    }

    /// Returns the index of the  Entry with given path in the flattened rows.
    pub(crate) fn position_of(&self, path: &IndexPath) -> Option<usize> {
        self.entities
            .iter()
            .position(|p| p.is_entry() && p.eq_index_path(path))
    }

    /// Returns the sections count in the cache.
    pub(crate) fn sections_count(&self) -> usize {
        self.sections.len()
    }

    /// Returns the rows count in the given section, if the section does not exist, returns 0.
    pub(crate) fn rows_count(&self, section: usize) -> usize {
        self.sections.get(section).cloned().unwrap_or(0)
    }

    /// Return prev row, if the row is the first in the first section, goes to the last row.
    pub(crate) fn prev(&self, path: Option<IndexPath>) -> IndexPath {
        let mut path = path.unwrap_or_default();

        if path.section == 0 && path.row == 0 {
            path.section = self.sections_count().saturating_sub(1);
            path.row = self.rows_count(path.section).saturating_sub(1);
            return path;
        }

        if path.row > 0 {
            path.row -= 1;
        } else if path.section > 0 {
            path.section -= 1;
            path.row = self.rows_count(path.section).saturating_sub(1);
        }
        path
    }

    /// Returns the next row, if the row is the last in the last section, goes to the first row.
    pub(crate) fn next(&self, path: Option<IndexPath>) -> IndexPath {
        let Some(mut path) = path else {
            return IndexPath::default();
        };

        if path.section + 1 == self.sections_count()
            && path.row + 1 == self.rows_count(path.section)
        {
            path.section = 0;
            path.row = 0;
            return path;
        }

        if path.row + 1 < self.rows_count(path.section) {
            path.row += 1;
        } else if path.section + 1 < self.sections_count() {
            path.section += 1;
            path.row = 0;
        }

        path
    }

    pub(crate) fn prepare_if_needed<F>(
        &mut self,
        sections_count: usize,
        measured_size: MeasuredEntrySize,
        cx: &App,
        rows_count_f: F,
    ) where
        F: Fn(usize, &App) -> usize,
    {
        let mut new_sections = vec![];
        for section_ix in 0..sections_count {
            new_sections.push(rows_count_f(section_ix, cx));
        }

        let need_update = new_sections != *self.sections || self.measured_size != measured_size;

        if !need_update {
            return;
        }

        let mut entries_sizes = vec![];
        let mut total_items_count = 0;
        self.measured_size = measured_size;
        self.sections = Rc::new(new_sections);
        self.entities = Rc::new(
            self.sections
                .iter()
                .enumerate()
                .flat_map(|(section, items_count)| {
                    total_items_count += items_count;
                    let mut children = vec![];
                    children.push(RowEntry::SectionHeader(section));
                    entries_sizes.push(measured_size.section_header_size);
                    for row in 0..*items_count {
                        children.push(RowEntry::Entry(IndexPath {
                            section,
                            row,
                            ..Default::default()
                        }));
                        entries_sizes.push(measured_size.item_size);
                    }
                    children.push(RowEntry::SectionFooter(section));
                    entries_sizes.push(measured_size.section_footer_size);
                    children
                })
                .collect(),
        );
        self.entries_sizes = Rc::new(entries_sizes);
        self.items_count = total_items_count;
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::{IndexPath, list::cache::RowsCache};

    #[test]
    fn test_prev_next() {
        let mut row_cache = RowsCache::default();
        // section 0
        //  row 0
        //  row 1
        // section 1
        //  row 0
        //  row 1
        //  row 2
        //  row 3
        // section 2
        //  row 0
        //  row 1
        //  row 2
        row_cache.sections = Rc::new(vec![2, 4, 3]);

        assert_eq!(
            row_cache.next(Some(IndexPath::new(0).section(0))),
            IndexPath::new(1).section(0)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(1).section(0))),
            IndexPath::new(0).section(1)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(0).section(1))),
            IndexPath::new(1).section(1)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(3).section(1))),
            IndexPath::new(0).section(2)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(0).section(2))),
            IndexPath::new(1).section(2)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(1).section(2))),
            IndexPath::new(2).section(2)
        );
        assert_eq!(
            row_cache.next(Some(IndexPath::new(2).section(2))),
            IndexPath::new(0).section(0)
        );

        assert_eq!(
            row_cache.prev(Some(IndexPath::new(0).section(0))),
            IndexPath::new(2).section(2)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(1).section(0))),
            IndexPath::new(0).section(0)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(0).section(1))),
            IndexPath::new(1).section(0)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(1).section(1))),
            IndexPath::new(0).section(1)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(3).section(1))),
            IndexPath::new(2).section(1)
        );
        assert_eq!(
            row_cache.prev(Some(IndexPath::new(0).section(2))),
            IndexPath::new(3).section(1)
        );
    }
}
