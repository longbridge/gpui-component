use gpui::{
    actions, px, App, AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement,
    KeyBinding, ParentElement, Render, SharedString, Styled, Window,
};

use gpui_component::{
    checkbox::Checkbox,
    dropdown::{Dropdown, DropdownEvent, DropdownItem, SearchableVec},
    h_flex, v_flex, ActiveTheme, FocusableCycle, IconName, Sizable,
};

use crate::section;

actions!(dropdown_story, [Tab, TabPrev]);

const CONTEXT: &str = "DropdownStory";
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
    ])
}

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
    country_dropdown: Entity<Dropdown<Vec<Country>>>,
    fruit_dropdown: Entity<Dropdown<SearchableVec<SharedString>>>,
    simple_dropdown1: Entity<Dropdown<Vec<SharedString>>>,
    simple_dropdown2: Entity<Dropdown<SearchableVec<SharedString>>>,
    simple_dropdown3: Entity<Dropdown<Vec<SharedString>>>,
    disabled_dropdown: Entity<Dropdown<Vec<SharedString>>>,
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

        let country_dropdown = cx.new(|cx| {
            Dropdown::new("dropdown-country", countries, Some(6), window, cx).cleanable()
        });

        let fruits = SearchableVec::new(vec![
            "Apple".into(),
            "Orange".into(),
            "Banana".into(),
            "Grape".into(),
            "Pineapple".into(),
            "Watermelon & This is a long long long long long long long long long title".into(),
            "Avocado".into(),
        ]);
        let fruit_dropdown = cx.new(|cx| {
            Dropdown::new("dropdown-fruits", fruits, None, window, cx)
                .icon(IconName::Search)
                .width(px(320.))
                .menu_width(px(400.))
        });

        cx.new(|cx| {
            cx.subscribe_in(&country_dropdown, window, Self::on_dropdown_event)
                .detach();

            Self {
                disabled: false,
                country_dropdown,
                fruit_dropdown,
                simple_dropdown1: cx.new(|cx| {
                    Dropdown::new(
                        "string-list1",
                        vec!["QPUI".into(), "Iced".into(), "QT".into(), "Cocoa".into()],
                        Some(0),
                        window,
                        cx,
                    )
                    .small()
                    .placeholder("UI")
                    .title_prefix("UI: ")
                }),
                simple_dropdown2: cx.new(|cx| {
                    Dropdown::new(
                        "string-list2",
                        SearchableVec::new(vec![
                            "Rust".into(),
                            "Go".into(),
                            "C++".into(),
                            "JavaScript".into(),
                        ]),
                        None,
                        window,
                        cx,
                    )
                    .small()
                    .placeholder("Language")
                    .title_prefix("Language: ")
                }),
                simple_dropdown3: cx.new(|cx| {
                    Dropdown::new("string-list3", Vec::<SharedString>::new(), None, window, cx)
                        .small()
                        .empty(|_, cx| {
                            h_flex()
                                .h_24()
                                .justify_center()
                                .text_color(cx.theme().muted_foreground)
                                .child("No Data")
                        })
                }),
                disabled_dropdown: cx.new(|cx| {
                    Dropdown::new(
                        "disabled-dropdown",
                        Vec::<SharedString>::new(),
                        None,
                        window,
                        cx,
                    )
                    .small()
                    .disabled(true)
                }),
            }
        })
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        Self::new(window, cx)
    }

    fn on_dropdown_event(
        &mut self,
        _: &Entity<Dropdown<Vec<Country>>>,
        event: &DropdownEvent<Vec<Country>>,
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
        self.country_dropdown
            .update(cx, |this, _| this.set_disabled(disabled));
        self.fruit_dropdown
            .update(cx, |this, _| this.set_disabled(disabled));
        self.simple_dropdown1
            .update(cx, |this, _| this.set_disabled(disabled));
        self.simple_dropdown2
            .update(cx, |this, _| this.set_disabled(disabled));
        self.simple_dropdown3
            .update(cx, |this, _| this.set_disabled(disabled));
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
                section("Dropdown")
                    .max_w_128()
                    .child(self.country_dropdown.clone()),
            )
            .child(
                section("Searchable")
                    .max_w_128()
                    .child(self.fruit_dropdown.clone()),
            )
            .child(
                section("Disabled")
                    .max_w_128()
                    .child(self.disabled_dropdown.clone()),
            )
            .child(
                section("With preview label")
                    .max_w_128()
                    .child(self.simple_dropdown1.clone()),
            )
            .child(
                section("Searchable Dropdown")
                    .max_w_128()
                    .child(self.simple_dropdown2.clone()),
            )
            .child(
                section("Empty Items")
                    .max_w_128()
                    .child(self.simple_dropdown3.clone()),
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
