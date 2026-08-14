# gpui-fps

A realtime performance HUD for [GPUI](https://gpui.rs) applications: frames per
second, frame time, dropped frame rate, and this process' CPU and memory usage.

```
┌──────────────────────────┐
│  ﹋﹏  118 FPS  ﹋︿﹏﹋   │  ← the trace runs behind the headline
│ FRAME             8.4 ms │
│ DROP                0.0% │
│ CPU 12.4%      MEM 84 MB │
└──────────────────────────┘
```

Frame data comes from GPUI's own frame trace (`gpui::FrameTimingCollector`), so
the numbers are what the framework actually spent in `Window::draw` rather than
an estimate measured from the outside. The trace is colored against the frame
budget — green within budget, amber up to twice the budget, red beyond.

It does not depend on `gpui-component`, so it works in any GPUI application.

## Usage

### 1. Add the dependency

```toml
[dependencies]
gpui-fps = { git = "https://github.com/longbridge/gpui-component" }
```

It must resolve to the same `gpui` as your application. Both being git
dependencies on `zed-industries/zed` is enough — Cargo unifies them — but a
`[patch]` or a second checkout that pins a different revision will produce two
incompatible `gpui` crates, and the error will be about mismatched `Window`
types rather than about versions.

### 2. Render it

`fps_monitor` returns an element. Render it wherever the HUD should appear; the
parent must be `relative()`, because the HUD positions itself absolutely.

```rust
use gpui::*;
use gpui_fps::fps_monitor;

struct Example {
    show_fps: bool,
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .size_full()
            .child("your app")
            .when(self.show_fps, |this| this.child(fps_monitor(window, cx)))
    }
}
```

Call it at most once per window — a second call renders the same monitor twice.
The monitor behind it is created on first use and reused afterwards, one per
window, so calling this from `render` every frame is fine.

To show it unconditionally, drop the `when` and just `.child(fps_monitor(window, cx))`.

### 3. Toggle it

There is no `toggle` function. The flag lives with you, which keeps it
observable and easy to persist next to your own settings:

```rust
actions!(example, [ToggleFps]);

// At startup:
cx.bind_keys([KeyBinding::new("cmd-alt-f", ToggleFps, None)]);

// On the element that owns the flag:
div()
    .on_action(cx.listener(|this: &mut Example, _: &ToggleFps, _, cx| {
        this.show_fps = !this.show_fps;
        cx.notify();
    }))
```

If the flag lives in a global instead — so a menu elsewhere in the window can
flip it — remember to `window.refresh()` after changing it, since mutating a
global does not by itself mark the view dirty. That is what the GPUI Component
story does: `AppState.show_fps_monitor`, toggled from **FPS Monitor** in the
settings menu, and persisted to `target/state.json` through an
`observe_global::<AppState>`.

Click the HUD to collapse it to a small tag showing just the frame rate, and
click again to expand.

## Customization

The call takes no options. For a different corner or frame budget, compose the
two pieces it uses and render the monitor yourself — it is an ordinary view, so
it can live in a status bar just as well as in an overlay:

```rust
use gpui_fps::{FpsMonitor, FpsOverlay};

let monitor = cx.new(|cx| {
    FpsMonitor::new(window, cx)
        .capacity(240)                                  // frames kept in the trace (default 120)
        .frame_budget(Duration::from_micros(6_944))     // 144Hz (default is 60Hz)
        .continuous(true)                               // default true, see below
        .show_resources(true)                           // CPU / memory row (default true)
        .resource_interval(Duration::from_millis(500))  // default 500ms
});

// Embedded:
div().child(monitor.clone())
// Or pinned to a corner of a relative parent:
div().relative().child(FpsOverlay::new(&monitor).anchor(Anchor::BottomLeft))
```

The palette is not configurable. Its contrast is load bearing — see the note
below — and an application that could override it could just as easily make the
HUD unreadable.

### `continuous`

On by default, this requests a frame on every render so the window keeps drawing
back to back. That is what makes the reading behave like an in-game FPS counter,
and it carries the same caveat: **the window never idles, so the number is the
frame rate the application can sustain, not the rate it happens to be drawing
at**, and the HUD itself keeps the CPU and GPU busy.

Turn it off to measure the real workload. The HUD then only updates when the
window redraws for its own reasons, and reads zero while the window is idle.

## Notes

- The trace follows every frame, but the numbers are republished twice a second.
  Recomputed per frame they flicker through digits too fast to read. `FRAME` is
  the mean over that interval rather than the latest frame, which at this cadence
  would be an arbitrary sample.
- The frame rate is graded against the target _rate_ with a 5% tolerance, not by
  comparing `1/fps` against the budget. Under vsync a healthy 60Hz display reads
  58 to 60 and never exactly 60.00, so an exact comparison would paint a
  perfectly healthy application as over budget.
- The backdrop is nearly opaque (alpha 0.92) on purpose. GPUI cannot read the
  pixels under an element, so the HUD has no way to adapt to what it covers; the
  only way to stay readable over any window background is to keep that
  background out of the composite. At 0.92 every foreground clears 4.5:1 even
  over pure white.
- GPUI records frame timings into a process-wide buffer, so the monitor filters
  by window id. Each window needs its own monitor to get its own numbers, which
  is what `fps_monitor` does for you.
- Frame tracing is a global switch that clears its buffer when disabled, so
  monitors reference count it and never turn it off while another monitor — or
  the host application's own profiling — still needs it.
- CPU and memory are sampled with `sysinfo` on a background thread; the values
  are for this process, and CPU is normalized so 100 means every logical core is
  saturated. Resource sampling is unavailable on the web.

## Example

```bash
cargo run -p fps_monitor
```

A port of three.js' `webgl_lines_colors` demo — Hilbert curves smoothed with a
centripetal Catmull-Rom spline — whose curve count can be dialed up and down, so
the trace can be watched reacting to real rendering load.
