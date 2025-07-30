use std::rc::Rc;

use gpui::App;

use crate::IndexPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowEntry {
    Entry(IndexPath),
    SectionHeader(usize),
    SectionFooter(usize),
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
    pub(crate) sections: Rc<Vec<usize>>,
}

impl RowsCache {
    pub(crate) fn get(&self, flatten_ix: usize) -> Option<RowEntry> {
        self.entities.get(flatten_ix).cloned()
    }

    pub(crate) fn get_index_path(&self, flatten_ix: usize) -> Option<IndexPath> {
        self.entities
            .get(flatten_ix)
            .filter(|entry| entry.is_entry())
            .map(|entry| entry.index())
    }

    /// Returns the number of flattened rows.
    pub(crate) fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns the index of the given path in the flattened rows.
    pub(crate) fn position_of(&self, path: &IndexPath) -> Option<usize> {
        self.entities.iter().position(|p| p.eq_index_path(path))
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
        self.entities = Rc::new(
            self.sections
                .iter()
                .enumerate()
                .flat_map(|(section, items_count)| {
                    let mut items = vec![];
                    items.push(RowEntry::SectionHeader(section));
                    for row in 0..*items_count {
                        items.push(RowEntry::Entry(IndexPath {
                            section,
                            row,
                            ..Default::default()
                        }));
                    }
                    items.push(RowEntry::SectionFooter(section));
                    items
                })
                .collect(),
        );
    }
}
