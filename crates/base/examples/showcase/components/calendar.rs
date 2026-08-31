use super::*;

impl BaseShowcase {
    pub(in super::super) fn calendar(&self) -> impl IntoElement {
        Calendar::new("example-calendar", &self.calendar)
            // 7 × 32px cells + 12px padding on each side + 1px borders.
            .w(px(250.))
            .p_3()
            .border_1()
            .border_color(super::example_rgb(0xd4d4d4))
            .item(|item, state, _, _| {
                match state.kind() {
                    CalendarItemKind::Previous | CalendarItemKind::Next => item
                        .size_7()
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(|s| s.bg(super::example_rgb(0xf5f5f5))),
                    CalendarItemKind::MonthToggle | CalendarItemKind::YearToggle => item
                        .px_1()
                        .h_7()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .hover(|s| s.bg(super::example_rgb(0xf5f5f5))),
                    CalendarItemKind::Weekday => item
                        .size_8()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .text_color(super::example_rgb(0x737373)),
                    CalendarItemKind::Day => item
                        .size_8()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .when(state.is_muted(), |s| {
                            s.text_color(super::example_rgb(0xa3a3a3))
                        })
                        .when(state.is_today() && !state.is_active(), |s| {
                            s.border_1().border_color(super::example_rgb(0xd4d4d4))
                        })
                        .when(state.is_active(), |s| {
                            s.bg(super::example_rgb(0x171717))
                                .text_color(super::example_rgb(0xffffff))
                        })
                        .when(!state.is_disabled() && !state.is_active(), |s| {
                            s.hover(|s| s.bg(super::example_rgb(0xf5f5f5)))
                        }),
                    CalendarItemKind::Month | CalendarItemKind::Year => item
                        .w(px(74.))
                        .h_7()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .when(state.is_active(), |s| {
                            s.bg(super::example_rgb(0x171717))
                                .text_color(super::example_rgb(0xffffff))
                        })
                        .when(!state.is_active(), |s| {
                            s.hover(|s| s.bg(super::example_rgb(0xf5f5f5)))
                        }),
                }
                .into_any_element()
            })
            .label(|kind, value| match kind {
                CalendarItemKind::Previous => "‹".into(),
                CalendarItemKind::Next => "›".into(),
                CalendarItemKind::MonthToggle | CalendarItemKind::Month => [
                    "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
                    "Nov", "Dec",
                ][value as usize]
                    .into(),
                CalendarItemKind::Weekday => {
                    ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"][value as usize].into()
                }
                _ => value.to_string().into(),
            })
    }
}
