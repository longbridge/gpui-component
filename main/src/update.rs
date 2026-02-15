use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use futures::AsyncReadExt;
use gpui::http_client::{http, AsyncBody, HttpClient, Method, Request};
use gpui::prelude::FluentBuilder;
use gpui::{App, AppContext, Context, IntoElement, ParentElement, Render, Styled, Window, div, px};
use gpui_component::{
    ActiveTheme, WindowExt,
    dialog::DialogButtonProps,
    progress::Progress,
    v_flex,
};
use rust_i18n::t;
use semver::Version;
use serde::Deserialize;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::setting_tab::AppSettings;
use one_core::config::UpdateConfig;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const APPLY_UPDATE_FLAG: &str = "--apply-update";

#[derive(Clone, Debug)]
struct UpdateDialogInfo {
    current_version: String,
    latest_version: String,
    download_url: Option<String>,
    release_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateDownloads {
    #[serde(default)]
    #[allow(dead_code)]
    windows: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    macos: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    linux: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateResponse {
    version: String,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    downloads: Option<UpdateDownloads>,
    #[serde(default)]
    release_notes: Option<String>,
}

pub fn handle_update_command() -> bool {
    let mut args = std::env::args().skip(1);
    let Some(flag) = args.next() else {
        return false;
    };

    if flag != APPLY_UPDATE_FLAG {
        return false;
    }

    let Some(download_path) = args.next().map(PathBuf::from) else {
        eprintln!("缺少更新包路径");
        return true;
    };

    let target_path = args
        .next()
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
        .unwrap_or_else(|| download_path.clone());

    if let Err(err) = apply_update_helper(&download_path, &target_path) {
        eprintln!("更新失败: {}", err);
    }

    true
}

pub fn schedule_update_check(window: &mut Window, cx: &mut App) {
    if !AppSettings::global(cx).auto_update {
        return;
    }

    let config = UpdateConfig::get();
    if !config.is_valid() {
        tracing::info!("更新检查未启用：缺少 ONEHUB_UPDATE_URL");
        return;
    }

    let http_client = cx.http_client();
    let update_url = config.update_url.clone();
    let default_download_url = config.download_url.clone();
    let current_version = CURRENT_VERSION.to_string();

    window
        .spawn(cx, async move |cx| {
            let response = match fetch_update_info(http_client, &update_url).await {
                Ok(response) => response,
                Err(err) => {
                    tracing::warn!("更新检查失败: {}", err);
                    return;
                }
            };

            let latest_version = match parse_version(&response.version) {
                Some(version) => version,
                None => {
                    tracing::warn!("更新检查失败: 版本号无法解析 {}", response.version);
                    return;
                }
            };

            let current_semver = match parse_version(&current_version) {
                Some(version) => version,
                None => {
                    tracing::warn!("更新检查失败: 当前版本号无法解析 {}", current_version);
                    return;
                }
            };

            if latest_version <= current_semver {
                return;
            }

            let download_url = select_download_url(&response, default_download_url.clone());

            let info = UpdateDialogInfo {
                current_version: current_version.clone(),
                latest_version: response.version.clone(),
                download_url,
                release_notes: response.release_notes.clone(),
            };

            let _ = cx.update(|_view, cx: &mut App| {
                if let Some(window_id) = cx.active_window() {
                    let _ = cx.update_window(window_id, |_, window, cx| {
                        show_update_dialog(window, info.clone(), cx);
                    });
                }
            });
        })
        .detach();
}

async fn fetch_update_info(
    http_client: Arc<dyn HttpClient>,
    update_url: &str,
) -> Result<UpdateResponse, String> {
    let request = Request::builder()
        .method(Method::GET)
        .uri(update_url)
        .header("Accept", "application/json")
        .body(AsyncBody::empty())
        .map_err(|e| format!("构建更新请求失败: {}", e))?;

    let response = http_client
        .send(request)
        .await
        .map_err(|e| format!("发送更新请求失败: {}", e))?;

    let status = response.status();
    let mut body = response.into_body();
    let mut bytes = Vec::new();
    body.read_to_end(&mut bytes)
        .await
        .map_err(|e| format!("读取更新响应失败: {}", e))?;

    if !status.is_success() {
        return Err(format!("更新接口返回异常状态码: {}", status));
    }

    serde_json::from_slice::<UpdateResponse>(&bytes)
        .map_err(|e| format!("解析更新响应失败: {}", e))
}

fn select_download_url(
    response: &UpdateResponse,
    default_download_url: Option<String>,
) -> Option<String> {
    let platform_url = response.downloads.as_ref().and_then(|downloads| {
        #[cfg(target_os = "windows")]
        {
            return downloads.windows.clone();
        }
        #[cfg(target_os = "macos")]
        {
            return downloads.macos.clone();
        }
        #[cfg(target_os = "linux")]
        {
            return downloads.linux.clone();
        }
        #[allow(unreachable_code)]
        None
    });

    platform_url
        .or_else(|| response.download_url.clone())
        .or(default_download_url)
}

fn parse_version(value: &str) -> Option<Version> {
    let trimmed = value.trim();
    let trimmed = trimmed.strip_prefix('v').unwrap_or(trimmed);
    Version::parse(trimmed).ok()
}

fn show_update_dialog(window: &mut Window, info: UpdateDialogInfo, cx: &mut App) {
    let view = cx.new(|_cx| UpdateDialogView::new(info));
    let view_for_ok = view.clone();
    let view_for_cancel = view.clone();

    window.open_dialog(cx, move |dialog, _window, _cx| {
        let view_for_ok = view_for_ok.clone();
        let view_for_cancel = view_for_cancel.clone();
        dialog
            .title(t!("Update.title").to_string())
            .width(px(460.))
            .child(view.clone())
            .confirm()
            .button_props(
                DialogButtonProps::default()
                    .ok_text(t!("Update.action"))
                    .cancel_text(t!("Update.later")),
            )
            .on_ok(move |_, window, cx| {
                view_for_ok.clone().update(cx, |view, cx| {
                    view.start_download(window, cx);
                });
                false
            })
            .on_cancel(move |_, window, cx| {
                if view_for_cancel.clone().read(cx).downloading {
                    window.push_notification(t!("Update.downloading_blocked").to_string(), cx);
                    return false;
                }
                true
            })
    });
}

struct UpdateDialogView {
    info: UpdateDialogInfo,
    downloading: bool,
    completed: bool,
    progress: f32,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    status_message: String,
    error_message: Option<String>,
}

impl UpdateDialogView {
    fn new(info: UpdateDialogInfo) -> Self {
        Self {
            info,
            downloading: false,
            completed: false,
            progress: 0.0,
            downloaded_bytes: 0,
            total_bytes: None,
            status_message: t!("Update.ready").to_string(),
            error_message: None,
        }
    }

    fn start_download(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.downloading || self.completed {
            return;
        }

        let Some(download_url) = self.info.download_url.clone() else {
            self.error_message = Some(t!("Update.missing_download_url").to_string());
            self.status_message = t!("Update.download_failed").to_string();
            cx.notify();
            return;
        };

        let download_path = match build_download_path(&self.info.latest_version, &download_url) {
            Ok(path) => path,
            Err(err) => {
                self.error_message = Some(err);
                self.status_message = t!("Update.download_failed").to_string();
                cx.notify();
                return;
            }
        };

        self.downloading = true;
        self.completed = false;
        self.progress = 0.0;
        self.downloaded_bytes = 0;
        self.total_bytes = None;
        self.error_message = None;
        self.status_message = t!("Update.downloading").to_string();
        cx.notify();

        let http_client = cx.http_client();

        cx.spawn(async move |this, cx| {
            let download_result = download_update_file(
                http_client,
                &download_url,
                &download_path,
                |downloaded, total| {
                    let _ = this.update(cx, |view, cx| {
                        view.update_progress(downloaded, total, cx);
                    });
                },
            )
            .await;

            match download_result {
                Ok(()) => {
                    let _ = this.update(cx, |view, cx| {
                        view.completed = true;
                        view.downloading = false;
                        view.progress = 100.0;
                        view.status_message = t!("Update.download_complete").to_string();
                        cx.notify();
                    });

                    match start_install_update(download_path.clone()) {
                        Ok(UpdateInstallAction::Quit) => {
                            let _ = cx.update(|cx| {
                                cx.quit();
                            });
                        }
                        Ok(UpdateInstallAction::Noop) => {}
                        Err(err) => {
                            let _ = this.update(cx, |view, cx| {
                                view.error_message = Some(err);
                                view.status_message = t!("Update.apply_failed").to_string();
                                view.completed = false;
                                cx.notify();
                            });
                        }
                    }
                }
                Err(err) => {
                    let _ = this.update(cx, |view, cx| {
                        view.downloading = false;
                        view.error_message = Some(err);
                        view.status_message = t!("Update.download_failed").to_string();
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn update_progress(&mut self, downloaded: u64, total: Option<u64>, cx: &mut Context<Self>) {
        self.downloaded_bytes = downloaded;
        self.total_bytes = total;
        if let Some(total) = total {
            if total > 0 {
                self.progress = ((downloaded as f32 / total as f32) * 100.0).min(100.0);
            }
        }
        cx.notify();
    }

    fn progress_value(&self) -> f32 {
        if self.total_bytes.is_some() {
            self.progress
        } else {
            -1.0
        }
    }

    fn progress_label(&self) -> String {
        match self.total_bytes {
            Some(total) if total > 0 => format!(
                "{} / {}",
                format_bytes(self.downloaded_bytes),
                format_bytes(total)
            ),
            _ => format_bytes(self.downloaded_bytes),
        }
    }
}

impl Render for UpdateDialogView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let message = t!(
            "Update.message",
            latest = self.info.latest_version,
            current = self.info.current_version
        )
        .to_string();

        let release_notes = self
            .info
            .release_notes
            .clone()
            .filter(|notes| !notes.trim().is_empty());

        let show_progress = self.downloading || self.completed;
        let status_message = if let Some(error) = self.error_message.as_ref() {
            format!("{}: {}", t!("Update.error_prefix"), error)
        } else {
            self.status_message.clone()
        };

        v_flex()
            .gap_3()
            .p_4()
            .child(
                div()
                    .text_base()
                    .text_color(cx.theme().foreground)
                    .child(message),
            )
            .when(show_progress, |this| {
                this.child(
                    v_flex()
                        .gap_2()
                        .child(Progress::new("update-progress").value(self.progress_value()))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(self.progress_label()),
                        ),
                )
            })
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(status_message),
            )
            .when(release_notes.is_some(), |this| {
                let notes = release_notes.clone().unwrap_or_default();
                this.child(
                    v_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(t!("Update.release_notes").to_string()),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(notes),
                        ),
                )
            })
    }
}

async fn download_update_file<F>(
    http_client: Arc<dyn HttpClient>,
    download_url: &str,
    download_path: &Path,
    mut on_progress: F,
) -> Result<(), String>
where
    F: FnMut(u64, Option<u64>),
{
    if let Some(parent) = download_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("创建下载目录失败: {}", e))?;
    }

    let request = Request::builder()
        .method(Method::GET)
        .uri(download_url)
        .header("Accept", "application/octet-stream")
        .body(AsyncBody::empty())
        .map_err(|e| format!("构建下载请求失败: {}", e))?;

    let response = http_client
        .send(request)
        .await
        .map_err(|e| format!("发送下载请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("更新包下载失败: {}", response.status()));
    }

    let total_bytes = response
        .headers()
        .get(http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());

    let mut body = response.into_body();
    let mut file = fs::File::create(download_path)
        .await
        .map_err(|e| format!("创建更新文件失败: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut buffer = vec![0u8; 8192];

    loop {
        let read = body
            .read(&mut buffer)
            .await
            .map_err(|e| format!("读取更新数据失败: {}", e))?;
        if read == 0 {
            break;
        }

        file.write_all(&buffer[..read])
            .await
            .map_err(|e| format!("写入更新文件失败: {}", e))?;

        downloaded += read as u64;
        on_progress(downloaded, total_bytes);
    }

    file.flush()
        .await
        .map_err(|e| format!("刷新更新文件失败: {}", e))?;
    file.sync_all()
        .await
        .map_err(|e| format!("同步更新文件失败: {}", e))?;

    set_executable_permission(download_path)?;

    Ok(())
}

fn build_download_path(version: &str, download_url: &str) -> Result<PathBuf, String> {
    let file_name = download_file_name(version, download_url);
    let dir = std::env::temp_dir().join("onehub-update");
    Ok(dir.join(file_name))
}

fn download_file_name(version: &str, download_url: &str) -> String {
    let parsed = http::Uri::try_from(download_url).ok();
    let extension = parsed
        .and_then(|uri| uri.path().rsplit('/').next().map(|p| p.to_string()))
        .and_then(|name| Path::new(&name).extension().map(|ext| ext.to_string_lossy().to_string()))
        .unwrap_or_else(|| {
            #[cfg(target_os = "windows")]
            {
                return "exe".to_string();
            }
            #[allow(unreachable_code)]
            String::new()
        });

    let base_name = format!("onehub-update-{}", version.replace('/', "-"));
    if extension.is_empty() {
        base_name
    } else {
        format!("{}.{}", base_name, extension)
    }
}

fn start_install_update(download_path: PathBuf) -> Result<UpdateInstallAction, String> {
    #[cfg(target_os = "windows")]
    {
        spawn_windows_helper(&download_path)?;
        return Ok(UpdateInstallAction::Quit);
    }

    #[cfg(target_os = "macos")]
    {
        apply_update_unix(&download_path)?;
        return Ok(UpdateInstallAction::Quit);
    }

    #[cfg(target_os = "linux")]
    {
        apply_update_unix(&download_path)?;
        return Ok(UpdateInstallAction::Quit);
    }

    #[allow(unreachable_code)]
    Ok(UpdateInstallAction::Noop)
}

fn apply_update_helper(download_path: &Path, target_path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        return apply_update_windows(download_path, target_path);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        return apply_update_unix_with_target(download_path, target_path);
    }

    #[allow(unreachable_code)]
    Ok(())
}

#[cfg(target_os = "windows")]
fn spawn_windows_helper(download_path: &Path) -> Result<(), String> {
    let target_path = std::env::current_exe().map_err(|e| format!("获取当前路径失败: {}", e))?;

    Command::new(download_path)
        .arg(APPLY_UPDATE_FLAG)
        .arg(download_path)
        .arg(&target_path)
        .spawn()
        .map_err(|e| format!("启动更新进程失败: {}", e))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_update_windows(download_path: &Path, target_path: &Path) -> Result<(), String> {
    let backup_path = target_path.with_extension("old");
    let mut last_error = None;

    for _ in 0..120 {
        match try_replace_windows(download_path, target_path, &backup_path) {
            Ok(()) => {
                restart_application(target_path)?;
                return Ok(());
            }
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                last_error = Some(err);
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
            Err(err) => return Err(format!("替换更新文件失败: {}", err)),
        }
    }

    Err(format!(
        "更新失败: {}",
        last_error
            .map(|err| err.to_string())
            .unwrap_or_else(|| "未知原因".to_string())
    ))
}

#[cfg(target_os = "windows")]
fn try_replace_windows(
    download_path: &Path,
    target_path: &Path,
    backup_path: &Path,
) -> std::io::Result<()> {
    if backup_path.exists() {
        let _ = std::fs::remove_file(backup_path);
    }

    if target_path.exists() {
        std::fs::rename(target_path, backup_path)?;
    }

    std::fs::copy(download_path, target_path)?;
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn apply_update_unix(download_path: &Path) -> Result<(), String> {
    let target_path = std::env::current_exe().map_err(|e| format!("获取当前路径失败: {}", e))?;
    apply_update_unix_with_target(download_path, &target_path)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn apply_update_unix_with_target(download_path: &Path, target_path: &Path) -> Result<(), String> {
    if target_path.exists() {
        std::fs::remove_file(target_path)
            .map_err(|e| format!("移除旧版本失败: {}", e))?;
    }

    match std::fs::rename(download_path, target_path) {
        Ok(()) => {}
        Err(err) if is_cross_device_link_error(&err) => {
            std::fs::copy(download_path, target_path)
                .map_err(|e| format!("复制更新文件失败: {}", e))?;
        }
        Err(err) => return Err(format!("替换更新文件失败: {}", err)),
    }

    set_executable_permission(target_path)?;
    restart_application(target_path)?;
    Ok(())
}

fn restart_application(target_path: &Path) -> Result<(), String> {
    Command::new(target_path)
        .spawn()
        .map_err(|e| format!("重启应用失败: {}", e))?;
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn is_cross_device_link_error(err: &std::io::Error) -> bool {
    err.raw_os_error() == Some(18)
}

fn set_executable_permission(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)
            .map_err(|e| format!("读取文件权限失败: {}", e))?
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)
            .map_err(|e| format!("设置可执行权限失败: {}", e))?;
    }

    Ok(())
}

fn format_bytes(value: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let value = value as f64;
    if value >= GB {
        format!("{:.2} GB", value / GB)
    } else if value >= MB {
        format!("{:.2} MB", value / MB)
    } else if value >= KB {
        format!("{:.1} KB", value / KB)
    } else {
        format!("{} B", value as u64)
    }
}

enum UpdateInstallAction {
    Quit,
    Noop,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        let parsed = parse_version("v1.2.3").expect("应解析版本号");
        assert_eq!(parsed, Version::new(1, 2, 3));
    }

    #[test]
    fn test_version_compare() {
        let newer = parse_version("1.2.0").unwrap();
        let older = parse_version("1.1.9").unwrap();
        assert!(newer > older);
        assert!(!is_newer("1.1.0", "1.1.0"));
        assert!(is_newer("1.1.1", "1.1.0"));
        assert!(!is_newer("1.0.9", "1.1.0"));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
    }

    fn is_newer(latest: &str, current: &str) -> bool {
        let latest = parse_version(latest).expect("最新版本解析失败");
        let current = parse_version(current).expect("当前版本解析失败");
        latest > current
    }
}
