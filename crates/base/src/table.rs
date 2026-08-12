use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement, Interactivity, IntoElement, ParentElement,
    RenderOnce, Role, Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
};

use crate::StyledExt as _;

macro_rules! table_part {
    ($name:ident, $role:expr, $docs:literal) => {
        #[doc = $docs]
        #[derive(IntoElement)]
        pub struct $name {
            base: Stateful<Div>,
            style: StyleRefinement,
            children: Vec<AnyElement>,
        }

        impl $name {
            #[doc = concat!("Create ", $docs)]
            pub fn new(id: impl Into<ElementId>) -> Self {
                Self {
                    base: div().id(id),
                    style: StyleRefinement::default(),
                    children: Vec::new(),
                }
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
                self.children.extend(children);
            }
        }

        impl InteractiveElement for $name {
            fn interactivity(&mut self) -> &mut Interactivity {
                self.base.interactivity()
            }
        }

        impl StatefulInteractiveElement for $name {}

        impl RenderOnce for $name {
            fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
                self.base
                    .role($role)
                    .children(self.children)
                    .refine_style(&self.style)
            }
        }
    };
}

table_part!(Table, Role::Table, "An unstyled semantic table root.");
table_part!(
    TableHeader,
    Role::RowGroup,
    "An unstyled table header group."
);
table_part!(TableBody, Role::RowGroup, "An unstyled table body group.");

/// An unstyled semantic table row.
#[derive(IntoElement)]
pub struct TableRow {
    base: Stateful<Div>,
    style: StyleRefinement,
    row_index: usize,
    children: Vec<AnyElement>,
}

impl TableRow {
    /// Create an unstyled semantic table row with a one-based accessibility index.
    pub fn new(id: impl Into<ElementId>, row_index: usize) -> Self {
        Self {
            base: div().id(id),
            style: StyleRefinement::default(),
            row_index,
            children: Vec::new(),
        }
    }
}

impl Styled for TableRow {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for TableRow {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}

impl InteractiveElement for TableRow {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for TableRow {}

impl RenderOnce for TableRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .role(Role::Row)
            .aria_row_index(self.row_index)
            .children(self.children)
            .refine_style(&self.style)
    }
}

macro_rules! table_cell {
    ($name:ident, $role:expr, $docs:literal) => {
        #[doc = $docs]
        #[derive(IntoElement)]
        pub struct $name {
            base: Stateful<Div>,
            style: StyleRefinement,
            column_index: usize,
            children: Vec<AnyElement>,
        }

        impl $name {
            #[doc = concat!("Create ", $docs, " with a one-based accessibility index.")]
            pub fn new(id: impl Into<ElementId>, column_index: usize) -> Self {
                Self {
                    base: div().id(id),
                    style: StyleRefinement::default(),
                    column_index,
                    children: Vec::new(),
                }
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
                self.children.extend(children);
            }
        }

        impl InteractiveElement for $name {
            fn interactivity(&mut self) -> &mut Interactivity {
                self.base.interactivity()
            }
        }

        impl StatefulInteractiveElement for $name {}

        impl RenderOnce for $name {
            fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
                self.base
                    .role($role)
                    .aria_column_index(self.column_index)
                    .children(self.children)
                    .refine_style(&self.style)
            }
        }
    };
}

table_cell!(
    TableHead,
    Role::ColumnHeader,
    "An unstyled table column header."
);
table_cell!(TableCell, Role::Cell, "An unstyled table data cell.");

/// An unstyled table caption slot.
#[derive(IntoElement)]
pub struct TableCaption {
    base: Stateful<Div>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl TableCaption {
    /// Create an unstyled table caption slot.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id),
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Styled for TableCaption {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for TableCaption {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}

impl InteractiveElement for TableCaption {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for TableCaption {}

impl RenderOnce for TableCaption {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base.children(self.children).refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Element as _, accesskit};

    #[gpui::test]
    fn row_and_cell_project_table_accessibility_indices(cx: &mut gpui::TestAppContext) {
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            let mut row = accesskit::Node::new(Role::Row);
            TableRow::new("row", 3)
                .render(window, cx)
                .into_element()
                .write_a11y_info(&mut row);
            assert_eq!(row.row_index(), Some(3));

            let mut cell = accesskit::Node::new(Role::Cell);
            TableCell::new("cell", 4)
                .render(window, cx)
                .into_element()
                .write_a11y_info(&mut cell);
            assert_eq!(cell.column_index(), Some(4));
        });
    }
}
