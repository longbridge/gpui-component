---
title: Attachment
description: Composable file and media attachment surfaces with upload states, previews, and actions.
---

# Attachment

`Attachment` presents a file or media item in a conversation. Its typed slots keep the shared layout consistent while existing GPUI Component controls provide progress and actions.

## Import

```rust
use gpui_component::attachment::{
    Attachment, AttachmentActions, AttachmentContent, AttachmentDescription, AttachmentGroup,
    AttachmentMedia, AttachmentStatus, AttachmentTitle,
};
```

## Composition

```text
Attachment
├── AttachmentMedia
├── AttachmentContent
│   ├── AttachmentTitle
│   ├── AttachmentDescription
│   └── Progress (optional)
└── AttachmentActions
    └── Button (optional)
```

```rust
Attachment::new()
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
    )
```

## Upload lifecycle

Use `AttachmentStatus::{Pending, Uploading, Processing, Failed, Complete}` for lifecycle styling. Keep the status meaning visible in `AttachmentDescription`; color alone should not communicate failure.

```rust
Attachment::new()
    .status(AttachmentStatus::Uploading)
    .content(
        AttachmentContent::new()
            .child(AttachmentTitle::new("design-assets.zip"))
            .child(AttachmentDescription::new("Uploading · 68%"))
            .child(Progress::new("attachment-progress").value(68.)),
    )
```

`Progress` remains an independent component so determinate, indeterminate, and application-specific progress behavior stay available without duplicating its API.

## Thumbnail and orientation

Use `Axis::Vertical` for a preview above the metadata. Horizontal attachments have a 160 px minimum width. Vertical attachments are 96 px wide without content and 120 px wide with content, and their media uses a square aspect ratio. An explicit media size or any `Styled` refinement can still override those defaults.

```rust
Attachment::new()
    .axis(Axis::Vertical)
    .media(AttachmentMedia::new().src(preview_url))
    .content(
        AttachmentContent::new()
            .child(AttachmentTitle::new("preview.png"))
            .child(AttachmentDescription::new("PNG · 1280 × 720")),
    )
```

The image fills the styled media bounds with `ObjectFit::Cover`. Image previews dim while uploading, processing, or failed, and return to full opacity while pending or complete.

## Groups

`AttachmentGroup` provides the Base UI horizontal gap and scrolling behavior. It requires a stable GPUI element id for scroll state:

```rust
AttachmentGroup::new("message-attachments")
    .child(first_attachment)
    .child(second_attachment)
```

## Custom styling

`Attachment` and every public slot implement `Styled`. Refinements apply after component defaults, so callers can replace width, spacing, radius, colors, media dimensions, typography, and action layout.

## Component boundaries

This API intentionally omits several shadcn/ui parts:

- Use `Button` directly instead of `AttachmentAction`; this preserves every Button variant, size, event, and accessibility option.
- Compose application navigation or preview behavior instead of a full-card `AttachmentTrigger`; GPUI focus and click ownership should remain explicit.
- `AttachmentGroup` stays thin and owns only the shared horizontal gap and overflow behavior. Use an application-owned container when snapping, selection, or custom scroll controls are required.
- Use `Progress` directly instead of an attachment-specific progress wrapper.

The remaining named slots have stable attachment semantics and each owns meaningful default layout, so they are useful component boundaries rather than styling-only wrappers.

## API reference

- [Attachment]
- [AttachmentGroup]
- [AttachmentMedia]
- [AttachmentContent]
- [AttachmentTitle]
- [AttachmentDescription]
- [AttachmentActions]

[Attachment]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.Attachment.html
[AttachmentGroup]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentGroup.html
[AttachmentMedia]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentMedia.html
[AttachmentContent]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentContent.html
[AttachmentTitle]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentTitle.html
[AttachmentDescription]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentDescription.html
[AttachmentActions]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentActions.html
