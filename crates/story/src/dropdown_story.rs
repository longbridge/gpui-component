use gpui::{
    px, App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render,
    SharedString, Styled, Window,
};

use gpui_component::{
    checkbox::Checkbox,
    dropdown::{Dropdown, DropdownEvent, DropdownItem, DropdownState, SearchableVec},
    h_flex, v_flex, ActiveTheme, IconName, Sizable,
};

use crate::section;

pub fn init(_: &mut App) {}

struct Country {
    name: SharedString,
    code: SharedString,
}

impl Country {
    pub fn new(name: impl Into<SharedString>, code: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            code: code.into(),
        }
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
    country_dropdown: Entity<DropdownState<Vec<Country>>>,
    fruit_dropdown: Entity<DropdownState<SearchableVec<SharedString>>>,
    simple_dropdown1: Entity<DropdownState<Vec<SharedString>>>,
    simple_dropdown2: Entity<DropdownState<SearchableVec<SharedString>>>,
    simple_dropdown3: Entity<DropdownState<Vec<SharedString>>>,
    disabled_dropdown: Entity<DropdownState<Vec<SharedString>>>,
}

impl super::Story for DropdownStory {
    fn title() -> &'static str {
        "Dropdown"
    }

    fn description() -> &'static str {
        "Displays a list of options for the user to pick from—triggered by a button."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
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
        let countries = vec![
            Country::new("United States", "US"),
            Country::new("Canada", "CA"),
            Country::new("Mexico", "MX"),
            Country::new("Brazil", "BR"),
            Country::new("Argentina", "AR"),
            Country::new("Chile", "CL"),
            Country::new("China", "CN"),
            Country::new("Peru", "PE"),
            Country::new("Colombia", "CO"),
            Country::new("Venezuela", "VE"),
            Country::new("Ecuador", "EC"),
        ];

        let country_dropdown = cx.new(|cx| DropdownState::new(countries, Some(6), window, cx));

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
                        Some(0),
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
            }
        })
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        Self::new(window, cx)
    }

    fn on_dropdown_event(
        &mut self,
        _: &Entity<DropdownState<Vec<Country>>>,
        event: &DropdownEvent<Vec<Country>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            DropdownEvent::Confirm(value) => println!("Selected country: {:?}", value),
        }
    }

    fn toggle_disabled(&mut self, disabled: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.disabled = disabled;
        cx.notify();
    }
}

impl Render for DropdownStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
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
                        .width(px(320.))
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
