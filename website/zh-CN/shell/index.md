---
title: GPUI Shell
description: 为 Rust GPUI 应用提供 JavaScript 扩展点——脚本能碰到什么由宿主授予，渲染与系统能力留在 Rust。
order: 1
---

# GPUI Shell

`gpui-shell` 为 Rust 的 [GPUI](https://gpui.rs) 应用提供 **JavaScript 扩展点**：由宿主构建运行时、决定脚本能碰到什么，而脚本在同一个进程里画出真正的界面。它直接构建在 [`gpui-base`](/base/) 之上，QuickJS 跑在宿主自己的线程上。Rust 负责渲染、布局、文本编辑、虚拟化、焦点、浮层以及全部系统能力；脚本负责界面组合、视觉呈现与业务逻辑。脚本也可以单独跑起来，开发一段脚本时通常就是这么做的。

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

## 核心特点

### 架构：脚本负责描述，宿主负责渲染

脚本从不持有 GPUI 元素，它记录的是元素的**描述**——builder 链上的每一次调用都会往一块 arena 里写入一条操作，等某一帧需要时，Rust 再把这些操作重放成真实元素。布局、绘制、命中测试、滚动、IME 与文本编辑全部留在 Rust，不会回调进脚本。[一次渲染是怎么走完的](#一次渲染是怎么走完的)完整走了一遍这个过程。

引擎是这套设计的一个参数，而不是其中一部分。今天只有 QuickJS 一种，但接缝之上的全部模块——arena、把描述变成真实元素的 `materialize`、CallScope、样式表、主题、能力模型、浮层宿主、热重载——源码里都没有出现任何 VM 的名字。见 [The Engine Seam](./engine.md)。

### 能力：一整层应用层，而不是一套控件

脚本拿到的，正是一个基于 `gpui-base` 的 Rust 应用能拿到的东西：元素与布局、建立在语义主题 token 之上的流式样式接口、通过 `init` / `render` / `cx.notify()` 管理的视图状态、由宿主留存的状态（例如文本输入的 rope 与选区）、dialog / sheet / toast、异步任务，以及需要授权才能用的系统接口——`fs`、`store`、`clipboard`、`log`、`process`。

围绕它的还有：`--watch` 存盘即重载且不会丢掉窗口，自动生成的 `gpui.d.ts` 把整套 API 描述给编辑器或模型，`check` 则在应用跑起来之前就报出问题。

### 性能：脚本不在每一帧里

`render` **不是**每帧跑一次。它把界面描述一次、存进一份 snapshot；在下一次 `cx.notify()` 之前，每一次重绘都由 Rust 重放这份 snapshot。指针划过按钮、光标闪烁、列表滚动、动画推进，这些重绘都不执行 JavaScript。

运行时把两件事分开计数，gallery 的 Shell story（`cargo run -- shell`）把这两个数摆在界面上：

<img class="architecture-light" src="/shell-render-frequency-light.svg" alt="一秒内的一块实时面板。JavaScript 的数据没有变化时，60 帧全部触发，而 JavaScript 那一行始终是空的；价格每 50 ms 变动一次时，仍是 60 帧，JavaScript 触发约 20 次。">
<img class="architecture-dark" src="/shell-render-frequency-dark.svg" alt="一秒内的一块实时面板。JavaScript 的数据没有变化时，60 帧全部触发，而 JavaScript 那一行始终是空的；价格每 50 ms 变动一次时，仍是 60 帧，JavaScript 触发约 20 次。">


| 界面在做什么 | 每秒画的帧 | 每秒跑的 JavaScript |
| --- | --- | --- |
| 只是重绘，JavaScript 的数据没有变化 | 60 | 0 |
| 价格每 50 ms 变动一次 | 60 | 19 |

帧数取决于屏幕，JavaScript 的次数取决于数据。第二行里另外 41 帧重放的是已有的描述。

成本因此按用户操作计，而不是按帧计。443 节点的面板，跑一遍 `render`、把整个界面记进 snapshot 要 1.1 ms，只在状态变化时付；之后每一帧 1.3 ms，那是渲染本身——把 snapshot 变成元素、布局、绘制，其中没有 JavaScript。

没有 snapshot 时，每一帧都要重跑一次 `render`，一帧就是 1.1 + 1.3 = 2.4 ms；有 snapshot 时，1.1 ms 只在状态变化的那一次付，其余每帧 1.3 ms。

面板变大也不改变这条性质：[基准测试](./engine.md#那次实测)覆盖到 8,403 个节点，各档的每一帧都不执行 JavaScript，最小一档由每次 CI 运行的断言保证。

### 安全：默认什么都没有，语言本身也一并收紧

`Capabilities::default()` 是空集——没有文件访问、没有存储、没有剪贴板、不能执行进程、没有网络。授权完全由宿主决定；每个入口都在调用时重新读取授权，因此收回一项能力在下一次调用即刻生效；`fs` 接口上的每一条路径都走**同一个**解析器，任何落在授权根之外的结果都会被拒绝。

在授权之下，沙箱还收紧了语言本身——因为一个 VM 早晚要同时承载多个插件：`eval` 与四个函数编译器全部移除，内置原型被冻结，避免一个插件改动 `Object.prototype` 波及另一个；模块解析被限制在应用目录内；堆（256 MiB）、解释器栈（1 MiB）与单次调用耗时（`render` 为 50 ms）都有上限。其中的耗时上限是一个 `catch` 无法吞掉的中断，这一点由测试保证。见 [Capabilities](./capabilities.md)。

## 一次渲染是怎么走完的

<img src="/shell-architecture.svg" alt="脚本如何变成界面：脚本描述元素，Rust 把它们变成真实元素，GPUI 负责绘制" class="shell-architecture" />

这张图画的是一帧的过程，而这张图的形状基本解释了本节文档的其余部分。

GPUI 的元素是**被消费**的值：`RenderOnce::render` 按值取走 `self`，`.child()` 按值取走子元素，视图每次重绘都从零重建整棵元素树。因此一个 JavaScript 对象永远不可能**就是**一个 GPUI 元素——它没有东西可以长期持有。

所以脚本不构建元素，而是**描述**元素。builder 链上的每一次调用，都会把一条操作记录进一块元素描述 arena；脚本手里的对象只带一个指向 arena 的整数下标。当 GPUI 要求视图渲染时，Rust 把这些记录下来的操作重放成真实元素、交给 GPUI，然后整块清空 arena。布局、绘制、命中测试、滚动与 IME 全程不再回到脚本。

由此直接推出三条结论，每条对应下面一个页面：

- **元素是一次性的。** 描述在本次渲染结束时就消失了，所以被保存下来的元素在下次使用时抛出异常，而不是画出一个意料之外的东西。见 [Elements](./elements.md)。
- **`cx` 只属于产生它的那次调用。** 它带着一个 generation 编号，每次使用都与实时的调用栈比对；一个跨过 `await` 仍在使用的 `cx` 会给出明确错误，而不是去访问一个早已失效的栈帧。见 [State and Views](./state.md)。
- **回调属于注册它的那次渲染。** 下一次渲染会整体替换它们，这正是脚本闭包不会在宿主里堆积的原因。见 [Elements](./elements.md)。

这些都不是设计上的花样，而是“把脚本绑到一个会消费其值的元素模型上”必然的结果。

## 呈现权在脚本一侧

大多数脚本层的做法，是把一批做好的控件交给脚本去摆放。这里没有这样的控件可交，因为它下面那一层同样没有。

`gpui-base` 的控件完全不带视觉样式。Rust 里的 `Button::new("save")` 没有内边距、没有背景、没有圆角、没有尺寸，这是接口约定，不是没做完。JavaScript 绑定原样保留了这一点：`Button.new("save")` 不写样式时，除了它的子元素之外什么都不画。

结论才是重点：**因为基础层不提供任何呈现，呈现权就完整地落在脚本一侧**——颜色、间距、hover 状态、圆角，全部由脚本决定。这与 Rust 应用选择基于 `gpui-base` 而不是 `gpui-component` 时做的取舍完全一样；区别在于，这里的取舍写在一个存盘就能立刻看到结果的文件里，中间不需要 `cargo build`。

多打的字换来的是整个应用层。改一个按钮的圆角，不必再回到 Rust。

## 适用场景

| | 运行时提供什么 |
| --- | --- |
| **为已有的 GPUI 应用增加插件能力** | 插件跑在宿主进程内，能力由宿主一项一项授予，起点是什么都没有。扩展产品不再意味着 fork 或者发一个新版本。 |
| **基于 `gpui-shell` 编写纯 JavaScript 的应用** | 整个应用层——元素、样式、视图状态、浮层与系统接口——都在 JavaScript 一侧，而渲染、文本编辑、虚拟化与动画仍留在 Rust。 |
| **为应用提供动态扩展能力** | 界面与业务逻辑以脚本形式交付，改动不需要重新编译、也不需要重新分发二进制；脚本出错会呈现为一个可恢复的错误，而不是把宿主一起带走。 |

文本编辑、语法高亮、LSP、虚拟化与动画都留在 Rust。这是分工，不是分工里的缺口：脚本负责组合与呈现，宿主负责所有必须贴着 GPU 与系统运行的部分。

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
| [Getting Started](./getting-started.md) | 运行示例、最小应用、`check` 与 `types` |
| [Elements](./elements.md) | 构造器、`child` / `children` / `when`，以及元素为什么是一次性的 |
| [Styling](./styling.md) | 流式样式接口、长度与颜色、语义 token、状态样式 |
| [State and Views](./state.md) | `init` / `render`、`cx.notify()`、留存状态、异步 |
| [Overlays](./overlays.md) | dialog、sheet、toast，以及 phase 规则 |
| [Capabilities](./capabilities.md) | 默认全部拒绝的模型，`fs` / `store` / `clipboard` / `log` / `process` |
| [The Engine Seam](./engine.md) | QuickJS、接缝存在的理由，以及把脚本成本与帧成本分开的三项实测 |

## 当前状态

该 crate 处于 **M0** 里程碑：一条可行性基线，而不是稳定接口。它没有发布到 crates.io，脚本 API 预计还会变化。本节文档写到的都是已经实现并可用的部分；缺失的部分，会写在你最可能去找它的那一页上。

设计详见 [GPUI Shell 设计文档](https://github.com/longbridge/gpui-component/blob/main/docs/gpui-shell.md)，代码位于 [`crates/shell`](https://github.com/longbridge/gpui-component/tree/main/crates/shell)。
