use crate::{
    actions::{Cancel, SelectDown, SelectUp},
    ActiveTheme, Sizable, Size,
};
use gpui::{
    actions, div, prelude::FluentBuilder, App, Edges, Entity, Focusable, InteractiveElement,
    IntoElement, KeyBinding, ParentElement, RenderOnce, Styled, Window,
};

mod column;
mod delegate;
mod loading;
mod state;

pub use column::*;
pub use delegate::*;
pub use state::*;

actions!(table, [SelectPrevColumn, SelectNextColumn]);

const CONTEXT: &'static str = "Table";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("left", SelectPrevColumn, Some(CONTEXT)),
        KeyBinding::new("right", SelectNextColumn, Some(CONTEXT)),
    ]);
}

struct TableOptions {
    scrollbar_visible: Edges<bool>,
    /// Set stripe style of the table.
    stripe: bool,
    /// Set to use border style of the table.
    bordered: bool,
    /// The cell size of the table.
    size: Size,
}

impl Default for TableOptions {
    fn default() -> Self {
        Self {
            scrollbar_visible: Edges::all(true),
            stripe: false,
            bordered: true,
            size: Size::default(),
        }
    }
}

/// A table element.
#[derive(IntoElement)]
pub struct Table<D: TableDelegate> {
    state: Entity<TableState<D>>,
    options: TableOptions,
}

impl<D> Table<D>
where
    D: TableDelegate,
{
    /// Create a new Table element with the given [`TableState`].
    pub fn new(state: &Entity<TableState<D>>) -> Self {
        Self {
            state: state.clone(),
            options: TableOptions::default(),
        }
    }

    /// Set to use stripe style of the table, default to false.
    pub fn stripe(mut self, stripe: bool) -> Self {
        self.options.stripe = stripe;
        self
    }

    /// Set to use border style of the table, default to true.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.options.bordered = bordered;
        self
    }

    /// Set scrollbar visibility.
    pub fn scrollbar_visible(mut self, vertical: bool, horizontal: bool) -> Self {
        self.options.scrollbar_visible = Edges {
            right: vertical,
            bottom: horizontal,
            ..Default::default()
        };
        self
    }
}

impl<D> Sizable for Table<D>
where
    D: TableDelegate,
{
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.options.size = size.into();
        self
    }
}

impl<D> RenderOnce for Table<D>
where
    D: TableDelegate,
{
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let bordered = self.options.bordered;
        let focus_handle = self.state.focus_handle(cx);
        self.state.update(cx, |state, _| {
            state.options = self.options;
        });

        div()
            .id("table")
            .size_full()
            .key_context(CONTEXT)
            .track_focus(&focus_handle)
            .on_action(window.listener_for(&self.state, TableState::action_cancel))
            .on_action(window.listener_for(&self.state, TableState::action_select_next))
            .on_action(window.listener_for(&self.state, TableState::action_select_prev))
            .on_action(window.listener_for(&self.state, TableState::action_select_next_col))
            .on_action(window.listener_for(&self.state, TableState::action_select_prev_col))
            .bg(cx.theme().table)
            .when(bordered, |this| {
                this.rounded(cx.theme().radius)
                    .border_1()
                    .border_color(cx.theme().border)
            })
            .child(self.state)
    }
}
