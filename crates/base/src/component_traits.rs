/// An element or component that exposes controlled selected state.
#[allow(patterns_in_fns_without_body)]
pub trait Selectable: Sized {
    fn selected(mut self, selected: bool) -> Self;
    fn is_selected(&self) -> bool;

    fn secondary_selected(self, _: bool) -> Self {
        self
    }
}

/// An element or component that can be disabled.
#[allow(patterns_in_fns_without_body)]
pub trait Disableable {
    fn disabled(mut self, disabled: bool) -> Self;
}
