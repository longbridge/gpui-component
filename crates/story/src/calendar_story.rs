use chrono::{Days, Duration, Utc};
use gpui::{
    px, IntoElement, ParentElement as _, Render, Styled as _, View, ViewContext,
    VisualContext as _, WindowContext,
};
use ui::{
    button::Button,
    date_picker::{DatePicker, DatePickerEvent, DateRangePreset},
    v_flex, Sizable as _, Size,
};

pub struct CalendarStory {
    size: Size,
    date_picker: View<DatePicker>,
    date_picker_small: View<DatePicker>,
    date_picker_large: View<DatePicker>,
    date_picker_value: Option<String>,
    date_range_picker: View<DatePicker>,
    default_range_mode_picker: View<DatePicker>,
}

impl super::Story for CalendarStory {
    fn title() -> &'static str {
        "Calendar"
    }

    fn description() -> &'static str {
        "A date picker and calendar component."
    }

    fn new_view(cx: &mut WindowContext) -> View<impl gpui::FocusableView> {
        Self::view(cx)
    }
}

impl CalendarStory {
    pub fn view(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(Self::new)
    }

    fn new(cx: &mut ViewContext<Self>) -> Self {
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
        let date_picker = cx.new_view(|cx| {
            let mut picker = DatePicker::new("date_picker_medium", cx)
                .cleanable()
                .width(px(220.))
                .presets(presets);
            picker.set_date(now, cx);
            picker
        });
        let date_picker_large = cx.new_view(|cx| {
            DatePicker::new("date_picker_large", cx)
                .large()
                .date_format("%Y-%m-%d")
                .width(px(300.))
        });
        let date_picker_small = cx.new_view(|cx| {
            let mut picker = DatePicker::new("date_picker_small", cx)
                .small()
                .width(px(180.));
            picker.set_date(now, cx);
            picker
        });
        let date_range_picker = cx.new_view(|cx| {
            let mut picker = DatePicker::new("date_range_picker", cx)
                .width(px(300.))
                .number_of_months(2)
                .cleanable()
                .presets(range_presets.clone());
            picker.set_date((now, now.checked_add_days(Days::new(4)).unwrap()), cx);
            picker
        });

        cx.subscribe(&date_picker, |this, _, ev, _| match ev {
            DatePickerEvent::Change(date) => {
                this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
            }
        })
        .detach();
        cx.subscribe(&date_range_picker, |this, _, ev, _| match ev {
            DatePickerEvent::Change(date) => {
                this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
            }
        })
        .detach();

        let default_range_mode_picker = cx.new_view(|cx| {
            DatePicker::range_picker("default_range_mode_picker", cx)
                .width(px(300.))
                .placeholder("Range mode picker")
                .cleanable()
                .presets(range_presets.clone())
        });

        cx.subscribe(&default_range_mode_picker, |this, _, ev, _| match ev {
            DatePickerEvent::Change(date) => {
                this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
            }
        })
        .detach();

        Self {
            size: Size::default(),
            date_picker,
            date_picker_large,
            date_picker_small,
            date_range_picker,
            default_range_mode_picker,
            date_picker_value: None,
        }
    }

    fn change_size(&mut self, size: Size, cx: &mut ViewContext<Self>) {
        self.size = size;
        self.date_picker
            .update(cx, |picker, cx| picker.set_size(size, cx));
        self.date_picker_large
            .update(cx, |picker, cx| picker.set_size(size, cx));
        self.date_picker_small
            .update(cx, |picker, cx| picker.set_size(size, cx));
        self.date_range_picker
            .update(cx, |picker, cx| picker.set_size(size, cx));
        self.default_range_mode_picker
            .update(cx, |picker, cx| picker.set_size(size, cx));
    }
}

impl gpui::FocusableView for CalendarStory {
    fn focus_handle(&self, cx: &gpui::AppContext) -> gpui::FocusHandle {
        self.date_picker.focus_handle(cx)
    }
}

impl Render for CalendarStory {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                Button::new("change-size")
                    .label(format!("size: {:?}", self.size))
                    .on_click(cx.listener(|this, _, cx| match this.size {
                        Size::Small => this.change_size(Size::Medium, cx),
                        Size::Large => this.change_size(Size::Small, cx),
                        _ => this.change_size(Size::Large, cx),
                    })),
            )
            .child(self.date_picker.clone())
            .child(self.date_picker_small.clone())
            .child(self.date_picker_large.clone())
            .child(self.date_range_picker.clone())
            .child(self.default_range_mode_picker.clone())
            .child(format!("Date picker value: {:?}", self.date_picker_value).into_element())
    }
}
