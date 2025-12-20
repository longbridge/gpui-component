use std::collections::VecDeque;
use std::time::Duration;

use gpui::{actions, prelude::FluentBuilder as _, *};
use gpui_component::ThemeMode;
use gpui_component::{
    ActiveTheme, Icon, IconName, Root, Sizable, Theme, TitleBar,
    chart::AreaChart,
    h_flex,
    progress::Progress,
    tab::{Tab, TabBar},
    table::{Column, ColumnSort, Table, TableDelegate, TableState},
    v_flex,
};
use smol::Timer;
use sysinfo::{Disks, Pid, System};

// Define the Quit action
actions!(system_monitor, [Quit]);

const INTERVAL: Duration = Duration::from_millis(500);
const MAX_DATA_POINTS: usize = 120;

/// Tab indices
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum MonitorTab {
    #[default]
    System = 0,
    Processes = 1,
}

impl MonitorTab {
    fn from_index(index: usize) -> Self {
        match index {
            0 => MonitorTab::System,
            1 => MonitorTab::Processes,
            _ => MonitorTab::System,
        }
    }
}

/// A single data point for system metrics
#[derive(Clone)]
struct MetricPoint {
    time: String,
    cpu: f64,
    memory: f64,
    gpu: f64,
    vram: f64,
}

/// Process info for display
#[derive(Clone)]
struct ProcessInfo {
    pid: Pid,
    name: String,
    cpu_usage: f32,
    memory: u64,
}

/// Disk info for display
#[derive(Clone)]
struct DiskInfo {
    #[allow(dead_code)]
    name: String,
    total: u64,
    used: u64,
}

/// Battery info for display
#[derive(Clone)]
struct BatteryInfo {
    #[allow(dead_code)]
    model: String,
    icon: IconName,
    percentage: f32,
}

/// Sort field for processes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ProcessSortField {
    Pid,
    Name,
    #[default]
    Cpu,
    Memory,
}

/// Process table delegate
struct ProcessTableDelegate {
    processes: Vec<ProcessInfo>,
    columns: Vec<Column>,
    sort_field: ProcessSortField,
    sort_order: ColumnSort,
}

impl ProcessTableDelegate {
    fn new() -> Self {
        Self {
            processes: Vec::new(),
            columns: vec![
                Column::new("pid", "PID").width(70.).sortable(),
                Column::new("name", "Name").width(380.).sortable(),
                Column::new("cpu", "CPU %")
                    .width(80.)
                    .sortable()
                    .sort(ColumnSort::Descending),
                Column::new("memory", "Memory").width(100.).sortable(),
            ],
            sort_field: ProcessSortField::Cpu,
            sort_order: ColumnSort::Descending,
        }
    }

    fn update_processes(&mut self, sys: &System) {
        self.processes = sys
            .processes()
            .iter()
            .map(|(pid, process)| ProcessInfo {
                pid: *pid,
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
            })
            .collect();

        self.sort_processes();
    }

    fn sort_processes(&mut self) {
        let is_descending = matches!(self.sort_order, ColumnSort::Descending);

        match self.sort_field {
            ProcessSortField::Pid => {
                self.processes.sort_by(|a, b| {
                    let cmp = a.pid.as_u32().cmp(&b.pid.as_u32());
                    if is_descending { cmp.reverse() } else { cmp }
                });
            }
            ProcessSortField::Name => {
                self.processes.sort_by(|a, b| {
                    let cmp = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                    if is_descending { cmp.reverse() } else { cmp }
                });
            }
            ProcessSortField::Cpu => {
                self.processes.sort_by(|a, b| {
                    let cmp = a
                        .cpu_usage
                        .partial_cmp(&b.cpu_usage)
                        .unwrap_or(std::cmp::Ordering::Equal);
                    if is_descending { cmp.reverse() } else { cmp }
                });
            }
            ProcessSortField::Memory => {
                self.processes.sort_by(|a, b| {
                    let cmp = a.memory.cmp(&b.memory);
                    if is_descending { cmp.reverse() } else { cmp }
                });
            }
        }

        // Keep top 200 processes
        self.processes.truncate(200);
    }
}

