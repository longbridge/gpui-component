---
title: 性能
description: 当帧率不再是变量之后，脚本真正的成本——失效频率乘以描述规模、把两者都框住的视图边界，以及 FPS 分辨不出来的那两类问题。
order: 13
---

# Performance

[脚本不在每一帧里](./index.md#性能-脚本不在每一帧里)是这个运行时立身的那句话。这一页讲的是它的推论：一旦重绘不再执行 JavaScript，剩下的成本就只剩一个足够简单的形状。

```text
脚本成本  =  一个视图多久失效一次  ×  描述这个视图要花多少
```

两个因子都不是帧率。以 120 Hz 重绘的窗口，执行的 JavaScript 不会比 30 Hz 的多；没有人让它失效的视图，一行都不执行。而这两个因子都在你手里：左边是你在哪里调用 `cx.notify()`，右边是一次 `notify` 背后压了多少界面。

下面每一节，要么是这两个因子之一，要么是分辨问题出在哪一个。

## 每个视图都有自己的 snapshot

GPUI Shell 给每一个 JavaScript 视图一份属于它自己的 snapshot：这个视图 `render` 产出的那份描述，保存在 Rust 一侧。

**只要视图本身没有变化，它的 snapshot 就一直被复用。** 中间的每一帧都从这份 snapshot 画出来——转成 GPUI 元素、布局、绘制——全部在 Rust 里完成，不执行任何 JavaScript。

```text
视图变了    ──▶  render()  ──▶  新的 snapshot  ──▶  帧
视图没变    ─────────────────▶  已有的那份 snapshot  ──▶  帧
```

snapshot 是按视图存的，不是按窗口存的。一个窗口里有一百个视图，就有一百份 snapshot，各自独立失效：

| 发生了什么 | 会执行什么 |
| --- | --- |
| `Watchlist` 调用 `cx.notify()` | `Watchlist.render`，其余什么都不跑 |
| 父视图调用 `cx.notify()` | 父视图的 `render`。每个子视图用自己的 snapshot 回答这一帧 |
| `this.chart.set_props({ symbol })` | 那个子视图的 `update` 与 `render`。父视图不重建 |
| 子视图的子视图调用 `cx.notify()` | 那个子视图的 `render`。失效不会向上传播 |
| 主题切换 | 每一个视图——因为 snapshot 里烘进了它构建时的颜色 |

<img class="architecture-light" src="/shell-view-invalidation-light.svg" alt="一个窗口，画成互相嵌套的视图：侧栏、一块装着四行（每行本身也是视图）的自选清单、图表，以及装着两个子视图的详情面板。三个阶段循环。价格跳动时，只有 MSFT 那一行被标为「脚本在执行」，其余每个视图都重放各自已发布的描述。列表重排时，自选清单本身执行，而它的四行不执行——父视图记录的是每个子视图的一个句柄，不是子视图的描述。主题切换时所有视图同时执行，因为 snapshot 里烘进了它构建时的颜色。">
<img class="architecture-dark" src="/shell-view-invalidation-dark.svg" alt="一个窗口，画成互相嵌套的视图：侧栏、一块装着四行（每行本身也是视图）的自选清单、图表，以及装着两个子视图的详情面板。三个阶段循环。价格跳动时，只有 MSFT 那一行被标为「脚本在执行」，其余每个视图都重放各自已发布的描述。列表重排时，自选清单本身执行，而它的四行不执行——父视图记录的是每个子视图的一个句柄，不是子视图的描述。主题切换时所有视图同时执行，因为 snapshot 里烘进了它构建时的颜色。">

## 把大视图拆成小视图

视图是整体重建的，内部没有局部重建：如果一个视图的描述有四百个节点，那么任何一点变化都会把这四百个节点全部重建一遍，无论变化多小。

这就是大视图贵的原因。它画的所有东西共用一份 snapshot，于是变化最频繁的那部分数据，会连带让那些从不变化的部分一起失效。在一个行情终端里，一个价格动一下，图表、侧栏、盘口也会被重新描述一遍——不是因为它们变了，而是因为它们和价格在同一个视图里。

拆分就是解法。把各自独立变化的部分用 `cx.new` 拆成各自的视图，一次变化就只会落到一份 snapshot 上，而不是全部：

```js
import { View } from "gpui";

export default class Terminal extends View {
  init(props, cx) {
    this.sidebar = cx.new(Sidebar);
    this.watchlist = cx.new(Watchlist, { symbols: props.symbols });
    this.chart = cx.new(PriceChart, { symbol: props.symbols[0] });
    this.detail = cx.new(Detail, { symbol: props.symbols[0] });
  }

  render() {
    return h_flex()
      .child(this.sidebar)
      .child(this.watchlist)
      .child(v_flex().child(this.chart).child(this.detail));
  }
}
```

在本页测量的那块 40 行看板上，描述整块面板要 **0.315 ms**，描述其中一行只要 **0.012 ms**——361 个节点对 9 个。

嵌套本身的开销和这个差距比几乎可以忽略：父视图为每个子视图记录的是一个句柄，不是子视图的描述。所以界面复杂本身不是性能问题，视图太大才是。

而且「为了性能而拆」指的是拆成**视图**，不是拆成多个插件、多个应用或多个进程。需要第二个应用，是因为你想要第二份**授权**，那是 [Capabilities](./capabilities.md) 的事，而不是因为你想要第二份缓存。

## 只为用户看得见的变化 notify

`cx.notify()` 就是这里全部的依赖系统，而它只表达一件事：**我的描述过期了。** 它不是事件通知，把它当事件通知用，是让脚本变贵的最常见方式。

行情回调是典型场景：

```js
onQuote(quote, cx) {
  this.quotes.set(quote.symbol, quote);
  cx.notify();                  // 每一跳都通知，包括没人在看的那些
}
```

如果这个视图从两千只订阅里只画二十只，这句 `notify` 会为它根本没画的标的的每一跳，付一次完整的面板描述。解法是一个条件，不是更快的 render：

```js
onQuote(quote, cx) {
  this.quotes.set(quote.symbol, quote);
  if (this.visible.has(quote.symbol)) cx.notify();
}
```

同一个想法推出三条规则：

- **让变化的那个视图失效。** 只属于某个子视图的状态，就应该放在那个子视图上、在那里 notify，而不是放在挂载它的父视图上。
- **一次回调里的多次 `notify` 会合并成一次 render。** 手动攒批换不来什么，加条件才有用。
- **在 Host 一侧，`cx.notify()` 与 `ScriptView::refresh` 是两个不同的请求。** 单纯的 `notify` 只是重绘已有的描述。如果 Rust 改的是脚本通过 [HostModule](./host-module.md) 读到的状态，那描述已经陈旧，只有 `refresh` 能说明这一点。见 [Hosting](./hosting.md#host-状态变了-怎么刷新视图)。

## 帧率与呈现延迟是两类问题

一个运行中的界面可能出两种问题，而只有一种会体现在 FPS 上：

```text
渲染帧率        画面流畅吗？
状态 → 呈现     状态变了以后，多久用户才看得到？
```

漏掉一次 `cx.notify()` 一帧都不会掉。GPUI 会继续以满帧率重放上一份完好的描述，于是 HUD 稳稳地读出 120 FPS，而界面显示的东西早就不成立了——然后在四分之一秒后，因为某件不相干的事让视图失效，画面突然跳一下。所有渲染指标都会把这种情况判为健康。

| 症状 | 哪个数字不对 | 常见原因 |
| --- | --- | --- |
| 应用里什么都没变，窗口却卡 | 帧率 | 每帧要物化的描述过大，或虚拟列表在按行做额外工作；见[那次实测](./engine.md#那次实测) |
| 行情在跑的时候窗口卡 | 帧率**和**失效频率 | 某个边界重建得太频繁、太大，或两者都有 |
| 画面很流畅，但数据慢半拍 | 呈现延迟 | 某次 `notify` 被漏掉、被压在 `await` 之后，或该用 `refresh` 的地方用了 Host 侧的 `cx.notify()` |

这两件事要分开诊断。FPS 从没掉过，并不能证明失效逻辑是对的。

## 怎么读那几个计数器

运行时把这两类事件分开计数，Host 用 `runtime.read_metrics()` 读取——接口本身以及“留一个基线再相减”得到每秒速率的用法，见[观察它花了多少](./hosting.md#观察它花了多少)。

| 读数 | 它回答什么 |
| --- | --- |
| `script_renders()` | JavaScript 执行了多少次。跟着 `cx.notify()`、hot-reload 与主题切换走，永远不跟帧走 |
| `materializations()` | snapshot 变成元素多少次。跟着帧走 |
| `mean_script_render()` | 一次描述要花多少，包含其中的 Host 调用 |
| `mean_native()` | 其中有多少是在 HostModule 函数里，而不是在描述界面 |
| `slowest_script_render()` | 这一段里最慢的那一次构建 |
| `frame_script_calls()` | 从帧路径进入 VM 的次数——只有[虚拟列表](./elements.md)的 item 渲染器与 [Dock](./dock.md) 的 chrome 回调会计入 |
| `structure_repeat_rate()` | 在有上一份描述可比的重建里，有多大比例产出了相同的**结构**——见下 |

一份读数的形状说明什么：

- **每秒 `script_renders` 远高于数据实际变化的频率**——`notify` 正在为用户看不见的东西触发。加条件。
- **`script_renders` 正常，但 `mean_script_render` 高**——边界太大。把视图拆开。
- **`mean_native` 占了 `mean_script_render` 的大部分**——成本在描述过程中调用的那些 Host 函数上，而不在描述本身。在 `render` 之前把它们一次性读进字段，不要按节点调用。
- **`slowest_script_render` 远高于均值**——某一次构建付了其余各次没付的东西：首次渲染物化的一份集合，或一个很少走到、却描述得多得多的分支。如果是均值整体在漂，那是系统负载，不是这个。

## snapshot 缓存止步于哪里

snapshot 消除的是**没有变化**的成本，它不消除**变化很小**的成本。

一份 snapshot 把结构和取值一起存着：

```text
StockRow
├── Symbol("AAPL")
├── Price("230.42")
└── Change("+1.42%")
```

当价格变成 `230.51`，结构完全一样，只有一个叶子不同——但要表达这一点，唯一的办法就是产出一份新的描述，于是整个视图被重新描述一遍：每个 `div()`、每个 `.gap()`、每个 `.bg()`、每一次进入 Rust 的跨越。这就是 dirty render 那条路径，行情一快，跑的就是它。

<img class="architecture-light" src="/shell-change-cost-light.svg" alt="三条泳道，长条按同一比例绘制。视图读到的东西没变：没有长条，不执行任何脚本，这一帧重放已经发布的描述。取值变了，也就是今天的情形：无论变化多小，整块面板都被重新描述一遍，0.315 毫秒。同样的变化，若这一行本身是一个留存的视图：0.012 毫秒，约为二十六分之一，因为描述的是 9 个节点而不是 361 个。">
<img class="architecture-dark" src="/shell-change-cost-dark.svg" alt="三条泳道，长条按同一比例绘制。视图读到的东西没变：没有长条，不执行任何脚本，这一帧重放已经发布的描述。取值变了，也就是今天的情形：无论变化多小，整块面板都被重新描述一遍，0.315 毫秒。同样的变化，若这一行本身是一个留存的视图：0.012 毫秒，约为二十六分之一，因为描述的是 9 个节点而不是 361 个。">

可用的杠杆就是本页开头那一个：**把必须重建的边界缩小。** 在上面那块看板上，描述整块面板 0.315 ms，描述其中一行 0.012 ms——361 个节点对 9 个。把这一行放进它自己的视图，就是把前一个数字变成后一个，而这是今天就能做的。

`structure_repeats()` 与 `structure_changes()` 是用来核对边界有没有在做你以为的事的。它们统计一次重建产出的**结构**与被替换那份是否相同——只有其中的取值不同。如果某块面板报出来的比例很低，这件事本身就值得知道：你以为只有一个数字在变，实际上有东西在改变结构。
