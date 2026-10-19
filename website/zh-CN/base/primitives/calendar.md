---
title: Calendar
description: 支持选择匹配器和自定义日期项渲染的状态驱动日期网格。
order: 5
---

# Calendar

支持选择匹配器和自定义日期项渲染的状态驱动日期网格。

和所有 GPUI Base 原语一样，Calendar 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- calendar
```

## 导入

```rust
use gpui_kit::base::{Calendar, CalendarState};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/calendar.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/calendar.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

CalendarState 保存可见月份和选择；回调负责同步受控值。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

<<< ../../../../crates/base/examples/showcase/components/calendar.rs{rust}

## 键盘

Calendar 是一个 Tab 停靠点。在日视图中，键盘移动当前聚焦的日期：

| 按键 | 操作 |
| --- | --- |
| Left / Right | 前一天 / 后一天 |
| Up / Down | 上一周 / 下一周 |
| PageUp / PageDown | 上个月 / 下个月，日期相同（不存在时取当月最后一天） |
| Shift-PageUp / Shift-PageDown | 上一年 / 下一年 |
| Home / End | 本周第一天 / 最后一天，以 `first_day_of_week` 为准 |
| Enter / Space | 选中聚焦的日期 |

显示的月份随聚焦日期翻动。按天或按周移动时会跳过禁用日期；其他移动会停在离目标最近的可用日期上，因此使用 `Matcher::interval(min, max)` 这类禁用规则时，焦点会停在边界上。日历持有键盘焦点时，`CalendarItemState::is_focused` 标记聚焦的日期，用于绘制焦点环；`CalendarState::focused_date` 和 `set_focused_date` 用于读取和移动它。按键绑定位于 `Calendar` key context 中。

## 可访问性

每个月是一个由行组成的 grid：星期名称是列标题，日期是以 ISO 日期命名的单元格；被选中或位于范围内的日期标记为已选中，被禁用规则排除的日期标记为禁用。日历持有焦点时，聚焦的日期作为 active descendant 报告给辅助技术。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
