use gpui::{
    actions, App, AppContext as _, Context, Entity, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window,
};

use gpui_component::{
    button::{ToggleButton, ToggleButtonGroup},
    checkbox::Checkbox,
    h_flex, v_flex, IconName,
};

actions!(button_story, [Disabled, Loading, Selected, Compact]);

pub struct ToggleButtonStory {
    focus_handle: gpui::FocusHandle,
    checked: Vec<bool>,
    toggle_multiple: bool,
}

impl ToggleButtonStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            checked: vec![false; 20],
            toggle_multiple: false,
        })
    }
}

impl super::Story for ToggleButtonStory {
    fn title() -> &'static str {
        "ToggleButton"
    }

    fn description() -> &'static str {
        ""
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl Focusable for ToggleButtonStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ToggleButtonStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let toggle_multiple = self.toggle_multiple;
        let checked = self.checked.clone();

        v_flex()
            .gap_6()
            .child(
                h_flex().gap_3().child("State").child(
                    Checkbox::new("multiple")
                        .label("Multiple")
                        .checked(self.toggle_multiple)
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.toggle_multiple = !view.toggle_multiple;
                            cx.notify();
                        })),
                ),
            )
            .child(
                h_flex()
                    .gap_5()
                    .child(
                        ToggleButtonGroup::new("toggle-button-group")
                            .multiple(toggle_multiple)
                            .child(ToggleButton::label("A").checked(checked[0]))
                            .child(ToggleButton::label("B").checked(checked[1]))
                            .child(ToggleButton::label("C").checked(checked[2]))
                            .child(ToggleButton::label("D").checked(checked[3]))
                            .on_change(cx.listener(|view, checkeds: &Vec<bool>, _, cx| {
                                view.checked[0] = checkeds[0];
                                view.checked[1] = checkeds[1];
                                view.checked[2] = checkeds[2];
                                view.checked[3] = checkeds[3];
                                cx.notify();
                            })),
                    )
                    .child(
                        ToggleButtonGroup::new("toggle-button-group")
                            .multiple(toggle_multiple)
                            .child(ToggleButton::icon(IconName::Bell).checked(self.checked[0]))
                            .child(ToggleButton::icon(IconName::Bot).checked(self.checked[1]))
                            .child(ToggleButton::icon(IconName::Inbox).checked(self.checked[2]))
                            .child(ToggleButton::icon(IconName::Check).checked(self.checked[3]))
                            .on_change(cx.listener(|view, checkeds: &Vec<bool>, _, cx| {
                                view.checked[0] = checkeds[0];
                                view.checked[1] = checkeds[1];
                                view.checked[2] = checkeds[2];
                                view.checked[3] = checkeds[3];
                                cx.notify();
                            })),
                    ),
            )
    }
}
