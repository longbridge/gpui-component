//! 右键菜单功能处理模块
//!
//! 本模块实现 FileListPanel 右键菜单的所有功能

use crate::{join_remote_path, FileListPanelEvent, PanelSide, SftpView, SftpViewEvent};
use gpui::{AppContext, ClipboardItem, Context, ParentElement, PathPromptOptions, Styled, Window};
use gpui_component::{
    dialog::DialogButtonProps,
    input::{Input, InputState},
    notification::Notification,
    v_flex, WindowExt,
};
use one_core::gpui_tokio::Tokio;
use rust_i18n::t;
use sftp::SftpClient;
use std::path::PathBuf;

fn is_valid_entry_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
}

/// 右键菜单处理 trait
/// 为 SftpView 实现右键菜单的各种功能
pub trait ContextMenuHandler {
    /// 处理本地面板的右键菜单事件
    fn handle_local_context_menu_event(
        &mut self,
        event: &FileListPanelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        Self: Sized;

    /// 处理远程面板的右键菜单事件
    fn handle_remote_context_menu_event(
        &mut self,
        event: &FileListPanelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        Self: Sized;

    /// 新建文件
    fn create_new_file(&mut self, side: PanelSide, window: &mut Window, cx: &mut Context<Self>)
    where
        Self: Sized;

    /// 重命名文件/文件夹
    fn rename_item(
        &mut self,
        name: &str,
        full_path: &str,
        side: PanelSide,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        Self: Sized;

    /// 复制文件名到剪贴板
    fn copy_file_name(&self, name: &str, window: &mut Window, cx: &mut Context<Self>)
    where
        Self: Sized;

    /// 复制绝对路径到剪贴板
    fn copy_absolute_path(&self, path: &str, window: &mut Window, cx: &mut Context<Self>)
    where
        Self: Sized;

    /// 修改权限
    fn change_permissions(
        &mut self,
        name: &str,
        full_path: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        Self: Sized;

    /// 在终端中打开（当前目录）
    fn open_in_terminal(&self, side: PanelSide, window: &mut Window, cx: &mut Context<Self>)
    where
        Self: Sized;

    /// 在终端中打开（指定路径）
    fn open_in_terminal_at(
        &self,
        path: &str,
        side: PanelSide,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        Self: Sized;

    /// 切换隐藏文件显示
    fn toggle_hidden_files(&mut self, side: PanelSide, cx: &mut Context<Self>)
    where
        Self: Sized;

    /// 选择本地文件并上传到远程
    fn select_and_upload_files(&mut self, window: &mut Window, cx: &mut Context<Self>)
    where
        Self: Sized;

    /// 选择本地文件夹并上传到远程
    fn select_and_upload_folder(&mut self, window: &mut Window, cx: &mut Context<Self>)
    where
        Self: Sized;
}

impl ContextMenuHandler for SftpView {
    fn handle_local_context_menu_event(
        &mut self,
        event: &FileListPanelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            FileListPanelEvent::NewFile => {
                self.create_new_file(PanelSide::Local, window, cx);
            }
            FileListPanelEvent::NewFolder => {
                self.show_new_folder_dialog(PanelSide::Local, window, cx);
            }
            FileListPanelEvent::Rename { name, full_path } => {
                self.rename_item(name, full_path, PanelSide::Local, window, cx);
            }
            FileListPanelEvent::Download {
                name: _,
                full_path: _,
            } => {
                // 本地文件不支持下载，无操作
            }
            FileListPanelEvent::ChangePermissions { name, full_path } => {
                self.change_permissions(name, full_path, window, cx);
            }
            FileListPanelEvent::OpenInTerminal => {
                self.open_in_terminal(PanelSide::Local, window, cx);
            }
            FileListPanelEvent::OpenInTerminalAt { full_path } => {
                self.open_in_terminal_at(full_path, PanelSide::Local, window, cx);
            }
            FileListPanelEvent::CopyFileName { name } => {
                self.copy_file_name(name, window, cx);
            }
            FileListPanelEvent::CopyAbsolutePath { full_path } => {
                self.copy_absolute_path(full_path, window, cx);
            }
            FileListPanelEvent::Delete {
                name: _,
                full_path: _,
            } => {
                self.delete_local_selected(window, cx);
            }
            FileListPanelEvent::UploadFile => {
                self.upload_selected(window, cx);
            }
            FileListPanelEvent::UploadFolder => {
                self.upload_selected(window, cx);
            }
            FileListPanelEvent::Refresh => {
                self.refresh_local_dir_with_window(window, cx);
            }
            FileListPanelEvent::ToggleHiddenFiles => {
                self.toggle_hidden_files(PanelSide::Local, cx);
            }
            _ => {}
        }
    }

    fn handle_remote_context_menu_event(
        &mut self,
        event: &FileListPanelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            FileListPanelEvent::NewFile => {
                self.create_new_file(PanelSide::Remote, window, cx);
            }
            FileListPanelEvent::NewFolder => {
                self.show_new_folder_dialog(PanelSide::Remote, window, cx);
            }
            FileListPanelEvent::Rename { name, full_path } => {
                self.rename_item(name, full_path, PanelSide::Remote, window, cx);
            }
            FileListPanelEvent::Download {
                name: _,
                full_path: _,
            } => {
                self.download_selected(window, cx);
            }
            FileListPanelEvent::ChangePermissions { name, full_path } => {
                self.change_permissions(name, full_path, window, cx);
            }
            FileListPanelEvent::OpenInTerminal => {
                self.open_in_terminal(PanelSide::Remote, window, cx);
            }
            FileListPanelEvent::OpenInTerminalAt { full_path } => {
                self.open_in_terminal_at(full_path, PanelSide::Remote, window, cx);
            }
            FileListPanelEvent::CopyFileName { name } => {
                self.copy_file_name(name, window, cx);
            }
            FileListPanelEvent::CopyAbsolutePath { full_path } => {
                self.copy_absolute_path(full_path, window, cx);
            }
            FileListPanelEvent::Delete {
                name: _,
                full_path: _,
            } => {
                self.delete_remote_selected(window, cx);
            }
            FileListPanelEvent::UploadFile => {
                self.select_and_upload_files(window, cx);
            }
            FileListPanelEvent::UploadFolder => {
                self.select_and_upload_folder(window, cx);
            }
            FileListPanelEvent::Refresh => {
                self.refresh_remote_dir_with_window(window, cx);
            }
            FileListPanelEvent::ToggleHiddenFiles => {
                self.toggle_hidden_files(PanelSide::Remote, cx);
            }
            _ => {}
        }
    }

    fn create_new_file(&mut self, side: PanelSide, window: &mut Window, cx: &mut Context<Self>) {
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("Placeholder.filename")));
        let view = cx.entity().downgrade();

        // 在打开对话框前设置焦点，避免闪烁
        input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let side = side;
            let view_clone = view.clone();
            let input_for_callback = input.clone();

            dialog
                .title(t!("File.new_file").to_string())
                .w(gpui::px(360.))
                .child(Input::new(&input))
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.create").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let file_name = input_for_callback.read(cx).text().to_string();
                    if file_name.is_empty() {
                        return false;
                    }
                    if !is_valid_entry_name(&file_name) {
                        window.push_notification(Notification::error(t!("Error.invalid_name")), cx);
                        return false;
                    }

                    let _ = view_clone.update(cx, |this, cx| {
                        match side {
                            PanelSide::Local => {
                                let path = this.local_current_path.join(&file_name);
                                if let Err(e) = std::fs::File::create(&path) {
                                    tracing::error!(
                                        "Failed to create file {}: {}",
                                        path.display(),
                                        e
                                    );
                                    window.push_notification(
                                        Notification::error(t!(
                                            "Error.create_file_failed",
                                            error = e
                                        )),
                                        cx,
                                    );
                                } else {
                                    window.close_dialog(cx);
                                }
                                this.refresh_local_dir(cx);
                            }
                            PanelSide::Remote => {
                                let Some(client) = this.sftp_client.clone() else {
                                    return;
                                };

                                let remote_path =
                                    join_remote_path(&this.remote_current_path, &file_name);

                                let task = Tokio::spawn(cx, async move {
                                    let mut client = client.lock().await;
                                    // 创建空文件
                                    client.write_file(&remote_path, &[]).await
                                });

                                let view = cx.entity().clone();
                                window
                                    .spawn(cx, async move |cx| match task.await {
                                        Ok(Ok(_)) => {
                                            let _ = view.update_in(cx, |this, window, cx| {
                                                window.close_dialog(cx);
                                                this.refresh_remote_dir(cx);
                                            });
                                        }
                                        Ok(Err(e)) => {
                                            tracing::error!("Failed to create remote file: {}", e);
                                            let _ = view.update_in(cx, |_this, window, cx| {
                                                window.push_notification(
                                                    Notification::error(t!(
                                                        "Error.create_file_failed",
                                                        error = e
                                                    )),
                                                    cx,
                                                );
                                            });
                                        }
                                        Err(e) => {
                                            tracing::error!("Task error: {}", e);
                                            let _ = view.update_in(cx, |_this, window, cx| {
                                                window.push_notification(
                                                    Notification::error(t!(
                                                        "Error.create_file_failed",
                                                        error = e
                                                    )),
                                                    cx,
                                                );
                                            });
                                        }
                                    })
                                    .detach();
                            }
                        }
                    });
                    false
                })
        });
    }

    fn rename_item(
        &mut self,
        name: &str,
        full_path: &str,
        side: PanelSide,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("Placeholder.new_name")));
        let view = cx.entity().downgrade();
        let old_name = name.to_string();
        let old_path = full_path.to_string();

        // 设置初始值为当前文件名
        input.update(cx, |state, cx| {
            state.set_value(&old_name, window, cx);
        });

        // 在打开对话框前设置焦点，避免闪烁
        input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let side = side;
            let view_clone = view.clone();
            let input_for_callback = input.clone();
            let old_path_for_callback = old_path.clone();

            dialog
                .title(t!("Common.rename").to_string())
                .w(gpui::px(360.))
                .child(Input::new(&input))
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.rename").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let new_name = input_for_callback.read(cx).text().to_string();
                    if new_name.is_empty() {
                        return false;
                    }
                    if !is_valid_entry_name(&new_name) {
                        window.push_notification(Notification::error(t!("Error.invalid_name")), cx);
                        return false;
                    }

                    let _ = view_clone.update(cx, |this, cx| match side {
                        PanelSide::Local => {
                            let old_full_path = PathBuf::from(&old_path_for_callback);
                            let new_full_path = old_full_path
                                .parent()
                                .unwrap_or(&old_full_path)
                                .join(&new_name);

                            if let Err(e) = std::fs::rename(&old_full_path, &new_full_path) {
                                tracing::error!(
                                    "Failed to rename {} to {}: {}",
                                    old_full_path.display(),
                                    new_full_path.display(),
                                    e
                                );
                                window.push_notification(
                                    Notification::error(t!("Error.rename_failed", error = e)),
                                    cx,
                                );
                            } else {
                                window.close_dialog(cx);
                            }
                            this.refresh_local_dir(cx);
                        }
                        PanelSide::Remote => {
                            let Some(client) = this.sftp_client.clone() else {
                                return;
                            };

                            let old_remote_path = old_path_for_callback.clone();
                            let new_remote_path = if let Some(pos) = old_remote_path.rfind('/') {
                                format!("{}/{}", &old_remote_path[..pos], new_name)
                            } else {
                                new_name.clone()
                            };

                            let task = Tokio::spawn(cx, async move {
                                let mut client = client.lock().await;
                                client.rename(&old_remote_path, &new_remote_path).await
                            });

                            let view = cx.entity().clone();
                            window
                                .spawn(cx, async move |cx| match task.await {
                                    Ok(Ok(_)) => {
                                        let _ = view.update_in(cx, |this, window, cx| {
                                            window.close_dialog(cx);
                                            this.refresh_remote_dir(cx);
                                        });
                                    }
                                    Ok(Err(e)) => {
                                        tracing::error!("Failed to rename remote file: {}", e);
                                        let _ = view.update_in(cx, |_this, window, cx| {
                                            window.push_notification(
                                                Notification::error(t!(
                                                    "Error.rename_failed",
                                                    error = e
                                                )),
                                                cx,
                                            );
                                        });
                                    }
                                    Err(e) => {
                                        tracing::error!("Task error: {}", e);
                                        let _ = view.update_in(cx, |_this, window, cx| {
                                            window.push_notification(
                                                Notification::error(t!(
                                                    "Error.rename_failed",
                                                    error = e
                                                )),
                                                cx,
                                            );
                                        });
                                    }
                                })
                                .detach();
                        }
                    });
                    false
                })
        });
    }

    fn copy_file_name(&self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(name.to_string()));
        window.push_notification(
            Notification::success(t!("Notification.copied_filename")),
            cx,
        );
    }

    fn copy_absolute_path(&self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(path.to_string()));
        window.push_notification(Notification::success(t!("Notification.copied_path")), cx);
    }

    fn change_permissions(
        &mut self,
        name: &str,
        full_path: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("Placeholder.permission")));
        let view = cx.entity().downgrade();
        let file_name = name.to_string();
        let file_path = full_path.to_string();

        // 在打开对话框前设置焦点，避免闪烁
        input.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let view_clone = view.clone();
            let input_for_callback = input.clone();
            let path_for_callback = file_path.clone();

            dialog
                .title(t!("Dialog.change_permission_title", name = file_name).to_string())
                .w(gpui::px(360.))
                .child(
                    v_flex()
                        .gap_2()
                        .child(t!("Notification.permission_hint").to_string())
                        .child(Input::new(&input)),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.modify").to_string())
                        .cancel_text(t!("Common.cancel").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    let perm_str = input_for_callback.read(cx).text().to_string();
                    if perm_str.is_empty() {
                        return false;
                    }

                    // 解析八进制权限值
                    let mode = match u32::from_str_radix(&perm_str, 8) {
                        Ok(m) if m <= 0o777 => m,
                        _ => {
                            window.push_notification(
                                Notification::error(t!("Notification.invalid_permission")),
                                cx,
                            );
                            return false;
                        }
                    };

                    let _ = view_clone.update(cx, |this, cx| {
                        let Some(client) = this.sftp_client.clone() else {
                            return;
                        };

                        let remote_path = path_for_callback.clone();
                        let task = Tokio::spawn(cx, async move {
                            let mut client = client.lock().await;
                            client.chmod(&remote_path, mode).await
                        });

                        let view = cx.entity().clone();
                        window
                            .spawn(cx, async move |cx| match task.await {
                                Ok(Ok(_)) => {
                                    let _ = view.update_in(cx, |this, window, cx| {
                                        window.close_dialog(cx);
                                        window.push_notification(
                                            Notification::success(t!(
                                                "Notification.permission_success"
                                            )),
                                            cx,
                                        );
                                        this.refresh_remote_dir(cx);
                                    });
                                }
                                Ok(Err(e)) => {
                                    tracing::error!("Failed to change permissions: {}", e);
                                    let _ = view.update_in(cx, |_this, window, cx| {
                                        window.push_notification(
                                            Notification::error(t!(
                                                "Error.permission_failed",
                                                error = e
                                            )),
                                            cx,
                                        );
                                    });
                                }
                                Err(e) => {
                                    tracing::error!("Task error: {}", e);
                                }
                            })
                            .detach();
                    });
                    false
                })
        });
    }

    fn open_in_terminal(&self, side: PanelSide, _window: &mut Window, cx: &mut Context<Self>) {
        match side {
            PanelSide::Local => {
                let path = self.local_current_path.to_string_lossy().to_string();
                cx.emit(SftpViewEvent::OpenLocalTerminal { working_dir: path });
            }
            PanelSide::Remote => {
                // 打开 SSH 终端连接到远程服务器
                cx.emit(SftpViewEvent::OpenSshTerminal {
                    connection: self.stored_connection.clone(),
                    working_dir: self.remote_current_path.to_string(),
                });
            }
        }
    }

    fn open_in_terminal_at(
        &self,
        path: &str,
        side: PanelSide,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match side {
            PanelSide::Local => {
                // 如果是文件，获取其所在目录
                let target_path = PathBuf::from(path);
                let target_path = if target_path.is_absolute() {
                    target_path
                } else {
                    self.local_current_path.join(path)
                };
                let dir_path = if target_path.is_file() {
                    target_path.parent().unwrap_or(&target_path).to_path_buf()
                } else {
                    target_path
                };
                let path_str = dir_path.to_string_lossy().to_string();
                cx.emit(SftpViewEvent::OpenLocalTerminal {
                    working_dir: path_str,
                });
            }
            PanelSide::Remote => {
                // 打开 SSH 终端连接到远程服务器
                let base_path = self.remote_current_path.as_str();
                let is_rooted = path.starts_with('/')
                    || path.starts_with("~")
                    || path.starts_with("./")
                    || path.starts_with("../");
                let has_base_prefix = !base_path.is_empty()
                    && (path == base_path || path.starts_with(&format!("{}/", base_path)));
                let working_dir = if is_rooted || has_base_prefix {
                    path.to_string()
                } else {
                    join_remote_path(base_path, path)
                };
                cx.emit(SftpViewEvent::OpenSshTerminal {
                    connection: self.stored_connection.clone(),
                    working_dir,
                });
            }
        }
    }

    fn toggle_hidden_files(&mut self, side: PanelSide, cx: &mut Context<Self>) {
        match side {
            PanelSide::Local => {
                self.local_panel.update(cx, |panel, cx| {
                    panel.toggle_show_hidden(cx);
                });
            }
            PanelSide::Remote => {
                self.remote_panel.update(cx, |panel, cx| {
                    panel.toggle_show_hidden(cx);
                });
            }
        }
    }

    fn select_and_upload_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(client) = self.sftp_client.clone() else {
            return;
        };

        let remote_path = self.remote_current_path.clone();
        let view = cx.entity().clone();

        // 打开文件选择对话框
        let future = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            multiple: true,
            directories: false,
            prompt: Some(t!("FilePicker.select_upload_files").to_string().into()),
        });

        window
            .spawn(cx, async move |cx| {
                if let Ok(Ok(Some(paths))) = future.await {
                    if paths.is_empty() {
                        return;
                    }

                    // 上传选中的文件
                    let _ = view.update_in(cx, |this, window, cx| {
                        this.upload_paths_to_remote(paths, remote_path, client, window, cx);
                    });
                }
            })
            .detach();
    }

    fn select_and_upload_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(client) = self.sftp_client.clone() else {
            return;
        };

        let remote_path = self.remote_current_path.clone();
        let view = cx.entity().clone();

        // 打开文件夹选择对话框
        let future = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            multiple: true,
            directories: true,
            prompt: Some(t!("FilePicker.select_upload_folder").to_string().into()),
        });

        window
            .spawn(cx, async move |cx| {
                if let Ok(Ok(Some(paths))) = future.await {
                    if paths.is_empty() {
                        return;
                    }

                    // 上传选中的文件夹
                    let _ = view.update_in(cx, |this, window, cx| {
                        this.upload_paths_to_remote(paths, remote_path, client, window, cx);
                    });
                }
            })
            .detach();
    }
}
