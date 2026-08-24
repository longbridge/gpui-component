---
title: Attachment
description: Composable file and media attachment surfaces with upload states, previews, and actions.
---

# Attachment

`Attachment` presents a file or media item in a conversation. Its typed slots keep the shared layout consistent while existing GPUI Component controls provide progress and actions.

## Import

```rust
use gpui_component::attachment::{
    Attachment, AttachmentActions, AttachmentContent, AttachmentDescription,
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

Use `Axis::Vertical` for a preview above the metadata. `Attachment` uses its axis to place the content and actions, and supplies its size as the media slot's default. An explicit media size or any `Styled` refinement can still override those defaults.

```rust
Attachment::new()
    .axis(Axis::Vertical)
    .media(
        AttachmentMedia::new()
            .src(preview_url)
            .w_full()
            .h(px(140.)),
    )
    .content(
        AttachmentContent::new()
            .child(AttachmentTitle::new("preview.png"))
            .child(AttachmentDescription::new("PNG · 1280 × 720")),
    )
```

The image fills the styled media bounds with `ObjectFit::Cover`.

## Custom styling

`Attachment` and every public slot implement `Styled`. Refinements apply after component defaults, so callers can replace width, spacing, radius, colors, media dimensions, typography, and action layout.

## Component boundaries

This API intentionally omits several shadcn/ui parts:

- Use `Button` directly instead of `AttachmentAction`; this preserves every Button variant, size, event, and accessibility option.
- Compose application navigation or preview behavior instead of a full-card `AttachmentTrigger`; GPUI focus and click ownership should remain explicit.
- Use `h_flex()`, `v_flex()`, or an application-owned scroll container for groups. A dedicated `AttachmentGroup` would only duplicate layout and scrolling APIs.
- Use `Progress` directly instead of an attachment-specific progress wrapper.

The remaining named slots have stable attachment semantics and each owns meaningful default layout, so they are useful component boundaries rather than styling-only wrappers.

## API reference

- [Attachment]
- [AttachmentMedia]
- [AttachmentContent]
- [AttachmentTitle]
- [AttachmentDescription]
- [AttachmentActions]

[Attachment]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.Attachment.html
[AttachmentMedia]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentMedia.html
[AttachmentContent]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentContent.html
[AttachmentTitle]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentTitle.html
[AttachmentDescription]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentDescription.html
[AttachmentActions]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentActions.html
