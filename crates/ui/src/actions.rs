use gpui::{Action, actions};
use serde::Deserialize;

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = ui, no_json)]
pub struct Confirm {
    /// Is confirm with secondary.
    pub secondary: bool,
}

actions!(
    ui,
    [
        Cancel,
        SelectUp,
        SelectDown,
        SelectLeft,
        SelectRight,
        SelectFirst,
        SelectLast,
        SelectPrevColumn,
        SelectNextColumn,
        SelectPageUp,
        SelectPageDown
    ]
);