impl TableDelegate for ProcessTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.processes.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let Some(process) = self.processes.get(row_ix) else {
            return div().into_any_element();
        };

        match col_ix {
            0 => div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!("{}", process.pid))
                .into_any_element(),
            1 => div()
                .text_sm()
                .text_color(cx.theme().foreground)
                .truncate()
                .child(process.name.clone())
                .into_any_element(),
            2 => div()
                .text_xs()
                .text_color(if process.cpu_usage > 50.0 {
                    cx.theme().red
                } else if process.cpu_usage > 20.0 {
                    cx.theme().yellow
                } else {
                    cx.theme().blue
                })
                .child(format!("{:.1}%", process.cpu_usage))
                .into_any_element(),
            3 => div()
                .text_xs()
                .text_color(cx.theme().green)
                .child(format_bytes(process.memory))
                .into_any_element(),
            _ => div().into_any_element(),
        }
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) {
        self.sort_order = sort;
        self.sort_field = match col_ix {
            0 => ProcessSortField::Pid,
            1 => ProcessSortField::Name,
            2 => ProcessSortField::Cpu,
            3 => ProcessSortField::Memory,
            _ => ProcessSortField::Cpu,
        };
        self.sort_processes();
    }
}

/// Format bytes to human readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// System monitor that collects and displays real-time metrics
pub struct SystemMonitor {
    sys: System,
    disks: Disks,
    data: VecDeque<MetricPoint>,
    time_index: usize,
    gpu_available: bool,
    active_tab: MonitorTab,
    process_table: Entity<TableState<ProcessTableDelegate>>,
    disk_info: Vec<DiskInfo>,
    battery_info: Vec<BatteryInfo>,
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
        let recommended_max = self.device.recommended_max_working_set_size() as f64;
        let current = self.device.current_allocated_size() as f64;
        let vram_usage = if recommended_max > 0.0 {
            (current / recommended_max * 100.0).min(100.0)
        } else {
            0.0
        };
        (0.0, vram_usage)
    }
}

#[cfg(target_os = "windows")]
struct WindowsGpuMonitor {
    #[allow(dead_code)]
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
        use windows::core::Interface;

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
    nvidia_available: bool,
}

#[cfg(target_os = "linux")]
impl LinuxGpuMonitor {
    fn new() -> Option<Self> {
        let nvidia_available = std::process::Command::new("nvidia-smi")
            .arg("--version")
            .output()
            .is_ok();

        Some(Self { nvidia_available })
    }

