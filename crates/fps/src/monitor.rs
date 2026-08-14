use std::time::Duration;

use instant::Instant;

use gpui::{
    Bounds, Context, Div, Hsla, InteractiveElement as _, IntoElement, ParentElement, PathBuilder,
    Pixels, Point, Render, StatefulInteractiveElement as _, Styled, Task, Window, canvas, div,
    point, prelude::FluentBuilder as _, px, relative,
};

use crate::{
    FrameTraceGuard,
    sampler::{FrameSampler, ResourceSample, minimum_resource_interval},
    style::FpsStyle,
};

/// One frame at 60Hz, the default budget a frame is judged against.
const DEFAULT_FRAME_BUDGET: Duration = Duration::from_nanos(16_666_667);
const DEFAULT_CAPACITY: usize = 120;
const DEFAULT_RESOURCE_INTERVAL: Duration = Duration::from_millis(500);

/// How fast the chart's y axis relaxes back down after a spike. Growth is
/// immediate so a slow frame is never clipped, while the decay is gradual so
/// the bars don't visibly rescale every frame.
const AXIS_DECAY: f32 = 0.04;

/// A fixed width keeps every row flush with the chart and stops the HUD from
/// resizing as the readings gain or lose digits. Collapsed, the HUD hugs its
/// text instead and only the figure gets a fixed box.
const HUD_WIDTH: Pixels = px(172.);
const COMPACT_FIGURE_WIDTH: Pixels = px(25.);

/// Size of every label and reading. Collapsed, the figure uses it too.
const TEXT_SIZE: Pixels = px(10.);

/// The trace sits behind the headline, so it is dimmed enough to stay out of
/// the figure's way while still showing its shape and color.
const TRACE_OPACITY: f32 = 0.35;

/// Tall enough to give the trace room to show its shape around the figure.
const HEADLINE_HEIGHT: Pixels = px(35.);

/// The headline figure. Its box has to fit four digits at [`FIGURE_SIZE`] —
/// a monospace digit runs about 0.6em, and an uncapped frame rate on a small
/// window reaches four figures — or the reading is clipped instead of merely
/// looking cramped.
const FIGURE_SIZE: Pixels = px(28.);
const FIGURE_WIDTH: Pixels = px(70.);

/// Width of the `FPS` unit, and of the empty box mirroring it on the other side
/// of the figure so the figure lands on the HUD's true center.
const UNIT_WIDTH: Pixels = px(22.);

/// How often the numbers are recomputed.
///
/// The trace keeps up with every frame, but the readings do not: recomputed
/// per frame they flicker through digits too fast to read, and the eye tracks
/// the churn rather than the value. Twice a second is slow enough to read and
/// fast enough to feel live.
const READOUT_INTERVAL: Duration = Duration::from_millis(500);

/// Fraction of the target frame rate that still counts as meeting it. Vsync and
/// the sampling window each cost a frame or so a second, so a 60Hz display that
/// is keeping up perfectly reports 58 to 60, never a flat 60.
const FPS_TOLERANCE: f32 = 0.95;

/// A monospace family that ships with the platform, so the value column stays
/// aligned without the application having to configure a font. The generic
/// `monospace` alias is not resolvable by every platform's font backend, hence
/// the concrete names.
#[cfg(target_os = "macos")]
const DEFAULT_FONT: &str = "Menlo";
#[cfg(target_os = "windows")]
const DEFAULT_FONT: &str = "Consolas";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const DEFAULT_FONT: &str = "monospace";

