---
title: Tabs
description: 带受控选择的标签列表和可访问标签控件。
order: 30
---

# Tabs

带受控选择的标签列表和可访问标签控件。

和所有 GPUI Base 原语一样，Tabs 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- tabs
```

## 导入

```rust
use gpui_kit::base::{Tab, Tabs};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/tabs.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/tabs.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

父级保存活动标签，激活标签后更新对应面板。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 键盘

`Tabs` 负责其下各个 `Tab` 的键盘操作，不论皮肤把它们放在哪一层。整个列表只占一个 Tab stop：Tab 键先落在已获得焦点的 Tab 上，其次是已选中的 Tab，最后是第一个可用的 Tab。沿 `axis` 方向的方向键（默认是 Left 和 Right，`.axis(Axis::Vertical)` 时是 Up 和 Down）把焦点移到上一个或下一个可用的 Tab，到两端时循环；Home 和 End 移到第一个和最后一个。与平台的标签视图一样，移动焦点时会通过该 Tab 的 `on_click` 选中它；设置 `.activate_on_focus(false)` 后，方向键只移动焦点，由 Space 或 Enter 选中。点击会选中 Tab，但不会夺走焦点，因此文档标签栏不会抢走文档的焦点。`Tabs::track_focus(&handle)` 为列表提供一个自身的焦点句柄：聚焦它时，焦点会转到持有 Tab stop 的 Tab 上。

## 完整 Rust 示例

<<< ../../../../crates/base/examples/showcase/components/tabs.rs{rust}

## 可访问性

保留标签列表、标签与面板关系，以及方向键和焦点行为。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
