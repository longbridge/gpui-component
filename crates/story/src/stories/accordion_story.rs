use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window, prelude::FluentBuilder as _,
};
use gpui_component::{
    IconName, Selectable, Sizable, Size,
    accordion::Accordion,
    button::{Button, ButtonGroup},
    checkbox::Checkbox,
    h_flex,
    switch::Switch,
    v_flex,
};

use crate::section;

pub struct AccordionStory {
    open_ixs: Vec<usize>,
    size: Size,
    bordered: bool,
    disabled: bool,
    multiple: bool,
    show_icon: bool,
    focus_handle: FocusHandle,
}

impl super::Story for AccordionStory {
    fn title() -> &'static str {
        "Accordion"
    }

    fn description() -> &'static str {
        "The accordion uses collapse internally to make it collapsible."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl AccordionStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            bordered: false,
            open_ixs: vec![0, 1, 2],
            size: Size::default(),
            disabled: false,
            multiple: true,
            show_icon: false,
            focus_handle: cx.focus_handle(),
        }
    }

    fn toggle_accordion(&mut self, open_ixs: Vec<usize>, _: &mut Window, cx: &mut Context<Self>) {
        self.open_ixs = open_ixs;
        cx.notify();
    }

    fn set_size(&mut self, size: Size, _: &mut Window, cx: &mut Context<Self>) {
        self.size = size;
        cx.notify();
    }
}

impl Focusable for AccordionStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AccordionStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_5()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .flex_wrap()
                    .child(
                        ButtonGroup::new("toggle-size")
                            .outline()
                            .compact()
                            .child(
                                Button::new("xsmall")
                                    .label("XSmall")
                                    .selected(self.size == Size::XSmall),
                            )
                            .child(
                                Button::new("small")
                                    .label("Small")
                                    .selected(self.size == Size::Small),
                            )
                            .child(
                                Button::new("medium")
                                    .label("Medium")
                                    .selected(self.size == Size::Medium),
                            )
                            .child(
                                Button::new("large")
                                    .label("Large")
                                    .selected(self.size == Size::Large),
                            )
                            .on_click(cx.listener(|this, selecteds: &Vec<usize>, window, cx| {
                                let size = match selecteds[0] {
                                    0 => Size::XSmall,
                                    1 => Size::Small,
                                    2 => Size::Medium,
                                    3 => Size::Large,
                                    _ => unreachable!(),
                                };
                                this.set_size(size, window, cx);
                            })),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Checkbox::new("multiple")
                                    .label("Multiple")
                                    .checked(self.multiple)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.multiple = *checked;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("show_icon")
                                    .label("Icon")
                                    .checked(self.show_icon)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.show_icon = *checked;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("disabled")
                                    .label("Disabled")
                                    .checked(self.disabled)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.disabled = *checked;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("bordered")
                                    .label("Bordered")
                                    .checked(self.bordered)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.bordered = *checked;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                section("Normal").max_w_md().child(
                    Accordion::new("test")
                        .bordered(self.bordered)
                        .with_size(self.size)
                        .disabled(self.disabled)
                        .multiple(self.multiple)
                        .item(|this| {
                            this.open(self.open_ixs.contains(&0))
                                .when(self.show_icon, |this| this.icon(IconName::Info))
                                .title("Is it accessible?")
                                .child("Yes. It adheres to the WAI-ARIA design pattern.")
                        })
                        .item(|this| {
                            this.open(self.open_ixs.contains(&1))
                            .when(self.show_icon, |this| this.icon(IconName::Inbox))
                            .title("Is it styled with complex elements?")
                            .child(
                                v_flex()
                                    .gap_4()
                                    .child(
                                        "We can put any view here, like a v_flex with a text view",
                                    )
                                    .child(
                                        h_flex()
                                            .gap_4()
                                            .child(Switch::new("switch1").label("Switch"))
                                            .child(
                                                Checkbox::new("checkbox1").label("Or a Checkbox"),
                                            ),
                                    ),
                            )
                        })
                        .item(|this| {
                            this.open(self.open_ixs.contains(&2))
                                .when(self.show_icon, |this| this.icon(IconName::Moon))
                                .title("This is third accordion")
                                .child(
                                    "This is the third accordion content. \
                                It can be any view, like a text view or a button.",
                                )
                        })
                        .on_toggle_click(cx.listener(|this, open_ixs: &[usize], window, cx| {
                            this.toggle_accordion(open_ixs.to_vec(), window, cx);
                        })),
                ),
            )
    }
}
