use gpui::{
    App, AppContext as _, Axis, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement as _, Render, Styled as _, Window, px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    attachment::{
        Attachment, AttachmentActions, AttachmentContent, AttachmentDescription, AttachmentMedia,
        AttachmentStatus, AttachmentTitle,
    },
    button::{Button, ButtonVariants as _},
    progress::Progress,
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
                    .description("Compose the media, metadata, and actions slots from existing controls.")
                    .w(px(680.))
                    .v_flex()
                    .gap_3()
                    .child(
                        Attachment::new()
                            .w_full()
                            .media(AttachmentMedia::new().child(Icon::new(IconName::File)))
                            .content(
                                AttachmentContent::new()
                                    .child(AttachmentTitle::new("quarterly-report.pdf"))
                                    .child(AttachmentDescription::new("PDF · 2.4 MB")),
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
                            .w_full()
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
                    .description("Keep progress and recovery actions composed from Progress and Button.")
                    .w(px(680.))
                    .v_flex()
                    .gap_3()
                    .child(
                        Attachment::new()
                            .status(AttachmentStatus::Uploading)
                            .w_full()
                            .media(AttachmentMedia::new().child(Icon::new(IconName::File)))
                            .content(
                                AttachmentContent::new()
                                    .child(AttachmentTitle::new("design-assets.zip"))
                                    .child(AttachmentDescription::new("Uploading · 68%"))
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
                            .status(AttachmentStatus::Failed)
                            .w_full()
                            .media(AttachmentMedia::new().child(Icon::new(IconName::File)))
                            .content(
                                AttachmentContent::new()
                                    .child(AttachmentTitle::new("archive.zip"))
                                    .child(
                                        AttachmentDescription::new("Upload failed")
                                            .status(AttachmentStatus::Failed),
                                    ),
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
                    .description("Vertical attachments can turn the media slot into a full-width preview.")
                    .w(px(680.))
                    .child(
                        Attachment::new()
                            .axis(Axis::Vertical)
                            .w(px(320.))
                            .media(
                                AttachmentMedia::new()
                                    .src("https://pub.lbkrs.com/files/202503/vEnnmgUM6bo362ya/sdk.svg")
                                    .w_full()
                                    .h(px(140.)),
                            )
                            .content(
                                AttachmentContent::new()
                                    .child(AttachmentTitle::new("sdk-preview.svg"))
                                    .child(AttachmentDescription::new("SVG · 1280 × 720")),
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
                section("Custom style")
                    .description("Every public part accepts caller style refinements.")
                    .w(px(680.))
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
                                    .child(AttachmentTitle::new("custom-theme.json"))
                                    .child(AttachmentDescription::new("JSON · 16 KB")),
                            ),
                    ),
            )
    }
}
