# gpui-perf

A realtime performance HUD for [GPUI](https://gpui.rs) applications: frames per
second, a rolling frame time trace, and this process' CPU and memory usage.

```
┌──────────────────────────┐
│ ﹋﹏  118 FPS  ﹋︿﹏﹋   │  ← the trace runs behind the headline
│ FRAME             8.4 ms │
│ DROP                0.0% │
│ CPU 12.4%     MEM 184 MB │
└──────────────────────────┘
```

Frame data comes from GPUI's own frame trace (`gpui::FrameTimingCollector`), so
the numbers are what the framework actually spent in `Window::draw` rather than
an estimate measured from the outside. The trace is colored against the frame
budget — green within budget, amber up to twice the budget, red beyond.

This crate depends only on `gpui`, so it works in any GPUI application.

## Usage

Mount it once in the window's root view:

```rust
use gpui::*;
use gpui_perf::fps_monitor;

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            // The HUD positions itself absolutely, so the parent must be relative.
            .relative()
            .size_full()
            .child("your app")
            .child(fps_monitor(window, cx))
    }
}
```

and switch it on and off from anywhere:

```rust
gpui_perf::toggle_fps(window, cx);
```

Nothing is drawn until it is switched on, so the mount point can be left in
place permanently. GPUI draws only what the element tree produces — `Window`'s
paint methods assert they are called from an element's paint phase — so there is
no way to put the HUD on screen without a mount point somewhere; this is the
smallest one.

Click the HUD to collapse it down to the FPS reading, and click again to expand.

## Customization

Neither call takes options. For a different corner or frame budget, compose the
two pieces they use and render the monitor yourself (it is an ordinary view, so
it can live in a status bar just as well as an overlay):

```rust
use gpui_perf::{FpsMonitor, FpsOverlay};

let monitor = cx.new(|cx| {
    FpsMonitor::new(window, cx)
        .capacity(240)                                  // frames kept in the chart (default 120)
        .frame_budget(Duration::from_micros(6_944))     // 144Hz (default is 60Hz)
        .continuous(true)                               // default true, see below
        .show_resources(true)                           // CPU / memory row (default true)
        .resource_interval(Duration::from_millis(500))  // default 500ms
        .font_family("monospace")                       // keeps digits from shifting
});

// Embedded:
div().child(monitor.clone())
// Or pinned to a corner of a relative parent:
div().relative().child(FpsOverlay::new(&monitor).anchor(Anchor::BottomLeft))
```

### `continuous`

On by default, this requests a frame on every render so the window keeps drawing
back to back. That is what makes the reading behave like an in-game FPS counter,
and it carries the same caveat: **the window never idles, so the number is the
frame rate the application can sustain, not the rate it happens to be drawing
at**, and the HUD itself keeps the CPU and GPU busy.

Turn it off to measure the real workload. The HUD then only updates when the
window redraws for its own reasons, and reads zero while the window is idle.

## Notes

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
