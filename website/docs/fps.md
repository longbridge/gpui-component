---
title: FPS Monitor
description: Read the gpui-fps HUD — what MAX FPS is, why it is derived rather than counted, and what each row measures.
order: -5
---

# FPS Monitor

`gpui-fps` overlays a performance HUD on a window: a headline rate, a rolling
frame time trace, and this process' CPU, GPU and memory. It depends only on
`gpui`, so any GPUI application can use it.

```rs
use gpui_fps::fps_monitor;

fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
        .relative()
        .size_full()
        .child(self.content.clone())
        .when(self.show_fps, |this| this.child(fps_monitor(window, cx)))
}
```

The parent must be `relative()`, the HUD positions itself absolutely, and
whether it is on screen is the caller's to decide.

## The headline

The big figure answers one of two questions, and the `MAX` marker says which.
**Right-click** to switch; **click** to collapse the HUD to a tag.

| | Reads | Means |
| --- | --- | --- |
| `MAX FPS` (default) | `1 / FRAME`, capped by the display | The rate a full redraw of this window could sustain |
| `FPS` | Frames presented per second | The rate the window is actually drawing at |

They are different questions, and an application that draws on demand answers
them very differently: a window sitting idle draws twice a second and could
draw a hundred and twenty times a second, and only one of those numbers is a
performance problem.

### Why MAX is derived rather than counted

The obvious way to make a frame counter read "as fast as this UI can go" is to
keep asking for frames, the way an in-game counter does. That is not free here.
Marking any view dirty schedules a **window** draw, and GPUI re-renders every
view in that window outside an [`Entity::cached`] boundary — so each frame the
HUD asked for would be a full layout and paint of the application, and the CPU
row underneath would be reporting work the HUD itself was causing. On the story
gallery's Table page that was ~62% CPU with nobody touching the window.

The frame cost already answers the question. `FRAME` is what a full redraw
costs, so its reciprocal is the rate those redraws could sustain, and nothing
has to be drawn to find it. The HUD never requests a frame.

### Why MAX is capped by the display

Counting presents had a ceiling for free: frames go to the compositor on vsync,
so a counted rate can never exceed the refresh rate. A derived figure has no
such ceiling — a frame drawn in 3ms reads as 333, a rate nobody could ever see
— so the cap is applied explicitly.

GPUI does not expose the refresh rate, so the sampler infers it from the gaps
between presents:

- Gaps outside 3ms–50ms are ignored. Below is a compositor catch-up burst, not
  a refresh; above is the application having had nothing to draw.
- The rest are grouped to half a millisecond and counted. A gap has to recur
  before it means anything: two presents 5.9ms apart on a 144Hz panel is a
  hiccup, not a 169Hz display.
- The estimate is the mean of the busiest group and its neighbours, because
  bucketing truncates the group it is measuring and the busiest bucket alone
  reads high.
- A faster group is preferred when it is at least twice as fast and arrives in
  bulk. A variable refresh panel spends most of its time below its ceiling: a
  ProMotion window that scrolls at 120Hz and rests at 60 must be capped at 120,
  not at the rate it happened to rest at.
- The result is snapped to a standard refresh rate when it lands within 2.5% of
  one, so a 144Hz panel reads 144 rather than 146. A panel that ships at
  nothing standard keeps its own rate rather than being rounded up to a ceiling
  it does not have.

Until the window has presented back to back often enough for that to mean
something — which a window nobody has touched never does — there is no cap, and
`MAX` is whatever the frame cost implies.

## The rows

| Row | Measures |
| --- | --- |
| `INTERVAL` | Mean time between presents. The same figure a platform overlay calls its frame interval, and the reciprocal of `FPS`. A wide gap between it and `MAX` is an idle window, not a slow one. |
| `FRAME` | Mean `Window::draw` cost. Graded against the frame budget: this is the row to read when something feels slow. |
| `P95` | The slow tail of the same frames, graded the same way. |
| `DROP` | Share of frames that overran the budget. |
| `INV` | Invalidations coalesced into one frame. Well above one means the window was asked to redraw more often than it could. |
| `CPU` | This process, on the scale `top` and Activity Monitor use: 100 is one saturated core, so a process spread across a core and a half reads 140. |
| `MEM` | Resident set. |

`FRAME`, `P95` and `DROP` are graded against the budget set by
`frame_budget()`, which defaults to one 60Hz frame. Set it to `1/144s` on a
high refresh rate display, or the chart will grade healthy frames amber.

## The first frames are not measured

A window's first frames are its most expensive — shaders, the glyph atlas, the
icons, every cache still cold — and they are not what the application costs to
run. One of them is a hundred milliseconds against a budget of sixteen, and a
HUD that has seen eight frames would report it as a twelfth of the window's
work, in amber, before the reader has done anything at all.

So the sampler discards two things: everything GPUI recorded before the HUD was
mounted, which is either somebody else's history or the cold start, and the
first few frames after it. The default reading of a window that just opened is
a healthy one.

## What the HUD itself costs

One frame every 500ms. It does not drive the frame loop, but it does need a
clock — nothing else would wake a HUD in a window that has stopped drawing, and
the figures would freeze at whatever the application last drew. That clock also
carries the CPU, GPU and memory sample.

[`Entity::cached`]: https://docs.rs/gpui/latest/gpui/struct.Entity.html
