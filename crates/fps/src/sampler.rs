use std::{collections::VecDeque, time::Duration};

use gpui::{
    WindowId,
    profiler::{FrameEvent, FrameTiming, FrameTimingCollector},
};
use instant::Instant;

/// Frames older than this stop contributing to the FPS readout.
const FPS_WINDOW: Duration = Duration::from_secs(1);

/// One drawn frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FrameSample {
    /// How long `Window::draw` took for this frame.
    pub draw: Duration,
    /// How many invalidations were coalesced into this frame. A number well
    /// above one means the window was asked to redraw more often than it could.
    pub invalidations: u64,
}

/// Collects per-frame timings for a single window out of GPUI's global frame
/// trace.
///
/// GPUI records frame timings into a process-wide ring buffer, so the sampler
/// filters by window: without that, every other open window's frames would be
/// counted as this window's.
pub(crate) struct FrameSampler {
    collector: FrameTimingCollector,
    window_id: WindowId,
    samples: VecDeque<FrameSample>,
    /// Arrival times of the frames still inside [`FPS_WINDOW`].
    frame_times: VecDeque<Instant>,
    capacity: usize,
}

impl FrameSampler {
    pub(crate) fn new(window_id: WindowId, capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            collector: FrameTimingCollector::new(),
            window_id,
            samples: VecDeque::with_capacity(capacity),
            frame_times: VecDeque::new(),
            capacity,
        }
    }

    /// Drains the frames drawn since the previous call. Call once per rendered
    /// frame.
    pub(crate) fn tick(&mut self) {
        let timings = self
            .collector
            .collect_unseen()
            .into_iter()
            .filter_map(|event| match event {
                FrameEvent::Draw(timing) => Some(timing),
                FrameEvent::Present(_) => None,
            })
            .collect();
        self.ingest(timings, Instant::now());
    }

    pub(crate) fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        while self.samples.len() > self.capacity {
            self.samples.pop_front();
        }
    }

    /// Frames per second measured over the frames still inside [`FPS_WINDOW`].
    ///
    /// `n` frames span `n - 1` intervals, so the rate is derived from the
    /// elapsed span rather than from the raw count; that keeps the readout
    /// correct before the rolling window has filled up.
    pub(crate) fn fps(&self) -> f32 {
        if self.frame_times.len() < 2 {
            return 0.;
        }
        let (Some(oldest), Some(newest)) = (self.frame_times.front(), self.frame_times.back())
        else {
            return 0.;
        };
        let span = newest.duration_since(*oldest).as_secs_f32();
        if span <= 0. {
            return 0.;
        }
        (self.frame_times.len() - 1) as f32 / span
    }

    pub(crate) fn samples(&self) -> impl ExactSizeIterator<Item = &FrameSample> {
        self.samples.iter()
    }

    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }

    /// Share of the retained frames that overran `budget`, in `0..1`.
    pub(crate) fn over_budget_ratio(&self, budget: Duration) -> f32 {
        if self.samples.is_empty() {
            return 0.;
        }
        let over = self
            .samples
            .iter()
            .filter(|sample| sample.draw > budget)
            .count();
        over as f32 / self.samples.len() as f32
    }

    /// Mean draw time across the retained frames.
    pub(crate) fn mean_draw(&self) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.samples.iter().map(|sample| sample.draw).sum();
        total / self.samples.len() as u32
    }

    /// The slowest retained frame, used to scale the chart's y axis.
    pub(crate) fn peak_draw(&self) -> Duration {
        self.samples
            .iter()
            .map(|sample| sample.draw)
            .max()
            .unwrap_or_default()
    }

    fn ingest(&mut self, timings: Vec<FrameTiming>, now: Instant) {
        for timing in timings {
            if timing.window_id != self.window_id {
                continue;
            }

            if self.samples.len() == self.capacity {
                self.samples.pop_front();
            }
            self.samples.push_back(FrameSample {
                draw: timing.draw_duration(),
                invalidations: timing.invalidations,
            });
            self.frame_times.push_back(now);
        }

        while let Some(oldest) = self.frame_times.front() {
            if now.duration_since(*oldest) > FPS_WINDOW {
                self.frame_times.pop_front();
            } else {
                break;
            }
        }
    }
}

