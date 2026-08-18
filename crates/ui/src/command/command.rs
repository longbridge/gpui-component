use std::rc::Rc;

use gpui::{
    AnyElement, App, DefiniteLength, Entity, IntoElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, rems,
};

use crate::command::state::CommandState;

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
/// The commands live in the [`CommandState`] this renders; the builders here
/// only decide how the palette looks.
///
/// ```ignore
/// let state = cx.new(|cx| {
///     CommandState::new(window, cx).group(
///         CommandGroup::new("Suggestions")
///             .item(CommandItem::new("Calendar").icon(IconName::Calendar)),
///     )
/// });
///
/// Command::new(&state).placeholder("Type a command or search...")
/// ```
#[derive(IntoElement)]
pub struct Command {
    state: Entity<CommandState>,
    options: CommandOptions,
}

impl Command {
    /// Render the palette held by `state`.
    pub fn new(state: &Entity<CommandState>) -> Self {
        Self {
            state: state.clone(),
            options: CommandOptions::default(),
        }
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
        self.state.update(cx, |state, _| state.options = options);

        self.state
    }
}
