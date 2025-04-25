use chrono::{Days, Duration, Utc};
use gpui::{
    px, App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement as _, Render,
    Styled as _, Subscription, Window,
};
use gpui_component::{
    calendar,
    date_picker::{DatePicker, DatePickerEvent, DateRangePreset},
    v_flex, Sizable as _,
};

use crate::section;

pub struct DatePickerStory {
    date_picker: Entity<DatePicker>,
    date_picker_small: Entity<DatePicker>,
    date_picker_large: Entity<DatePicker>,
    date_picker_value: Option<String>,
    date_range_picker: Entity<DatePicker>,
    default_range_mode_picker: Entity<DatePicker>,

    _subscriptions: Vec<Subscription>,
}

impl super::Story for DatePickerStory {
    fn title() -> &'static str {
        "DatePicker"
    }

    fn description() -> &'static str {
        "A date picker to select a date or date range."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl DatePickerStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
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
        let now = chrono::Local::now().naive_local().date();
        let date_picker = cx.new(|cx| {
            let mut picker = DatePicker::new("date_picker_medium", window, cx)
                .cleanable()
                .presets(presets);
            picker.set_date(now, window, cx);
            picker.set_disabled(vec![0, 6], window, cx);
            picker
        });
        let date_picker_large = cx.new(|cx| {
            let mut picker = DatePicker::new("date_picker_large", window, cx)
                .large()
                .date_format("%Y-%m-%d")
                .width(px(300.));
            picker.set_disabled(
                calendar::Matcher::range(Some(now), now.checked_add_days(Days::new(7))),
                window,
                cx,
            );
            picker.set_date(
                now.checked_sub_days(Days::new(1)).unwrap_or_default(),
                window,
                cx,
            );
            picker
        });
        let date_picker_small = cx.new(|cx| {
            let mut picker = DatePicker::new("date_picker_small", window, cx)
                .small()
                .width(px(180.));
            picker.set_disabled(
                calendar::Matcher::interval(Some(now), now.checked_add_days(Days::new(5))),
                window,
                cx,
            );
            picker.set_date(now, window, cx);
            picker
        });
        let date_range_picker = cx.new(|cx| {
            let mut picker = DatePicker::new("date_range_picker", window, cx)
                .number_of_months(2)
                .cleanable()
                .presets(range_presets.clone());
            picker.set_date(
                (now, now.checked_add_days(Days::new(4)).unwrap()),
                window,
                cx,
            );
            picker
        });

        let default_range_mode_picker = cx.new(|cx| {
            DatePicker::range_picker("default_range_mode_picker", window, cx)
                .placeholder("Range mode picker")
                .cleanable()
                .presets(range_presets.clone())
        });

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
            date_range_picker,
            default_range_mode_picker,
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
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(section("Normal").max_w_md().child(self.date_picker.clone()))
            .child(
                section("Small with 180px width")
                    .max_w_md()
                    .child(self.date_picker_small.clone()),
            )
            .child(
                section("Large")
                    .max_w_md()
                    .child(self.date_picker_large.clone()),
            )
            .child(
                section("Date Range")
                    .max_w_md()
                    .child(self.date_range_picker.clone()),
            )
            .child(
                section("Default Range Mode")
                    .max_w_md()
                    .child(self.default_range_mode_picker.clone()),
            )
            .child(
                section("Date Picker Value").max_w_md().child(
                    format!("Date picker value: {:?}", self.date_picker_value).into_element(),
                ),
            )
    }
}