/// A realtime performance HUD: frames per second, a rolling frame time chart,
/// and this process' CPU and memory usage.
///
/// This is a view rather than a stateless component on purpose. Driving
/// continuous redraws goes through [`Window::request_animation_frame`], which
/// notifies the *current* view — from inside a stateless component that would
/// be the parent, forcing the whole parent tree to redraw every frame. As its
/// own view, only the HUD subtree repaints.
///
/// ```no_run
/// # use gpui::*;
/// # use gpui_fps::FpsMonitor;
/// # fn example(window: &mut Window, cx: &mut App) {
/// let monitor = cx.new(|cx| FpsMonitor::new(window, cx).capacity(240));
/// # }
/// ```
/// The numbers as last published to the screen.
#[derive(Clone, Copy, Default)]
struct Readout {
    fps: f32,
    frame_millis: f32,
    dropped_percent: f32,
}

pub struct FpsMonitor {
    sampler: FrameSampler,
    readout: Readout,
    readout_at: Option<Instant>,
    style: FpsStyle,
    frame_budget: Duration,
    continuous: bool,
    show_resources: bool,
    resource_interval: Duration,
    resources: Option<ResourceSample>,
    compact: bool,
    /// Upper bound of the chart's y axis, in seconds.
    axis_max: f32,
    resource_task: Option<Task<()>>,
    _frame_trace: FrameTraceGuard,
}

impl FpsMonitor {
    pub fn new(window: &Window, _cx: &mut Context<Self>) -> Self {
        let frame_budget = DEFAULT_FRAME_BUDGET;
        Self {
            sampler: FrameSampler::new(window.window_handle().window_id(), DEFAULT_CAPACITY),
            readout: Readout::default(),
            readout_at: None,
            style: FpsStyle::default(),
            frame_budget,
            continuous: true,
            show_resources: true,
            resource_interval: DEFAULT_RESOURCE_INTERVAL,
            resources: None,
            compact: false,
            axis_max: frame_budget.as_secs_f32() * 2.,
            resource_task: None,
            _frame_trace: FrameTraceGuard::acquire(),
        }
    }

