mod code_action_menu;
mod completion_menu;
mod diagnostic_popover;
mod hover_popover;

pub(crate) use code_action_menu::*;
pub(crate) use completion_menu::*;
pub(crate) use diagnostic_popover::*;
pub(crate) use hover_popover::*;

use gpui::{
    App, Div, ElementId, InteractiveElement as _, SharedString, Stateful, Styled as _, Window, div,
    px, rems,
};

use crate::{
    ActiveTheme, ThemeStyled as _,
    text::{TextView, text_view_style},
};

pub(super) fn render_markdown(
    id: impl Into<ElementId>,
    markdown: impl Into<SharedString>,
    _: &mut Window,
    cx: &mut App,
) -> TextView {
    let style = text_view_style(cx.theme());
    let code_block = style
        .code_block
        .clone()
        .bg(cx.theme().transparent)
        .p_0()
        .text_size(px(11.));
    TextView::markdown(id, markdown)
        .style(
            style
                .paragraph_gap(rems(0.5))
                .heading_font_size(|level, rem_size| match level {
                    1..=3 => rem_size * 1,
                    4 => rem_size * 0.9,
                    _ => rem_size * 0.8,
                })
                .code_block(code_block),
        )
        .selectable(true)
}

pub(super) fn editor_popover(id: impl Into<ElementId>, cx: &App) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .occlude()
        .popover_style(cx)
        .shadow_md()
        .text_xs()
        .p_1()
}
