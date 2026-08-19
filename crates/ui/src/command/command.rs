use std::rc::Rc;

use gpui::{
    AnyElement, App, DefiniteLength, Entity, IntoElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, rems,
};

use crate::command::{
    item::{CommandEntry, CommandGroup, CommandItem},
    state::{CommandFilter, CommandModel, CommandState, OnCancel, OnQuery, OnValue},
};

pub(crate) type CommandSlot = dyn Fn(&CommandState, &mut Window, &mut App) -> AnyElement;

/// Presentation of a [`Command`], pushed into its state on every render.
#[derive(Clone)]
pub(crate) struct CommandOptions {
    style: StyleRefinement,
    placeholder: Option<SharedString>,
    empty_message: Option<SharedString>,
    max_h: DefiniteLength,
    bordered: bool,
    header: Option<Rc<CommandSlot>>,
    footer: Option<Rc<CommandSlot>>,
}

impl Default for CommandOptions {
    fn default() -> Self {
        Self {
            style: StyleRefinement::default(),
            placeholder: None,
            empty_message: None,
            max_h: rems(18.75).into(),
            bordered: true,
            header: None,
            footer: None,
        }
    }
}

impl CommandOptions {
    pub(crate) fn style(&self) -> &StyleRefinement {
        &self.style
    }

    pub(crate) fn placeholder(&self) -> Option<&SharedString> {
        self.placeholder.as_ref()
    }

    pub(crate) fn empty_message(&self) -> Option<&SharedString> {
        self.empty_message.as_ref()
    }

    pub(crate) fn max_h(&self) -> DefiniteLength {
        self.max_h
    }

    pub(crate) fn is_bordered(&self) -> bool {
        self.bordered
    }

    pub(crate) fn header(&self) -> Option<&Rc<CommandSlot>> {
        self.header.as_ref()
    }

    pub(crate) fn footer(&self) -> Option<&Rc<CommandSlot>> {
        self.footer.as_ref()
    }
}

/// A command palette: a search field over a filtered list of commands.
///
/// Entries and rendering policy are configured on each `Command`; interaction
/// state such as the query and highlighted item lives in [`CommandState`].
///
/// ```ignore
/// let state = cx.new(|cx| CommandState::new(window, cx));
///
/// Command::new(&state)
///     .group(
///         CommandGroup::new().label("Suggestions")
///             .item(CommandItem::new("Calendar").icon(IconName::Calendar)),
///     )
///     .placeholder("Type a command or search...")
/// ```
#[derive(IntoElement)]
pub struct Command {
    state: Entity<CommandState>,
    entries: Vec<CommandEntry>,
    searchable: bool,
    filter: Option<Rc<CommandFilter>>,
    on_query: Option<Rc<OnQuery>>,
    on_select: Option<Rc<OnValue>>,
    on_confirm: Option<Rc<OnValue>>,
    on_cancel: Option<Rc<OnCancel>>,
    options: CommandOptions,
}

impl Command {
    /// Render the palette held by `state`.
    pub fn new(state: &Entity<CommandState>) -> Self {
        Self {
            state: state.clone(),
            entries: Vec::new(),
            searchable: true,
            filter: None,
            on_query: None,
            on_select: None,
            on_confirm: None,
            on_cancel: None,
            options: CommandOptions::default(),
        }
    }

    /// Add an ungrouped command item.
    pub fn item(mut self, item: CommandItem) -> Self {
        self.entries.push(CommandEntry::Item(item));
        self
    }

    /// Add multiple ungrouped command items.
    pub fn items(mut self, items: impl IntoIterator<Item = CommandItem>) -> Self {
        self.entries
            .extend(items.into_iter().map(CommandEntry::Item));
        self
    }

    /// Add a group of command items.
    pub fn group(mut self, group: CommandGroup) -> Self {
        self.entries.push(CommandEntry::Group(group));
        self
    }

    /// Add a separator between the preceding and following entries.
    pub fn separator(mut self) -> Self {
        self.entries.push(CommandEntry::Separator);
        self
    }

    /// Show or hide the query field and local filtering.
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    /// Set the predicate used to decide whether an item matches the query.
    pub fn filter<F>(mut self, filter: F) -> Self
    where
        F: Fn(&CommandItem, &str) -> bool + 'static,
    {
        self.filter = Some(Rc::new(filter));
        self
    }

    /// Run a callback after a searchable query actually changes and the
    /// current [`CommandState`] update releases its lease.
    pub fn on_query<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str, &mut Window, &mut App) + 'static,
    {
        self.on_query = Some(Rc::new(callback));
        self
    }

    /// Run a callback after the highlighted item value changes and the current
    /// [`CommandState`] update releases its lease.
    pub fn on_select<F>(mut self, callback: F) -> Self
    where
        F: Fn(&SharedString, &mut Window, &mut App) + 'static,
    {
        self.on_select = Some(Rc::new(callback));
        self
    }

    /// Run a callback after a confirmed item's Action has been dispatched,
    /// provided the source window remains live. The callback runs after the
    /// current [`CommandState`] update releases its lease.
    pub fn on_confirm<F>(mut self, callback: F) -> Self
    where
        F: Fn(&SharedString, &mut Window, &mut App) + 'static,
    {
        self.on_confirm = Some(Rc::new(callback));
        self
    }

    /// Run a callback synchronously before an empty-query Cancel action
    /// propagates. A hosting Dialog should perform the dismissal after this
    /// callback instead of being closed by the callback itself.
    pub fn on_cancel<F>(mut self, callback: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.on_cancel = Some(Rc::new(callback));
        self
    }

    /// Set the placeholder of the search field.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.options.placeholder = Some(placeholder.into());
        self
    }

    /// Set the message shown when no command matches the query.
    pub fn empty(mut self, message: impl Into<SharedString>) -> Self {
        self.options.empty_message = Some(message.into());
        self
    }

    /// Set the max height of the list, default: 18.75rem (300px).
    pub fn max_h(mut self, height: impl Into<DefiniteLength>) -> Self {
        self.options.max_h = height.into();
        self
    }

    /// Set whether to draw the surrounding border and rounding, default: `true`.
    ///
    /// Turn it off when the palette already sits inside a frame of its own,
    /// such as a [`crate::Dialog`].
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.options.bordered = bordered;
        self
    }

    /// Render a custom element above the search field and command list.
    pub fn header<F, E>(mut self, f: F) -> Self
    where
        F: Fn(&CommandState, &mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.options.header = Some(Rc::new(move |state, window, cx| {
            f(state, window, cx).into_any_element()
        }));
        self
    }

    /// Render a custom element below the command list.
    pub fn footer<F, E>(mut self, f: F) -> Self
    where
        F: Fn(&CommandState, &mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.options.footer = Some(Rc::new(move |state, window, cx| {
            f(state, window, cx).into_any_element()
        }));
        self
    }
}

impl Styled for Command {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.options.style
    }
}

impl RenderOnce for Command {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let options = self.options;
        let model = CommandModel {
            entries: self.entries,
            searchable: self.searchable,
            filter: self.filter,
            on_query: self.on_query,
            on_select: self.on_select,
            on_confirm: self.on_confirm,
            on_cancel: self.on_cancel,
        };
        self.state.update(cx, |state, cx| {
            state.options = options;
            state.install_model(model, cx);
        });

        self.state
    }
}
