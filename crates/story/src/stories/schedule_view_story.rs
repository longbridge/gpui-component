use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window, prelude::FluentBuilder as _,
};
use gpui_component::{
    Sizable,
    schedule_view::{ScheduleEvent, ScheduleView, ScheduleViewState, SlotInterval},
    v_flex,
};

use crate::section;

pub struct ScheduleViewStory {
    focus_handle: FocusHandle,
    single_day: Entity<ScheduleViewState>,
    multi_day: Entity<ScheduleViewState>,
    half_hour: Entity<ScheduleViewState>,
    last_event: String,
}

impl super::Story for ScheduleViewStory {
    fn title() -> &'static str {
        "ScheduleView"
    }

    fn description() -> &'static str {
        "A day/week schedule view with configurable time slot intervals and drag-to-select."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ScheduleViewStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let single_day =
            cx.new(|cx| ScheduleViewState::new(window, cx).visible_hours(8, 18));
        cx.subscribe_in(&single_day, window, Self::on_schedule_event)
            .detach();

        let multi_day = cx.new(|cx| {
            ScheduleViewState::new(window, cx)
                .num_days(5)
                .visible_hours(6, 22)
        });
        cx.subscribe_in(&multi_day, window, Self::on_schedule_event)
            .detach();

        let half_hour = cx.new(|cx| {
            ScheduleViewState::new(window, cx)
                .interval(SlotInterval::minutes(30))
                .num_days(3)
                .visible_hours(9, 17)
        });
        cx.subscribe_in(&half_hour, window, Self::on_schedule_event)
            .detach();

        Self {
            focus_handle: cx.focus_handle(),
            single_day,
            multi_day,
            half_hour,
            last_event: String::new(),
        }
    }

    fn on_schedule_event(
        &mut self,
        _: &Entity<ScheduleViewState>,
        ev: &ScheduleEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match ev {
            ScheduleEvent::SlotClicked(pos) => {
                self.last_event = format!(
                    "Clicked: {} @ {}",
                    pos.date.format("%Y-%m-%d"),
                    pos.time.format("%H:%M")
                );
            }
            ScheduleEvent::RangeSelected(range) => {
                if range.start_date == range.end_date {
                    self.last_event = format!(
                        "Range: {} {} – {}",
                        range.start_date.format("%Y-%m-%d"),
                        range.start.format("%H:%M"),
                        range.end.format("%H:%M")
                    );
                } else {
                    self.last_event = format!(
                        "Range: {} {} – {} {}",
                        range.start_date.format("%Y-%m-%d"),
                        range.start.format("%H:%M"),
                        range.end_date.format("%Y-%m-%d"),
                        range.end.format("%H:%M")
                    );
                }
            }
        }
        cx.notify();
    }
}

impl Focusable for ScheduleViewStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ScheduleViewStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .when(!self.last_event.is_empty(), |this| {
                this.child(
                    gpui::div()
                        .text_sm()
                        .child(format!("Last event: {}", self.last_event)),
                )
            })
            .child(
                section("Single Day (08:00–18:00, 1h slots)")
                    .child(ScheduleView::new(&self.single_day)),
            )
            .child(
                section("Work Week (06:00–22:00, 1h slots)")
                    .child(ScheduleView::new(&self.multi_day)),
            )
            .child(
                section("3 Days (09:00–17:00, 30min slots)")
                    .child(ScheduleView::new(&self.half_hour).with_size(gpui_component::Size::Small)),
            )
    }
}
