---
title: Performance
description: What a script costs once frame rate stops being the variable — invalidation against description size, the view as the boundary that bounds both, and the two failures FPS cannot tell apart.
order: 13
---

# Performance

[The script is not in the frame](./index.md#performance-the-script-is-not-in-the-frame) is the claim the runtime is built on. This page is what follows from it: once a repaint no longer runs JavaScript, the cost that is left has a shape small enough to write down.

```text
script cost  =  how often a view is invalidated  ×  what describing that view costs
```

Neither factor is the frame rate. A window repainting at 120 Hz runs no more JavaScript than one repainting at 30 Hz, and a view nobody has invalidated runs none at all. Both factors are yours: the left one is where you call `cx.notify()`, the right one is how much interface sits behind a single call.

Everything below is one of those two, or a way of telling which is the problem.

## Every view has its own snapshot

GPUI Shell gives each JavaScript view a snapshot of its own: the description that view's `render` produced, kept in Rust.

**A view's snapshot is reused until that view changes.** Every frame in between is drawn from it — turned into GPUI elements, laid out, painted — entirely in Rust. No JavaScript runs.

```text
the view changed  ──▶  render()  ──▶  a new snapshot  ──▶  frame
the view did not  ─────────────────▶  the snapshot it has  ──▶  frame
```

Snapshots are per view, not per window. A window holding a hundred views holds a hundred snapshots, and each one is invalidated on its own:

| What happens | What runs |
| --- | --- |
| `Watchlist` calls `cx.notify()` | `Watchlist.render`, and nothing else |
| The parent calls `cx.notify()` | The parent's `render`. Each child answers the frame from its own snapshot |
| `this.chart.set_props({ symbol })` | That child's `update` and `render`. The parent is not rebuilt |
| A child of a child calls `cx.notify()` | That child's `render`. Invalidation does not travel upward |
| The theme changes | Every view, because a snapshot bakes in the colours it was built with |

<img class="architecture-light" src="/shell-view-invalidation-light.svg" alt="A window drawn as nested views: a sidebar, a watchlist holding four rows that are views of their own, a chart, and a detail pane holding two more. Three phases repeat. A price ticks and only the MSFT row is marked as running its script, while every other view replays the description it already published. The list reorders and the watchlist itself runs while its four rows do not, because a parent records a handle per child rather than the child's description. The theme changes and every view runs at once, because a snapshot bakes in the colours it was built with.">
<img class="architecture-dark" src="/shell-view-invalidation-dark.svg" alt="A window drawn as nested views: a sidebar, a watchlist holding four rows that are views of their own, a chart, and a detail pane holding two more. Three phases repeat. A price ticks and only the MSFT row is marked as running its script, while every other view replays the description it already published. The list reorders and the watchlist itself runs while its four rows do not, because a parent records a handle per child rather than the child's description. The theme changes and every view runs at once, because a snapshot bakes in the colours it was built with.">

## Split a large view into small ones

A view is rebuilt whole. There is no partial rebuild inside one: if a view's description is four hundred nodes, any change rebuilds all four hundred, however small the change was.

That is what makes a large view expensive. Everything it draws shares one snapshot, so the data that changes most often invalidates the parts that never change along with it. In a market terminal, one price moving re-describes the chart, the sidebar and the order book too — not because they changed, but because they sit inside the same view.

Splitting is the fix. Give each part that changes on its own a view of its own with `cx.new`, and a change reaches one snapshot instead of all of them:

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

On the 40-row watchlist this page measures, describing the whole panel costs **0.315 ms** and describing one row costs **0.012 ms** — 361 nodes against 9.

Nesting itself costs almost nothing to weigh against that: a parent records a handle per child, not the child's description. So a complex interface is not, by itself, a performance problem. A large view is.

And splitting for performance means splitting into **views** — not into plugins, applications or processes. Reach for a second application when you want a second *authority*, which is [Capabilities](./capabilities.md), not when you want a second cache.

## Notify what a reader can see

`cx.notify()` is the whole dependency system, and it means one specific thing: **my description is out of date.** It is not an event notification, and using it as one is the most common way to make a script expensive.

A feed handler is the usual case:

```js
onQuote(quote, cx) {
  this.quotes.set(quote.symbol, quote);
  cx.notify();                  // every tick, including the ones nobody is looking at
}
```

If the view draws twenty symbols out of a subscription of two thousand, that `notify` pays for a full description of the panel on every tick of every symbol it does not draw. The fix is a condition, not a faster render:

```js
onQuote(quote, cx) {
  this.quotes.set(quote.symbol, quote);
  if (this.visible.has(quote.symbol)) cx.notify();
}
```

Three rules follow from the same idea:

- **Invalidate the view that changed.** State that belongs to one child should live on that child and be notified there, rather than on the parent that mounts it.
- **Several `notify` calls in one handler collapse into one render.** Batching by hand buys nothing; conditioning does.
- **From the host, `cx.notify()` and `ScriptView::refresh` are different requests.** A bare `notify` repaints the description that already exists. If Rust changed state the script reads through a [HostModule](./host-module.md), the description is stale and only `refresh` says so. See [Hosting](./hosting.md#refreshing-a-view-from-host-state).

## Frame rate and presentation latency are different failures

Two things can be wrong with a running interface, and only one of them shows up as FPS:

```text
Rendering FPS          is the frame smooth?
State → presentation   how long after state changes does the reader see it?
```

Missing a `cx.notify()` costs no frames at all. GPUI keeps replaying the last good description at full rate, so the HUD reads a steady 120 FPS while the interface is showing something that stopped being true — and then jumps a quarter of a second later, when something unrelated invalidates the view. Every rendering measurement calls this healthy.

| Symptom | Which number is wrong | Usual cause |
| --- | --- | --- |
| The window stutters while nothing in the application is changing | FPS | Description too large per frame, or a virtual list doing per-row work; see [the measurement](./engine.md#the-measurement) |
| The window stutters while a feed is running | FPS *and* invalidation | One boundary being rebuilt too often, too large, or both |
| The window is smooth and the data is late | Presentation latency | A `notify` that was skipped, deferred behind an `await`, or issued as a host `cx.notify()` where `refresh` was meant |

Diagnose them separately. An FPS reading that never dropped is not evidence that invalidation is correct.

## Reading the counters

The runtime counts the two events apart, and the host can read them with `runtime.read_metrics()` — see [Watching what it costs](./hosting.md#watching-what-it-costs) for the API and the delta-against-a-baseline pattern that turns them into per-second rates.

| Reading | The question it answers |
| --- | --- |
| `script_renders()` | How often JavaScript ran. Follows `cx.notify()`, reloads and theme changes — never frames |
| `materializations()` | How often a snapshot became elements. Follows frames |
| `mean_script_render()` | What one description costs, host calls included |
| `mean_native()` | How much of that was inside HostModule functions rather than describing |
| `slowest_script_render()` | The worst single build in the run |
| `frame_script_calls()` | Entries into the VM from the frame path — [virtual list](./elements.md) item renderers and [dock](./dock.md) chrome handlers, which are the only two |
| `structure_repeat_rate()` | Of the rebuilds that had a predecessor, what fraction described the same *shape* — see below |

What the shape of a reading says:

- **`script_renders` per second far above the rate the data actually changes** — a `notify` is firing on things the reader cannot see. Condition it.
- **`script_renders` reasonable, `mean_script_render` high** — the boundary is too large. Split the view.
- **`mean_native` most of `mean_script_render`** — the cost is in the host functions the description calls, not in the description. Read them once into fields before `render`, not per node.
- **`slowest_script_render` far above the mean** — one build is paying for something the rest are not: a collection materialized on first render, or a rarely-taken branch that describes far more than the common one. A mean that drifts as a whole is system load instead.

## Where the snapshot cache stops

The snapshot removes the cost of **no change**. It does not remove the cost of a **small change**.

A snapshot holds structure and values together:

```text
StockRow
├── Symbol("AAPL")
├── Price("230.42")
└── Change("+1.42%")
```

When the price becomes `230.51`, the structure is identical and only one leaf differs — but a new description is the only way to say so, so the whole view is described again: every `div()`, every `.gap()`, every `.bg()`, every crossing into Rust. That is the dirty-render path, and on a fast feed it is the one that runs.

<img class="architecture-light" src="/shell-change-cost-light.svg" alt="Three lanes, bars to the same scale. Nothing the view reads changed: no bar, no script runs, the frame replays the description already published. A value changed, which is what happens today: the whole panel is described again at 0.315 milliseconds, however small the change. The same change with the row a retained view of its own: 0.012 milliseconds, about a twenty-sixth as long, because nine nodes are described instead of 361.">
<img class="architecture-dark" src="/shell-change-cost-dark.svg" alt="Three lanes, bars to the same scale. Nothing the view reads changed: no bar, no script runs, the frame replays the description already published. A value changed, which is what happens today: the whole panel is described again at 0.315 milliseconds, however small the change. The same change with the row a retained view of its own: 0.012 milliseconds, about a twenty-sixth as long, because nine nodes are described instead of 361.">

The lever is the one this page opens with: **shrink the boundary that has to be rebuilt.** On the watchlist above, describing the whole panel costs 0.315 ms and describing one row costs 0.012 ms — 361 nodes against 9. Putting the row behind a view of its own is what turns the first number into the second, and it is available today.

`structure_repeats()` and `structure_changes()` are how you check that the boundary is doing what you think. They count how often a rebuild produced the same *shape* as the description it replaced, differing only in the values inside it. A panel reporting a low rate is worth knowing about on its own: something in it is changing structure when you thought only a number was.
