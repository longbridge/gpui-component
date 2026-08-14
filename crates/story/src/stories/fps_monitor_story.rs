use std::time::Duration;

use gpui::{
    Anchor, App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, div, px,
};
use gpui_component::{ActiveTheme as _, v_flex};
use gpui_fps::{FpsMonitor, FpsOverlay};

use crate::section;

pub struct FpsMonitorStory {
    focus_handle: gpui::FocusHandle,
    embedded: Entity<FpsMonitor>,
    overlaid: Entity<FpsMonitor>,
}

impl super::Story for FpsMonitorStory {
    fn title() -> &'static str {
        "FpsMonitor"
    }

    fn description() -> &'static str {
        "Displays frames per second, frame time and process resource usage in realtime."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl FpsMonitorStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            embedded: cx.new(|cx| FpsMonitor::new(window, cx)),
            overlaid: cx.new(|cx| {
                FpsMonitor::new(window, cx)
                    .capacity(180)
                    // A 144Hz budget, so the baseline and the trace colors are
                    // judged against a high refresh rate display.
                    .frame_budget(Duration::from_micros(6_944))
            }),
        }
    }
}

impl Focusable for FpsMonitorStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FpsMonitorStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .child(
                section("Default")
                    .description(
                        "A HUD reading GPUI's frame trace. Click it to collapse to just the FPS.",
                    )
                    .child(self.embedded.clone()),
            )
            .child(
                section("Overlay")
                    .description(
                        "Pinned to an anchor of a relative parent, the way a game overlays its \
                         frame counter. This one is judged against a 144Hz frame budget.",
                    )
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(160.))
                            .rounded(cx.theme().radius)
                            .bg(cx.theme().secondary)
                            .child(FpsOverlay::new(&self.overlaid).anchor(Anchor::BottomRight)),
                    ),
            )
    }
}
