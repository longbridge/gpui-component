//! A command palette: a search field over a filtered list of commands, with
//! groups, shortcut hints and keyboard navigation.
//!
//! [`Command`] owns the entries and rendering policy; [`CommandState`] holds
//! interaction state such as the query, focus, selection, and scrolling.
#[allow(clippy::module_inception)]
mod command;
mod item;
mod state;

pub use command::Command;
pub use item::{CommandEntry, CommandGroup, CommandItem};
pub use state::{CommandEvent, CommandState};

pub(crate) use state::init;