    /// How many frames the chart keeps. Defaults to 120.
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.sampler.set_capacity(capacity);
        self
    }

    /// The per-frame budget used for the chart's baseline and bar colors.
    /// Defaults to one 60Hz frame; set it to `1/144s` on a high refresh rate
    /// display.
    pub fn frame_budget(mut self, budget: Duration) -> Self {
        self.frame_budget = budget;
        self.axis_max = budget.as_secs_f32() * 2.;
        self
    }

    /// Whether to request a frame on every render, keeping the window drawing
    /// back to back. Defaults to `true`.
    ///
    /// This is what makes the readout behave like an in-game FPS counter, and
    /// it has the same caveat: the window never idles, so the number is the
    /// frame rate the application *can* sustain, not the rate it happens to be
    /// drawing at. Turn it off to measure the real workload — the HUD then only
    /// updates when the window redraws for its own reasons, and reads zero
    /// while the window is idle.
    pub fn continuous(mut self, continuous: bool) -> Self {
        self.continuous = continuous;
        self
    }

    /// Whether to sample and show this process' CPU and memory usage.
    /// Defaults to `true`, and is always off on the web.
    pub fn show_resources(mut self, show_resources: bool) -> Self {
        self.show_resources = show_resources;
        self
    }

    /// How often CPU and memory are resampled. Defaults to 500ms, and is
    /// clamped up to the shortest interval that yields a meaningful CPU delta.
    pub fn resource_interval(mut self, interval: Duration) -> Self {
        self.resource_interval = interval;
        self
    }

    /// Sampling starts on the first render rather than in `new` so that the
    /// builder methods have already been applied by the time the interval is
    /// read.
    #[cfg(not(target_family = "wasm"))]
    fn start_resource_sampling(&mut self, cx: &mut Context<Self>) {
        use crate::sampler::ResourceProbe;

        if !self.show_resources || self.resource_task.is_some() {
            return;
        }

        let interval = self.resource_interval.max(minimum_resource_interval());
        self.resource_task = Some(cx.spawn(async move |this, cx| {
            let executor = cx.background_executor().clone();
            // Probing walks the process table, so it never runs on the render
            // thread. The probe moves in and out of each background task rather
            // than living behind a lock.
            let Some(mut probe) = executor.spawn(async { ResourceProbe::new() }).await else {
                return;
            };

            loop {
                executor.timer(interval).await;

                let (returned, sample) = executor
                    .spawn(async move {
                        let sample = probe.sample();
                        (probe, sample)
                    })
                    .await;
                probe = returned;

                let Some(sample) = sample else { continue };
                let updated = this.update(cx, |this, cx| {
                    this.resources = Some(sample);
                    cx.notify();
                });
                if updated.is_err() {
                    break;
                }
            }
        }));
    }

    #[cfg(target_family = "wasm")]
    fn start_resource_sampling(&mut self, _cx: &mut Context<Self>) {
        let _ = minimum_resource_interval();
    }

    /// Republishes the readings if [`READOUT_INTERVAL`] has passed.
    fn update_readout(&mut self) {
        let now = Instant::now();
        let due = self
            .readout_at
            .is_none_or(|at| now.duration_since(at) >= READOUT_INTERVAL);
        if !due {
            return;
        }

        self.readout = Readout {
            fps: self.sampler.fps(),
            // The mean over the interval rather than the latest frame, which
            // at this cadence would be an arbitrary sample.
            frame_millis: self.sampler.mean_draw().as_secs_f32() * 1000.,
            dropped_percent: self.sampler.over_budget_ratio(self.frame_budget) * 100.,
        };
        self.readout_at = Some(now);
    }

    /// Grows immediately to fit the slowest retained frame and decays back
    /// slowly, so a single spike doesn't make the whole chart jump.
    fn update_axis(&mut self) {
        let floor = self.frame_budget.as_secs_f32() * 2.;
        let target = self.sampler.peak_draw().as_secs_f32().max(floor);
        self.axis_max = if target > self.axis_max {
            target
        } else {
            self.axis_max + (target - self.axis_max) * AXIS_DECAY
        };
    }

    /// The frame time trace, drawn behind the readings so it fills the HUD
    /// instead of taking a band of its own. It is dimmed to stay legible under
    /// the text.
    fn render_chart(&self) -> impl IntoElement {
        let style = self.style;
        let budget = self.frame_budget.as_secs_f32();
        let axis_max = self.axis_max.max(f32::EPSILON);
        let capacity = self.sampler.capacity();
        let samples: Vec<(f32, Hsla)> = self
            .sampler
            .samples()
            .map(|sample| {
                let seconds = sample.draw.as_secs_f32();
                (
                    (seconds / axis_max).clamp(0., 1.),
                    style.level_color(seconds, budget).opacity(TRACE_OPACITY),
                )
            })
            .collect();

        canvas(
            |_, _, _| (),
            move |bounds: Bounds<Pixels>, _, window, _| {
                let slot = bounds.size.width / capacity as f32;
                // Fewer samples than the capacity means the chart is still
                // filling up; keep the newest frame pinned to the right edge so
                // the history scrolls instead of stretching.
                let leading = capacity.saturating_sub(samples.len());
                let points: Vec<(Point<Pixels>, Hsla)> = samples
                    .iter()
                    .enumerate()
                    .map(|(index, (ratio, color))| {
                        (
                            point(
                                bounds.origin.x + slot * (leading + index) as f32 + slot / 2.,
                                bounds.origin.y + bounds.size.height * (1. - *ratio),
                            ),
                            *color,
                        )
                    })
                    .collect();

                // The line is drawn as runs of equal color rather than one
                // segment per frame: a single path can only carry one color,
                // and in the common case where nothing is dropped the whole
                // chart collapses into one path.
                let mut start = 0;
                while start + 1 < points.len() {
                    // A segment is as slow as the frame it ends on, so the
                    // color of the later point decides the run.
                    let color = points[start + 1].1;
                    let mut path = PathBuilder::stroke(px(1.));
                    path.move_to(points[start].0);

                    let mut end = start + 1;
                    while end < points.len() && points[end].1 == color {
                        path.line_to(points[end].0);
                        end += 1;
                    }

                    if let Ok(path) = path.build() {
                        window.paint_path(path, color);
                    }
                    // Share the boundary point with the next run so the line
                    // stays connected across a color change.
                    start = end - 1;
                }
            },
        )
        .absolute()
        .inset_0()
    }

    /// The headline reading, with the frame time trace painted behind it.
    ///
    /// The trace lives in this row rather than spanning the whole HUD because
    /// this is its emptiest part — the figure is centered and short, leaving
    /// both flanks open — so the trace stays readable instead of being cut up
    /// by the denser rows below.
    ///
    /// The figure is centered in a fixed box so neither the unit nor the group
    /// shifts as the count gains or loses a digit; the two share a bottom edge.
    fn render_headline(&self, fps: f32, color: Hsla) -> Div {
        let style = self.style;

        div()
            .relative()
            .overflow_hidden()
            .w_full()
            .h(HEADLINE_HEIGHT)
            .child(self.render_chart())
            .child(
                div()
                    .flex()
                    .size_full()
                    .items_end()
                    .justify_center()
                    .gap_1()
                    // An empty box matching the unit on the right. Without it
                    // the unit's own width pushes the figure off center by half
                    // of it, which reads as misalignment.
                    .child(div().w(UNIT_WIDTH))
                    .child(
                        div()
                            .w(FIGURE_WIDTH)
                            .text_center()
                            .text_size(FIGURE_SIZE)
                            .line_height(relative(1.))
                            .text_color(color)
                            .child(format!("{fps:.0}")),
                    )
                    .child(div().w(UNIT_WIDTH).text_color(style.muted).child("FPS")),
            )
    }
}

