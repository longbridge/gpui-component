use gpui::{App, ClickEvent, InteractiveElement, Stateful, Window};

pub trait InteractiveElementExt: InteractiveElement {
    /// Set the listener for a double click event.
    /// Note: Currently simplified to single click due to API changes
    fn on_double_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        // TODO: Implement proper double-click detection when GPUI API supports it
        self.interactivity().on_click(move |event, window, cx| {
            if event.click_count() == 2 {
                listener(event, window, cx);
            }
        });
        self
    }
}

impl<E: InteractiveElement> InteractiveElementExt for Stateful<E> {}
