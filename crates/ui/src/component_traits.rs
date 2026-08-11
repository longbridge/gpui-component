/// A trait for defining an element that can be selected.
#[allow(patterns_in_fns_without_body)]
pub trait Selectable: Sized {
    fn selected(mut self, selected: bool) -> Self;
    fn is_selected(&self) -> bool;
    fn secondary_selected(self, _: bool) -> Self {
        self
    }
}

/// A trait for defining an element that can be disabled.
#[allow(patterns_in_fns_without_body)]
pub trait Disableable {
    fn disabled(mut self, disabled: bool) -> Self;
}

pub trait Collapsible {
    fn collapsed(self, collapsed: bool) -> Self;
    fn is_collapsed(&self) -> bool;
}
