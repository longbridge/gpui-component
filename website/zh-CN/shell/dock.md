---
title: Dock Panels
description: 把脚本视图放进可停靠布局——重启后还在的面板、被卸载后仍然保住位置的应用，以及 chrome 由谁来画。
order: 11
---

# Dock Panels

只能占满一个窗口的插件算不上插件。这一页讲的是脚本视图怎么变成停靠布局里的一块**面板**：可拖、可停靠、可缩放，重启之后还在原处。

::: warning 脚本这一侧还没接通
这一页的内容今天都能从 Rust 用起来。但让*脚本*自己声明面板钩子、或者自己画 dock chrome 的那两个 trait——`PanelScript` 与 `DockChrome`——虽然已经定义好也有测试，却还没有任何引擎实现它们；所以现在想要这两样的宿主，得先用 Rust 写。分界线画好了，脚本那一侧[还没有](#还没有的东西)。
:::

## base 已经提供的部分

`gpui_base::dock` 早就有了插件系统里难做的那一半：一棵**纯数据**的布局树、一个能按名字从持久化文件里重建面板的 `PanelRegistry`，以及跟着每块面板走的一份 `serde_json::Value`。它缺的只是“让面板来自宿主二进制之外”的办法，而这正是这个模块补上的东西。

## 把脚本视图变成面板

```rust
use gpui_shell::dock::ScriptPanel;

let panel = ScriptPanel::new("quotes", script_view, cx)
    .with_closable(true)
    .with_zoomable(true);
```

`ScriptPanel` 只实现 `gpui_base::dock::Panel`——也就是行为——再往上的那一层不碰。它的标题、工具栏与菜单都不归它自己：那些由下面讲的 skin 用脚本的元素画出来。一旦牵扯到面板，“呈现权在脚本”指的就是这件事。

名字会被 intern 并且总是带前缀，所以脚本面板不可能和恰好同名的宿主面板撞车。

## 挺过一次重启

布局是按数据保存的：有哪些面板、各在哪里、每块面板一份 JSON。要把面板**取回来**，得先按名字注册一个 builder：

```rust
let name = gpui_shell::dock::register_panel("todolist", "quotes", script, cx);
```

要在 `DockArea::load` 之前调用。builder 会先执行 `PanelScript::build` 造一个新的脚本视图，再把持久化下来的那份 payload 交给 `PanelScript::deserialize`。跨越边界的钩子只有三个：

| 钩子 | 什么时候 | 说明 |
| --- | --- | --- |
| `build` | 注册表正在重建这块面板 | 返回 `None` 表示脚本实例化不了；此时 payload 被原样带走而不是丢掉 |
| `serialize` | 布局正在保存 | 拿到的是 `&App` 而不是 `&mut Window`——这是一次读取，没有 CallScope，所以脚本的 `serialize()` 必须是返回纯值、不回调宿主的方法 |
| `deserialize` | 紧跟在 `build` 之后 | 这是一次真正的宿主调用，可以开 scope、可以碰实体 |

面板的其余一切——它在哪、显不显示、叫什么——都是布局的事，从不进到脚本里。

没有接上脚本钩子的 `ScriptPanel` 一样能用；它只会持久化自己的位置，别的什么都不留。

## 被卸载的应用仍然保住位置

这一段值得在发布插件之前先知道。

如果一个应用**没有**被加载，那么它名下什么都没注册，`DockArea::load` 也就找不到对应的 builder。它不会把面板丢掉：base 会替换上一块什么都不画的占位面板，而这块占位面板对 `Panel::dump` 的回答，正是它收到的那份状态——于是下一次保存会把这块面板的名字、payload 与位置原样写回去。

卸载一个应用，照常用这个窗口一星期，再把它装回来：它的面板会回到原来的位置，带着原来的状态。这个模块把同样的承诺又往里延伸了一步——已经注册、但 `build` 失败的面板，也按同样的方式被带走，而不是因为脚本坏了就把状态丢了。

## chrome 由谁来画

base **完全不画 chrome**。一个没有 renderer 的区域照样能停靠、拖动、调整大小、持久化，但除了面板本身之外什么都不画——没有标签栏、没有 dock 外框、没有拖拽条。这些都得经由三个 renderer trait 回来，而 `ScriptDockSkin` 把它们统一转发给一个 `DockChrome`：

```rust
use gpui_shell::dock::ScriptDockSkin;

dock_area.with_renderer(ScriptDockSkin::new(chrome));
```

| `DockChrome` 方法 | 画什么 |
| --- | --- |
| `tab_bar` | 一组面板当前显示项上方的标签栏 |
| `empty_group` | 一组里没有可显示面板时给出的东西 |
| `drop_indicator` | 被拖动的面板将会落在哪里 |
| `dock` | 一个 dock 包住内容的外框——标题条、折叠、调整大小 |
| `tile_drag_bar` | 拖动一块 tile 用的那根条，高度固定为 base 的 `DRAG_BAR_HEIGHT` |
| `tile_resize_handles` | 一块 tile 的缩放手柄，尺寸取自 base 的 `HANDLE_SIZE` |

每个方法拿到的都是**已经解析好的**上下文——从不包含拖拽事件、鼠标位置或命中测试，因为 base 会把这些自己挂到拿回去的元素上。要做的事是把状态变成元素，并调用上下文自带的回调（`select_tab`、`close`、`toggle_zoom`、`resize_to`），而不是自己再实现一遍。

`ScriptDockSkin::default()` 是一个什么都不画的 skin，这也正是 base 自己的行为：一个能用的 dock，里面只有光秃秃的面板。

为将来的脚本侧准备的还有 `tab_group_data`、`dock_data` 与 `tile_data`：它们把各自上下文里的状态部分转成纯 JSON，正是引擎交给脚本代码的那种形态。

## 还没有的东西

- **这两个 trait 的脚本侧。** 还没有引擎实现 `PanelScript` 或 `DockChrome`，所以脚本还不能为自己的面板声明 `serialize()` / `deserialize(data)`，也还画不了标签栏。Rust 侧的 trait 与那几个 JSON 转换函数已经就位，等着引擎接上。
- **脚本自己打开一块面板。** 面板由宿主创建，脚本侧没有 `cx.open_panel(...)`。
- **从脚本改动布局。** 移动、拆分、关闭面板都是宿主的事，走 base 自己的 API。
