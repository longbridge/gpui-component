use gpui::*;
use gpui_component::{button::*, checkbox::*, divider::*, dropdown::*, input::*, *};
use itertools::Itertools as _;
use serde::{Deserialize, Serialize};

use crate::section;
use crate::{Tab, TabPrev};

const CONTEXT: &str = "DropdownStory";
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
    ])
}

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

impl DropdownItem for Country {
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

pub struct DropdownStory {
    disabled: bool,
    country_dropdown: Entity<DropdownState<SearchableVec<DropdownItemGroup<Country>>>>,
    fruit_dropdown: Entity<DropdownState<SearchableVec<SharedString>>>,
    simple_dropdown1: Entity<DropdownState<Vec<SharedString>>>,
    simple_dropdown2: Entity<DropdownState<SearchableVec<SharedString>>>,
    simple_dropdown3: Entity<DropdownState<Vec<SharedString>>>,
    disabled_dropdown: Entity<DropdownState<Vec<SharedString>>>,
    appearance_dropdown: Entity<DropdownState<Vec<SharedString>>>,
    input_state: Entity<InputState>,
}

impl super::Story for DropdownStory {
    fn title() -> &'static str {
        "Dropdown"
    }

    fn description() -> &'static str {
        "Displays a list of options for the user to pick from—triggered by a button."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl Focusable for DropdownStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.fruit_dropdown.focus_handle(cx)
    }
}

impl DropdownStory {
    fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let countries =
            serde_json::from_str::<Vec<Country>>(include_str!("./fixtures/countries.json"))
                .unwrap();
        let mut grouped_countries: SearchableVec<DropdownItemGroup<Country>> =
            SearchableVec::new(vec![]);
        for (prefix, items) in countries.iter().chunk_by(|c| c.letter_prefix()).into_iter() {
            let items = items.cloned().collect::<Vec<Country>>();
            grouped_countries.push(DropdownItemGroup::new(prefix.to_string()).items(items));
        }

        let country_dropdown = cx.new(|cx| {
            DropdownState::new(
                grouped_countries,
                Some(IndexPath::default().row(8).section(2)),
                window,
                cx,
            )
        });
        let appearance_dropdown = cx.new(|cx| {
            DropdownState::new(
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
            "Apple".into(),
            "Orange".into(),
            "Banana".into(),
            "Grape".into(),
            "Pineapple".into(),
            "Watermelon & This is a long long long long long long long long long title".into(),
            "Avocado".into(),
        ]);
        let fruit_dropdown = cx.new(|cx| DropdownState::new(fruits, None, window, cx));

        cx.new(|cx| {
            cx.subscribe_in(&country_dropdown, window, Self::on_dropdown_event)
                .detach();

            Self {
                disabled: false,
                country_dropdown,
                fruit_dropdown,
                simple_dropdown1: cx.new(|cx| {
                    DropdownState::new(
                        vec![
                            "GPUI".into(),
                            "Iced".into(),
                            "egui".into(),
                            "Makepad".into(),
                            "Slint".into(),
                            "QT".into(),
                            "ImGui".into(),
                            "Cocoa".into(),
                            "WinUI".into(),
                        ],
                        Some(IndexPath::default()),
                        window,
                        cx,
                    )
                }),
                simple_dropdown2: cx.new(|cx| {
                    let mut dropdown =
                        DropdownState::new(SearchableVec::new(vec![]), None, window, cx);

                    dropdown.set_items(
                        SearchableVec::new(vec![
                            "Rust".into(),
                            "Go".into(),
                            "C++".into(),
                            "JavaScript".into(),
                        ]),
                        window,
                        cx,
                    );

                    dropdown
                }),
                simple_dropdown3: cx
                    .new(|cx| DropdownState::new(Vec::<SharedString>::new(), None, window, cx)),
                disabled_dropdown: cx
                    .new(|cx| DropdownState::new(Vec::<SharedString>::new(), None, window, cx)),
                appearance_dropdown,
                input_state,
            }
        })
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        Self::new(window, cx)
    }

