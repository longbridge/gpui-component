use gpui::{actions, Action};
use serde::Deserialize;

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = ui, no_json)]
pub struct Confirm {
    /// Is confirm with secondary.
    pub secondary: bool,
}

actions!(
    list,
    [Cancel, SelectPrev, SelectNext, SelectLeft, SelectRight]
);
