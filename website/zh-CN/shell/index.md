---
title: GPUI Shell
description: 面向 GPUI 的可脚本化应用运行时——应用代码用 JavaScript 编写，渲染与系统能力留在 Rust。
order: 1
---

# GPUI Shell

`gpui-shell` 是构建在 [`gpui-base`](/base/) 之上的可脚本化应用运行时，面向 [GPUI](https://gpui.rs)。应用用 **JavaScript** 编写，运行在宿主进程内的 QuickJS 上。Rust 负责渲染、布局、文本编辑、虚拟化、焦点、浮层以及全部系统能力；脚本负责界面组合、视觉呈现与业务逻辑。

```js
import { View, v_flex, text, Button } from "gpui";

export default class Counter extends View {
  init() {
    this.count = 0;
  }

  render() {
    return v_flex()
      .size_full()
      .items_center()
      .justify_center()
      .gap(20)
      .bg("background")
      .child(text(`${this.count}`).text_3xl().text_color("foreground"))
      .child(
        Button.new("increment")
          .h(32)
          .px(14)
          .items_center()
          .justify_center()
          .bg("primary")
          .text_color("primary_foreground")
          .rounded(6)
          .on_click((_event, cx) => {
            this.count += 1;
            cx.notify();
          })
          .child(text("Increment")),
      );
  }
}
```

## 它与别的脚本层不同在哪

大多数脚本层的做法，是把一批做好的控件交给脚本去摆放。这里不是——因为它下面那一层根本没有做好的控件可交。

`gpui-base` 的控件完全不带视觉样式。Rust 里的 `Button::new("save")` 没有内边距、没有背景、没有圆角、没有尺寸，这是接口约定，不是没做完。JavaScript 绑定原样保留了这一点：`Button.new("save")` 不写样式时，除了它的子元素之外什么都不画。

结论才是重点：**因为基础层不提供任何呈现，呈现权就完整地落在脚本一侧**——颜色、间距、hover 状态、圆角，全部由脚本决定。这与 Rust 应用选择基于 `gpui-base` 而不是 `gpui-component` 时做的取舍完全一样；区别在于，这里的取舍写在一个存盘就能立刻看到结果的文件里，中间不需要 `cargo build`。

多打的字换来的是整个应用层。改一个按钮的圆角，不必再回到 Rust。

## 一次渲染是怎么走完的

<img src="/shell-architecture.svg" alt="脚本如何变成界面：脚本描述元素，Rust 物化为真实元素，GPUI 负责绘制" class="shell-architecture" />

这张图画的是一帧的过程，而这张图的形状基本解释了本节文档的其余部分。

GPUI 的元素是**被消费**的值：`RenderOnce::render` 按值取走 `self`，`.child()` 按值取走子元素，视图每次重绘都从零重建整棵元素树。因此一个 JavaScript 对象永远不可能**就是**一个 GPUI 元素——它没有东西可以长期持有。

所以脚本不构建元素，而是**描述**元素。builder 链上的每一次调用，都会把一条操作记录进一块元素描述 arena；脚本手里的对象只带一个指向 arena 的整数下标。当 GPUI 要求视图渲染时，Rust 把这些记录下来的操作重放成真实元素、交给 GPUI，然后整块清空 arena。布局、绘制、命中测试、滚动与 IME 全程不再回到脚本。

由此直接推出三条结论，每条对应下面一个页面：

- **元素是一次性的。** 描述在本次渲染结束时就消失了，所以被保存下来的元素在下次使用时抛出异常，而不是画出一个意料之外的东西。见 [元素](./elements.md)。
- **`cx` 只属于产生它的那次调用。** 它携带一个代次号，每次使用都与实时的调用栈比对；一个跨过 `await` 仍在使用的 `cx` 会给出明确错误，而不是去访问一个早已失效的栈帧。见 [状态与视图](./state.md)。
- **回调属于注册它的那次渲染。** 下一次渲染会整体替换它们，这正是脚本闭包不会在宿主里堆积的原因。见 [元素](./elements.md)。

这些都不是设计上的花样，而是"把脚本绑到一个会消费其值的元素模型上"必然的结果。

## 适合谁

| 你的情况 | 运行时能给你什么 |
| --- | --- |
| 给已有的 Rust GPUI 应用加面板、命令或工具 | 受沙箱约束、由宿主决定授权的脚本接口，扩展不再意味着 fork |
| 写内部工具——仪表盘、运维面板、数据查看器 | 起步成本低、真实的桌面窗口、存盘即见而不是等编译 |
| 用模型生成界面 | 语料覆盖最好的语言、可恢复而非致命的错误，以及一份自动生成的 `gpui.d.ts` |

它明确**不是**用来把产品核心改写成 JavaScript 的。文本编辑、语法高亮、LSP、虚拟化与动画都留在 Rust。

## 它在架构中的位置

```text
  JavaScript 应用            main.js · views · 样式 · 业务逻辑
            │  import { … } from "gpui"
            ▼
  gpui-shell                 引擎接缝 · 元素描述 · CallScope
                             样式表 · 主题 token · 能力模型
                             ShellRoot（dialog / sheet / toast）· 调度器
            │
            ▼
  gpui-base                  行为 · 状态 · 基础设施（无样式）
            │
            ▼
  gpui / gpui_platform       元素 · 样式 · 渲染 · GPU · 平台
```

`gpui-shell` 与 `gpui-component` 是并列关系，而不是在它下游：两者都是 `gpui-base` 的使用者，都补上了 Base 不提供的那一层呈现。`gpui-component` 用 Rust 提供了一套成品且统一的呈现；`gpui-shell` 提供的是让脚本自己去提供呈现的那套机制。

## 接着读

| 页面 | 内容 |
| --- | --- |
| [快速开始](./getting-started.md) | 运行示例、最小应用、`check` 与 `types` |
| [元素](./elements.md) | 构造器、`child` / `children` / `when`，以及元素为什么是一次性的 |
| [样式](./styling.md) | 流式样式接口、长度与颜色、语义 token、状态样式 |
| [状态与视图](./state.md) | `init` / `render`、`cx.notify()`、留存状态、异步 |
| [浮层](./overlays.md) | dialog、sheet、toast，以及 phase 规则 |
| [能力](./capabilities.md) | 默认全部拒绝的模型，`fs` / `store` / `clipboard` / `log` / `process` |
| [引擎接缝](./engine.md) | QuickJS、LuaJIT 退路，以及决定二者取舍的那次实测 |

## 当前状态

该 crate 处于 **M0** 里程碑：一条可行性基线，而不是稳定接口。它没有发布到 crates.io，脚本 API 预计还会变化。本节文档写到的都是已经实现并可用的部分；缺失的部分，会写在你最可能去找它的那一页上。

设计详见 [GPUI Shell 设计文档](https://github.com/longbridge/gpui-component/blob/main/docs/gpui-shell.md)，代码位于 [`crates/shell`](https://github.com/longbridge/gpui-component/tree/main/crates/shell)。
