use chrono::{Days, Duration, Utc};
use gpui::{
    px, App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement as _, Render,
    Styled as _, Window,
};
use gpui_component::{
    button::Button,
    calendar,
    date_picker::{DatePicker, DatePickerEvent, DateRangePreset},
    v_flex, Sizable as _, Size,
};

pub struct CalendarStory {
    size: Size,
    date_picker: Entity<DatePicker>,
    date_picker_small: Entity<DatePicker>,
    date_picker_large: Entity<DatePicker>,
    date_picker_value: Option<String>,
    date_range_picker: Entity<DatePicker>,
    default_range_mode_picker: Entity<DatePicker>,
}

impl super::Story for CalendarStory {
    fn title() -> &'static str {
        "Calendar"
    }

    fn description() -> &'static str {
        "A date picker and calendar component."
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
                .width(px(220.))
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
            picker.set_date(now, window, cx);
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
                .width(px(300.))
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

        let default_range_mode_picker = cx.new(|cx| {
            DatePicker::range_picker("default_range_mode_picker", window, cx)
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

    fn change_size(&mut self, size: Size, window: &mut Window, cx: &mut Context<Self>) {
        self.size = size;
        self.date_picker
            .update(cx, |picker, cx| picker.set_size(size, window, cx));
        self.date_picker_large
            .update(cx, |picker, cx| picker.set_size(size, window, cx));
        self.date_picker_small
            .update(cx, |picker, cx| picker.set_size(size, window, cx));
        self.date_range_picker
            .update(cx, |picker, cx| picker.set_size(size, window, cx));
        self.default_range_mode_picker
            .update(cx, |picker, cx| picker.set_size(size, window, cx));
    }
}

impl Focusable for CalendarStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.date_picker.focus_handle(cx)
    }
}

impl Render for CalendarStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                Button::new("change-size")
                    .label(format!("size: {:?}", self.size))
                    .on_click(cx.listener(|this, _, window, cx| match this.size {
                        Size::Small => this.change_size(Size::Medium, window, cx),
                        Size::Large => this.change_size(Size::Small, window, cx),
                        _ => this.change_size(Size::Large, window, cx),
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
