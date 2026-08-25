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
use gpui_component::shimmer::ShimmerStyle;
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
    )
```

## Upload lifecycle

Use `AttachmentStatus::{Pending, Uploading, Processing, Failed, Complete}` for lifecycle styling. Keep the status meaning visible in `AttachmentDescription`; color alone should not communicate failure.

```rust
Attachment::new()
    .status(AttachmentStatus::Uploading)
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("design-assets.zip"))
            .description(AttachmentDescription::new("Uploading · 68%"))
            .child(Progress::new("attachment-progress").value(68.)),
    )
```

`Progress` remains an independent component so determinate, indeterminate, and application-specific progress behavior stay available without duplicating its API.

Titles and descriptions added through `.title(...)` and `.description(...)` automatically inherit their parent attachment's status. Uploading and processing titles use the shared shimmer treatment, and failed descriptions use the destructive theme color. An explicit `.status(...)` on either child takes precedence over the inherited status:

```rust
Attachment::new()
    .status(AttachmentStatus::Failed)
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("archive.zip"))
            .description(
                AttachmentDescription::new("Previous upload completed")
                    .status(AttachmentStatus::Complete),
            ),
    )
```

Customize a title's loading animation with `AttachmentTitle::with_shimmer_style(...)`:

```rust
AttachmentTitle::new("design-assets.zip")
    .with_shimmer_style(
        ShimmerStyle::new()
            .duration(std::time::Duration::from_secs(3))
            .spread(0.45)
            .reverse(true),
    )
```

`ShimmerStyle::highlight_color(...)` can also replace the theme-aware default highlight.

The existing `.child(AttachmentTitle::new(...))` and `.child(AttachmentDescription::new(...))` forms remain supported. Because `.child(...)` erases the concrete element type, these legacy children do not inherit the attachment status automatically; use the typed builders when status-aware appearance is required.

## Thumbnail and orientation

Use `Axis::Vertical` for a preview above the metadata. Horizontal attachments use the `min_w_40()` scale step. Vertical attachments use the `w_24()` step without content and the equivalent of Tailwind's `w-30` step with content; their media uses a square aspect ratio. All of these dimensions scale with the application's base font size. An explicit media size or any `Styled` refinement can still override those defaults.

```rust
Attachment::new()
    .axis(Axis::Vertical)
    .media(AttachmentMedia::new().src(preview_url))
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("preview.png"))
            .description(AttachmentDescription::new("PNG · 1280 × 720")),
    )
```

The image fills the styled media bounds with `ObjectFit::Cover`. Image previews dim while uploading, processing, or failed, and return to full opacity while pending or complete. Children remain visible above image sources; `.overlay(...)` additionally centers an element across the entire media area:

```rust
Attachment::new()
    .status(AttachmentStatus::Uploading)
    .media(
        AttachmentMedia::new()
            .src(preview_url)
            .overlay(Spinner::new().small()),
    )
```

Only the image receives the loading opacity, so overlay icons, progress indicators, and custom controls remain fully legible.

## Groups

`AttachmentGroup` provides the Base UI horizontal gap and scrolling behavior. It requires a stable GPUI element id for scroll state:

```rust
AttachmentGroup::new("message-attachments")
    .child(first_attachment)
    .child(second_attachment)
```

## Custom styling

`Attachment` and every public slot implement `Styled`. Refinements apply after component defaults, so callers can replace width, spacing, radius, colors, media dimensions, typography, and action layout. The default attachment radius derives from `Theme::radius_2xl()`, while media corners use a smaller theme radius; both follow the application theme. Attachment sizing, spacing, and typography use the shared rem-based design scale. Attachment surfaces use the existing `group_box.background` and `group_box.foreground` theme colors, keeping card-like surfaces independently configurable from popovers without introducing a component-specific theme token.

Use `.title(...)` and `.description(...)` for status-aware metadata, `.child(...)` for arbitrary custom content, `.status(...)` on individual titles or descriptions to override inherited appearance, `.with_shimmer_style(...)` to customize a title's loading animation, and `.overlay(...)` or `.child(...)` to compose content above image previews.

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
