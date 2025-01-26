use gpui::{Window, AppContext, ClickEvent, Focusable, InteractiveElement, Stateful, };

pub trait InteractiveElementExt: InteractiveElement {
    /// Set the listener for a double click event.
    fn on_double_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut WindowContext) + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        self.interactivity().on_click(move |event, context| {
            if event.up.click_count == 2 {
                listener(event, context);
            }
        });
        self
    }
}

impl<E: InteractiveElement> InteractiveElementExt for Focusable<E> {}
impl<E: InteractiveElement> InteractiveElementExt for Stateful<E> {}
