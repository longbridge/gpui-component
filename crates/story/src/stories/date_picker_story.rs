use chrono::{Datelike, Days, Duration, Utc};
use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement as _, Render,
    Styled as _, Subscription, Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, Sizable as _, calendar,
    date_picker::{DatePicker, DatePickerEvent, DatePickerState, DateRangePreset},
    v_flex,
};

use crate::section;

pub struct DatePickerStory {
    date_picker: Entity<DatePickerState>,
    date_picker_small: Entity<DatePickerState>,
    date_picker_large: Entity<DatePickerState>,
    data_picker_custom: Entity<DatePickerState>,
    date_picker_value: Option<String>,
    date_range_picker: Entity<DatePickerState>,
    default_range_mode_picker: Entity<DatePickerState>,
    without_appearance_picker: Entity<DatePickerState>,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for DatePickerStory {
    fn title() -> &'static str {
        "DatePicker"
    }

    fn description() -> &'static str {
        "A date picker to select a date or date range."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl DatePickerStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let now = chrono::Local::now().naive_local().date();
        let date_picker = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(vec![0, 6]);
            picker.set_date(now, window, cx);
            picker
        });
        let date_picker_large = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx)
                .date_format("%Y-%m-%d")
                .disabled_matcher(calendar::Matcher::range(
                    Some(now),
                    now.checked_add_days(Days::new(7)),
                ));
            picker.set_date(
                now.checked_sub_days(Days::new(1)).unwrap_or_default(),
                window,
                cx,
            );
            picker
        });
        let date_picker_small = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(
                calendar::Matcher::interval(Some(now), now.checked_add_days(Days::new(5))),
            );
            picker.set_date(now, window, cx);
            picker
        });
        let data_picker_custom = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx)
                .disabled_matcher(calendar::Matcher::custom(|date| date.day0() < 5));
            picker.set_date(now, window, cx);
            picker
        });
        let date_range_picker = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx);
            picker.set_date(
                (now, now.checked_add_days(Days::new(4)).unwrap()),
                window,
                cx,
            );
            picker
        });

        let default_range_mode_picker = cx.new(|cx| DatePickerState::range(window, cx));

        let without_appearance_picker = cx.new(|cx| DatePickerState::new(window, cx));

        let _subscriptions = vec![
            cx.subscribe(&date_picker, |this, _, ev, _| match ev {
                DatePickerEvent::Change(date) => {
                    this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
                }
            }),
            cx.subscribe(&date_range_picker, |this, _, ev, _| match ev {
                DatePickerEvent::Change(date) => {
                    this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
                }
            }),
            cx.subscribe(&default_range_mode_picker, |this, _, ev, _| match ev {
                DatePickerEvent::Change(date) => {
                    this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
                }
            }),
        ];

        Self {
            date_picker,
            date_picker_large,
            date_picker_small,
            data_picker_custom,
            date_range_picker,
            default_range_mode_picker,
            without_appearance_picker,
            date_picker_value: None,
            _subscriptions,
        }
    }
}

impl Focusable for DatePickerStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.date_picker.focus_handle(cx)
    }
}

impl Render for DatePickerStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let presets = vec![
            DateRangePreset::single(
                "Yesterday",
                (Utc::now() - Duration::days(1)).naive_local().date(),
            ),
            DateRangePreset::single(
                "Last Week",
                (Utc::now() - Duration::weeks(1)).naive_local().date(),
            ),
            DateRangePreset::single(
                "Last Month",
                (Utc::now() - Duration::days(30)).naive_local().date(),
            ),
        ];
        let range_presets = vec![
            DateRangePreset::range(
                "Last 7 Days",
                (Utc::now() - Duration::days(7)).naive_local().date(),
                Utc::now().naive_local().date(),
            ),
            DateRangePreset::range(
                "Last 14 Days",
                (Utc::now() - Duration::days(14)).naive_local().date(),
                Utc::now().naive_local().date(),
            ),
            DateRangePreset::range(
                "Last 30 Days",
                (Utc::now() - Duration::days(30)).naive_local().date(),
                Utc::now().naive_local().date(),
            ),
            DateRangePreset::range(
                "Last 90 Days",
                (Utc::now() - Duration::days(90)).naive_local().date(),
                Utc::now().naive_local().date(),
            ),
        ];

        v_flex()
            .gap_3()
            .child(
                section("Normal").max_w_128().child(
                    DatePicker::new(&self.date_picker)
                        .cleanable(true)
                        .presets(presets),
                ),
            )
            .child(
                section("Small with 180px width")
                    .max_w_128()
                    .child(DatePicker::new(&self.date_picker_small).small().w(px(180.))),
            )
            .child(
                section("Large")
                    .max_w_128()
                    .child(DatePicker::new(&self.date_picker_large).large().w(px(300.))),
            )
            .child(
                section("Custom (First 5 days of each month disabled)")
                    .max_w_128()
                    .child(DatePicker::new(&self.data_picker_custom)),
            )
            .child(
                section("Date Range").max_w_128().child(
                    DatePicker::new(&self.date_range_picker)
                        .number_of_months(2)
                        .cleanable(true)
                        .presets(range_presets.clone()),
                ),
            )
            .child(
                section("Default Range Mode").max_w_128().child(
                    DatePicker::new(&self.default_range_mode_picker)
                        .placeholder("Range mode picker")
                        .cleanable(true)
                        .presets(range_presets.clone()),
                ),
            )
            .child(
                section("Date Picker Value").max_w_128().child(
                    format!("Date picker value: {:?}", self.date_picker_value).into_element(),
                ),
            )
            .child(
                section("Without Appearance").max_w_128().child(
                    div().w_full().bg(cx.theme().secondary).child(
                        DatePicker::new(&self.without_appearance_picker)
                            .appearance(false)
                            .placeholder("Without appearance"),
                    ),
                ),
            )
    }
}
