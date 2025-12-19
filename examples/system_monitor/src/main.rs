use std::collections::VecDeque;
use std::time::Duration;

use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{ActiveTheme, Root, TitleBar, chart::AreaChart, h_flex, v_flex};
use smol::Timer;
use sysinfo::System;

const MAX_DATA_POINTS: usize = 60;
const UPDATE_INTERNAL: Duration = Duration::from_secs(1);

/// A single data point for system metrics
#[derive(Clone)]
struct MetricPoint {
    /// Time index (seconds from start)
    time: String,
    /// CPU usage percentage (0-100)
    cpu: f64,
    /// Memory usage percentage (0-100)
    memory: f64,
    /// GPU usage percentage (0-100), None if not available
    gpu: f64,
    /// GPU memory/VRAM usage percentage (0-100), None if not available
    vram: f64,
}

/// System monitor that collects and displays real-time metrics
pub struct SystemMonitor {
    /// System info collector
    sys: System,
    /// Historical data points
    data: VecDeque<MetricPoint>,
    /// Current time index
    time_index: usize,
    /// Whether GPU monitoring is available
    gpu_available: bool,
    /// GPU monitor (platform-specific)
    #[cfg(target_os = "macos")]
    gpu_monitor: Option<MacGpuMonitor>,
    #[cfg(target_os = "windows")]
    gpu_monitor: Option<WindowsGpuMonitor>,
    #[cfg(target_os = "linux")]
    gpu_monitor: Option<LinuxGpuMonitor>,
}

// Platform-specific GPU monitoring

#[cfg(target_os = "macos")]
struct MacGpuMonitor {
    device: metal::Device,
}

#[cfg(target_os = "macos")]
impl MacGpuMonitor {
    fn new() -> Option<Self> {
        metal::Device::system_default().map(|device| Self { device })
    }

    fn get_usage(&self) -> (f64, f64) {
        // Metal doesn't provide direct GPU utilization API
        // We can get memory info though
        let recommended_max = self.device.recommended_max_working_set_size() as f64;
        let current = self.device.current_allocated_size() as f64;
        let vram_usage = if recommended_max > 0.0 {
            (current / recommended_max * 100.0).min(100.0)
        } else {
            0.0
        };
        // GPU utilization is not directly available on macOS Metal
        // Return 0 for GPU usage, only VRAM is accurate
        (0.0, vram_usage)
    }
}

#[cfg(target_os = "windows")]
struct WindowsGpuMonitor {
    // Windows GPU monitoring via DXGI
    adapter_desc: String,
}

#[cfg(target_os = "windows")]
impl WindowsGpuMonitor {
    fn new() -> Option<Self> {
        use windows::Win32::Graphics::Dxgi::*;

        unsafe {
            let factory: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
            let adapter = factory.EnumAdapters1(0).ok()?;
            let desc = adapter.GetDesc1().ok()?;
            let name = String::from_utf16_lossy(
                &desc.Description[..desc
                    .Description
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(desc.Description.len())],
            );
            Some(Self { adapter_desc: name })
        }
    }

    fn get_usage(&self) -> (f64, f64) {
        use windows::Win32::Graphics::Dxgi::*;

        unsafe {
            if let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory4>() {
                if let Ok(adapter) = factory.EnumAdapters1(0) {
                    if let Ok(adapter3) = adapter.cast::<IDXGIAdapter3>() {
                        let mut info = Default::default();
                        if adapter3
                            .QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut info)
                            .is_ok()
                        {
                            let budget = info.Budget as f64;
                            let usage = info.CurrentUsage as f64;
                            if budget > 0.0 {
                                return (0.0, (usage / budget * 100.0).min(100.0));
                            }
                        }
                    }
                }
            }
        }
        (0.0, 0.0)
    }
}

#[cfg(target_os = "linux")]
struct LinuxGpuMonitor {
    // Linux GPU monitoring via sysfs or nvidia-smi
    nvidia_available: bool,
}

#[cfg(target_os = "linux")]
impl LinuxGpuMonitor {
    fn new() -> Option<Self> {
        // Check if nvidia-smi is available
        let nvidia_available = std::process::Command::new("nvidia-smi")
            .arg("--version")
            .output()
            .is_ok();

        Some(Self { nvidia_available })
    }

    fn get_usage(&self) -> (f64, f64) {
        if self.nvidia_available {
            // Try to get NVIDIA GPU stats via nvidia-smi
            if let Ok(output) = std::process::Command::new("nvidia-smi")
                .args([
                    "--query-gpu=utilization.gpu,memory.used,memory.total",
                    "--format=csv,noheader,nounits",
                ])
                .output()
            {
                if let Ok(stdout) = String::from_utf8(output.stdout) {
                    let parts: Vec<&str> = stdout.trim().split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 3 {
                        let gpu_util = parts[0].parse::<f64>().unwrap_or(0.0);
                        let mem_used = parts[1].parse::<f64>().unwrap_or(0.0);
                        let mem_total = parts[2].parse::<f64>().unwrap_or(1.0);
                        let vram_usage = if mem_total > 0.0 {
                            (mem_used / mem_total * 100.0).min(100.0)
                        } else {
                            0.0
                        };
                        return (gpu_util, vram_usage);
                    }
                }
            }
        }
        (0.0, 0.0)
    }
}

impl SystemMonitor {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        // Initialize GPU monitor based on platform
        #[cfg(target_os = "macos")]
        let (gpu_monitor, gpu_available) = {
            let monitor = MacGpuMonitor::new();
            let available = monitor.is_some();
            (monitor, available)
        };

