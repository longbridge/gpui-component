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

## 核心特点

### 架构：脚本负责描述，宿主负责渲染

脚本从不持有 GPUI 元素，它记录的是元素的**描述**——builder 链上的每一次调用都会往一块 arena 里写入一条操作，等某一帧需要时，Rust 再把这些操作重放成真实元素。布局、绘制、命中测试、滚动、IME 与文本编辑全部留在 Rust，不会回调进脚本。[一次渲染是怎么走完的](#一次渲染是怎么走完的)完整走了一遍这个过程。

引擎是这套设计的一个参数，而不是其中一部分。今天只有 QuickJS 一种，但接缝之上的全部模块——arena、物化器、CallScope、样式表、主题、能力模型、浮层宿主、热重载——源码里都没有出现任何 VM 的名字。见[引擎接缝](./engine.md)。

### 能力：一整层应用层，而不是一套控件

脚本拿到的，正是一个基于 `gpui-base` 的 Rust 应用能拿到的东西：元素与布局、建立在语义主题 token 之上的流式样式接口、通过 `init` / `render` / `cx.notify()` 管理的视图状态、由宿主留存的状态（例如文本输入的 rope 与选区）、dialog / sheet / toast、异步任务，以及受能力门控的系统接口——`fs`、`store`、`clipboard`、`log`、`process`。

围绕它的还有：`--watch` 存盘即重载且不会丢掉窗口，自动生成的 `gpui.d.ts` 把整套 API 描述给编辑器或模型，`check` 则在应用跑起来之前就报出问题。

### 性能：脚本成本按变化次数付，而不是按帧付

`render` **不是**每帧跑一次。它只把界面描述一次、写进一份快照；在下一次 `cx.notify()` 之前的每一次重绘，都由 Rust 重放这份快照，不再进入 VM。指针在界面上移动、光标闪烁、列表滚动、动画推进，全程不执行任何 JavaScript。

一个规模说明不了问题，所以基准测试会走完四档面板：

| 面板规模 | 描述一次（每次变化） | 重绘一次（每帧） | 每次重绘的脚本渲染次数 | 若没有快照，每帧 |
| --- | --- | --- | --- | --- |
| 443 节点 | 1.1 ms | 1.3 ms | **0** | 2.4 ms |
| 2,103 节点 | 5.1 ms | 5.9 ms | **0** | 11.0 ms |
| 4,203 节点 | 10.3 ms | 12.0 ms | **0** | 22.3 ms |
| 8,403 节点 | 20.5 ms | 27.0 ms | **0** | 47.5 ms |

```bash
cargo test -p gpui-shell --release --test benchmark -- --ignored --nocapture
```

最后一列是前两列相加：如果每一帧都要重新描述一遍界面，那一帧就要花这么多。在跨越十九倍的规模区间里，这个比值稳定在 **1.8 倍左右**——也就是说，无论面板多大，"描述界面"都占了朴素做法中约 45% 的帧成本，而这正是快照省掉的部分。

那个 0 不只是观测结果。443 节点那一行由每次 CI 都会跑的测试守住：一旦某次重绘进入了 VM，[基准测试会直接失败](./engine.md#那次实测)，而不是仅仅变慢一点。

这张表同时也说清了天花板在哪，而它不在 JavaScript 一侧：8,403 节点时，一帧要 27 ms，其间没有任何脚本在跑。超过几千个节点之后，账单来自 Rust 侧的物化、布局与绘制，该上的手段是虚拟化，而不是换一个更快的引擎——何况这么大的一个视图，无论用什么来描述它，形状本身就不对。绝对数值取自 Apple Silicon 上的 release 构建，会随机器变化；那个比值不会。

### 安全：默认什么都没有，语言本身也一并收紧

`Capabilities::default()` 是空集——没有文件访问、没有存储、没有剪贴板、不能执行进程、没有网络。授权完全由宿主决定；每个入口都在调用时重新读取授权，因此收回一项能力在下一次调用即刻生效；`fs` 接口上的每一条路径都走**同一个**解析器，任何落在授权根之外的结果都会被拒绝。

在授权之下，沙箱还收紧了语言本身——因为一个 VM 早晚要同时承载多个插件：`eval` 与四个函数编译器全部移除，内置原型被冻结，避免一个插件改动 `Object.prototype` 波及另一个；模块解析被限制在应用目录内；堆（256 MiB）、解释器栈（1 MiB）与单次调用耗时（`render` 为 50 ms）都有上限。其中的耗时上限是一个 `catch` 无法吞掉的中断，这一点由测试保证。见[能力](./capabilities.md)。

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

## 呈现权在脚本一侧

大多数脚本层的做法，是把一批做好的控件交给脚本去摆放。这里没有这样的控件可交，因为它下面那一层同样没有。

`gpui-base` 的控件完全不带视觉样式。Rust 里的 `Button::new("save")` 没有内边距、没有背景、没有圆角、没有尺寸，这是接口约定，不是没做完。JavaScript 绑定原样保留了这一点：`Button.new("save")` 不写样式时，除了它的子元素之外什么都不画。

结论才是重点：**因为基础层不提供任何呈现，呈现权就完整地落在脚本一侧**——颜色、间距、hover 状态、圆角，全部由脚本决定。这与 Rust 应用选择基于 `gpui-base` 而不是 `gpui-component` 时做的取舍完全一样；区别在于，这里的取舍写在一个存盘就能立刻看到结果的文件里，中间不需要 `cargo build`。

多打的字换来的是整个应用层。改一个按钮的圆角，不必再回到 Rust。

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
| [引擎接缝](./engine.md) | QuickJS、接缝存在的理由，以及把脚本成本与帧成本分开的三项实测 |

## 当前状态

该 crate 处于 **M0** 里程碑：一条可行性基线，而不是稳定接口。它没有发布到 crates.io，脚本 API 预计还会变化。本节文档写到的都是已经实现并可用的部分；缺失的部分，会写在你最可能去找它的那一页上。

设计详见 [GPUI Shell 设计文档](https://github.com/longbridge/gpui-component/blob/main/docs/gpui-shell.md)，代码位于 [`crates/shell`](https://github.com/longbridge/gpui-component/tree/main/crates/shell)。
