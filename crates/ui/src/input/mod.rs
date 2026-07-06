/// The character used to mask password input fields.
pub(super) const MASK_CHAR: char = '•';

mod blink_cursor;
mod change;
mod clear_button;
mod content_type;
mod cursor;
mod display_map;
mod element;
mod indent;
mod input;
mod lsp;
mod mask_pattern;
mod mode;
mod movement;
#[cfg(target_os = "macos")]
mod native;
mod number_input;
mod otp_input;
pub(crate) mod popovers;
mod rope_ext;
mod search;
mod selection;
mod state;

pub(crate) use clear_button::*;
pub use content_type::*;
pub use cursor::*;
#[cfg(not(feature = "tree-sitter"))]
pub use display_map::Tree;
pub use display_map::{BufferPoint, DisplayMap, DisplayPoint, FoldRange};
pub use indent::TabSize;
pub use input::*;
pub use lsp::*;
pub use lsp_types::Position;
pub use mask_pattern::MaskPattern;
pub use number_input::{NumberInput, NumberInputEvent, NumberStep, StepAction};
pub use otp_input::*;
pub use rope_ext::{InputEdit, Point, RopeExt, RopeLines};
pub use ropey::Rope;
pub use state::*;
