use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    SharedString, Styled, Window, div,
};
use gpui_component::{
    ActiveTheme, Disableable, Sizable, TitleBar,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    v_flex,
};
use rust_i18n::t;

use crate::home_tab::HomePage;

pub struct WorkspaceFormWindowConfig {
    pub parent: Entity<HomePage>,
    pub workspace_id: Option<i64>,
    pub initial_name: String,
}

pub struct WorkspaceFormWindow {
    focus_handle: FocusHandle,
    parent: Entity<HomePage>,
    workspace_id: Option<i64>,
    title: SharedString,
    name_input: Entity<InputState>,
}

impl WorkspaceFormWindow {
    pub fn new(
        config: WorkspaceFormWindowConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title: SharedString = if config.workspace_id.is_some() {
            t!("Workspace.edit").to_string()
        } else {
            t!("Workspace.new").to_string()
        }
        .into();

        let name_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("Workspace.name_placeholder"))
                .clean_on_escape();
            if !config.initial_name.is_empty() {
                state.set_value(config.initial_name.clone(), window, cx);
            }
            state
        });
        name_input.update(cx, |state: &mut InputState, cx| {
            state.focus(window, cx);
        });

        Self {
            focus_handle: cx.focus_handle(),
            parent: config.parent,
            workspace_id: config.workspace_id,
            title,
            name_input,
        }
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.name_input.read(cx).text().to_string();
        if name.is_empty() {
            return;
        }

        let _ = self.parent.update(cx, |home, cx| {
            home.handle_save_workspace(self.workspace_id, name, cx);
        });
        window.remove_window();
    }
}

impl Focusable for WorkspaceFormWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for WorkspaceFormWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let can_save = !self.name_input.read(cx).text().to_string().is_empty();

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                TitleBar::new().child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .flex_1()
                        .text_sm()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(self.title.clone()),
                ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_3()
                    .p_6()
                    .child(Input::new(&self.name_input).w_full()),
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .px_6()
                    .py_4()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("cancel")
                            .small()
                            .label(t!("Common.cancel").to_string())
                            .on_click(|_, window, _cx| {
                                window.remove_window();
                            }),
                    )
                    .child(
                        Button::new("save-workspace")
                            .small()
                            .primary()
                            .disabled(!can_save)
                            .label(t!("Common.ok").to_string())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.save(window, cx);
                            })),
                    ),
            )
    }
}
