//! A command palette: a search field over a filtered list of commands, with
//! groups, shortcut hints and keyboard navigation.
//!
//! [`CommandState`] holds the commands and the query; [`Command`] renders it.
//! [`crate::WindowExt::open_command_dialog`] presents the same palette in a
//! dialog.
#[allow(clippy::module_inception)]
mod command;
mod item;
mod state;

pub use command::Command;
pub use item::{CommandEntry, CommandGroup, CommandItem};
pub use state::{CommandEvent, CommandState};

pub(crate) use state::init;
