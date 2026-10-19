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
cargo run -p gpui-base-examples -- select
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

## 键盘

Enter、Space 与方向键会打开收起的 select，并把焦点移到 `content_focus_handle`；
Escape 关闭它并把焦点还给 trigger。Tab 与 Shift-Tab 会像 Escape 一样关闭展开的 select、
不做选择，然后照常从 trigger 继续移动焦点。

提供 `.options(labels)` 后，根节点还会负责展开列表内的导航。Up 与 Down 移动高亮项，
到两端即停止；Page Up 与 Page Down 按一页移动，即列表可见的行数（没有可供测量的
`.scroll_handle(...)` 时为全部选项）；Home 与 End 跳到第一项和最后一项。select 收起时，这些键会先打开它，
方向键从 `.selected_index(...)` 开始。输入标签的开头会高亮下一个匹配项，并在收起时打开
select；重复同一个字母会在以它开头的选项间依次切换。在上一个字母后一秒内输入的 Space 会继续这次搜索，
因此可以输入含空格的标签；否则 Space 执行激活。应用通过 `.on_highlight(...)`
同步 `.highlighted_index(...)`，用 `aria_active_descendant()` 标记该选项，并在
`.on_confirm(...)` 中提交它；该回调运行时焦点在内容上，因此可以关闭列表并聚焦 trigger。
`.scroll_handle(...)` 会把高亮项滚动到可见区域；提供它后，指针移到某个选项上也会高亮该选项。不提供 options 时，这些键会继续交给
获得焦点的内容，由它自行导航。

## 可访问性

在受控根节点上设置 `.accessibility_label(...)`，并把 `.accessibility_value(...)`
设为已提交的选中项，而不是临时的搜索游标。根节点会暴露展开状态与可访问的激活操作。
激活会请求切换展开状态，并在 trigger 与内容之间移动焦点。禁用的控件不暴露激活操作。
带样式的 `Select` 会自动提供已提交的值，未选中时回退到 placeholder。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
