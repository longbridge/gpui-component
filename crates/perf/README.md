# gpui-perf

A realtime performance HUD for [GPUI](https://gpui.rs) applications: frames per
second, a rolling frame time trace, and this process' CPU and memory usage.

```
┌──────────────────────────┐
│ 118 FPS          8.4 ms  │
│ ╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌  │  ← the frame budget
│ ﹋﹏﹋︿﹏﹋﹏︿﹋﹏﹋﹏﹋  │  ← one point per drawn frame
│ CPU 12.4%      MEM 184 MB│
└──────────────────────────┘
```

Frame data comes from GPUI's own frame trace (`gpui::FrameTimingCollector`), so
the numbers are what the framework actually spent in `Window::draw` rather than
an estimate measured from the outside. The trace is colored against the frame
budget — green within budget, amber up to twice the budget, red beyond — and the
budget itself is drawn as a baseline across the chart.

This crate depends only on `gpui`, so it works in any GPUI application. It is
also re-exported from GPUI Component as `gpui_component::perf`.

## Usage

One call, which decides only *where* the HUD sits. Creating the monitor, keeping
it alive across frames, enabling the frame trace and sampling CPU and memory are
all handled internally.

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
            .child(fps_monitor(Anchor::TopRight, window, cx))
    }
}
```

Any [`gpui::Anchor`] works, so the HUD can sit in a corner or centered on an
edge. Click it to collapse down to the FPS reading, and click again to expand.

## Customization

`fps_monitor` reuses one monitor per window with the default settings. Build an
`FpsMonitor` yourself when you need a different frame budget, palette or
placement, and render it directly (it is an ordinary view, so it can live in a
status bar just as well as in an overlay):

```rust
use gpui_perf::{FpsMonitor, PerfOverlay, PerfStyle};

let monitor = cx.new(|cx| {
    FpsMonitor::new(window, cx)
        .capacity(240)                                  // frames kept in the chart (default 120)
        .frame_budget(Duration::from_micros(6_944))     // 144Hz (default is 60Hz)
        .continuous(true)                               // default true, see below
        .show_resources(true)                           // CPU / memory row (default true)
        .resource_interval(Duration::from_millis(500))  // default 500ms
        .font_family("monospace")                       // keeps digits from shifting
        .style(PerfStyle::light())                      // default PerfStyle::dark()
});

// Embedded:
div().child(monitor.clone())
// Or pinned to a relative parent:
div().relative().child(PerfOverlay::new(&monitor).anchor(Anchor::BottomLeft))
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

Renders animated Bézier ribbons whose count can be dialed up and down, so the
trace can be watched reacting to real rendering load.
