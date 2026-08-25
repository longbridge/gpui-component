use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window, px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    badge::Badge,
    marker::{Marker, MarkerContent, MarkerIcon, MarkerLoadingStyle, MarkerVariant},
    shimmer::{ShimmerStyle, ShimmerText},
    spinner::Spinner,
    v_flex,
};
use std::time::Duration;

use crate::{Story, section};

pub struct MarkerStory {
    focus_handle: FocusHandle,
}

impl MarkerStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Story for MarkerStory {
    fn title() -> &'static str {
        "Marker"
    }

    fn description() -> &'static str {
        "A compact row for conversation status, notifications, and separators."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for MarkerStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MarkerStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Status")
                    .description(
                        "Compose icons, spinners, and labels without a fixed status model.",
                    )
                    .w(px(640.))
                    .v_flex()
                    .gap_3()
                    .child(
                        Marker::new()
                            .text_color(cx.theme().green)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::CircleCheck)))
                            .content(MarkerContent::new().child("Online")),
                    )
                    .child(
                        Marker::new()
                            .icon(MarkerIcon::new().child(Spinner::new().xsmall()))
                            .content(MarkerContent::new().child("Alice is typing…")),
                    )
                    .child(
                        Marker::new()
                            .icon(
                                MarkerIcon::new()
                                    .child(Badge::new().count(3).child(Icon::new(IconName::Bell))),
                            )
                            .content(MarkerContent::new().child("Unread notifications")),
                    ),
            )
            .child(
                section("Loading styles")
                    .description("Choose a spinner or a sweeping, ChatGPT-style text shimmer.")
                    .w(px(640.))
                    .v_flex()
                    .gap_4()
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Spinner)
                            .content(MarkerContent::new().text("shadcn/ui · Loading messages…")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .content(MarkerContent::new().text("ChatGPT · Thinking")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
                            .content(MarkerContent::new().text("正在探索 4 个文件…")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(
                                ShimmerStyle::new()
                                    .duration(Duration::from_secs(3))
                                    .highlight_color(cx.theme().primary)
                                    .spread(0.45)
                                    .reverse(true),
                            )
                            .content(
                                MarkerContent::new().text("Custom color, width, and direction"),
                            ),
                    )
                    .child(
                        ShimmerText::new("Reusable shimmer without a Marker")
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            .child(
                section("Separator")
                    .description("Place a conversation boundary between two semantic lines.")
                    .w(px(640.))
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Separator)
                            .content(MarkerContent::new().child("Today")),
                    ),
            )
            .child(
                section("Border")
                    .description("Use a bottom edge for an unread or section boundary.")
                    .w(px(640.))
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Border)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
                            .content(MarkerContent::new().child("3 unread messages")),
                    ),
            )
            .child(
                section("Custom style")
                    .description("Caller refinements can replace spacing, color, and surface.")
                    .w(px(640.))
                    .child(
                        Marker::new()
                            .px_3()
                            .py_2()
                            .rounded(cx.theme().radius)
                            .bg(cx.theme().accent)
                            .text_color(cx.theme().accent_foreground)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Star)))
                            .content(MarkerContent::new().child("Pinned message")),
                    ),
            )
    }
}