/// A sample of the resource usage shown beside the frame numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ResourceSample {
    /// CPU used by this process, normalized so that 100 means every logical
    /// core is saturated. `sysinfo` reports 100 per saturated core, so the raw
    /// value is divided by the core count.
    pub cpu_percent: f32,
    /// Resident memory of this process, in bytes.
    pub memory_bytes: u64,
    /// The share of the GPU this process is using, and `None` on a platform
    /// that does not attribute GPU time per process. See [`crate::gpu`].
    pub gpu_percent: Option<f32>,
}

/// Samples this process' CPU, memory and GPU usage.
///
/// Refreshing is a blocking syscall walk, so this must be driven from a
/// background thread rather than from the render loop.
#[cfg(not(target_family = "wasm"))]
pub(crate) struct ResourceProbe {
    system: sysinfo::System,
    pid: sysinfo::Pid,
    cores: f32,
    /// `None` when this platform publishes no per-process GPU counter, which
    /// the HUD shows by leaving the reading out rather than at a flat zero.
    gpu: Option<crate::gpu::GpuProbe>,
}

#[cfg(not(target_family = "wasm"))]
impl ResourceProbe {
    /// Returns `None` when the current process id cannot be determined, which
    /// is the only way sampling can be unavailable on a supported platform.
    pub(crate) fn new() -> Option<Self> {
        let pid = sysinfo::get_current_pid().ok()?;
        let cores = std::thread::available_parallelism()
            .map(|cores| cores.get() as f32)
            .unwrap_or(1.);

        let mut probe = Self {
            system: sysinfo::System::new(),
            pid,
            cores,
            gpu: crate::gpu::GpuProbe::new(),
        };
        // The first refresh only establishes the baseline; `cpu_usage` is a
        // delta against the previous refresh and reads zero until then.
        probe.refresh();
        Some(probe)
    }

    pub(crate) fn sample(&mut self) -> Option<ResourceSample> {
        self.refresh();
        // Sampled before the process is borrowed out of `self.system`, so the
        // two borrows of `self` do not overlap.
        let gpu_percent = self.gpu.as_mut().and_then(crate::gpu::GpuProbe::sample);

        let process = self.system.process(self.pid)?;
        Some(ResourceSample {
            cpu_percent: (process.cpu_usage() / self.cores).min(100.),
            memory_bytes: process.memory(),
            gpu_percent,
        })
    }

    fn refresh(&mut self) {
        self.system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[self.pid]),
            false,
            sysinfo::ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory(),
        );
    }
}

/// The shortest interval at which CPU usage can be meaningfully resampled.
#[cfg(not(target_family = "wasm"))]
pub(crate) fn minimum_resource_interval() -> Duration {
    sysinfo::MINIMUM_CPU_UPDATE_INTERVAL
}

#[cfg(target_family = "wasm")]
pub(crate) fn minimum_resource_interval() -> Duration {
    Duration::from_millis(200)
}

#[cfg(test)]
mod tests {
    use super::*;

    // GPUI stamps frames with `scheduler::Instant`, which re-exports
    // `std::time::Instant` off the web, so tests can build one without pulling
    // in the `scheduler` crate.
    fn timing(window_id: WindowId, draw: Duration) -> FrameTiming {
        let start = std::time::Instant::now();
        FrameTiming {
            window_id,
            dirty_at: None,
            invalidations: 1,
            draw_start: start,
            draw_end: start + draw,
        }
    }

    #[test]
    fn ignores_frames_from_other_windows() {
        let ours = WindowId::from(1);
        let theirs = WindowId::from(2);
        let mut sampler = FrameSampler::new(ours, 8);
        let now = Instant::now();

        sampler.ingest(
            vec![
                timing(ours, Duration::from_millis(8)),
                timing(theirs, Duration::from_millis(40)),
                timing(ours, Duration::from_millis(9)),
            ],
            now,
        );

        assert_eq!(sampler.samples().len(), 2);
        assert_eq!(sampler.peak_draw(), Duration::from_millis(9));
    }

