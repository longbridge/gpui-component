mod code_action_menu;
mod completion_menu;
mod diagnostic_popover;
mod hover_popover;

pub(crate) use code_action_menu::*;
pub(crate) use completion_menu::*;
pub(crate) use diagnostic_popover::*;
pub(crate) use hover_popover::*;

use gpui::{
    div, rems, App, Div, ElementId, Entity, InteractiveElement as _, IntoElement, SharedString,
    Stateful, Styled as _, Window,
};

use crate::{
    text::{TextView, TextViewStyle},
    ActiveTheme as _,
};

pub(crate) enum ContextMenu {
    Completion(Entity<CompletionMenu>),
    CodeAction(Entity<CodeActionMenu>),
}

impl ContextMenu {
    pub(crate) fn is_open(&self, cx: &App) -> bool {
        match self {
            ContextMenu::Completion(menu) => menu.read(cx).is_open(),
            ContextMenu::CodeAction(menu) => menu.read(cx).is_open(),
        }
    }

    pub(crate) fn render(&self) -> impl IntoElement {
        match self {
            ContextMenu::Completion(menu) => menu.clone().into_any_element(),
            ContextMenu::CodeAction(menu) => menu.clone().into_any_element(),
        }
    }
}

pub(super) fn render_markdown(
    id: impl Into<ElementId>,
    markdown: impl Into<SharedString>,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    TextView::markdown(id, markdown, window, cx)
        .style(
            TextViewStyle::default()
                .paragraph_gap(rems(0.5))
                .heading_font_size(|level, rem_size| match level {
                    1..=3 => rem_size * 1,
                    4 => rem_size * 0.9,
                    _ => rem_size * 0.8,
                }),
        )
        .selectable()
}

pub(super) fn popover(id: impl Into<ElementId>, cx: &App) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .occlude()
        .p_1()
        .text_xs()
        .text_color(cx.theme().popover_foreground)
        .bg(cx.theme().popover)
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius)
        .shadow_md()
}
