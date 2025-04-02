use gpui::{actions, impl_internal_actions};
use serde::Deserialize;

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct Confirm {
    /// Is confirm with secondary.
    pub secondary: bool,
}

actions!(list, [Cancel, SelectPrev, SelectNext]);
impl_internal_actions!(list, [Confirm]);