impl Render for FpsMonitor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sampler.tick();
        self.update_readout();
        self.update_axis();
        self.start_resource_sampling(cx);
        if self.continuous {
            window.request_animation_frame();
        }

        let style = self.style;
        let budget = self.frame_budget;
        let Readout {
            fps,
            frame_millis,
            dropped_percent: dropped,
        } = self.readout;
        let fps_color = fps_color(fps, budget, style);
        let resources = self.resources.filter(|_| self.show_resources);
        let compact = self.compact;

        div()
            .id("gpui-fps-hud")
            .flex()
            .bg(style.background)
            .font_family(DEFAULT_FONT)
            .text_size(TEXT_SIZE)
            .text_color(style.muted)
            .on_click(cx.listener(|this, _, _, cx| {
                this.compact = !this.compact;
                cx.notify();
            }))
            .map(|this| {
                if compact {
                    // Collapsed, the HUD is one small tag: the figure drops to
                    // the same size as its unit, the box shrinks to the text,
                    // and everything else is dropped, so it sits over the
                    // interface without competing with it.
                    this.items_center()
                        .gap_1()
                        .px_1p5()
                        .py_0p5()
                        .rounded(px(3.))
                        .child(
                            div()
                                .w(COMPACT_FIGURE_WIDTH)
                                .text_right()
                                .text_color(fps_color)
                                .child(format!("{fps:.0}")),
                        )
                        .child("FPS")
                } else {
                    this.flex_col()
                        .w(HUD_WIDTH)
                        .px_2()
                        .py_1p5()
                        .rounded(px(4.))
                        .child(self.render_headline(fps, fps_color))
                        .child(reading(
                            "FRAME",
                            format!("{frame_millis:.1} ms"),
                            style.foreground,
                            style,
                        ))
                        .child(reading(
                            "DROP",
                            format!("{dropped:.1}%"),
                            style.level_color(if dropped > 0. { 1. } else { 0. }, 0.5),
                            style,
                        ))
                        .when_some(resources, |this, resources| {
                            this.child(
                                // CPU and memory share a row: both are coarse
                                // background samples, unlike the per-frame
                                // numbers.
                                div()
                                    .flex()
                                    .w_full()
                                    .justify_between()
                                    .gap_2()
                                    .py(px(1.))
                                    .child(pair(
                                        "CPU",
                                        format!("{:.1}%", resources.cpu_percent),
                                        style,
                                    ))
                                    .child(pair(
                                        "MEM",
                                        format_bytes(resources.memory_bytes),
                                        style,
                                    )),
                            )
                        })
                }
            })
    }
}