    #[test]
    fn drops_oldest_samples_beyond_capacity() {
        let window_id = WindowId::from(1);
        let mut sampler = FrameSampler::new(window_id, 2);
        let now = Instant::now();

        for millis in [5, 6, 7] {
            sampler.ingest(vec![timing(window_id, Duration::from_millis(millis))], now);
        }

        let draws: Vec<_> = sampler.samples().map(|sample| sample.draw).collect();
        assert_eq!(
            draws,
            vec![Duration::from_millis(6), Duration::from_millis(7)]
        );
    }

    /// Feeds `count` frames spaced `interval` apart and returns the resulting
    /// rate.
    fn measure_fps(count: u64, interval: Duration) -> f32 {
        let window_id = WindowId::from(1);
        let mut sampler = FrameSampler::new(window_id, 256);
        let start = Instant::now();

        for frame in 0..count {
            sampler.ingest(
                vec![timing(window_id, Duration::from_millis(1))],
                start + interval * frame as u32,
            );
        }
        sampler.fps()
    }

    #[test]
    fn fps_is_frames_divided_by_the_span_they_cover() {
        // The rate is `(n - 1) / span`, not `n / span`: n frames delimit n - 1
        // intervals. Counting the frames instead would over-report by
        // `1 / span`, which is a whole frame per second at these rates.
        //
        // 11 frames spaced 10ms apart cover 100ms => 10 intervals => 100 fps.
        assert!((measure_fps(11, Duration::from_millis(10)) - 100.).abs() < 0.5);

        // The same span sampled more finely reports the same rate.
        assert!((measure_fps(101, Duration::from_millis(1)) - 1000.).abs() < 5.);
    }

    #[test]
    fn fps_matches_the_common_refresh_rates() {
        for (interval_micros, expected) in [
            (16_667, 60.), // 60Hz
            (8_333, 120.), // 120Hz
            (33_333, 30.), // 30Hz
            (6_944, 144.), // 144Hz
        ] {
            let interval = Duration::from_micros(interval_micros);
            // A full second of frames at that interval.
            let count = 1_000_000 / interval_micros;
            let measured = measure_fps(count, interval);
            assert!(
                (measured - expected).abs() < 1.,
                "{interval_micros}us frames measured {measured}, expected {expected}"
            );
        }
    }

    #[test]
    fn fps_needs_two_frames_to_have_a_rate_at_all() {
        // A single frame delimits no interval, so there is nothing to divide by
        // and the honest answer is zero rather than a guess.
        assert_eq!(measure_fps(0, Duration::from_millis(10)), 0.);
        assert_eq!(measure_fps(1, Duration::from_millis(10)), 0.);
        assert!(measure_fps(2, Duration::from_millis(10)) > 0.);
    }

    #[test]
    fn simultaneous_frames_do_not_divide_by_zero() {
        let window_id = WindowId::from(1);
        let mut sampler = FrameSampler::new(window_id, 64);
        let now = Instant::now();

        // A first tick can drain several frames at once, stamping them all with
        // the same arrival time; the span between them is zero.
        sampler.ingest(
            vec![
                timing(window_id, Duration::from_millis(4)),
                timing(window_id, Duration::from_millis(4)),
                timing(window_id, Duration::from_millis(4)),
            ],
            now,
        );

        assert_eq!(sampler.fps(), 0.);
    }

    #[test]
    fn frames_outside_the_rolling_window_stop_counting() {
        let window_id = WindowId::from(1);
        let mut sampler = FrameSampler::new(window_id, 64);
        let start = Instant::now();

        for frame in 0..10 {
            sampler.ingest(
                vec![timing(window_id, Duration::from_millis(4))],
                start + Duration::from_millis(frame * 10),
            );
        }
        assert!(sampler.fps() > 0.);

        // Two seconds later the window has gone idle: every retained frame is
        // now older than the rolling window, so the rate collapses to zero.
        sampler.ingest(vec![], start + Duration::from_secs(2));
        assert_eq!(sampler.fps(), 0.);
        // The chart history survives so the last known shape stays on screen.
        assert_eq!(sampler.samples().len(), 10);
    }
}
