use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window,
};
use gpui_component::{calendar::Calendar, v_flex};

use crate::section;

pub struct CalendarStory {
    focus_handle: FocusHandle,
    calendar: Entity<Calendar>,
    calendar_wide: Entity<Calendar>,
}

impl super::Story for CalendarStory {
    fn title() -> &'static str {
        "Calendar"
    }

    fn description() -> &'static str {
        "A calendar to select a date or date range."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl CalendarStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let calendar = cx.new(|cx| Calendar::new(window, cx));
        let calendar_wide = cx.new(|cx| Calendar::new(window, cx).number_of_months(3));

        Self {
            calendar,
            calendar_wide,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for CalendarStory {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CalendarStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(section("Normal").max_w_md().child(self.calendar.clone()))
            .child(
                section("With 3 Months")
                    .max_w_md()
                    .child(self.calendar_wide.clone()),
            )
    }
}
