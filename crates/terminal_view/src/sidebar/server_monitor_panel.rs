//! 终端侧边栏服务器监控面板

use anyhow::{Context as _, Result, anyhow};
use chrono::Utc;
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, Context, EventEmitter, FocusHandle, Focusable, Hsla, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Task,
    Window, div, linear_color_stop, linear_gradient, px,
};
use gpui_component::{
    ActiveTheme, Disableable, IconName, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    chart::{AreaChart, LineChart, PieChart},
    h_flex,
    progress::Progress,
    spinner::Spinner,
    tooltip::Tooltip,
    v_flex,
};
use one_core::gpui_tokio::Tokio;
use one_core::storage::get_config_dir;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use ssh::{ChannelEvent, RusshClient, SshChannel, SshClient, SshConnectConfig};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

const REFRESH_INTERVAL_SECS: u64 = 3;
const HISTORY_LIMIT: usize = 30;
const MAX_HISTORY_X_AXIS_LABELS: usize = 6;
const SERVER_MONITOR_PREFS_FILE: &str = "server-monitor.json";
const REMOTE_HELPER_DIR: &str = "$HOME/.onetcli-monitor";
const REMOTE_HELPER_SCRIPT: &str = "$HOME/.onetcli-monitor/collect.sh";

const REMOTE_MONITOR_SCRIPT: &str = r#"#!/usr/bin/env bash
set -u

session_id=""
while [ $# -gt 0 ]; do
  case "$1" in
    --session)
      session_id="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done

escape_yaml() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

emit_text() {
  printf '  %s: "%s"\n' "$1" "$(escape_yaml "$2")"
}

emit_number() {
  printf '  %s: %s\n' "$1" "$2"
}

supported=true
reason=""
if [ "$(uname -s 2>/dev/null)" != "Linux" ] || [ ! -r /proc/stat ]; then
  supported=false
  reason="Current host is not Linux or /proc is unavailable"
fi

echo "meta:"
emit_text "sessionId" "${session_id:-unknown}"
emit_text "supported" "$supported"
emit_text "reason" "$reason"

echo "os:"
if [ -r /etc/os-release ]; then
  . /etc/os-release
  [ -n "${NAME:-}" ] && emit_text "type" "$NAME"
  [ -n "${ID:-}" ] && emit_text "id" "$ID"
  [ -n "${VERSION:-}" ] && emit_text "version" "$VERSION"
  [ -n "${PRETTY_NAME:-}" ] && emit_text "prettyName" "$PRETTY_NAME"
  [ -n "${VERSION_CODENAME:-}" ] && emit_text "versionCodename" "$VERSION_CODENAME"
else
  emit_text "type" "$(uname -s 2>/dev/null || printf unknown)"
  emit_text "prettyName" "$(uname -sr 2>/dev/null || printf unknown)"
fi

echo "time:"
emit_number "timestamp" "$(date +%s)"
if [ -r /proc/uptime ]; then
  emit_number "uptimeSeconds" "$(cut -d' ' -f1 /proc/uptime)"
fi
emit_text "timezone" "GMT$(date +%z)"
emit_text "timezoneName" "$(date +%Z)"

echo "cpu:"
if [ -r /proc/cpuinfo ]; then
  emit_number "cores" "$(grep -c '^processor' /proc/cpuinfo 2>/dev/null || printf 0)"
fi
echo "  snapshot:"
if [ -r /proc/stat ]; then
  awk '
    /^cpu[0-9]* / || /^cpu / {
      total = $2 + $3 + $4 + $5 + $6 + $7 + $8 + $9 + $10 + $11
      printf "    - name: \"%s\"\n", $1
      printf "      loadUser: %s\n", $2
      printf "      loadSystem: %s\n", $4
      printf "      loadIdle: %s\n", $5
      printf "      loadTotal: %s\n", total
    }
  ' /proc/stat
fi

echo "memory:"
if [ -r /proc/meminfo ]; then
  total="$(awk '/^MemTotal:/ {print $2}' /proc/meminfo)"
  free="$(awk '/^MemFree:/ {print $2}' /proc/meminfo)"
  buffers="$(awk '/^Buffers:/ {print $2}' /proc/meminfo)"
  cached="$(awk '/^Cached:/ {print $2}' /proc/meminfo)"
  swap_total="$(awk '/^SwapTotal:/ {print $2}' /proc/meminfo)"
  swap_free="$(awk '/^SwapFree:/ {print $2}' /proc/meminfo)"
  buffcache=$((buffers + cached))
  used=$((total - free - buffcache))
  swap_used=$((swap_total - swap_free))
  emit_number "total" "$total"
  emit_number "free" "$free"
  emit_number "used" "$used"
  emit_number "buffcache" "$buffcache"
  emit_number "swapTotal" "$swap_total"
  emit_number "swapUsed" "$swap_used"
  emit_number "swapFree" "$swap_free"
fi

echo "fsSize:"
if df -kP -x tmpfs -x devtmpfs -x overlay >/dev/null 2>&1; then
  df -kP -x tmpfs -x devtmpfs -x overlay | tail -n +2 | while read -r fs blocks used available percent mount; do
    printf '  - fs: "%s"\n' "$(escape_yaml "$fs")"
    printf '    size: %s\n' "$blocks"
    printf '    available: %s\n' "$available"
    printf '    percent: "%s"\n' "$percent"
    printf '    mount: "%s"\n' "$(escape_yaml "$mount")"
  done
fi

