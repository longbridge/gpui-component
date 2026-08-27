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

/// A component that exposes whether its UI layer should draw a focus ring.
///
/// This trait carries state only. Focus-ring geometry and presentation belong
/// to the component's visual layer.
pub trait FocusableExt: Sized {
    fn focus_ring(self, enabled: bool) -> Self;
    fn is_focus_ring_enabled(&self) -> bool;
}

/// An element or component that exposes collapsed state.
pub trait Collapsible {
    fn collapsed(self, collapsed: bool) -> Self;
    fn is_collapsed(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::FocusableExt;

    struct CustomControl {
        focus_ring_enabled: bool,
    }

    impl FocusableExt for CustomControl {
        fn focus_ring(mut self, enabled: bool) -> Self {
            self.focus_ring_enabled = enabled;
            self
        }

        fn is_focus_ring_enabled(&self) -> bool {
            self.focus_ring_enabled
        }
    }

    #[test]
    fn focus_ring_api_carries_state_without_visuals() {
        let control = CustomControl {
            focus_ring_enabled: true,
        }
        .focus_ring(false);

        assert!(!control.is_focus_ring_enabled());
    }
}