    fn on_dropdown_event(
        &mut self,
        _: &Entity<DropdownState<SearchableVec<DropdownItemGroup<Country>>>>,
        event: &DropdownEvent<SearchableVec<DropdownItemGroup<Country>>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            DropdownEvent::Confirm(value) => println!("Selected country: {:?}", value),
        }
    }

    fn on_key_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
        cx.notify();
    }

    fn on_key_shift_tab(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(false, window, cx);
        cx.notify();
    }

    fn toggle_disabled(&mut self, disabled: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.disabled = disabled;
        cx.notify();
    }
}

impl FocusableCycle for DropdownStory {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<gpui::FocusHandle>
    where
        Self: Sized,
    {
        vec![
            self.country_dropdown.focus_handle(cx),
            self.fruit_dropdown.focus_handle(cx),
            self.simple_dropdown1.focus_handle(cx),
            self.simple_dropdown2.focus_handle(cx),
            self.simple_dropdown3.focus_handle(cx),
        ]
    }
}

impl Render for DropdownStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_key_tab))
            .on_action(cx.listener(Self::on_key_shift_tab))
            .size_full()
            .gap_4()
            .child(
                Checkbox::new("disable-dropdowns")
                    .label("Disabled")
                    .checked(self.disabled)
                    .on_click(cx.listener(|this, checked, window, cx| {
                        this.toggle_disabled(*checked, window, cx);
                    })),
            )
            .child(
                section("Dropdown").max_w_128().child(
                    Dropdown::new(&self.country_dropdown)
                        .cleanable()
                        .disabled(self.disabled),
                ),
            )
            .child(
                section("Searchable").max_w_128().child(
                    Dropdown::new(&self.fruit_dropdown)
                        .disabled(self.disabled)
                        .icon(IconName::Search)
                        .w(px(320.))
                        .menu_width(px(400.)),
                ),
            )
            .child(
                section("Disabled")
                    .max_w_128()
                    .child(Dropdown::new(&self.disabled_dropdown).disabled(true)),
            )
            .child(
                section("With preview label").max_w_128().child(
                    Dropdown::new(&self.simple_dropdown1)
                        .disabled(self.disabled)
                        .small()
                        .placeholder("UI")
                        .title_prefix("UI: "),
                ),
            )
            .child(
                section("Searchable Dropdown").max_w_128().child(
                    Dropdown::new(&self.simple_dropdown2)
                        .disabled(self.disabled)
                        .small()
                        .placeholder("Language")
                        .title_prefix("Language: "),
                ),
            )
            .child(
                section("Empty Items").max_w_128().child(
                    Dropdown::new(&self.simple_dropdown3)
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
                section("Appearance false with TextInput")
                    .max_w_128()
                    .child(
                        h_flex()
                            .border_1()
                            .border_color(cx.theme().input)
                            .rounded_lg()
                            .text_color(cx.theme().secondary_foreground)
                            .w_full()
                            .gap_1()
                            .child(
                                div().w(px(140.)).child(
                                    Dropdown::new(&self.appearance_dropdown)
                                        .appearance(false)
                                        .py_2()
                                        .pl_3(),
                                ),
                            )
                            .child(Divider::vertical())
                            .child(
                                div().flex_1().child(
                                    TextInput::new(&self.input_state)
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
                            self.country_dropdown.read(cx).selected_value()
                        ))
                        .child(format!(
                            "fruit: {:?}",
                            self.fruit_dropdown.read(cx).selected_value()
                        ))
                        .child(format!(
                            "UI: {:?}",
                            self.simple_dropdown1.read(cx).selected_value()
                        ))
                        .child(format!(
                            "Language: {:?}",
                            self.simple_dropdown2.read(cx).selected_value()
                        ))
                        .child("This is other text."),
                ),
            )
    }
}
