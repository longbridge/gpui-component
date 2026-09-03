---
title: History
description: 用于应用状态的浏览器式导航轨迹。
order: 7
---

# History

`History<T>` 是浏览器式的线性导航轨迹，支持后退和前进。它不持有 GPUI 状态，由调用方把返回的值应用到模型。

## 引入

```rust
use gpui_base::History;
```

## 导航轨迹

每到一个位置就 push 一条。当前条目是轨迹中的最后一个值。例如访问完 `A -> B -> C` 后，当前是 `C`；后退会返回新的当前条目 `B`：

```rust
let mut history = History::new();
history.push("A");
history.push("B");
history.push("C");

assert_eq!(history.back(), Some("B"));
assert_eq!(history.current(), Some(&"B"));
```

`back()` 不会越过根条目，到根时返回 `None`。`forward()` 会恢复最近一个此前离开的条目。后退后再 push 新条目会丢弃前进分支，和浏览器打开新页面时的行为相同。`max_entries` 限制从根到当前的活动条目：降低上限会立即删除最旧的多余活动条目；达到上限时前进，会先删除最旧的活动条目，再恢复下一个条目。

`entries()` 按从根到当前条目的顺序迭代。完整的 `A -> B -> C` 轨迹会依次得到 `A`、`B`、`C`；`entries().rev()` 则得到 `C`、`B`、`A`。`forward_entries()` 从最近的前进条目到最远的前进条目迭代。用 `retain` 删除已失效的位置，用 `replace_current` 原地更新当前的位置，用 `remove_current` 删除当前条目而不丢弃前进分支。

| 方法 | 作用 |
| --- | --- |
| `new()` | 创建空轨迹。`max_entries` 默认是 1000。 |
| `max_entries(n)` | 限制从根到当前的条目数，并立即删除最旧的多余条目。 |
| `push(entry)` | 让 `entry` 成为当前条目，并丢弃前进分支。 |
| `back()`、`forward()` | 在轨迹中移动，返回移动后的当前条目。 |
| `current()` | 返回当前条目。 |
| `can_back()`、`can_forward()` | 判断对应方向是否可以移动。 |
| `entries()`、`forward_entries()` | 按导航顺序迭代当前轨迹和前进分支。 |
| `replace_current(entry)`、`remove_current()` | 更新或删除当前条目。 |
| `retain(keep)`、`clear()` | 从两侧删除不保留的条目，或清空轨迹。 |
