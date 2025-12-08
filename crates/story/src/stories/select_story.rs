use gpui::*;
use gpui_component::{button::*, checkbox::*, divider::*, input::*, select::*, *};
use itertools::Itertools as _;
use serde::{Deserialize, Serialize};

use crate::section;

pub fn init(_: &mut App) {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Country {
    name: SharedString,
    code: SharedString,
}

impl Country {
    pub fn letter_prefix(&self) -> char {
        self.name.chars().next().unwrap_or(' ')
    }
}

impl SelectItem for Country {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn display_title(&self) -> Option<gpui::AnyElement> {
        Some(format!("{} ({})", self.name, self.code).into_any_element())
    }

    fn value(&self) -> &Self::Value {
        &self.code
    }
}

pub struct SelectStory {
    disabled: bool,
    country_select: Entity<SelectState<SearchableVec<SelectGroup<Country>>>>,
    fruit_select: Entity<SelectState<SearchableVec<&'static str>>>,
    simple_select1: Entity<SelectState<Vec<&'static str>>>,
    simple_select2: Entity<SelectState<SearchableVec<&'static str>>>,
    simple_select3: Entity<SelectState<Vec<SharedString>>>,
    disabled_select: Entity<SelectState<Vec<SharedString>>>,
    appearance_select: Entity<SelectState<Vec<SharedString>>>,
    input_state: Entity<InputState>,
}

impl super::Story for SelectStory {
    fn title() -> &'static str {
        "Select"
    }

    fn description() -> &'static str {
        "Displays a list of options for the user to pick from—triggered by a button."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for SelectStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.fruit_select.focus_handle(cx)
    }
}

impl SelectStory {
    fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let countries =
            serde_json::from_str::<Vec<Country>>(include_str!("../fixtures/countries.json"))
                .unwrap();
        let mut grouped_countries: SearchableVec<SelectGroup<Country>> = SearchableVec::new(vec![]);
        for (prefix, items) in countries.iter().chunk_by(|c| c.letter_prefix()).into_iter() {
            let items = items.cloned().collect::<Vec<Country>>();
            grouped_countries.push(SelectGroup::new(prefix.to_string()).items(items));
        }

        let country_select = cx.new(|cx| {
            SelectState::new(
                grouped_countries,
                Some(IndexPath::default().row(8).section(2)),
                window,
                cx,
            )
            .searchable(true)
        });
        let appearance_select = cx.new(|cx| {
            SelectState::new(
                vec![
                    "CN".into(),
                    "US".into(),
                    "HK".into(),
                    "JP".into(),
                    "KR".into(),
                ],
                Some(IndexPath::default()),
                window,
                cx,
            )
        });
        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder("Your phone number"));

        let fruits = SearchableVec::new(vec![
            "Apple",
            "Orange",
            "Banana",
            "Grape",
            "Pineapple",
            "Watermelon & This is a long long long long long long long long long title",
            "Avocado",
        ]);
        let fruit_select = cx.new(|cx| SelectState::new(fruits, None, window, cx).searchable(true));

        cx.new(|cx| {
            cx.subscribe_in(&country_select, window, Self::on_select_event)
                .detach();

            Self {
                disabled: false,
                country_select,
                fruit_select,
                simple_select1: cx.new(|cx| {
                    SelectState::new(
                        vec![
                            "GPUI", "Iced", "egui", "Makepad", "Slint", "QT", "ImGui", "Cocoa",
                            "WinUI",
                        ],
                        Some(IndexPath::default()),
                        window,
                        cx,
                    )
                }),
                simple_select2: cx.new(|cx| {
                    let mut select = SelectState::new(SearchableVec::new(vec![]), None, window, cx);

                    select.set_items(
                        SearchableVec::new(vec!["Rust", "Go", "C++", "JavaScript"]),
                        window,
                        cx,
                    );

                    select
                }),
                simple_select3: cx
                    .new(|cx| SelectState::new(Vec::<SharedString>::new(), None, window, cx)),
                disabled_select: cx
                    .new(|cx| SelectState::new(Vec::<SharedString>::new(), None, window, cx)),
                appearance_select,
                input_state,
            }
        })
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        Self::new(window, cx)
    }

    fn on_select_event(
        &mut self,
        _: &Entity<SelectState<SearchableVec<SelectGroup<Country>>>>,
        event: &SelectEvent<SearchableVec<SelectGroup<Country>>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            SelectEvent::Confirm(value) => println!("Selected country: {:?}", value),
        }
    }

    fn toggle_disabled(&mut self, disabled: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.disabled = disabled;
        cx.notify();
    }
}

