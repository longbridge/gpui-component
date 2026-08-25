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
use gpui_component::shimmer::ShimmerStyle;
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

## 上传生命周期

使用 `AttachmentStatus::{Pending, Uploading, Processing, Failed, Complete}` 表达生命周期样式。状态含义仍应写入 `AttachmentDescription`，失败不能只靠颜色传达。

```rust
Attachment::new()
    .status(AttachmentStatus::Uploading)
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("design-assets.zip"))
            .description(AttachmentDescription::new("上传中 · 68%"))
            .child(Progress::new("attachment-progress").value(68.)),
    )
```

`Progress` 保持为独立组件，因此确定进度、不确定进度和应用专属行为都能继续使用，无需复制一套 API。

通过 `.title(...)` 和 `.description(...)` 添加的标题与描述会自动继承父级 Attachment 的状态。上传中和处理中的标题会显示共享的 shimmer 动画，失败状态的描述会使用主题中的危险色。如果为标题或描述显式调用 `.status(...)`，该状态优先于父级状态：

```rust
Attachment::new()
    .status(AttachmentStatus::Failed)
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("archive.zip"))
            .description(
                AttachmentDescription::new("之前的上传已完成")
                    .status(AttachmentStatus::Complete),
            ),
    )
```

可以通过 `AttachmentTitle::with_shimmer_style(...)` 自定义标题的加载动画：

```rust
AttachmentTitle::new("design-assets.zip")
    .with_shimmer_style(
        ShimmerStyle::new()
            .duration(std::time::Duration::from_secs(3))
            .spread(0.45)
            .reverse(true),
    )
```

`ShimmerStyle::highlight_color(...)` 可以将默认的主题高光替换成自定义颜色，`.once(true)` 可以让上传中或处理中的标题只播放一次动画。

原有的 `.child(AttachmentTitle::new(...))` 和 `.child(AttachmentDescription::new(...))` 写法仍然可以使用。由于 `.child(...)` 会擦除元素的具体类型，通过这种方式添加的标题和描述无法自动继承 Attachment 状态；需要状态感知样式时，请使用对应的具名方法。

## 缩略图与方向

使用 `Axis::Vertical` 将预览放在元数据上方。横向 Attachment 使用 `min_w_40()` 对应的最小宽度层级；纵向 Attachment 在没有内容时使用 `w_24()`，有内容时使用相当于 Tailwind `w-30` 的宽度层级，媒体区域默认保持正方形。这些尺寸都会跟随应用的基础字号缩放。显式设置媒体尺寸或使用任意 `Styled` 样式覆盖，仍然可以调整默认值。

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

图片会使用 `ObjectFit::Cover` 填满媒体区域。上传中、处理中或失败时，图片预览会降低透明度；等待上传和完成状态恢复为完全不透明。图片上的子元素会继续显示，`.overlay(...)` 还会将元素居中覆盖在整个媒体区域上：

```rust
Attachment::new()
    .status(AttachmentStatus::Uploading)
    .media(
        AttachmentMedia::new()
            .src(preview_url)
            .overlay(Spinner::new().small()),
    )
```

加载状态只调整图片本身的透明度，覆盖层中的图标、进度指示器和自定义控件会保持清晰。

## 分组

`AttachmentGroup` 提供 Base UI 的横向间距与滚动行为。它需要稳定的 GPUI element id 保存滚动状态：

```rust
AttachmentGroup::new("message-attachments")
    .child(first_attachment)
    .child(second_attachment)
```

## 自定义样式

`Attachment` 与每个公开 slot 都实现了 `Styled`。自定义样式会在默认样式之后应用，因此调用方可以替换宽度、间距、圆角、颜色、媒体尺寸、文字与操作区域布局。Attachment 的默认圆角来自 `Theme::radius_2xl()`，媒体区域使用更小的主题圆角，两者都会跟随应用主题变化。尺寸、间距和字号使用共享的 rem 设计刻度。Attachment 表面复用现有的 `group_box.background` 和 `group_box.foreground` 主题颜色，因此可以独立调整卡片类表面与 popover，同时避免增加组件专属主题 token。

使用 `.title(...)` 和 `.description(...)` 可以添加自动感知状态的元数据，使用 `.child(...)` 可以添加任意自定义内容，标题和描述各自的 `.status(...)` 可以覆盖继承状态，标题的 `.with_shimmer_style(...)` 可以调整加载动画，`.overlay(...)` 和 `.child(...)` 可以在预览图片上组合额外内容。

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
