---
title: Attachment
description: 支持上传状态、预览和操作的可组合文件与媒体附件表面。
---

# Attachment

`Attachment` 用于在会话中展示文件或媒体条目。具名 slot 负责稳定的公共布局，上传进度与操作则复用已有 GPUI Component 控件。

## 导入

```rust
use gpui_component::attachment::{
    Attachment, AttachmentActions, AttachmentContent, AttachmentDescription, AttachmentGroup,
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

使用 `Axis::Vertical` 将预览放在元数据上方。横向 Attachment 的最小宽度为 160 px；纵向 Attachment 在无 content 时宽 96 px，有 content 时宽 120 px，media 默认保持正方形。显式 media 尺寸或任意 `Styled` refinement 仍可覆盖这些默认值。

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

图片会用 `ObjectFit::Cover` 填满 media 区域。上传中、处理中或失败时，图片预览会降低透明度；等待上传和完成状态恢复为完全不透明。

## 分组

`AttachmentGroup` 提供 Base UI 的横向间距与滚动行为。它需要稳定的 GPUI element id 保存滚动状态：

```rust
AttachmentGroup::new("message-attachments")
    .child(first_attachment)
    .child(second_attachment)
```

## 自定义样式

`Attachment` 与每个公开 slot 都实现了 `Styled`。refinement 在默认样式之后应用，因此调用方可以替换宽度、间距、圆角、颜色、media 尺寸、文字与 actions 布局。

## 组件边界

此 API 有意省略了 shadcn/ui 中的几个部分：

- 直接使用 `Button`，不增加 `AttachmentAction`，以保留 Button 的全部 variant、size、事件与可访问性选项。
- 应用自行组合导航或预览行为，不增加覆盖整个卡片的 `AttachmentTrigger`；GPUI 的焦点与点击所有权应保持明确。
- `AttachmentGroup` 只负责共享的横向间距和 overflow 行为。需要 snap、选择或自定义滚动控件时，使用应用自己的容器。
- 直接使用 `Progress`，不增加附件专属进度包装。

保留的具名 slot 都有稳定的附件语义，并分别负责有意义的默认布局，因此它们属于有效的组件边界，而非只为样式增加的包装层。

## API 参考

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
