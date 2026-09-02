---
title: History
description: 带游标的线性历史：undo/redo、前进后退，或最近优先列表，适用于任何需要记住的条目。
order: 7
---

# History

`History<I>` 是一个带游标的条目列表。`push` 在游标处追加一条，`undo` 把游标退一格并返回跨过的条目，`redo` 再往前走。撤销之后再 `push` 会开启新分支：游标之前的条目被丢弃，就像浏览器打开新页面时丢掉前进页一样。

它不持有任何 GPUI 状态，也不关心条目是什么。这正是它的价值：同一个类型可以是 undo manager、导航轨迹或最近列表，取决于你 push 进去的是什么。

## 引入

```rust
use gpui_base::{History, HistoryItem};
```

条目是任意 `Clone + PartialEq` 且能携带版本号的类型：

```rust
#[derive(Clone, PartialEq)]
struct Visit {
    symbol: String,
    version: usize,
}

impl HistoryItem for Visit {
    fn version(&self) -> usize { self.version }
    fn set_version(&mut self, version: usize) { self.version = version; }
}
```

版本号由 `push` 打上。同一个分组间隔内 push 的条目共享一个版本，`undo` 时一起返回，所以一次拖拽产生的许多小改动会作为一步撤销。

## 适用场景

**改动的 undo 与 redo。** 每次改动 push 一条；`undo` 返回要回退的改动，`redo` 返回要重放的改动。Dock 的 tiles 画布就是这样记录 bounds 变化的，用一个很短的分组间隔把一次拖拽合成一步。边界明确时用 `start_grouping` 和 `end_grouping`，不用靠时间。

**位置的前进与后退。** 每到一个地方 push 一条；`undo` 是后退，`redo` 是前进，`current()` 是当前所在。位置可能失效，比如所在的 tab 被关掉了，用 `retain` 在游标两侧一起剪枝。加载完成前就记下的位置，用 `replace_current` 原地更正，保留版本号、长度不变，和浏览器的 `replaceState` 一样。[Nav Stack](./primitives/nav-stack.md) 就建立在这之上：页面是条目，`pop` 是 `undo`，`forward` 是 `redo`。

**最近优先列表。** 开启 `unique()` 后，push 一个已存在的条目会把它挪到最前而不是重复出现，`max_undos` 限制长度。用户打开过的股票、碰过的文件、执行过的命令：每次使用 push 一条，从后往前读 `undos()`。

## API

| 方法 | 作用 |
| --- | --- |
| `new()` | 空历史。`max_undos` 默认 1000。 |
| `max_undos(n)`、`unique()`、`group_interval(d)` | 构建器：限制长度、每个条目只保留一份、把间隔小于 `d` 的 push 合成一步。 |
| `push(item)` | 在游标处记录 `item`，丢弃游标之前的所有条目。 |
| `undo()`、`redo()` | 移动游标并返回跨过的条目，最新的在前；到头时返回 `None`。 |
| `current()` | 游标处的条目。 |
| `replace_current(item)` | 覆盖游标处的条目，保留其版本号。为空时等于 push。 |
| `retain(keep)` | 丢弃 `keep` 拒绝的条目，游标两侧同时生效。 |
| `undos()`、`redos()` | 游标之后与之前的条目，最旧的在前。 |
| `start_grouping()`、`end_grouping()` | 冻结版本号，中间的 push 作为一步撤销。 |
| `set_ignoring(bool)`、`is_ignoring()` | 供调用方在回放自己的历史时跳过记录的标志位。 |
| `clear()` | 清空两侧。 |

## 说明

`History` 从不修改你的模型。它只把条目交回来，由调用方去应用，所以同一个类型既能服务"必须回退的改动"，也能服务"要重新访问的位置"。先想清楚条目是什么，再决定是否分组或去重：改动需要分组，位置两者都不要，最近列表需要 `unique`。
