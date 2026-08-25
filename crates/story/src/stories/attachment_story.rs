use gpui::{
    App, AppContext as _, Axis, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement as _, Render, Styled as _, Window, rems,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    attachment::{
        Attachment, AttachmentActions, AttachmentContent, AttachmentDescription, AttachmentGroup,
        AttachmentMedia, AttachmentStatus, AttachmentTitle,
    },
    button::{Button, ButtonVariants as _},
    progress::Progress,
    shimmer::ShimmerStyle,
    spinner::Spinner,
    v_flex,
};

use crate::{Story, section};

pub struct AttachmentStory {
    focus_handle: FocusHandle,
}

impl AttachmentStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Story for AttachmentStory {
    fn title() -> &'static str {
        "Attachment"
    }

    fn description() -> &'static str {
        "Composable file and media attachments with lifecycle states and actions."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for AttachmentStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AttachmentStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("File metadata")
                    .description(
                        "Compose typed metadata and actions, or keep using existing child elements.",
                    )
                    .w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        Attachment::new()
                            .media(AttachmentMedia::new().child(Icon::new(IconName::File)))
                            .content(
                                AttachmentContent::new()
                                    .title(AttachmentTitle::new("quarterly-report.pdf"))
                                    .description(AttachmentDescription::new("PDF · 2.4 MB")),
                            )
                            .actions(
                                AttachmentActions::new().child(
                                    Button::new("remove-report")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Close),
                                ),
                            ),
                    )
                    .child(
                        Attachment::new()
                            .small()
                            .media(AttachmentMedia::new().child(Icon::new(IconName::File)))
                            .content(
                                AttachmentContent::new()
                                    .child(AttachmentTitle::new("research-data.csv"))
                                    .child(AttachmentDescription::new("CSV · 840 KB")),
                            ),
                    ),
            )
            .child(
                section("Upload states")
                    .description(
                        "Typed titles and descriptions inherit loading and failure states automatically.",
                    )
                    .w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        Attachment::new()
                            .status(AttachmentStatus::Pending)
                            .media(AttachmentMedia::new().child(Icon::new(IconName::File)))
                            .content(
                                AttachmentContent::new()
                                    .title(AttachmentTitle::new("meeting-notes.pdf"))
                                    .description(AttachmentDescription::new("Ready to upload")),
                            ),
                    )
                    .child(
                        Attachment::new()
                            .status(AttachmentStatus::Uploading)
                            .media(AttachmentMedia::new().child(Icon::new(IconName::File)))
                            .content(
                                AttachmentContent::new()
                                    .title(AttachmentTitle::new("design-assets.zip"))
                                    .description(AttachmentDescription::new("Uploading · 68%"))
                                    .child(Progress::new("attachment-upload-progress").value(68.)),
                            )
                            .actions(
                                AttachmentActions::new().child(
                                    Button::new("cancel-upload")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Close),
                                ),
                            ),
                    )
                    .child(
                        Attachment::new()
                            .status(AttachmentStatus::Processing)
                            .media(AttachmentMedia::new().child(Icon::new(IconName::File)))
                            .content(
                                AttachmentContent::new()
                                    .title(
                                        AttachmentTitle::new("transcript.pdf").with_shimmer_style(
                                            ShimmerStyle::new()
                                                .highlight_color(cx.theme().primary)
                                                .spread(0.45)
                                                .reverse(true),
                                        ),
                                    )
                                    .description(AttachmentDescription::new("Processing document")),
                            ),
                    )
                    .child(
                        Attachment::new()
                            .status(AttachmentStatus::Failed)
                            .media(AttachmentMedia::new().child(Icon::new(IconName::File)))
                            .content(
                                AttachmentContent::new()
                                    .title(AttachmentTitle::new("archive.zip"))
                                    .description(AttachmentDescription::new("Upload failed")),
                            )
                            .actions(
                                AttachmentActions::new()
                                    .child(Button::new("retry-upload").xsmall().label("Retry"))
                                    .child(
                                        Button::new("remove-failed-upload")
                                            .danger()
                                            .xsmall()
                                            .icon(IconName::Delete),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Thumbnail")
                    .description(
                        "Vertical attachments can turn the media slot into a full-width preview.",
                    )
                    .w(rems(42.5))
                    .child(
                        Attachment::new()
                            .axis(Axis::Vertical)
                            .media(
                                AttachmentMedia::new().src(
                                    "https://pub.lbkrs.com/files/202503/vEnnmgUM6bo362ya/sdk.svg",
                                ),
                            )
                            .content(
                                AttachmentContent::new()
                                    .title(AttachmentTitle::new("sdk-preview.svg"))
                                    .description(AttachmentDescription::new("SVG · 1280 × 720")),
                            )
                            .actions(
                                AttachmentActions::new().child(
                                    Button::new("remove-preview")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Close),
                                ),
                            ),
                    ),
            )
            .child(
                section("Image overlays")
                    .description(
                        "Image previews keep their overlays visible while only the image dims during upload.",
                    )
                    .w(rems(42.5))
                    .child(
                        Attachment::new()
                            .axis(Axis::Vertical)
                            .status(AttachmentStatus::Uploading)
                            .media(
                                AttachmentMedia::new()
                                    .src(
                                        "https://pub.lbkrs.com/files/202503/vEnnmgUM6bo362ya/sdk.svg",
                                    )
                                    .overlay(Spinner::new().small().color(cx.theme().foreground)),
                            )
                            .content(
                                AttachmentContent::new()
                                    .title(AttachmentTitle::new("preview.svg"))
                                    .description(AttachmentDescription::new("Uploading · 72%")),
                            ),
                    ),
            )
            .child(
                section("Sizes and group")
                    .description(
                        "Attachments size to their content and groups scroll horizontally.",
                    )
                    .w(rems(42.5))
                    .child(
                        AttachmentGroup::new("attachment-story-group")
                            .child(
                                Attachment::new()
                                    .media(AttachmentMedia::new().child(Icon::new(IconName::File)))
                                    .content(
                                        AttachmentContent::new()
                                            .title(AttachmentTitle::new("default.pdf"))
                                            .description(AttachmentDescription::new("PDF · 2.4 MB")),
                                    ),
                            )
                            .child(
                                Attachment::new()
                                    .small()
                                    .media(AttachmentMedia::new().child(Icon::new(IconName::File)))
                                    .content(
                                        AttachmentContent::new()
                                            .title(AttachmentTitle::new("small.csv"))
                                            .description(AttachmentDescription::new("CSV · 840 KB")),
                                    ),
                            )
                            .child(
                                Attachment::new()
                                    .xsmall()
                                    .media(AttachmentMedia::new().child(Icon::new(IconName::File)))
                                    .content(
                                        AttachmentContent::new()
                                            .title(AttachmentTitle::new("compact.txt")),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Custom style")
                    .description("Every public part accepts caller style refinements.")
                    .w(rems(42.5))
                    .child(
                        Attachment::new()
                            .w_full()
                            .rounded(cx.theme().radius)
                            .bg(cx.theme().accent)
                            .border_color(cx.theme().accent.opacity(0.5))
                            .media(
                                AttachmentMedia::new()
                                    .rounded(cx.theme().radius)
                                    .bg(cx.theme().primary.opacity(0.12))
                                    .text_color(cx.theme().primary)
                                    .child(Icon::new(IconName::File)),
                            )
                            .content(
                                AttachmentContent::new()
                                    .title(
                                        AttachmentTitle::new("custom-theme.json")
                                            .text_color(cx.theme().primary),
                                    )
                                    .description(AttachmentDescription::new("JSON · 16 KB")),
                            ),
                    ),
            )
    }
}