    fn get_usage(&self) -> (f64, f64) {
        if self.nvidia_available {
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
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let disks = Disks::new_with_refreshed_list();

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

        // Create process table
        let process_delegate = ProcessTableDelegate::new();
        let process_table = cx.new(|cx| {
            TableState::new(process_delegate, window, cx)
                .col_resizable(true)
                .col_movable(false)
        });

        let mut monitor = Self {
            sys,
            disks,
            data: VecDeque::with_capacity(MAX_DATA_POINTS),
            time_index: 0,
            gpu_available,
            active_tab: MonitorTab::System,
            process_table,
            disk_info: Vec::new(),
            battery_info: Vec::new(),
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            gpu_monitor,
        };

        // Collect initial data
        monitor.collect_metrics(cx);

        // Start the update loop
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(INTERVAL).await;

                let result = this.update(cx, |this, cx| {
                    this.collect_metrics(cx);
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

    fn collect_metrics(&mut self, cx: &mut Context<Self>) {
        // Refresh system info
        self.sys.refresh_all();
        self.disks.refresh(true);

        // Calculate CPU usage
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

        // Update process table
        self.process_table.update(cx, |table, cx| {
            table.delegate_mut().update_processes(&self.sys);
            cx.notify();
        });

        // Update disk info (take first disk for status bar)
        self.disk_info = self
            .disks
            .iter()
            .map(|disk| DiskInfo {
                name: disk.name().to_string_lossy().to_string(),
                total: disk.total_space(),
                used: disk.total_space() - disk.available_space(),
            })
            .collect();

        // Update battery info
        self.update_battery_info();
    }

    fn update_battery_info(&mut self) {
        self.battery_info.clear();

        if let Ok(manager) = battery::Manager::new() {
            if let Ok(batteries) = manager.batteries() {
                for battery in batteries.flatten() {
                    let icon = match battery.state() {
                        battery::State::Charging => IconName::BatteryCharging,
                        battery::State::Discharging => IconName::BatteryMedium,
                        battery::State::Full => IconName::BatteryFull,
                        battery::State::Empty => IconName::Battery,
                        _ => IconName::Battery,
                    };

                    self.battery_info.push(BatteryInfo {
                        model: battery
                            .model()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "Battery".to_string()),
                        icon,
                        percentage: battery.state_of_charge().value * 100.0,
                    });
                }
            }
        }
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

    fn set_active_tab(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) {
        self.active_tab = MonitorTab::from_index(index);
        cx.notify();
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
            .min_h(px(160.))
            .flex_1()
            .gap_2()
            .border_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .justify_between()
                    .py_1()
                    .px_3()
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

    fn render_system_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        let data: Vec<MetricPoint> = self.data.iter().cloned().collect();
        let has_gpu = !cfg!(target_os = "macos");

        v_flex()
            .gap_4()
            .flex_1()
            .child(self.render_chart("CPU Usage", data.clone(), |d| d.cpu, cx.theme().red, cx))
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
                    cx.theme().yellow,
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
                cx.theme().green,
                cx,
            ))
    }

    fn render_processes_tab(&self, _cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(Table::new(&self.process_table).stripe(true).small())
    }

    fn render_status_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let primary_disk = self.disk_info.first();
        let primary_battery = self.battery_info.first();

        h_flex()
            .px_3()
            .gap_4()
            .h_7()
            .text_sm()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .text_color(cx.theme().muted_foreground)
            .child(
                h_flex()
                    .gap_4()
                    // Disk info
                    .when_some(primary_disk, |this, disk| {
                        let used_percent = if disk.total > 0 {
                            (disk.used as f64 / disk.total as f64 * 100.0) as f32
                        } else {
                            0.0
                        };
                        this.child(
                            h_flex()
                                .gap_2()
                                .w(px(135.))
                                .items_center()
                                .child(Icon::new(IconName::HardDrive))
                                .child(
                                    Progress::new("status-disk")
                                        .w_12()
                                        .h_2()
                                        .value(used_percent),
                                )
                                .child(format!("{:.0}%", used_percent)),
                        )
                    })
                    // Memory info
                    .child({
                        let mem_percent = self.data.back().map(|p| p.memory as f32).unwrap_or(0.0);
                        h_flex()
                            .gap_2()
                            .w(px(135.))
                            .items_center()
                            .child(Icon::new(IconName::MemoryStick))
                            .child(Progress::new("status-mem").w_12().h_2().value(mem_percent))
                            .child(format!("{:.0}%", mem_percent))
                    })
                    // CPU info
                    .child({
                        let cpu_percent = self.data.back().map(|p| p.cpu as f32).unwrap_or(0.0);
                        h_flex()
                            .gap_2()
                            .w(px(135.))
                            .items_center()
                            .child(Icon::new(IconName::Cpu))
                            .child(Progress::new("status-cpu").w_12().h_2().value(cpu_percent))
                            .child(format!("{:.0}%", cpu_percent))
                    }),
            )
            .child(
                // Battery info
                div().when_some(primary_battery, |this, battery| {
                    this.child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Icon::new(battery.icon.clone()))
                            .child(format!("{:.0}%", battery.percentage)),
                    )
                }),
            )
    }
}

impl Render for SystemMonitor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab_index = self.active_tab as usize;

        v_flex()
            .size_full()
            .child(
                TitleBar::new()
                    .child(
                        TabBar::new("monitor-tabs")
                            .mt(px(1.))
                            .segmented()
                            .py(px(2.))
                            .bg(cx.theme().title_bar)
                            .selected_index(active_tab_index)
                            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                                this.set_active_tab(*ix, window, cx);
                            }))
                            .child(Tab::new().label("System"))
                            .child(Tab::new().label("Processes")),
                    )
                    .child(
                        div()
                            .mr_4()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "{:.1} GB",
                                self.sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0
                            )),
                    ),
            )
            .bg(cx.theme().background)
            .child(
                div()
                    .id("tab-content")
                    .flex_1()
                    .p_3()
                    .overflow_y_scroll()
                    .map(|this| match self.active_tab {
                        MonitorTab::System => this.child(self.render_system_tab(cx)),
                        MonitorTab::Processes => this.child(self.render_processes_tab(cx)),
                    }),
            )
            .child(self.render_status_bar(cx))
    }
}

fn main() {
    let app = Application::new().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        cx.bind_keys([
            #[cfg(target_os = "macos")]
            KeyBinding::new("cmd-q", Quit, None),
            #[cfg(not(target_os = "macos"))]
            KeyBinding::new("alt-f4", Quit, None),
        ]);

        // Handle the Quit action
        cx.on_action(|_: &Quit, cx: &mut App| {
            cx.quit();
        });

        let window_options = WindowOptions {
            titlebar: Some(TitleBar::title_bar_options()),
            window_bounds: Some(WindowBounds::centered(size(px(680.), px(600.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                window.activate_window();
                window.set_window_title("System Monitor");

                Theme::change(ThemeMode::Dark, Some(window), cx);

                let view = cx.new(|cx| SystemMonitor::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
