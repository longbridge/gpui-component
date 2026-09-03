---
title: Select
description: 由锚定、支持键盘导航的弹层驱动的选择控件。
order: 25
---

# Select

由锚定、支持键盘导航的弹层驱动的选择控件。

和所有 GPUI Base 原语一样，Select 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base --example components -- select
```

## 导入

```rust
use gpui_kit::base::{Select};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/select.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/select.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

应用保存当前值；打开状态、焦点项与选择行为由控件协调。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

<<< ../../../../crates/base/examples/showcase/components/select.rs{rust}

## 可访问性

提供标签，暴露当前值，并保留上下键、Enter、Escape 与类型检索。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
