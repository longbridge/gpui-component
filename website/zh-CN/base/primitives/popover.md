---
title: Popover
description: 支持受控或内部开关状态的锚定浮层。
order: 19
---

# Popover

支持受控或内部开关状态的锚定浮层。

和所有 GPUI Base 原语一样，Popover 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- popover
```

## 导入

```rust
use gpui_kit::base::{Popover};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/popover.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/popover.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

使用 `.anchor(Anchor::TopCenter).offset(px(8.))` 可让弹层在触发器下方居中，并留出八像素间距。
Base 的 offset 默认为零。`Top*` 在下方，`Bottom*` 在上方，
`LeftCenter` 在右侧，`RightCenter` 在左侧。anchor 描述弹层自身的锚点。
窗口边界限制不会翻转弹层或更改 anchor。

弹层锚定到触发器容器，也就是 `Popover` 自身样式所作用的元素。在块级父元素中，这个容器会撑满父元素宽度；
将它放在 flex 行中（或为它指定宽度），弹层才会锚定到触发器本身。

`on_position` 在内容 prepaint 前提供最终弹层和触发器边界，供自定义绘制使用。
`.arrow(PopupArrow::new(px(6.)))` 从内容边缘绘制一个箭头，指向 anchor 所指的触发器位置，
箭头底边避开 `.corner_radius(..)` 指定的圆角。箭头本身不带样式：背景色填充箭头，边框色描出两条斜边。
箭头位于内容之外，因此 `offset` 需要包含箭头深度。`ArrowGeometry` 提供同一套几何信息（所在边、顶点、边界），
可用来自行绘制箭头。带样式的 Component Popover 通过 `.arrow(true)` 绘制这个箭头。

触发器切换打开状态；点击外部或 Escape 可按配置关闭。
`.disabled(true)` 会忽略触发器的点击和按键，并让弹层保持关闭，即使 `open(true)` 要求打开也是如此；触发器元素本身也应同时禁用。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

<<< ../../../../crates/base/examples/showcase/components/popover.rs{rust}

## 可访问性

管理触发器与内容的关系和焦点，不要让关闭后焦点丢失。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
