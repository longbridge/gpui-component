use gpui::{
    actions, px, AppContext, InteractiveElement, IntoElement, KeyBinding, ParentElement, Render,
    SharedString, Styled, View, ViewContext, VisualContext, WindowContext,
};

use ui::{
    checkbox::Checkbox,
    dropdown::{Dropdown, DropdownEvent, DropdownItem, SearchableVec},
    h_flex,
    theme::ActiveTheme,
    v_flex, FocusableCycle, IconName, Sizable,
};

actions!(dropdown_story, [Tab, TabPrev]);

const CONTEXT: &str = "DropdownStory";
pub fn init(cx: &mut AppContext) {
    cx.bind_keys([
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
    ])
}

#[derive(Clone)]
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

    fn value(&self) -> &Self::Value {
        &self.code
    }
}

pub struct DropdownStory {
    disabled: bool,
    country_dropdown: View<Dropdown<Vec<Country>>>,
    country_dropdown_ftl_on: bool,
    fruit_dropdown: View<Dropdown<SearchableVec<SharedString>>>,
    simple_dropdown1: View<Dropdown<Vec<SharedString>>>,
    simple_dropdown2: View<Dropdown<SearchableVec<SharedString>>>,
    simple_dropdown3: View<Dropdown<Vec<SharedString>>>,
    disabled_dropdown: View<Dropdown<Vec<SharedString>>>,
    changing_dropdown: View<Dropdown<Vec<SharedString>>>,
}

impl super::Story for DropdownStory {
    fn title() -> &'static str {
        "Dropdown"
    }

    fn description() -> &'static str {
        "Displays a list of options for the user to pick from—triggered by a button."
    }

    fn new_view(cx: &mut WindowContext) -> View<impl gpui::FocusableView> {
        Self::view(cx)
    }
}

impl gpui::FocusableView for DropdownStory {
    fn focus_handle(&self, cx: &gpui::AppContext) -> gpui::FocusHandle {
        self.fruit_dropdown.focus_handle(cx)
    }
}

impl DropdownStory {
    fn new(cx: &mut WindowContext) -> View<Self> {
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
        let country_dropdown = cx.new_view(|cx| {
            Dropdown::new("dropdown-country", cx)
                .delegate(countries, cx)
                .index(Some(6), cx)
                .cleanable()
        });

        let fruits = SearchableVec::new(vec![
            "Apple".into(),
            "Orange".into(),
            "Banana".into(),
            "Grape".into(),
            "Pineapple".into(),
            "Watermelon & This is a longlonglonglonglonglonglonglonglong title".into(),
            "Avocado".into(),
        ]);

        let fruit_dropdown = cx.new_view(|cx| {
            Dropdown::new("dropdown-fruits", cx)
                .delegate(fruits, cx)
                .icon(IconName::Search)
                .width(px(200.))
                .menu_width(px(320.))
        });

        cx.new_view(|cx| {
            cx.subscribe(&country_dropdown, Self::on_dropdown_event)
                .detach();

            Self {
                disabled: false,
                country_dropdown,
                fruit_dropdown,
                country_dropdown_ftl_on: false,
                simple_dropdown1: cx.new_view(|cx| {
                    Dropdown::new("string-list1", cx)
                        .delegate(
                            vec!["QPUI".into(), "Iced".into(), "QT".into(), "Cocoa".into()],
                            cx,
                        )
                        .index(Some(0), cx)
                        .small()
                        .placeholder("UI")
                        .title_prefix("UI: ")
                }),
                simple_dropdown2: cx.new_view(|cx| {
                    Dropdown::new("string-list2", cx)
                        .delegate(
                            SearchableVec::new(vec![
                                "Rust".into(),
                                "Go".into(),
                                "C++".into(),
                                "JavaScript".into(),
                            ]),
                            cx,
                        )
                        .index(None, cx)
                        .small()
                        .placeholder("Language")
                        .title_prefix("Language: ")
                }),
                simple_dropdown3: cx.new_view(|cx| {
                    Dropdown::new("string-list3", cx).small().empty(|cx| {
                        h_flex()
                            .h_24()
                            .justify_center()
                            .text_color(cx.theme().muted_foreground)
                            .child("No Data")
                    })
                }),
                disabled_dropdown: cx.new_view(|cx| {
                    Dropdown::new("disabled-dropdown", cx)
                        .delegate(Vec::<SharedString>::new(), cx)
                        .small()
                        .disabled(true)
                }),
                changing_dropdown: cx.new_view(|cx| Dropdown::new("string-list4", cx).small()),
            }
        })
    }

    pub fn view(cx: &mut WindowContext) -> View<Self> {
        Self::new(cx)
    }

    fn on_dropdown_event(
        &mut self,
        _: View<Dropdown<Vec<Country>>>,
        event: &DropdownEvent<Vec<Country>>,
        _cx: &mut ViewContext<Self>,
    ) {
        match event {
            DropdownEvent::Confirm(value) => println!("Selected country: {:?}", value),
        }
    }

    fn on_key_tab(&mut self, _: &Tab, cx: &mut ViewContext<Self>) {
        self.cycle_focus(true, cx);
        cx.notify();
    }

    fn on_key_shift_tab(&mut self, _: &TabPrev, cx: &mut ViewContext<Self>) {
        self.cycle_focus(false, cx);
        cx.notify();
    }

    fn toggle_disabled(&mut self, disabled: bool, cx: &mut ViewContext<Self>) {
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
    fn cycle_focus_handles(&self, cx: &mut ViewContext<Self>) -> Vec<gpui::FocusHandle>
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
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
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
                    .on_click(cx.listener(|this, checked, cx| {
                        this.toggle_disabled(*checked, cx);
                    })),
            )
            .child(
                Checkbox::new("mock-ftl-countries")
                    .label("Country ftl")
                    .checked(self.country_dropdown_ftl_on)
                    .on_click(cx.listener(|view, _, cx| {
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
                        // sorry, this is ai generated
                        let countries_mock_ftl = vec![
                            Country::new("美国", "US"),
                            Country::new("加拿大", "CA"),
                            Country::new("墨西哥", "MX"),
                            Country::new("巴西", "BR"),
                            Country::new("阿根廷", "AR"),
                            Country::new("智利", "CL"),
                            Country::new("中国", "CN"),
                            Country::new("秘鲁", "PE"),
                            Country::new("哥伦比亚", "CO"),
                            Country::new("委内瑞拉", "VE"),
                            Country::new("厄瓜多尔", "EC"),
                        ];
                        view.country_dropdown_ftl_on = !view.country_dropdown_ftl_on;
                        view.country_dropdown.update(cx, |this, cx| {
                            match view.country_dropdown_ftl_on {
                                true => this.delegate(countries_mock_ftl, cx),
                                false => this.delegate(countries, cx),
                            }
                        });
                        cx.notify();
                    })),
            )
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_4()
                    .child(self.country_dropdown.clone())
                    .child(self.fruit_dropdown.clone()),
            )
            .child(
                v_flex()
                    .w_full()
                    .items_center()
                    .p_10()
                    .rounded_lg()
                    .bg(cx.theme().card)
                    .border_1()
                    .border_color(cx.theme().border)
                    .gap_4()
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
            )
            .child(
                h_flex()
                    .items_center()
                    .w_128()
                    .gap_2()
                    .child(self.simple_dropdown1.clone())
                    .child(self.simple_dropdown2.clone())
                    .child(self.simple_dropdown3.clone())
                    .child(self.changing_dropdown.clone())
                    .child(self.disabled_dropdown.clone()),
            )
    }
}
