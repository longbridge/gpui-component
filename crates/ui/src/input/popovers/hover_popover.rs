use std::{ops::Range, rc::Rc};

use gpui::{
    canvas, deferred, div, prelude::FluentBuilder as _, px, App, AppContext as _, Bounds, Context,
    Entity, InteractiveElement, IntoElement, ParentElement as _, Pixels, Point, Render, Styled,
    Window,
};

use crate::input::{
    popovers::{popover, render_markdown},
    InputState,
};

pub struct HoverPopover {
    editor: Entity<InputState>,
    /// The range byte of the hover trigger.
    pub(crate) range: Range<usize>,
    pub(crate) hover: Rc<lsp_types::Hover>,
    bounds: Bounds<Pixels>,
    open: bool,
}

impl HoverPopover {
    pub fn new(
        editor: Entity<InputState>,
        range: Range<usize>,
        hover: &lsp_types::Hover,
        cx: &mut App,
    ) -> Entity<Self> {
        let hover = Rc::new(hover.clone());

        cx.new(|_| Self {
            editor,
            range,
            hover,
            bounds: Bounds::default(),
            open: true,
        })
    }

    fn origin(&self, cx: &App) -> Option<Point<Pixels>> {
        let editor = self.editor.read(cx);
        let Some(last_layout) = editor.last_layout.as_ref() else {
            return None;
        };

        let line_number_width = last_layout.line_number_width;
        let (_, _, start_pos) = editor.line_and_position_for_offset(self.range.start);

        start_pos.map(|pos| pos + Point::new(line_number_width, px(0.)))
    }

    pub(crate) fn is_same(&self, offset: usize) -> bool {
        self.range.contains(&offset)
    }

    #[allow(unused)]
    pub(crate) fn show(&mut self, cx: &mut Context<Self>) {
        if self.open {
            return;
        }

        self.open = true;
        cx.notify();
    }

    #[allow(unused)]
    pub(crate) fn hide(&mut self, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }

        self.open = false;
        cx.notify();
    }
}

impl Render for HoverPopover {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().cursor_text().into_any_element();
        }

        let view = cx.entity();

        let contents = match self.hover.contents.clone() {
            lsp_types::HoverContents::Scalar(scalar) => match scalar {
                lsp_types::MarkedString::String(s) => s,
                lsp_types::MarkedString::LanguageString(ls) => ls.value,
            },
            lsp_types::HoverContents::Array(arr) => arr
                .into_iter()
                .map(|item| match item {
                    lsp_types::MarkedString::String(s) => s,
                    lsp_types::MarkedString::LanguageString(ls) => ls.value,
                })
                .collect::<Vec<_>>()
                .join("\n\n"),
            lsp_types::HoverContents::Markup(markup) => markup.value,
        };

        let Some(pos) = self.origin(cx) else {
            return div().cursor_text().into_any_element();
        };

        let scroll_origin = self.editor.read(cx).scroll_handle.offset();

        // +2px to move down to cover the text to overlap the block mouse move events
        let y = pos.y - self.bounds.size.height + scroll_origin.y + px(1.);
        let x = pos.x + scroll_origin.x - px(2.);
        let max_width = px(500.).min(window.bounds().size.width - x).max(px(200.));

        deferred(
            popover("hover-popover", cx)
                .cursor_text()
                .absolute()
                .left(x)
                .top(y)
                .p_1p5()
                .when(self.bounds.is_empty(), |s| s.invisible())
                .max_w(max_width)
                .child(render_markdown("message", contents, window, cx))
                .child(
                    canvas(
                        move |bounds, _, cx| {
                            view.update(cx, |r, cx| {
                                if r.bounds != bounds {
                                    r.bounds = bounds;
                                    cx.notify();
                                }
                            })
                        },
                        |_, _, _, _| {},
                    )
                    .top_0()
                    .left_0()
                    .absolute()
                    .size_full(),
                )
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.open = false;
                    cx.notify();
                })),
        )
        .into_any_element()
    }
}
