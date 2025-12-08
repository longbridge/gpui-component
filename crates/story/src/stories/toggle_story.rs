use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window,
};

use gpui_component::{
    IconName, Sizable, StyledExt,
    button::{Toggle, ToggleGroup, ToggleVariants},
    v_flex,
};

use crate::section;

pub struct ToggleStory {
    focus_handle: FocusHandle,
    single_toggle: usize,
    checked: Vec<bool>,
}

impl ToggleStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            single_toggle: 0,
            checked: vec![false; 20],
        })
    }
}

impl super::Story for ToggleStory {
    fn title() -> &'static str {
        "ToggleButton"
    }

    fn description() -> &'static str {
        ""
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ToggleStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ToggleStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_6()
            .child(
                section("Toggle")
                    .child(
                        Toggle::new("item1")
                            .label("Single Toggle Item 1")
                            .large()
                            .checked(self.single_toggle == 1)
                            .on_click(cx.listener(|view, checked, _, cx| {
                                if *checked {
                                    view.single_toggle = 1;
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        Toggle::new("item2")
                            .label("Single Toggle Item 2")
                            .large()
                            .checked(self.single_toggle == 2)
                            .on_click(cx.listener(|view, checked, _, cx| {
                                if *checked {
                                    view.single_toggle = 2;
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        Toggle::new("item3")
                            .icon(IconName::Eye)
                            .large()
                            .checked(self.single_toggle == 3)
                            .on_click(cx.listener(|view, checked, _, cx| {
                                if *checked {
                                    view.single_toggle = 3;
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("Toggle Group with Ghost Style")
                    .v_flex()
                    .gap_4()
                    .child(
                        ToggleGroup::new("toggle-button-group1")
                            .child(Toggle::new(0).icon(IconName::Bell).checked(self.checked[0]))
                            .child(Toggle::new(1).icon(IconName::Bot).checked(self.checked[1]))
                            .child(
                                Toggle::new(2)
                                    .icon(IconName::Inbox)
                                    .checked(self.checked[2]),
                            )
                            .child(
                                Toggle::new(3)
                                    .icon(IconName::Check)
                                    .checked(self.checked[3]),
                            )
                            .child(Toggle::new(4).label("Other").checked(self.checked[4]))
                            .on_click(cx.listener(|view, checkeds: &Vec<bool>, _, cx| {
                                view.checked[0] = checkeds[0];
                                view.checked[1] = checkeds[1];
                                view.checked[2] = checkeds[2];
                                view.checked[3] = checkeds[3];
                                view.checked[4] = checkeds[4];
                                cx.notify();
                            })),
                    )
                    .child(
                        ToggleGroup::new("toggle-button-group1-sm")
                            .small()
                            .child(Toggle::new(0).icon(IconName::Bell).checked(self.checked[0]))
                            .child(Toggle::new(1).icon(IconName::Bot).checked(self.checked[1]))
                            .child(
                                Toggle::new(2)
                                    .icon(IconName::Inbox)
                                    .checked(self.checked[2]),
                            )
                            .child(
                                Toggle::new(3)
                                    .icon(IconName::Check)
                                    .checked(self.checked[3]),
                            )
                            .child(Toggle::new(4).label("Other").checked(self.checked[4]))
                            .on_click(cx.listener(|view, checkeds: &Vec<bool>, _, cx| {
                                view.checked[0] = checkeds[0];
                                view.checked[1] = checkeds[1];
                                view.checked[2] = checkeds[2];
                                view.checked[3] = checkeds[3];
                                view.checked[4] = checkeds[4];
                                cx.notify();
                            })),
                    )
                    .child(
                        ToggleGroup::new("toggle-button-group1-xs")
                            .xsmall()
                            .child(Toggle::new(0).icon(IconName::Bell).checked(self.checked[0]))
                            .child(Toggle::new(1).icon(IconName::Bot).checked(self.checked[1]))
                            .child(
                                Toggle::new(2)
                                    .icon(IconName::Inbox)
                                    .checked(self.checked[2]),
                            )
                            .child(
                                Toggle::new(3)
                                    .icon(IconName::Check)
                                    .checked(self.checked[3]),
                            )
                            .child(Toggle::new(4).label("Other").checked(self.checked[4]))
                            .on_click(cx.listener(|view, checkeds: &Vec<bool>, _, cx| {
                                view.checked[0] = checkeds[0];
                                view.checked[1] = checkeds[1];
                                view.checked[2] = checkeds[2];
                                view.checked[3] = checkeds[3];
                                view.checked[4] = checkeds[4];
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("Toggle Group with Outline Style")
                    .v_flex()
                    .gap_4()
                    .child(
                        ToggleGroup::new("toggle-button-group2")
                            .outline()
                            .child(Toggle::new(0).icon(IconName::Bell).checked(self.checked[0]))
                            .child(Toggle::new(1).icon(IconName::Bot).checked(self.checked[1]))
                            .child(
                                Toggle::new(2)
                                    .icon(IconName::Inbox)
                                    .checked(self.checked[2]),
                            )
                            .child(
                                Toggle::new(3)
                                    .icon(IconName::Check)
                                    .checked(self.checked[3]),
                            )
                            .child(Toggle::new(4).label("Other").checked(self.checked[4]))
                            .on_click(cx.listener(|view, checkeds: &Vec<bool>, _, cx| {
                                view.checked[0] = checkeds[0];
                                view.checked[1] = checkeds[1];
                                view.checked[2] = checkeds[2];
                                view.checked[3] = checkeds[3];
                                view.checked[4] = checkeds[4];
                                cx.notify();
                            })),
                    )
                    .child(
                        ToggleGroup::new("toggle-button-group2-sm")
                            .outline()
                            .small()
                            .child(Toggle::new(0).icon(IconName::Bell).checked(self.checked[0]))
                            .child(Toggle::new(1).icon(IconName::Bot).checked(self.checked[1]))
                            .child(
                                Toggle::new(2)
                                    .icon(IconName::Inbox)
                                    .checked(self.checked[2]),
                            )
                            .child(
                                Toggle::new(3)
                                    .icon(IconName::Check)
                                    .checked(self.checked[3]),
                            )
                            .child(Toggle::new(4).label("Other").checked(self.checked[4]))
                            .on_click(cx.listener(|view, checkeds: &Vec<bool>, _, cx| {
                                view.checked[0] = checkeds[0];
                                view.checked[1] = checkeds[1];
                                view.checked[2] = checkeds[2];
                                view.checked[3] = checkeds[3];
                                view.checked[4] = checkeds[4];
                                cx.notify();
                            })),
                    )
                    .child(
                        ToggleGroup::new("toggle-button-group2-xs")
                            .outline()
                            .xsmall()
                            .child(Toggle::new(0).icon(IconName::Bell).checked(self.checked[0]))
                            .child(Toggle::new(1).icon(IconName::Bot).checked(self.checked[1]))
                            .child(
                                Toggle::new(2)
                                    .icon(IconName::Inbox)
                                    .checked(self.checked[2]),
                            )
                            .child(
                                Toggle::new(3)
                                    .icon(IconName::Check)
                                    .checked(self.checked[3]),
                            )
                            .child(Toggle::new(4).label("Other").checked(self.checked[4]))
                            .on_click(cx.listener(|view, checkeds: &Vec<bool>, _, cx| {
                                view.checked[0] = checkeds[0];
                                view.checked[1] = checkeds[1];
                                view.checked[2] = checkeds[2];
                                view.checked[3] = checkeds[3];
                                view.checked[4] = checkeds[4];
                                cx.notify();
                            })),
                    ),
            )
    }
}
