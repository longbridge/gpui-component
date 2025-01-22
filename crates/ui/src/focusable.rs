use gpui::{FocusHandle, WindowContext};

/// A trait for views that can cycle focus between its children.
///
/// This will provide a default implementation for the `cycle_focus` method that will cycle focus.
///
/// You should implement the `cycle_focus_handles` method to return a list of focus handles that
/// should be cycled, and the cycle will follow the order of the list.
pub trait FocusableCycle {
    /// Returns a list of focus handles that should be cycled.
    fn cycle_focus_handles(&self, cx: &mut WindowContext) -> Vec<FocusHandle>
    where
        Self: Sized;

    /// Cycles focus between the focus handles returned by `cycle_focus_handles`.
    /// If `is_next` is `true`, it will cycle to the next focus handle, otherwise it will cycle to prev.
    fn cycle_focus(&self, is_next: bool, cx: &mut WindowContext)
    where
        Self: Sized,
    {
        let focused_handle = cx.focused();
        let handles = self.cycle_focus_handles(cx);
        let handles = if is_next {
            handles
        } else {
            handles.into_iter().rev().collect()
        };

        let fallback_handle = handles[0].clone();
        let target_focus_handle = handles
            .into_iter()
            .skip_while(|handle| Some(handle) != focused_handle.as_ref())
            .skip(1)
            .next()
            .unwrap_or(fallback_handle);

        target_focus_handle.focus(cx);
        cx.stop_propagation();
    }
}