/// Grades the frame rate against the rate the budget implies.
///
/// This deliberately does not compare `1/fps` against the budget the way the
/// per-frame trace does. Under vsync the measured rate lands just under the
/// refresh rate essentially always — a 60Hz display reads 58 to 60, never
/// exactly 60.00 — so an exact comparison would paint a perfectly healthy
/// application as over budget. Anything within [`FPS_TOLERANCE`] of the target
/// counts as meeting it.
fn fps_color(fps: f32, budget: Duration, style: FpsStyle) -> Hsla {
    if fps <= 0. {
        return style.muted;
    }

    let target = 1. / budget.as_secs_f32();
    if fps >= target * FPS_TOLERANCE {
        style.good
    } else if fps >= target * 0.5 {
        style.warn
    } else {
        style.bad
    }
}

/// A `LABEL value` pair kept together, for rows that carry more than one
/// reading. The label stays muted so it reads as a caption, not as data.
fn pair(label: &'static str, value: String, style: FpsStyle) -> Div {
    div()
        .flex()
        .gap_1()
        .child(div().text_color(style.muted).child(label))
        .child(div().text_color(style.foreground).child(value))
}

/// One `LABEL … value` row. The value is right aligned against the HUD's inner
/// edge, so in a monospace font every row's digits line up in a column and
/// nothing shifts as the readings change width.
fn reading(label: &'static str, value: String, value_color: Hsla, style: FpsStyle) -> Div {
    div()
        .flex()
        .w_full()
        .justify_between()
        .gap_2()
        .py(px(1.))
        .child(div().text_color(style.muted).child(label))
        .child(div().text_color(value_color).child(value))
}

fn format_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024. * 1024.;
    const GIB: f64 = MIB * 1024.;

    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.2} GB", bytes / GIB)
    } else {
        format!("{:.0} MB", bytes / MIB)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;

    #[gpui::test]
    fn test_fps_monitor_builder(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let budget = Duration::from_micros(6_944);
            let monitor = cx.new(|cx| {
                FpsMonitor::new(window, cx)
                    .capacity(240)
                    .frame_budget(budget)
                    .continuous(false)
                    .show_resources(false)
                    .resource_interval(Duration::from_secs(2))
            });

            let monitor = monitor.read(cx);
            assert_eq!(monitor.sampler.capacity(), 240);
            assert_eq!(monitor.frame_budget, budget);
            assert!(!monitor.continuous);
            assert!(!monitor.show_resources);
            assert_eq!(monitor.resource_interval, Duration::from_secs(2));
            // The axis floor tracks the budget so a 144Hz budget doesn't leave
            // the chart scaled for 60Hz frames.
            assert_eq!(monitor.axis_max, budget.as_secs_f32() * 2.);
        });
    }

    #[test]
    fn a_display_keeping_up_is_never_graded_as_falling_behind() {
        let style = FpsStyle::dark();
        let budget = DEFAULT_FRAME_BUDGET;

        // What a healthy 60Hz display actually reports.
        for rate in [58., 59., 59.7, 60., 61.] {
            assert_eq!(
                fps_color(rate, budget, style),
                style.good,
                "{rate} fps should read as healthy on a 60Hz display"
            );
        }

        assert_eq!(fps_color(45., budget, style), style.warn);
        assert_eq!(fps_color(20., budget, style), style.bad);
        assert_eq!(fps_color(0., budget, style), style.muted);
    }

    #[test]
    fn formats_memory_by_magnitude() {
        assert_eq!(format_bytes(184 * 1024 * 1024), "184 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.00 GB");
    }
}
