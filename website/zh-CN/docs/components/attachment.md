---
title: Attachment
description: 支持上传状态、预览和操作的可组合文件与媒体附件表面。
---

# Attachment

`Attachment` 用于在会话中展示文件或媒体条目。具名 slot 负责稳定的公共布局，上传进度与操作则复用已有 GPUI Component 控件。

## 导入

```rust
use gpui_component::attachment::{
    Attachment, AttachmentActions, AttachmentContent, AttachmentDescription,
    AttachmentMedia, AttachmentStatus, AttachmentTitle,
};
```

## 组合结构

```text
Attachment
├── AttachmentMedia
├── AttachmentContent
│   ├── AttachmentTitle
│   ├── AttachmentDescription
│   └── Progress（可选）
└── AttachmentActions
    └── Button（可选）
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

## 上传生命周期

使用 `AttachmentStatus::{Pending, Uploading, Processing, Failed, Complete}` 表达生命周期样式。状态含义仍应写入 `AttachmentDescription`，失败不能只靠颜色传达。

```rust
Attachment::new()
    .status(AttachmentStatus::Uploading)
    .content(
        AttachmentContent::new()
            .child(AttachmentTitle::new("design-assets.zip"))
            .child(AttachmentDescription::new("上传中 · 68%"))
            .child(Progress::new("attachment-progress").value(68.)),
    )
```

`Progress` 保持为独立组件，因此确定进度、不确定进度和应用专属行为都能继续使用，无需复制一套 API。

## 缩略图与方向

使用 `Axis::Vertical` 将预览放在元数据上方。`Attachment` 会用 axis 放置 content 与 actions，并把自身 size 作为 media slot 的默认尺寸。显式 media 尺寸或任意 `Styled` refinement 仍可覆盖这些默认值。

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

图片会用 `ObjectFit::Cover` 填满调用方设置的 media 区域。

## 自定义样式

`Attachment` 与每个公开 slot 都实现了 `Styled`。refinement 在默认样式之后应用，因此调用方可以替换宽度、间距、圆角、颜色、media 尺寸、文字与 actions 布局。

## 组件边界

此 API 有意省略了 shadcn/ui 中的几个部分：

- 直接使用 `Button`，不增加 `AttachmentAction`，以保留 Button 的全部 variant、size、事件与可访问性选项。
- 应用自行组合导航或预览行为，不增加覆盖整个卡片的 `AttachmentTrigger`；GPUI 的焦点与点击所有权应保持明确。
- 多附件布局使用 `h_flex()`、`v_flex()` 或应用自己的滚动容器。单独的 `AttachmentGroup` 只会重复布局与滚动 API。
- 直接使用 `Progress`，不增加附件专属进度包装。

保留的具名 slot 都有稳定的附件语义，并分别负责有意义的默认布局，因此它们属于有效的组件边界，而非只为样式增加的包装层。

## API 参考

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
