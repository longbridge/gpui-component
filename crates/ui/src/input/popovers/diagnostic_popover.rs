use std::rc::Rc;

use gpui::{
    App, AppContext as _, Bounds, Context, Empty, Entity, IntoElement, Pixels, Point, Render,
    Styled, Window, prelude::FluentBuilder as _, px,
};

use crate::{
    highlighter::DiagnosticEntry,
    input::{
        InputState,
        popovers::{Popover, render_markdown},
    },
};

pub struct DiagnosticPopover {
    state: Entity<InputState>,
    pub(crate) diagnostic: Rc<DiagnosticEntry>,
    bounds: Bounds<Pixels>,
    open: bool,
}

impl DiagnosticPopover {
    pub fn new(
        diagnostic: &DiagnosticEntry,
        state: Entity<InputState>,
        cx: &mut App,
    ) -> Entity<Self> {
        let diagnostic = Rc::new(diagnostic.clone());

        cx.new(|_| Self {
            diagnostic,
            state,
            bounds: Bounds::default(),
            open: true,
        })
    }

    pub(crate) fn show(&mut self, cx: &mut Context<Self>) {
        self.open = true;
        cx.notify();
    }

    pub(crate) fn hide(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        cx.notify();
    }

    pub(crate) fn check_to_hide(&mut self, mouse_position: Point<Pixels>, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }

        let padding = px(5.);
        let bounds = Bounds {
            origin: self.bounds.origin.map(|v| v - padding),
            size: self.bounds.size.map(|v| v + padding * 2.),
        };

        if !bounds.contains(&mouse_position) {
            self.hide(cx);
        }
    }
}

impl Render for DiagnosticPopover {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return Empty.into_any_element();
        }

        let message = self.diagnostic.message.clone();

        let (border, bg, fg) = (
            self.diagnostic.severity.border(cx),
            self.diagnostic.severity.bg(cx),
            self.diagnostic.severity.fg(cx),
        );

        Popover::new(
            "diagnostic-popover",
            self.state.clone(),
            self.diagnostic.range.clone(),
            move |window, cx| render_markdown("message", message.clone(), window, cx),
        )
        .when(!self.open, |this| this.invisible())
        .px_1()
        .py_0p5()
        .bg(bg)
        .text_color(fg)
        .border_1()
        .border_color(border)
        .into_any_element()
    }
}