echo "network:"
echo "  interfaces:"
if [ -d /sys/class/net ]; then
  for nic in /sys/class/net/*; do
    name="$(basename "$nic")"
    case "$name" in
      lo|docker0|veth*|br-*) continue ;;
    esac
    [ -r "$nic/statistics/rx_bytes" ] || continue
    rx="$(cat "$nic/statistics/rx_bytes")"
    tx="$(cat "$nic/statistics/tx_bytes")"
    printf '    - name: "%s"\n' "$(escape_yaml "$name")"
    printf '      rxBytesTotal: %s\n' "$rx"
    printf '      txBytesTotal: %s\n' "$tx"
  done
fi

echo "process:"
emit_number "all" "$(ps -eo pid= | wc -l | awk '{print $1}')"
emit_number "running" "$(ps -eo stat= | awk '$1 ~ /^R/ {c++} END {print c+0}')"
emit_number "blocked" "$(ps -eo stat= | awk '$1 ~ /^D/ {c++} END {print c+0}')"
emit_number "sleeping" "$(ps -eo stat= | awk '$1 ~ /^S/ {c++} END {print c+0}')"
echo "  topsCostCpu:"
ps -eo pid=,pcpu=,rss=,args= --sort=-pcpu | head -n 5 | awk '
  {
    pid=$1; cpu=$2; mem=$3; $1=""; $2=""; $3=""
    sub(/^ +/, "", $0)
    gsub(/\\/,"\\\\",$0); gsub(/"/,"\\\"",$0)
    printf "    - pid: %s\n      cpu: %s\n      memory: %s\n      command: \"%s\"\n", pid, cpu, mem, $0
  }
'
echo "  topsCostMemory:"
ps -eo pid=,pcpu=,rss=,args= --sort=-rss | head -n 5 | awk '
  {
    pid=$1; cpu=$2; mem=$3; $1=""; $2=""; $3=""
    sub(/^ +/, "", $0)
    gsub(/\\/,"\\\\",$0); gsub(/"/,"\\\"",$0)
    printf "    - pid: %s\n      cpu: %s\n      memory: %s\n      command: \"%s\"\n", pid, cpu, mem, $0
  }
'
"#;

#[derive(Clone, Debug)]
pub enum ServerMonitorPanelEvent {
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryLimit(usize);

impl HistoryLimit {
    pub fn new(limit: usize) -> Self {
        Self(limit.max(1))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CpuSnapshot {
    pub name: String,
    pub load_user: u64,
    pub load_system: u64,
    pub load_idle: u64,
    pub load_total: u64,
}

impl CpuSnapshot {
    #[cfg(test)]
    pub fn new(
        name: &str,
        load_user: u64,
        load_system: u64,
        load_idle: u64,
        load_total: u64,
    ) -> Self {
        Self {
            name: name.to_string(),
            load_user,
            load_system,
            load_idle,
            load_total,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CpuUsageCore {
    pub name: String,
    pub percent: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CpuUsageSample {
    pub total_percent: f64,
    pub cores: Vec<CpuUsageCore>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryStats {
    pub total: u64,
    pub free: u64,
    pub used: u64,
    pub buffcache: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub swap_free: u64,
}

impl MemoryStats {
    #[cfg(test)]
    pub fn new(
        total: u64,
        free: u64,
        used: u64,
        buffcache: u64,
        swap_total: u64,
        swap_used: u64,
        swap_free: u64,
    ) -> Self {
        Self {
            total,
            free,
            used,
            buffcache,
            swap_total,
            swap_used,
            swap_free,
        }
    }

    pub fn used_percent(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.used as f64 * 100.0 / self.total as f64
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessEntry {
    pub pid: u32,
    pub cpu: f64,
    pub memory: u64,
    pub command: String,
}

impl ProcessEntry {
    #[cfg(test)]
    pub fn new(pid: u32, cpu: f64, memory: u64, command: &str) -> Self {
        Self {
            pid,
            cpu,
            memory,
            command: command.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct OsStats {
    pretty_name: String,
    kind: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct TimeStats {
    timestamp: i64,
    uptime_seconds: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
struct DiskStats {
    fs: String,
    size: u64,
    available: u64,
    percent: f64,
    mount: String,
}

#[derive(Clone, Debug, PartialEq)]
struct ProcessStats {
    all: u64,
    running: u64,
    blocked: u64,
    sleeping: u64,
    top_cpu: Vec<ProcessEntry>,
    top_memory: Vec<ProcessEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkTotals {
    interfaces: Vec<NetworkInterfaceTotal>,
}

#[derive(Clone, Debug, PartialEq)]
struct NetworkInterfaceTotal {
    name: String,
    rx_total: u64,
    tx_total: u64,
}

impl NetworkTotals {
    #[cfg(test)]
    pub fn new(entries: Vec<(&str, u64, u64)>) -> Self {
        Self {
            interfaces: entries
                .into_iter()
                .map(|(name, rx_total, tx_total)| NetworkInterfaceTotal {
                    name: name.to_string(),
                    rx_total,
                    tx_total,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkRateSample {
    pub rx_delta: u64,
    pub tx_delta: u64,
    pub rx_bytes_per_sec: f64,
    pub tx_bytes_per_sec: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct MonitorMeta {
    supported: bool,
    reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct ServerStats {
    meta: MonitorMeta,
    os: Option<OsStats>,
    time: Option<TimeStats>,
    cpu_snapshots: Vec<CpuSnapshot>,
    cpu_usage: Option<CpuUsageSample>,
    memory: Option<MemoryStats>,
    disks: Vec<DiskStats>,
    network_totals: Option<NetworkTotals>,
    network_rate: Option<NetworkRateSample>,
    process: Option<ProcessStats>,
}

#[derive(Clone)]
struct HistoryPoint {
    label: SharedString,
    value: f64,
    ceiling: f64,
}

#[derive(Clone)]
struct NetworkHistoryPoint {
    label: SharedString,
    rx: f64,
    tx: f64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ServerMonitorPreferences {
    #[serde(default)]
    enabled_connections: BTreeMap<String, bool>,
}

fn server_monitor_prefs_path() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)?;
    }
    Ok(config_dir.join(SERVER_MONITOR_PREFS_FILE))
}

fn load_server_monitor_preferences() -> ServerMonitorPreferences {
    let Ok(path) = server_monitor_prefs_path() else {
        return ServerMonitorPreferences::default();
    };
    if !path.exists() {
        return ServerMonitorPreferences::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
        Err(_) => ServerMonitorPreferences::default(),
    }
}

fn save_server_monitor_preferences(preferences: &ServerMonitorPreferences) -> Result<()> {
    let path = server_monitor_prefs_path()?;
    let json = serde_json::to_string_pretty(preferences)?;
    std::fs::write(&path, json)?;
    Ok(())
}

pub struct ServerMonitorPanel {
    connection_id: Option<i64>,
    ssh_config: SshConnectConfig,
    session_id: String,
    focus_handle: FocusHandle,
    auto_show: bool,
    monitor_enabled: bool,
    preparing: bool,
    in_flight: bool,
    last_error: Option<String>,
    refresh_task: Option<Task<()>>,
    client: Option<Arc<Mutex<RusshClient>>>,
    current_stats: Option<ServerStats>,
    previous_cpu: Option<Vec<CpuSnapshot>>,
    previous_network: Option<NetworkTotals>,
    previous_timestamp: Option<i64>,
    cpu_history: Vec<f64>,
    rx_history: Vec<f64>,
    tx_history: Vec<f64>,
}

impl ServerMonitorPanel {
    pub fn load_monitor_enabled(connection_id: Option<i64>) -> bool {
        let Some(connection_id) = connection_id else {
            return false;
        };

        load_server_monitor_preferences()
            .enabled_connections
            .get(&connection_id.to_string())
            .copied()
            .unwrap_or(false)
    }

    pub fn new(
        connection_id: Option<i64>,
        ssh_config: SshConnectConfig,
        auto_show: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            connection_id,
            ssh_config,
            session_id: format!("session-{}", Utc::now().timestamp_millis()),
            focus_handle: cx.focus_handle(),
            auto_show,
            monitor_enabled: false,
            preparing: false,
            in_flight: false,
            last_error: None,
            refresh_task: None,
            client: None,
            current_stats: None,
            previous_cpu: None,
            previous_network: None,
            previous_timestamp: None,
            cpu_history: Vec::new(),
            rx_history: Vec::new(),
            tx_history: Vec::new(),
        }
    }

    pub fn restore_monitoring(&mut self, cx: &mut Context<Self>) {
        if !self.auto_show || self.monitor_enabled || self.preparing {
            return;
        }
        self.monitor_enabled = true;
        self.refresh_now(cx);
        self.ensure_refresh_loop(cx);
    }

    pub fn reconnect(&mut self, cx: &mut Context<Self>) {
        self.client = None;
        self.in_flight = false;
        if self.monitor_enabled {
            self.refresh_now(cx);
        } else {
            cx.notify();
        }
    }

    fn start_monitoring(&mut self, cx: &mut Context<Self>) {
        if self.preparing {
            return;
        }

        self.auto_show = true;
        self.persist_monitor_enabled(true);
        self.preparing = true;
        self.last_error = None;
        cx.notify();

        let config = self.ssh_config.clone();
        let session_id = self.session_id.clone();
        let task = Tokio::spawn(cx, async move {
            let client = Arc::new(Mutex::new(RusshClient::connect(config).await?));
            prepare_remote_monitor(client.clone()).await?;
            let payload = collect_remote_stats(client.clone(), &session_id).await?;
            let stats = parse_server_stats(&payload)?;
            Ok::<_, anyhow::Error>((client, stats))
        });

        cx.spawn(async move |this, cx| match task.await {
            Ok(Ok((client, stats))) => {
                let _ = this.update(cx, |this, cx| {
                    this.preparing = false;
                    this.monitor_enabled = true;
                    this.client = Some(client);
                    this.last_error = None;
                    this.apply_stats(stats);
                    this.ensure_refresh_loop(cx);
                    cx.notify();
                });
            }
            Ok(Err(error)) => {
                let _ = this.update(cx, |this, cx| {
                    this.preparing = false;
                    this.monitor_enabled = false;
                    this.client = None;
                    this.last_error = Some(format!("{error}"));
                    cx.notify();
                });
            }
            Err(error) => {
                let _ = this.update(cx, |this, cx| {
                    this.preparing = false;
                    this.monitor_enabled = false;
                    this.client = None;
                    this.last_error = Some(format!("{error}"));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn ensure_refresh_loop(&mut self, cx: &mut Context<Self>) {
        if self.refresh_task.is_some() {
            cx.notify();
            return;
        }

        self.refresh_task = Some(cx.spawn(async move |this, cx| {
            loop {
                let should_continue = this
                    .update(cx, |this, cx| {
                        if !this.monitor_enabled {
                            this.refresh_task = None;
                            return false;
                        }
                        this.refresh_now(cx);
                        true
                    })
                    .unwrap_or(false);

                if !should_continue {
                    break;
                }

                cx.background_executor()
                    .timer(Duration::from_secs(REFRESH_INTERVAL_SECS))
                    .await;
            }
        }));
    }

    fn refresh_now(&mut self, cx: &mut Context<Self>) {
        if !self.monitor_enabled || self.preparing || self.in_flight {
            return;
        }

        self.in_flight = true;
        let config = self.ssh_config.clone();
        let session_id = self.session_id.clone();
        let existing_client = self.client.clone();
        let needs_prepare = existing_client.is_none();

        let task = Tokio::spawn(cx, async move {
            refresh_remote_stats(config, existing_client, &session_id, needs_prepare).await
        });

        cx.spawn(async move |this, cx| match task.await {
            Ok(Ok((client, stats))) => {
                let _ = this.update(cx, |this, cx| {
                    this.in_flight = false;
                    this.client = Some(client);
                    this.last_error = None;
                    this.apply_stats(stats);
                    cx.notify();
                });
            }
            Ok(Err(error)) => {
                let _ = this.update(cx, |this, cx| {
                    this.in_flight = false;
                    this.client = None;
                    this.last_error = Some(format!("{error}"));
                    cx.notify();
                });
            }
            Err(error) => {
                let _ = this.update(cx, |this, cx| {
                    this.in_flight = false;
                    this.client = None;
                    this.last_error = Some(format!("{error}"));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn apply_stats(&mut self, mut stats: ServerStats) {
        if let Some(time) = &stats.time {
            if let Some(previous) = &self.previous_cpu {
                stats.cpu_usage = Some(sample_cpu_usage(previous, &stats.cpu_snapshots));
                if let Some(cpu_usage) = &stats.cpu_usage {
                    push_history_point(
                        &mut self.cpu_history,
                        cpu_usage.total_percent,
                        HistoryLimit::new(HISTORY_LIMIT),
                    );
                }
            }

            if let Some(current_network) = &stats.network_totals {
                if let (Some(previous_network), Some(previous_timestamp)) =
                    (&self.previous_network, self.previous_timestamp)
                {
                    let interval_secs = (time.timestamp - previous_timestamp).max(1) as f64;
                    let sampled =
                        sample_network_rates(previous_network, current_network, interval_secs);
                    push_history_point(
                        &mut self.rx_history,
                        sampled.rx_bytes_per_sec,
                        HistoryLimit::new(HISTORY_LIMIT),
                    );
                    push_history_point(
                        &mut self.tx_history,
                        sampled.tx_bytes_per_sec,
                        HistoryLimit::new(HISTORY_LIMIT),
                    );
                    stats.network_rate = Some(sampled);
                }
            }

            self.previous_timestamp = Some(time.timestamp);
        }

        if !stats.cpu_snapshots.is_empty() {
            self.previous_cpu = Some(stats.cpu_snapshots.clone());
        }
        if let Some(network_totals) = &stats.network_totals {
            self.previous_network = Some(network_totals.clone());
        }

        self.current_stats = Some(stats);
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let subtitle = self
            .current_stats
            .as_ref()
            .and_then(|stats| stats.os.as_ref().map(|os| os.pretty_name.clone()))
            .unwrap_or_else(|| t!("ServerMonitor.title").to_string());
        let uptime = self
            .current_stats
            .as_ref()
            .and_then(|stats| stats.time.as_ref())
            .and_then(|time| time.uptime_seconds)
            .map(format_uptime)
            .unwrap_or_default();

        h_flex()
            .h_11()
            .px_3()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                v_flex()
                    .gap_0p5()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .child(t!("ServerMonitor.title")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(if uptime.is_empty() {
                                subtitle
                            } else {
                                format!("{subtitle} · {uptime}")
                            }),
                    ),
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("server-monitor-refresh")
                            .ghost()
                            .small()
                            .icon(IconName::Refresh)
                            .disabled(!self.monitor_enabled || self.preparing)
                            .tooltip(t!("ServerMonitor.refresh"))
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.refresh_now(cx);
                            })),
                    )
                    .child(
                        Button::new("server-monitor-close")
                            .ghost()
                            .small()
                            .icon(IconName::Close)
                            .tooltip(t!("ServerMonitor.close"))
                            .on_click(cx.listener(|_this, _, _window, cx| {
                                cx.emit(ServerMonitorPanelEvent::Close);
                            })),
                    ),
            )
    }

    fn render_start_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .px_6()
            .child(
                div()
                    .text_center()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("ServerMonitor.start_hint")),
            )
            .when_some(self.last_error.clone(), |this, error| {
                this.child(
                    div()
                        .w_full()
                        .rounded_md()
                        .bg(cx.theme().danger.opacity(0.08))
                        .p_3()
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .child(error),
                )
            })
            .child(
                Button::new("server-monitor-start")
                    .label(t!("ServerMonitor.start"))
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.start_monitoring(cx);
                    })),
            )
    }

    fn persist_monitor_enabled(&self, enabled: bool) {
        let Some(connection_id) = self.connection_id else {
            return;
        };

        let mut preferences = load_server_monitor_preferences();
        let key = connection_id.to_string();
        let current = preferences
            .enabled_connections
            .get(&key)
            .copied()
            .unwrap_or(false);
        if current == enabled {
            return;
        }

        if enabled {
            preferences.enabled_connections.insert(key, true);
        } else {
            preferences.enabled_connections.remove(&key);
        }

        if let Err(error) = save_server_monitor_preferences(&preferences) {
            tracing::warn!(
                "Failed to persist server monitor preference for connection {}: {}",
                connection_id,
                error
            );
        }
    }

    fn render_preparing(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child(Spinner::new().small())
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("ServerMonitor.preparing")),
            )
    }

    fn render_banner(&self, cx: &mut Context<Self>) -> AnyElement {
        let unsupported = self
            .current_stats
            .as_ref()
            .and_then(|stats| (!stats.meta.supported).then_some(stats.meta.reason.clone()))
            .flatten();

        let message = unsupported.or_else(|| self.last_error.clone());
        match message {
            Some(message) => div()
                .rounded_md()
                .bg(cx.theme().warning.opacity(0.12))
                .p_3()
                .text_xs()
                .text_color(cx.theme().warning)
                .child(message)
                .into_any_element(),
            None => div().into_any_element(),
        }
    }

    fn render_metrics(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let stats = self.current_stats.as_ref();
        let cpu_value = stats
            .and_then(|stats| stats.cpu_usage.as_ref())
            .map(|cpu| format!("{:.1}%", cpu.total_percent))
            .unwrap_or_else(|| t!("ServerMonitor.unavailable").to_string());
        let memory_text = stats
            .and_then(|stats| stats.memory.as_ref())
            .map(|memory| {
                format!(
                    "{} / {} ({:.1}%)",
                    format_kib(memory.used),
                    format_kib(memory.total),
                    memory.used_percent()
                )
            })
            .unwrap_or_else(|| t!("ServerMonitor.unavailable").to_string());
        let network_text = stats
            .and_then(|stats| stats.network_rate.as_ref())
            .map(|rate| {
                format!(
                    "↓{} ↑{}",
                    format_bytes_per_sec(rate.rx_bytes_per_sec),
                    format_bytes_per_sec(rate.tx_bytes_per_sec)
                )
            })
            .unwrap_or_else(|| t!("ServerMonitor.unavailable").to_string());

        v_flex()
            .gap_3()
            .child(self.render_banner(cx))
            .child(self.render_card(
                t!("ServerMonitor.cpu"),
                cpu_value,
                self.render_cpu_chart(cx),
                cx,
            ))
            .child(self.render_card(
                t!("ServerMonitor.memory"),
                memory_text,
                self.render_memory_chart(cx),
                cx,
            ))
            .child(self.render_card(
                t!("ServerMonitor.disk"),
                t!("ServerMonitor.disk_hint").to_string(),
                self.render_disk_list(cx),
                cx,
            ))
            .child(self.render_card(
                t!("ServerMonitor.network"),
                network_text,
                self.render_network_chart(cx),
                cx,
            ))
            .child(self.render_card(
                t!("ServerMonitor.processes"),
                t!("ServerMonitor.process_hint").to_string(),
                self.render_process_lists(cx),
                cx,
            ))
    }

    fn render_card(
        &self,
        title: impl Into<SharedString>,
        summary: impl Into<SharedString>,
        body: AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let title: SharedString = title.into();
        let summary: SharedString = summary.into();
        v_flex()
            .w_full()
            .gap_2()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .p_3()
            .child(
                h_flex()
                    .justify_between()
                    .items_start()
                    .child(div().text_sm().font_semibold().child(title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(summary),
                    ),
            )
            .child(body)
            .into_any_element()
    }

    fn render_cpu_chart(&self, cx: &mut Context<Self>) -> AnyElement {
        let cpu_usage = self
            .current_stats
            .as_ref()
            .and_then(|stats| stats.cpu_usage.as_ref());

        if self.cpu_history.is_empty() {
            return cpu_usage
                .filter(|usage| !usage.cores.is_empty())
                .map(|usage| render_cpu_core_grid(&usage.cores, cx))
                .unwrap_or_else(|| placeholder(t!("ServerMonitor.awaiting_data"), cx));
        }

        cpu_usage
            .filter(|usage| !usage.cores.is_empty())
            .map(|usage| {
                v_flex()
                    .gap_3()
                    .child(render_cpu_history_chart(&self.cpu_history, cx))
                    .child(render_cpu_core_grid(&usage.cores, cx))
                    .into_any_element()
            })
            .unwrap_or_else(|| render_cpu_history_chart(&self.cpu_history, cx))
    }

    fn render_memory_chart(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(memory) = self
            .current_stats
            .as_ref()
            .and_then(|stats| stats.memory.as_ref())
        else {
            return placeholder(t!("ServerMonitor.unavailable"), cx);
        };

        let segments = vec![
            MemorySegment::new("used", memory.used as f64, cx.theme().chart_2),
            MemorySegment::new("cache", memory.buffcache as f64, cx.theme().chart_3),
            MemorySegment::new("free", memory.free as f64, cx.theme().chart_4),
        ];

        h_flex()
            .items_center()
            .justify_between()
            .child(
                div().h(px(112.0)).w(px(112.0)).child(
                    PieChart::new(segments.clone())
                        .value(|segment| segment.value as f32)
                        .color(|segment| segment.color)
                        .inner_radius(30.0)
                        .outer_radius(52.0),
                ),
            )
            .child(
                v_flex()
                    .gap_1()
                    .children(segments.into_iter().map(|segment| {
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().size_2().rounded_full().bg(segment.color))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!(
                                        "{} {}",
                                        segment.label,
                                        format_kib(segment.value as u64)
                                    )),
                            )
                    })),
            )
            .into_any_element()
    }

    fn render_disk_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let disks = self
            .current_stats
            .as_ref()
            .map(|stats| stats.disks.clone())
            .unwrap_or_default();

        if disks.is_empty() {
            return placeholder(t!("ServerMonitor.unavailable"), cx);
        }

        v_flex()
            .gap_2()
            .children(disks.into_iter().map(|disk| {
                let mount = disk.mount.clone();
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .justify_between()
                            .child(div().text_xs().child(mount.clone()))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!(
                                        "{:.0}% · {}",
                                        disk.percent,
                                        format_kib(disk.available)
                                    )),
                            ),
                    )
                    .child(
                        Progress::new(SharedString::from(format!("disk-{mount}")))
                            .value(disk.percent as f32),
                    )
            }))
            .into_any_element()
    }

    fn render_network_chart(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.rx_history.is_empty() || self.tx_history.is_empty() {
            return placeholder(t!("ServerMonitor.awaiting_data"), cx);
        }

        let points = network_history_points(&self.rx_history, &self.tx_history);
        div()
            .h(px(120.0))
            .child(
                LineChart::new(points)
                    .x(|point| point.label.clone())
                    .y(|point| point.rx)
                    .stroke(cx.theme().chart_2)
                    .dot()
                    .y(|point| point.tx)
                    .stroke(cx.theme().chart_4)
                    .tick_margin(history_tick_margin(self.rx_history.len())),
            )
            .into_any_element()
    }

    fn render_process_lists(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(process) = self
            .current_stats
            .as_ref()
            .and_then(|stats| stats.process.as_ref())
        else {
            return placeholder(t!("ServerMonitor.unavailable"), cx);
        };

        h_flex()
            .gap_4()
            .items_start()
            .w_full()
            .min_w(px(0.0))
            .child(render_process_column(
                t!("ServerMonitor.by_cpu").to_string(),
                process.top_cpu.clone(),
                cx,
            ))
            .child(render_process_column(
                t!("ServerMonitor.by_memory").to_string(),
                process.top_memory.clone(),
                cx,
            ))
            .into_any_element()
    }
}

impl EventEmitter<ServerMonitorPanelEvent> for ServerMonitorPanel {}

impl Focusable for ServerMonitorPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ServerMonitorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_header(cx))
            .child(
                div()
                    .id("server-monitor-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(
                        v_flex()
                            .flex_shrink_0()
                            .p_3()
                            .gap_3()
                            .child(if self.preparing {
                                self.render_preparing(cx).into_any_element()
                            } else if self.monitor_enabled {
                                self.render_metrics(cx).into_any_element()
                            } else {
                                self.render_start_state(cx).into_any_element()
                            }),
                    ),
            )
    }
}

#[derive(Clone)]
struct MemorySegment {
    label: SharedString,
    value: f64,
    color: Hsla,
}

impl MemorySegment {
    fn new(label: &str, value: f64, color: Hsla) -> Self {
        Self {
            label: SharedString::from(label.to_string()),
            value,
            color,
        }
    }
}

fn render_process_column(
    title: String,
    entries: Vec<ProcessEntry>,
    cx: &mut Context<ServerMonitorPanel>,
) -> AnyElement {
    v_flex()
        .flex_1()
        .min_w(px(0.0))
        .gap_1()
        .child(div().text_xs().font_semibold().child(title))
        .children(entries.into_iter().map(|entry| {
            let command = entry.command.clone();
            let tooltip_command = command.clone();
            let display_command = command.clone();
            v_flex()
                .id(SharedString::from(format!(
                    "process-{}-{}",
                    entry.pid, command
                )))
                .w_full()
                .min_w(px(0.0))
                .rounded_md()
                .bg(cx.theme().secondary)
                .p_2()
                .gap_0p5()
                .cursor_pointer()
                .hover(|style| style.bg(cx.theme().list_active))
                .tooltip(move |window, cx| Tooltip::new(tooltip_command.clone()).build(window, cx))
                .child(
                    h_flex()
                        .justify_between()
                        .child(div().text_xs().child(format!("#{}", entry.pid)))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{:.1}% · {}", entry.cpu, format_kib(entry.memory))),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .min_w(px(0.0))
                        .text_xs()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(display_command),
                )
        }))
        .into_any_element()
}

fn render_cpu_core_grid(
    cores: &[CpuUsageCore],
    cx: &mut Context<ServerMonitorPanel>,
) -> AnyElement {
    v_flex()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("Per-core"),
        )
        .child(
            h_flex()
                .w_full()
                .flex_wrap()
                .gap_2()
                .children(cores.iter().map(|core| {
                    let value = core.percent.clamp(0.0, 100.0);
                    let label = core.name.clone();
                    v_flex()
                        .w(px(108.0))
                        .gap_1()
                        .child(
                            h_flex()
                                .justify_between()
                                .child(div().text_xs().child(label.clone()))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("{value:.1}%")),
                                ),
                        )
                        .child(
                            Progress::new(SharedString::from(format!("cpu-core-{label}")))
                                .value(value as f32),
                        )
                })),
        )
        .into_any_element()
}

fn render_cpu_history_chart(values: &[f64], cx: &App) -> AnyElement {
    let points = history_points(values);
    div()
        .h(px(120.0))
        .child(
            AreaChart::new(points)
                .x(|point| point.label.clone())
                .y(|point| point.value)
                .stroke(cx.theme().chart_1)
                .fill(linear_gradient(
                    0.0,
                    linear_color_stop(cx.theme().chart_1.opacity(0.35), 1.0),
                    linear_color_stop(cx.theme().background.opacity(0.1), 0.0),
                ))
                // Keep the Y axis stable at 0..100 so idle CPUs still render a visible chart.
                .y(|point| point.ceiling)
                .stroke(cx.theme().chart_1.opacity(0.0))
                .fill(cx.theme().chart_1.opacity(0.0))
                .tick_margin(history_tick_margin(values.len())),
        )
        .into_any_element()
}

fn placeholder(message: impl Into<SharedString>, cx: &App) -> AnyElement {
    let message: SharedString = message.into();
    div()
        .h(px(72.0))
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(message)
        .into_any_element()
}

fn history_points(values: &[f64]) -> Vec<HistoryPoint> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| HistoryPoint {
            label: SharedString::from(index.to_string()),
            value: *value,
            ceiling: 100.0,
        })
        .collect()
}

fn network_history_points(rx: &[f64], tx: &[f64]) -> Vec<NetworkHistoryPoint> {
    rx.iter()
        .zip(tx.iter())
        .enumerate()
        .map(|(index, (rx, tx))| NetworkHistoryPoint {
            label: SharedString::from(index.to_string()),
            rx: *rx,
            tx: *tx,
        })
        .collect()
}

fn history_tick_margin(point_count: usize) -> usize {
    if point_count <= MAX_HISTORY_X_AXIS_LABELS {
        1
    } else {
        (point_count + MAX_HISTORY_X_AXIS_LABELS - 1) / MAX_HISTORY_X_AXIS_LABELS
    }
}

pub fn push_history_point(history: &mut Vec<f64>, value: f64, limit: HistoryLimit) {
    history.push(value);
    if history.len() > limit.0 {
        let excess = history.len() - limit.0;
        history.drain(0..excess);
    }
}

pub fn sample_cpu_usage(previous: &[CpuSnapshot], current: &[CpuSnapshot]) -> CpuUsageSample {
    let mut total_percent = 0.0;
    let mut cores = Vec::new();

    for current_snapshot in current {
        let Some(previous_snapshot) = previous
            .iter()
            .find(|snapshot| snapshot.name == current_snapshot.name)
        else {
            continue;
        };

        let delta_total = current_snapshot
            .load_total
            .saturating_sub(previous_snapshot.load_total);
        let delta_idle = current_snapshot
            .load_idle
            .saturating_sub(previous_snapshot.load_idle);
        let percent = if delta_total == 0 {
            0.0
        } else {
            (delta_total.saturating_sub(delta_idle)) as f64 * 100.0 / delta_total as f64
        };

        if current_snapshot.name == "cpu" {
            total_percent = percent;
        } else {
            cores.push(CpuUsageCore {
                name: current_snapshot.name.clone(),
                percent,
            });
        }
    }

    CpuUsageSample {
        total_percent,
        cores,
    }
}

pub fn sample_network_rates(
    previous: &NetworkTotals,
    current: &NetworkTotals,
    interval_secs: f64,
) -> NetworkRateSample {
    let mut rx_delta = 0u64;
    let mut tx_delta = 0u64;

    for current_interface in &current.interfaces {
        if let Some(previous_interface) = previous
            .interfaces
            .iter()
            .find(|candidate| candidate.name == current_interface.name)
        {
            rx_delta += current_interface
                .rx_total
                .saturating_sub(previous_interface.rx_total);
            tx_delta += current_interface
                .tx_total
                .saturating_sub(previous_interface.tx_total);
        }
    }

    NetworkRateSample {
        rx_delta,
        tx_delta,
        rx_bytes_per_sec: rx_delta as f64 / interval_secs.max(1.0),
        tx_bytes_per_sec: tx_delta as f64 / interval_secs.max(1.0),
    }
}

pub fn split_sections(input: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current_section = None::<String>;

    for raw_line in input.lines() {
        let line = raw_line.trim_end();
        if line.trim().is_empty() {
            continue;
        }

        if !raw_line.starts_with(' ') && line.ends_with(':') && !line.contains(": ") {
            let section = line.trim_end_matches(':').to_string();
            grouped.entry(section.clone()).or_default();
            current_section = Some(section);
            continue;
        }

        if let Some(section) = &current_section {
            grouped
                .entry(section.clone())
                .or_default()
                .push(raw_line.to_string());
        }
    }

    grouped
        .into_iter()
        .map(|(section, body)| (section, flatten_section(&body)))
        .collect()
}

fn flatten_section(lines: &[String]) -> BTreeMap<String, String> {
    #[derive(Clone)]
    struct ParseContext {
        indent: usize,
        path: String,
        next_index: usize,
    }

    let mut output = BTreeMap::new();
    let mut stack: Vec<ParseContext> = Vec::new();
    let mut top_level_index = 0usize;

    for raw_line in lines {
        let indent = raw_line.chars().take_while(|c| *c == ' ').count();
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        while stack.last().is_some_and(|context| indent <= context.indent) {
            stack.pop();
        }

        if let Some(item) = trimmed.strip_prefix("- ") {
            let (path, parent_indent) = if let Some(parent) = stack.last_mut() {
                let index = parent.next_index;
                parent.next_index += 1;
                (format!("{}[{index}]", parent.path), parent.indent)
            } else {
                let index = top_level_index;
                top_level_index += 1;
                (format!("[{index}]"), 0)
            };
            if let Some((key, value)) = split_key_value(item) {
                output.insert(format!("{path}.{key}"), clean_scalar(value));
            }
            stack.push(ParseContext {
                indent: indent.max(parent_indent),
                path,
                next_index: 0,
            });
            continue;
        }

        if trimmed.ends_with(':') && !trimmed.contains(": ") {
            let key = trimmed.trim_end_matches(':').trim();
            let path = stack
                .last()
                .map(|parent| format!("{}.{}", parent.path, key))
                .unwrap_or_else(|| key.to_string());
            stack.push(ParseContext {
                indent,
                path,
                next_index: 0,
            });
            continue;
        }

        if let Some((key, value)) = split_key_value(trimmed) {
            let path = stack
                .last()
                .map(|parent| format!("{}.{}", parent.path, key))
                .unwrap_or_else(|| key.to_string());
            output.insert(path, clean_scalar(value));
        }
    }

    output
}

fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(':')?;
    Some((key.trim(), value.trim()))
}

fn clean_scalar(raw: &str) -> String {
    raw.trim_matches('"').to_string()
}

fn parse_server_stats(input: &str) -> Result<ServerStats> {
    let sections = split_sections(input);
    let meta_section = sections.get("meta");
    let supported = meta_section
        .and_then(|section| section.get("supported"))
        .map(|value| value == "true")
        .unwrap_or(true);
    let reason = meta_section
        .and_then(|section| section.get("reason"))
        .cloned()
        .filter(|value| !value.is_empty());

    let os = sections.get("os").and_then(|section| {
        let pretty_name = section.get("prettyName")?.clone();
        Some(OsStats {
            pretty_name,
            kind: section.get("type").cloned(),
        })
    });

    let time = sections.get("time").and_then(|section| {
        Some(TimeStats {
            timestamp: parse_i64(section.get("timestamp")?)?,
            uptime_seconds: section
                .get("uptimeSeconds")
                .and_then(|value| parse_f64(value)),
        })
    });

    let cpu_snapshots = parse_cpu_snapshots(sections.get("cpu"));
    let memory = sections.get("memory").and_then(parse_memory_stats);
    let disks = parse_disk_stats(sections.get("fsSize"));
    let network_totals = sections.get("network").map(parse_network_totals);
    let process = sections.get("process").and_then(parse_process_stats);

    Ok(ServerStats {
        meta: MonitorMeta { supported, reason },
        os,
        time,
        cpu_snapshots,
        cpu_usage: None,
        memory,
        disks,
        network_totals,
        network_rate: None,
        process,
    })
}

fn parse_cpu_snapshots(section: Option<&BTreeMap<String, String>>) -> Vec<CpuSnapshot> {
    let Some(section) = section else {
        return Vec::new();
    };

    parse_indexed("snapshot", section)
        .into_iter()
        .filter_map(|index| {
            Some(CpuSnapshot {
                name: section.get(&format!("snapshot[{index}].name"))?.clone(),
                load_user: parse_u64(section.get(&format!("snapshot[{index}].loadUser"))?)?,
                load_system: parse_u64(section.get(&format!("snapshot[{index}].loadSystem"))?)?,
                load_idle: parse_u64(section.get(&format!("snapshot[{index}].loadIdle"))?)?,
                load_total: parse_u64(section.get(&format!("snapshot[{index}].loadTotal"))?)?,
            })
        })
        .collect()
}

fn parse_memory_stats(section: &BTreeMap<String, String>) -> Option<MemoryStats> {
    Some(MemoryStats {
        total: parse_u64(section.get("total")?)?,
        free: parse_u64(section.get("free")?)?,
        used: parse_u64(section.get("used")?)?,
        buffcache: parse_u64(section.get("buffcache")?)?,
        swap_total: parse_u64(section.get("swapTotal")?)?,
        swap_used: parse_u64(section.get("swapUsed")?)?,
        swap_free: parse_u64(section.get("swapFree")?)?,
    })
}

fn parse_disk_stats(section: Option<&BTreeMap<String, String>>) -> Vec<DiskStats> {
    let Some(section) = section else {
        return Vec::new();
    };

    parse_indexed("", section)
        .into_iter()
        .filter_map(|index| {
            Some(DiskStats {
                fs: section.get(&format!("[{index}].fs"))?.clone(),
                size: parse_u64(section.get(&format!("[{index}].size"))?)?,
                available: parse_u64(section.get(&format!("[{index}].available"))?)?,
                percent: parse_percent(section.get(&format!("[{index}].percent"))?)?,
                mount: section.get(&format!("[{index}].mount"))?.clone(),
            })
        })
        .collect()
}

fn parse_network_totals(section: &BTreeMap<String, String>) -> NetworkTotals {
    NetworkTotals {
        interfaces: parse_indexed("interfaces", section)
            .into_iter()
            .filter_map(|index| {
                Some(NetworkInterfaceTotal {
                    name: section.get(&format!("interfaces[{index}].name"))?.clone(),
                    rx_total: parse_u64(
                        section.get(&format!("interfaces[{index}].rxBytesTotal"))?,
                    )?,
                    tx_total: parse_u64(
                        section.get(&format!("interfaces[{index}].txBytesTotal"))?,
                    )?,
                })
            })
            .collect(),
    }
}

fn parse_process_stats(section: &BTreeMap<String, String>) -> Option<ProcessStats> {
    Some(ProcessStats {
        all: parse_u64(section.get("all")?)?,
        running: parse_u64(section.get("running")?)?,
        blocked: parse_u64(section.get("blocked")?)?,
        sleeping: parse_u64(section.get("sleeping")?)?,
        top_cpu: parse_process_entries("topsCostCpu", section),
        top_memory: parse_process_entries("topsCostMemory", section),
    })
}

fn parse_process_entries(prefix: &str, section: &BTreeMap<String, String>) -> Vec<ProcessEntry> {
    parse_indexed(prefix, section)
        .into_iter()
        .filter_map(|index| {
            Some(ProcessEntry {
                pid: parse_u32(section.get(&format!("{prefix}[{index}].pid"))?)?,
                cpu: parse_f64(section.get(&format!("{prefix}[{index}].cpu"))?)?,
                memory: parse_u64(section.get(&format!("{prefix}[{index}].memory"))?)?,
                command: section.get(&format!("{prefix}[{index}].command"))?.clone(),
            })
        })
        .collect()
}

fn parse_indexed(prefix: &str, section: &BTreeMap<String, String>) -> Vec<usize> {
    let needle = if prefix.is_empty() {
        "[".to_string()
    } else {
        format!("{prefix}[")
    };

    let mut indices = section
        .keys()
        .filter_map(|key| {
            let rest = key.strip_prefix(&needle)?;
            let (index, _) = rest.split_once(']')?;
            index.parse::<usize>().ok()
        })
        .collect::<Vec<_>>();
    indices.sort_unstable();
    indices.dedup();
    indices
}

fn parse_u64(value: &str) -> Option<u64> {
    value.parse::<u64>().ok().or_else(|| {
        let parsed = value.parse::<f64>().ok()?;
        if !parsed.is_finite() || parsed < 0.0 {
            return None;
        }
        Some(parsed.round() as u64)
    })
}

fn parse_u32(value: &str) -> Option<u32> {
    value.parse::<u32>().ok()
}

fn parse_i64(value: &str) -> Option<i64> {
    value.parse::<i64>().ok()
}

fn parse_f64(value: &str) -> Option<f64> {
    value.trim_end_matches('%').parse::<f64>().ok()
}

fn parse_percent(value: &str) -> Option<f64> {
    parse_f64(value)
}

async fn refresh_remote_stats(
    config: SshConnectConfig,
    existing_client: Option<Arc<Mutex<RusshClient>>>,
    session_id: &str,
    needs_prepare: bool,
) -> Result<(Arc<Mutex<RusshClient>>, ServerStats)> {
    match refresh_remote_stats_inner(
        config.clone(),
        existing_client.clone(),
        session_id,
        needs_prepare,
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(_error) if existing_client.is_some() => {
            let client = Arc::new(Mutex::new(RusshClient::connect(config).await?));
            prepare_remote_monitor(client.clone()).await?;
            let payload = collect_remote_stats(client.clone(), session_id).await?;
            let stats = parse_server_stats(&payload)?;
            Ok((client, stats))
        }
        Err(error) => Err(error),
    }
}

async fn refresh_remote_stats_inner(
    config: SshConnectConfig,
    existing_client: Option<Arc<Mutex<RusshClient>>>,
    session_id: &str,
    needs_prepare: bool,
) -> Result<(Arc<Mutex<RusshClient>>, ServerStats)> {
    let client = match existing_client {
        Some(client) => client,
        None => Arc::new(Mutex::new(RusshClient::connect(config).await?)),
    };

    if needs_prepare {
        prepare_remote_monitor(client.clone()).await?;
    }

    let payload = collect_remote_stats(client.clone(), session_id).await?;
    let stats = parse_server_stats(&payload)?;
    Ok((client, stats))
}

async fn prepare_remote_monitor(client: Arc<Mutex<RusshClient>>) -> Result<()> {
    exec_capture(client, &build_prepare_command())
        .await
        .map(|_| ())
}

async fn collect_remote_stats(client: Arc<Mutex<RusshClient>>, session_id: &str) -> Result<String> {
    let output = exec_capture(client, &build_collect_command(session_id)).await?;
    if output.trim().is_empty() {
        Err(anyhow!("empty monitor payload"))
    } else {
        Ok(output)
    }
}

async fn exec_capture(client: Arc<Mutex<RusshClient>>, command: &str) -> Result<String> {
    let mut guard = client.lock().await;
    let mut channel = guard.open_channel().await?;
    channel.exec(command).await?;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit_status = 0u32;

    while let Some(event) = channel.recv().await {
        match event {
            ChannelEvent::Data(data) => stdout.extend(data),
            ChannelEvent::ExtendedData { data, .. } => stderr.extend(data),
            ChannelEvent::ExitStatus(status) => exit_status = status,
            ChannelEvent::ExitSignal {
                signal_name,
                error_message,
            } => {
                return Err(anyhow!(
                    "remote command failed with signal {signal_name}: {error_message}"
                ));
            }
            ChannelEvent::Eof | ChannelEvent::Close => break,
        }
    }

    let _ = channel.close().await;
    if exit_status != 0 {
        let stderr = String::from_utf8_lossy(&stderr);
        return Err(anyhow!(
            "remote command exited with status {exit_status}: {stderr}"
        ));
    }

    String::from_utf8(stdout).context("monitor payload is not valid utf-8")
}

fn build_prepare_command() -> String {
    format!(
        "mkdir -p {dir} && cat > {script} <<'__ONETCLI_MONITOR__'\n{body}\n__ONETCLI_MONITOR__\nchmod 700 {script}",
        dir = REMOTE_HELPER_DIR,
        script = REMOTE_HELPER_SCRIPT,
        body = REMOTE_MONITOR_SCRIPT
    )
}

fn build_collect_command(session_id: &str) -> String {
    format!(
        "{REMOTE_HELPER_SCRIPT} --session {}",
        shell_quote(session_id)
    )
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    let mut quoted = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            quoted.push_str("'\"'\"'");
        } else {
            quoted.push(ch);
        }
    }
    quoted.push('\'');
    quoted
}

fn format_uptime(seconds: f64) -> String {
    let seconds = seconds as u64;
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn format_kib(value: u64) -> String {
    format_bytes(value * 1024)
}

fn format_bytes(value: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let value = value as f64;
    if value >= GB {
        format!("{:.1}G", value / GB)
    } else if value >= MB {
        format!("{:.1}M", value / MB)
    } else if value >= KB {
        format!("{:.1}K", value / KB)
    } else {
        format!("{value:.0}B")
    }
}

fn format_bytes_per_sec(value: f64) -> String {
    format!("{}/s", format_bytes(value as u64))
}

#[cfg(test)]
mod tests {
    use super::{
        CpuSnapshot, HistoryLimit, MemoryStats, NetworkTotals, ProcessEntry, history_points,
        history_tick_margin, parse_server_stats, push_history_point, sample_cpu_usage,
        sample_network_rates, split_sections,
    };

    #[test]
    fn split_sections_parses_nested_monitor_document() {
        let sections = split_sections(
            r#"
meta:
  supported: true
os:
  prettyName: Ubuntu 24.04 LTS
memory:
  total: 2048
process:
  topsCostCpu:
    - pid: 10
      cpu: 2.5
      memory: 128
      command: sshd
"#,
        );

        assert_eq!(
            sections
                .get("os")
                .and_then(|section| section.get("prettyName")),
            Some(&"Ubuntu 24.04 LTS".to_string())
        );
        assert_eq!(
            sections
                .get("process")
                .and_then(|section| section.get("topsCostCpu[0].command")),
            Some(&"sshd".to_string())
        );
    }

    #[test]
    fn sample_cpu_usage_computes_total_and_per_core_percentages() {
        let previous = vec![
            CpuSnapshot::new("cpu", 100, 40, 860, 1000),
            CpuSnapshot::new("cpu0", 50, 20, 430, 500),
            CpuSnapshot::new("cpu1", 50, 20, 430, 500),
        ];
        let current = vec![
            CpuSnapshot::new("cpu", 130, 60, 910, 1100),
            CpuSnapshot::new("cpu0", 70, 30, 450, 550),
            CpuSnapshot::new("cpu1", 60, 30, 460, 550),
        ];

        let sampled = sample_cpu_usage(&previous, &current);

        assert_eq!(sampled.total_percent.round() as i32, 50);
        assert_eq!(sampled.cores.len(), 2);
        assert_eq!(sampled.cores[0].name, "cpu0");
        assert_eq!(sampled.cores[0].percent.round() as i32, 60);
        assert_eq!(sampled.cores[1].percent.round() as i32, 40);
    }

    #[test]
    fn parse_server_stats_keeps_total_cpu_snapshot_with_scientific_notation() {
        let previous = parse_server_stats(
            r#"
time:
  timestamp: 1
cpu:
  snapshot:
    - name: "cpu"
      loadUser: 90851036
      loadSystem: 231738113
      loadIdle: 5141899108
      loadTotal: 5.464488257e+09
    - name: "cpu0"
      loadUser: 12295114
      loadSystem: 31524889
      loadIdle: 411583038
      loadTotal: 455719704
"#,
        )
        .expect("previous payload should parse");
        let current = parse_server_stats(
            r#"
time:
  timestamp: 2
cpu:
  snapshot:
    - name: "cpu"
      loadUser: 90852036
      loadSystem: 231739113
      loadIdle: 5141899208
      loadTotal: 5.464490357e+09
    - name: "cpu0"
      loadUser: 12295214
      loadSystem: 31524989
      loadIdle: 411583138
      loadTotal: 455720004
"#,
        )
        .expect("current payload should parse");

        let sampled = sample_cpu_usage(&previous.cpu_snapshots, &current.cpu_snapshots);

        assert!(
            previous
                .cpu_snapshots
                .iter()
                .any(|snapshot| snapshot.name == "cpu")
        );
        assert!(
            current
                .cpu_snapshots
                .iter()
                .any(|snapshot| snapshot.name == "cpu")
        );
        assert!(sampled.total_percent > 0.0);
    }

    #[test]
    fn sample_network_rates_uses_interval_and_total_delta() {
        let previous = NetworkTotals::new(vec![("eth0", 1_000, 2_000), ("ens5", 2_000, 4_000)]);
        let current = NetworkTotals::new(vec![("eth0", 1_900, 2_300), ("ens5", 2_300, 5_200)]);

        let sampled = sample_network_rates(&previous, &current, 2.0);

        assert_eq!(sampled.rx_bytes_per_sec.round() as i32, 600);
        assert_eq!(sampled.tx_bytes_per_sec.round() as i32, 750);
        assert_eq!(sampled.rx_delta, 1_200);
        assert_eq!(sampled.tx_delta, 1_500);
    }

    #[test]
    fn push_history_point_keeps_latest_points_only() {
        let mut history = vec![10.0, 20.0, 30.0];

        push_history_point(&mut history, 40.0, HistoryLimit::new(3));

        assert_eq!(history, vec![20.0, 30.0, 40.0]);
    }

    #[test]
    fn history_points_keep_cpu_chart_domain_non_degenerate() {
        let points = history_points(&[0.0, 0.0, 0.0]);

        assert_eq!(points.len(), 3);
        assert!(points.iter().all(|point| point.value == 0.0));
        assert!(points.iter().all(|point| point.ceiling == 100.0));
    }

    #[test]
    fn history_tick_margin_scales_down_dense_axes() {
        assert_eq!(history_tick_margin(0), 1);
        assert_eq!(history_tick_margin(6), 1);
        assert_eq!(history_tick_margin(7), 2);
        assert_eq!(history_tick_margin(30), 5);
    }

    #[test]
    fn process_entry_and_memory_stats_support_partial_linux_payloads() {
        let process = ProcessEntry::new(1234, 1.5, 2048, "python worker");
        let memory = MemoryStats::new(1024, 128, 512, 384, 0, 0, 0);

        assert_eq!(process.command, "python worker");
        assert_eq!(memory.used_percent().round() as i32, 50);
    }
}