        #[cfg(target_os = "windows")]
        let (gpu_monitor, gpu_available) = {
            let monitor = WindowsGpuMonitor::new();
            let available = monitor.is_some();
            (monitor, available)
        };

        #[cfg(target_os = "linux")]
        let (gpu_monitor, gpu_available) = {
            let monitor = LinuxGpuMonitor::new();
            let available = monitor
                .as_ref()
                .map(|m| m.nvidia_available)
                .unwrap_or(false);
            (monitor, available)
        };

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        let (gpu_monitor, gpu_available): (Option<()>, bool) = (None, false);

        let mut monitor = Self {
            sys,
            data: VecDeque::with_capacity(MAX_DATA_POINTS),
            time_index: 0,
            gpu_available,
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            gpu_monitor,
        };

        // Collect initial data point
        monitor.collect_metrics();

        // Start the update loop
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(Duration::from_secs(1)).await;

                let result = this.update(cx, |this, cx| {
                    this.collect_metrics();
                    cx.notify();
                });

                if result.is_err() {
                    break;
                }
            }
        })
        .detach();

        monitor
    }

    fn collect_metrics(&mut self) {
        // Refresh system info
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        // Calculate CPU usage (average across all cores)
        let cpu_usage = self.sys.global_cpu_usage() as f64;

        // Calculate memory usage
        let total_memory = self.sys.total_memory() as f64;
        let used_memory = self.sys.used_memory() as f64;
        let memory_usage = if total_memory > 0.0 {
            (used_memory / total_memory * 100.0).min(100.0)
        } else {
            0.0
        };

        // Get GPU metrics
        let (gpu_usage, vram_usage) = self.get_gpu_metrics();

        // Create data point
        let point = MetricPoint {
            time: format!("{}s", self.time_index),
            cpu: cpu_usage,
            memory: memory_usage,
            gpu: gpu_usage,
            vram: vram_usage,
        };

        // Add to history
        if self.data.len() >= MAX_DATA_POINTS {
            self.data.pop_front();
        }
        self.data.push_back(point);
        self.time_index += 1;
    }

    fn get_gpu_metrics(&self) -> (f64, f64) {
        #[cfg(target_os = "macos")]
        if let Some(ref monitor) = self.gpu_monitor {
            return monitor.get_usage();
        }

        #[cfg(target_os = "windows")]
        if let Some(ref monitor) = self.gpu_monitor {
            return monitor.get_usage();
        }

        #[cfg(target_os = "linux")]
        if let Some(ref monitor) = self.gpu_monitor {
            return monitor.get_usage();
        }

        (0.0, 0.0)
    }

    fn render_chart(
        &self,
        title: &str,
        data: Vec<MetricPoint>,
        value_fn: impl Fn(&MetricPoint) -> f64 + 'static,
        color: Hsla,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .flex_1()
            .gap_2()
            .border_1()
            .border_color(cx.theme().border)
            .h(px(120.))
            .child(
                h_flex()
                    .justify_between()
                    .p_3()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(title.to_string()),
                    )
                    .child({
                        let current_value = data.last().map(|p| value_fn(p)).unwrap_or(0.0);
                        div()
                            .text_sm()
                            .text_color(color)
                            .child(format!("{:.1}%", current_value))
                    }),
            )
            .child(
                AreaChart::new(data)
                    .x(|d| d.time.clone())
                    .y(value_fn)
                    .stroke(color)
                    .fill(linear_gradient(
                        0.,
                        linear_color_stop(color.opacity(0.4), 1.),
                        linear_color_stop(cx.theme().background.opacity(0.1), 0.),
                    ))
                    .tick_margin(15),
            )
    }
}

impl Render for SystemMonitor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let data: Vec<MetricPoint> = self.data.iter().cloned().collect();
        let has_gpu = if cfg!(target_os = "macos") {
            false
        } else {
            true
        };

        v_flex()
            .size_full()
            .child(TitleBar::new().child("System Monitor"))
            .bg(cx.theme().background)
            .child(
                v_flex()
                    .p_4()
                    .gap_4()
                    .flex_1()
                    .w_full()
                    .child(self.render_chart(
                        "CPU Usage",
                        data.clone(),
                        |d| d.cpu,
                        cx.theme().red,
                        cx,
                    ))
                    .child(self.render_chart(
                        "Memory Usage",
                        data.clone(),
                        |d| d.memory,
                        cx.theme().blue,
                        cx,
                    ))
                    .when(has_gpu, |this| {
                        this.child(self.render_chart(
                            if self.gpu_available {
                                "GPU Usage"
                            } else {
                                "GPU Usage (N/A)"
                            },
                            data.clone(),
                            |d| d.gpu,
                            cx.theme().green,
                            cx,
                        ))
                    })
                    .child(self.render_chart(
                        if self.gpu_available {
                            "VRAM Usage"
                        } else {
                            "VRAM Usage (N/A)"
                        },
                        data,
                        |d| d.vram,
                        cx.theme().yellow,
                        cx,
                    ))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "Total Memory: {:.1} GB | Update Interval: {} ms",
                                self.sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
                                UPDATE_INTERNAL.as_millis()
                            )),
                    ),
            )
    }
}

fn main() {
    let app = Application::new();

    app.run(move |cx| {
        // Initialize GPUI Component
        gpui_component::init(cx);

        let window_options = WindowOptions {
            // Setup GPUI to use custom title bar
            titlebar: Some(TitleBar::title_bar_options()),
            window_bounds: Some(WindowBounds::centered(size(px(800.), px(800.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                window.set_window_title("System Monitor");

                let view = cx.new(|cx| SystemMonitor::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