impl Render for SelectStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_4()
            .child(
                Checkbox::new("disable-selects")
                    .label("Disabled")
                    .checked(self.disabled)
                    .on_click(cx.listener(|this, checked, window, cx| {
                        this.toggle_disabled(*checked, window, cx);
                    })),
            )
            .child(
                section("Select").max_w_128().child(
                    Select::new(&self.country_select)
                        .search_placeholder("Search country by name or code")
                        .cleanable(true)
                        .disabled(self.disabled),
                ),
            )
            .child(
                section("Searchable").max_w_128().child(
                    Select::new(&self.fruit_select)
                        .disabled(self.disabled)
                        .icon(IconName::Search)
                        .w(px(320.))
                        .menu_width(px(400.)),
                ),
            )
            .child(
                section("Disabled")
                    .max_w_128()
                    .child(Select::new(&self.disabled_select).disabled(true)),
            )
            .child(
                section("With preview label").max_w_128().child(
                    Select::new(&self.simple_select1)
                        .disabled(self.disabled)
                        .small()
                        .placeholder("UI")
                        .title_prefix("UI: "),
                ),
            )
            .child(
                section("Searchable Select").max_w_128().child(
                    Select::new(&self.simple_select2)
                        .disabled(self.disabled)
                        .small()
                        .placeholder("Language")
                        .title_prefix("Language: "),
                ),
            )
            .child(
                section("Empty Items").max_w_128().child(
                    Select::new(&self.simple_select3)
                        .disabled(self.disabled)
                        .small()
                        .empty(
                            h_flex()
                                .h_24()
                                .justify_center()
                                .text_color(cx.theme().muted_foreground)
                                .child("No Data"),
                        ),
                ),
            )
            .child(
                section("Appearance false with Input").max_w_128().child(
                    h_flex()
                        .border_1()
                        .border_color(cx.theme().input)
                        .rounded_lg()
                        .text_color(cx.theme().secondary_foreground)
                        .w_full()
                        .gap_1()
                        .child(
                            div().w(px(140.)).child(
                                Select::new(&self.appearance_select)
                                    .appearance(false)
                                    .py_2()
                                    .pl_3(),
                            ),
                        )
                        .child(Divider::vertical())
                        .child(
                            div().flex_1().child(
                                Input::new(&self.input_state)
                                    .appearance(false)
                                    .pr_3()
                                    .py_2(),
                            ),
                        )
                        .child(
                            div()
                                .p_2()
                                .child(Button::new("send").small().ghost().label("Send")),
                        ),
                ),
            )
            .child(
                section("Selected Values").max_w_lg().child(
                    v_flex()
                        .gap_3()
                        .child(format!(
                            "Country: {:?}",
                            self.country_select.read(cx).selected_value()
                        ))
                        .child(format!(
                            "fruit: {:?}",
                            self.fruit_select.read(cx).selected_value()
                        ))
                        .child(format!(
                            "UI: {:?}",
                            self.simple_select1.read(cx).selected_value()
                        ))
                        .child(format!(
                            "Language: {:?}",
                            self.simple_select2.read(cx).selected_value()
                        ))
                        .child("This is other text."),
                ),
            )
    }
}
