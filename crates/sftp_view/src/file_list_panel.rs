use gpui::{
    App, Context, Entity, FocusHandle, Focusable, IntoElement, ListSizingBehavior, MouseButton,
    MouseDownEvent, ParentElement, Render, SharedString, Styled, UniformListScrollHandle, Window,
    div, prelude::*, px, uniform_list,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, InteractiveElementExt, Sizable, Size, h_flex,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt, PopupMenu, PopupMenuItem},
    tooltip::Tooltip,
    v_flex,
};
use rust_i18n::t;
use std::collections::HashSet;
use std::ops::Range;
use std::time::SystemTime;

#[derive(Clone, Debug)]
pub struct FileItem {
    pub name: String,
    pub size: u64,
    pub modified: SystemTime,
    pub is_dir: bool,
    pub permissions: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortColumn {
    Name,
    Modified,
    Size,
    Kind,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortOrder {
    Ascending,
    Descending,
}

fn format_file_size(size: u64) -> String {
    if size == 0 {
        return "- -".to_string();
    }
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} kB", size as f64 / KB as f64)
    } else {
        format!("{} Bytes", size)
    }
}

fn format_modified_time(time: SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Local> = time.into();
    datetime.format("%m/%d/%Y, %I:%M %p").to_string()
}

fn get_file_kind(name: &str) -> String {
    if let Some(ext) = name.rsplit('.').next() {
        if ext != name {
            return ext.to_lowercase();
        }
    }
    "file".to_string()
}

pub struct FileListPanel {
    current_path: String,
    is_remote: bool,

    items: Vec<FileItem>,
    filtered_indices: Vec<usize>,
    selected_indices: HashSet<usize>,
    sort_column: SortColumn,
    sort_order: SortOrder,

    show_hidden: bool,
    search_query: String,
    search_input: Entity<InputState>,

    path_editing: bool,
    path_input: Entity<InputState>,

    scroll_handle: UniformListScrollHandle,
    focus_handle: FocusHandle,
    _subscriptions: Vec<gpui::Subscription>,
}

impl FileListPanel {
    pub fn new(
        initial_path: String,
        is_remote: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let path_input = cx.new(|cx| InputState::new(window, cx).placeholder("Enter path..."));
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));

        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe(
            &path_input,
            |this, _, event: &InputEvent, cx| match event {
                InputEvent::PressEnter { .. } => {
                    this.on_path_input_enter(cx);
                }
                InputEvent::Blur => {
                    this.cancel_path_editing(cx);
                }
                _ => {}
            },
        ));

        subscriptions.push(
            cx.subscribe(&search_input, |this, input, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = input.read(cx).text().to_string();
                    this.on_search_change(text, cx);
                }
            }),
        );

        Self {
            current_path: initial_path,
            is_remote,
            items: Vec::new(),
            filtered_indices: Vec::new(),
            selected_indices: HashSet::new(),
            sort_column: SortColumn::Name,
            sort_order: SortOrder::Ascending,
            show_hidden: false,
            search_query: String::new(),
            search_input,
            path_editing: false,
            path_input,
            scroll_handle: UniformListScrollHandle::new(),
            focus_handle,
            _subscriptions: subscriptions,
        }
    }

    pub fn set_items(&mut self, items: Vec<FileItem>, cx: &mut Context<Self>) {
        self.items = items;
        self.selected_indices.clear();
        self.sort_items();
        self.apply_filter();
        cx.notify();
    }

    pub fn set_path(&mut self, path: String, _window: &mut Window, cx: &mut Context<Self>) {
        self.current_path = path;
        self.path_editing = false;
        cx.notify();
    }

    pub fn set_current_path(&mut self, path: String, cx: &mut Context<Self>) {
        self.current_path = path;
        cx.notify();
    }

    pub fn current_path(&self) -> &str {
        &self.current_path
    }

    fn cancel_path_editing(&mut self, cx: &mut Context<Self>) {
        self.path_editing = false;
        cx.notify();
    }

    fn on_path_input_enter(&mut self, cx: &mut Context<Self>) {
        let new_path = self.path_input.read(cx).text().to_string();
        if !new_path.is_empty() && new_path != self.current_path {
            cx.emit(FileListPanelEvent::PathChanged(new_path));
        }
        self.path_editing = false;
        cx.notify();
    }

    fn on_search_change(&mut self, query: String, cx: &mut Context<Self>) {
        self.search_query = query;
        self.apply_filter();
        self.selected_indices.clear();
        cx.notify();
    }

    fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        let show_hidden = self.show_hidden;

        self.filtered_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                // 隐藏文件过滤：以 . 开头的文件
                if !show_hidden && item.name.starts_with('.') {
                    return false;
                }
                // 搜索过滤
                if query.is_empty() {
                    true
                } else {
                    item.name.to_lowercase().contains(&query)
                }
            })
            .map(|(i, _)| i)
            .collect();
    }

    /// 切换隐藏文件显示
    pub fn toggle_show_hidden(&mut self, cx: &mut Context<Self>) {
        self.show_hidden = !self.show_hidden;
        self.apply_filter();
        self.selected_indices.clear();
        cx.notify();
    }

    pub fn selected_items(&self, _cx: &App) -> Vec<FileItem> {
        self.selected_indices
            .iter()
            .filter_map(|&filtered_ix| {
                self.filtered_indices
                    .get(filtered_ix)
                    .and_then(|&real_ix| self.items.get(real_ix).cloned())
            })
            .collect()
    }

    /// 获取用于拖拽的文件项列表
    /// 如果 filtered_ix 在选中列表中，返回所有选中的文件；否则只返回当前文件
    pub fn get_drag_items(&self, filtered_ix: usize) -> Vec<(usize, FileItem)> {
        if self.selected_indices.contains(&filtered_ix) && self.selected_indices.len() > 1 {
            // 当前文件在选中列表中，返回所有选中的文件
            self.selected_indices
                .iter()
                .filter_map(|&idx| {
                    self.filtered_indices
                        .get(idx)
                        .and_then(|&real_ix| self.items.get(real_ix).cloned())
                        .map(|item| (idx, item))
                })
                .collect()
        } else {
            // 当前文件不在选中列表中，只返回当前文件
            self.filtered_indices
                .get(filtered_ix)
                .and_then(|&real_ix| self.items.get(real_ix).cloned())
                .map(|item| vec![(filtered_ix, item)])
                .unwrap_or_default()
        }
    }

    /// 检查某个 filtered_ix 是否在选中列表中
    pub fn is_selected(&self, filtered_ix: usize) -> bool {
        self.selected_indices.contains(&filtered_ix)
    }

    /// 获取选中项的数量
    pub fn selected_count(&self) -> usize {
        self.selected_indices.len()
    }

    pub fn items(&self) -> &[FileItem] {
        &self.items
    }

    fn is_at_root(&self) -> bool {
        if self.is_remote {
            self.current_path == "/" || self.current_path == "." || self.current_path.is_empty()
        } else {
            self.current_path == "/" || std::path::Path::new(&self.current_path).parent().is_none()
        }
    }

    fn toggle_selection(&mut self, row_ix: usize, multi_select: bool) {
        if multi_select {
            if self.selected_indices.contains(&row_ix) {
                self.selected_indices.remove(&row_ix);
            } else {
                self.selected_indices.insert(row_ix);
            }
        } else {
            if !self.selected_indices.contains(&row_ix) {
                self.selected_indices.clear();
                self.selected_indices.insert(row_ix);
            }
        }
    }

    fn set_sort(&mut self, column: SortColumn, cx: &mut Context<Self>) {
        if self.sort_column == column {
            self.sort_order = match self.sort_order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
        } else {
            self.sort_column = column;
            self.sort_order = SortOrder::Ascending;
        }
        self.sort_items();
        self.apply_filter();
        self.selected_indices.clear();
        cx.notify();
    }

    fn sort_items(&mut self) {
        let sort_column = self.sort_column;
        let sort_order = self.sort_order;

        self.items.sort_by(|a, b| {
            if a.is_dir != b.is_dir {
                return if a.is_dir {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }

            let cmp = match sort_column {
                SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortColumn::Modified => a.modified.cmp(&b.modified),
                SortColumn::Size => a.size.cmp(&b.size),
                SortColumn::Kind => get_file_kind(&a.name).cmp(&get_file_kind(&b.name)),
            };

            match sort_order {
                SortOrder::Ascending => cmp,
                SortOrder::Descending => cmp.reverse(),
            }
        });
    }

    fn render_search_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let search_input = self.search_input.clone();
        let has_query = !self.search_query.is_empty();
        let filtered_count = self.filtered_indices.len();
        let total_count = self.items.len();

        h_flex()
            .h_8()
            .px_2()
            .gap_2()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                Icon::new(IconName::Search)
                    .xsmall()
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div().flex_1().child(
                    Input::new(&search_input)
                        .xsmall()
                        .appearance(false)
                        .cleanable(has_query),
                ),
            )
            .when(has_query, |el| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{}/{}", filtered_count, total_count)),
                )
            })
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let sort_column = self.sort_column;
        let sort_order = self.sort_order;

        h_flex()
            .h_8()
            .px_3()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().title_bar)
            .child(self.render_header_cell(
                "Name",
                SortColumn::Name,
                px(250.),
                sort_column,
                sort_order,
                cx,
            ))
            .child(self.render_header_cell(
                "Date Modified",
                SortColumn::Modified,
                px(180.),
                sort_column,
                sort_order,
                cx,
            ))
            .child(self.render_header_cell(
                "Size",
                SortColumn::Size,
                px(100.),
                sort_column,
                sort_order,
                cx,
            ))
            .child(self.render_header_cell(
                "Kind",
                SortColumn::Kind,
                px(80.),
                sort_column,
                sort_order,
                cx,
            ))
    }

    fn render_header_cell(
        &self,
        label: &str,
        column: SortColumn,
        width: gpui::Pixels,
        current_sort: SortColumn,
        sort_order: SortOrder,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_sorted = current_sort == column;
        let label = label.to_string();

        h_flex()
            .w(width)
            .h_full()
            .px_2()
            .items_center()
            .gap_1()
            .cursor_pointer()
            .hover(|s| s.bg(cx.theme().list_active))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _window, cx| {
                    this.set_sort(column, cx);
                }),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(label),
            )
            .when(is_sorted, |el| {
                el.child(
                    Icon::new(if sort_order == SortOrder::Ascending {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    })
                    .xsmall()
                    .text_color(cx.theme().muted_foreground),
                )
            })
    }

    fn render_file_row(
        &self,
        _ix: usize,
        item: &FileItem,
        is_selected: bool,
        cx: &App,
    ) -> impl IntoElement {
        let name = item.name.clone();
        let is_dir = item.is_dir;
        let size = item.size;
        let modified = item.modified;

        h_flex()
            .h(px(44.))
            .px_2()
            .items_center()
            .when(is_selected, |el| el.bg(cx.theme().selection))
            .child(
                h_flex()
                    .w(px(250.))
                    .gap_2()
                    .items_center()
                    .child(
                        Icon::new(if is_dir {
                            IconName::Folder1
                        } else {
                            IconName::File
                        })
                        .with_size(Size::Large)
                        .color(),
                    )
                    .child({
                        let tooltip_name = name.clone();
                        v_flex()
                            .flex_1()
                            .overflow_hidden()
                            .child(
                                div()
                                    .id(SharedString::from(name.clone()))
                                    .text_base()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(name.clone())
                                    .tooltip(move |window, cx| {
                                        Tooltip::new(tooltip_name.clone()).build(window, cx)
                                    }),
                            )
                            .when(!item.permissions.is_empty(), |el| {
                                el.child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(item.permissions.clone()),
                                )
                            })
                    }),
            )
            .child(
                div()
                    .w(px(180.))
                    .px_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(format_modified_time(modified)),
            )
            .child(
                div()
                    .w(px(100.))
                    .px_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(if is_dir {
                        "- -".to_string()
                    } else {
                        format_file_size(size)
                    }),
            )
            .child(
                div()
                    .w(px(80.))
                    .px_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(if is_dir {
                        "folder".to_string()
                    } else {
                        get_file_kind(&name)
                    }),
            )
    }

    fn render_parent_row(&self, _cx: &App) -> impl IntoElement {
        h_flex()
            .h(px(44.))
            .px_2()
            .items_center()
            .child(
                h_flex()
                    .w(px(250.))
                    .gap_2()
                    .items_center()
                    .child(Icon::new(IconName::Folder1).with_size(Size::Large).color())
                    .child(div().text_base().child("..")),
            )
            .child(div().w(px(180.)).px_2())
            .child(div().w(px(100.)).px_2())
            .child(div().w(px(80.)).px_2())
    }

    /// 构建文件项的右键菜单
    /// 根据 is_remote（远程/本地）和 is_dir（文件夹/文件）显示不同的菜单项
    fn build_file_context_menu(
        menu: PopupMenu,
        name: &str,
        full_path: &str,
        is_dir: bool,
        is_remote: bool,
        view: &Entity<Self>,
        window: &mut Window,
        _cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        let name_for_rename = name.to_string();
        let path_for_rename = full_path.to_string();
        let name_for_download = name.to_string();
        let path_for_download = full_path.to_string();
        let name_for_permissions = name.to_string();
        let path_for_permissions = full_path.to_string();
        let path_for_terminal = full_path.to_string();
        let name_for_copy = name.to_string();
        let path_for_copy = full_path.to_string();
        let name_for_delete = name.to_string();
        let path_for_delete = full_path.to_string();

        let view_ref = view.clone();

        let mut menu = menu;

        // 文件夹专属操作：新建文件、新建文件夹
        if is_dir {
            let view_new_file = view_ref.clone();
            let view_new_folder = view_ref.clone();

            menu = menu
                .item(
                    PopupMenuItem::new(t!("File.new_file").to_string())
                        .icon(IconName::File)
                        .on_click(window.listener_for(&view_new_file, move |_this, _, _, cx| {
                            cx.emit(FileListPanelEvent::NewFile);
                        })),
                )
                .item(
                    PopupMenuItem::new(t!("File.new_folder").to_string())
                        .icon(IconName::Folder)
                        .on_click(
                            window.listener_for(&view_new_folder, move |_this, _, _, cx| {
                                cx.emit(FileListPanelEvent::NewFolder);
                            }),
                        ),
                )
                .separator();
        }

        // 重命名（通用）
        let view_rename = view_ref.clone();
        menu = menu.item(
            PopupMenuItem::new(t!("File.rename").to_string())
                .icon(IconName::Edit)
                .on_click(window.listener_for(&view_rename, move |_this, _, _, cx| {
                    cx.emit(FileListPanelEvent::Rename {
                        name: name_for_rename.clone(),
                        full_path: path_for_rename.clone(),
                    });
                })),
        );

        // 远程面板：下载
        if is_remote {
            let view_download = view_ref.clone();
            menu = menu.item(
                PopupMenuItem::new(t!("Common.download").to_string())
                    .icon(IconName::ArrowDown)
                    .on_click(window.listener_for(&view_download, move |_this, _, _, cx| {
                        cx.emit(FileListPanelEvent::Download {
                            name: name_for_download.clone(),
                            full_path: path_for_download.clone(),
                        });
                    })),
            );
        }

        // 本地面板：上传
        if !is_remote {
            let view_upload = view_ref.clone();
            menu = menu.item(
                PopupMenuItem::new(t!("Common.upload").to_string())
                    .icon(IconName::ArrowUp)
                    .on_click(window.listener_for(&view_upload, move |_this, _, _, cx| {
                        cx.emit(FileListPanelEvent::UploadFile);
                    })),
            );
        }

        // 远程面板：修改权限
        if is_remote {
            let view_permissions = view_ref.clone();
            menu = menu.item(
                PopupMenuItem::new(t!("File.change_permission").to_string())
                    .icon(IconName::Key)
                    .on_click(
                        window.listener_for(&view_permissions, move |_this, _, _, cx| {
                            cx.emit(FileListPanelEvent::ChangePermissions {
                                name: name_for_permissions.clone(),
                                full_path: path_for_permissions.clone(),
                            });
                        }),
                    ),
            );
        }

        // 文件夹专属：终端操作
        if is_dir {
            let view_terminal_at = view_ref.clone();
            let view_terminal = view_ref.clone();

            menu = menu
                .separator()
                .item(
                    PopupMenuItem::new(t!("Terminal.open_here").to_string())
                        .icon(IconName::Terminal)
                        .on_click(window.listener_for(
                            &view_terminal_at,
                            move |_this, _, _, cx| {
                                cx.emit(FileListPanelEvent::OpenInTerminalAt {
                                    full_path: path_for_terminal.clone(),
                                });
                            },
                        )),
                )
                .item(
                    PopupMenuItem::new(t!("Terminal.open_in_current").to_string())
                        .icon(IconName::SquareTerminal)
                        .on_click(window.listener_for(&view_terminal, move |_this, _, _, cx| {
                            cx.emit(FileListPanelEvent::OpenInTerminal);
                        })),
                );
        }

        // 复制操作（通用）
        let view_copy_name = view_ref.clone();
        let view_copy_path = view_ref.clone();
        menu = menu
            .separator()
            .item(
                PopupMenuItem::new(t!("File.copy_name").to_string())
                    .icon(IconName::Copy)
                    .on_click(
                        window.listener_for(&view_copy_name, move |_this, _, _, cx| {
                            cx.emit(FileListPanelEvent::CopyFileName {
                                name: name_for_copy.clone(),
                            });
                        }),
                    ),
            )
            .item(
                PopupMenuItem::new(t!("File.copy_path").to_string())
                    .icon(IconName::Copy)
                    .on_click(
                        window.listener_for(&view_copy_path, move |_this, _, _, cx| {
                            cx.emit(FileListPanelEvent::CopyAbsolutePath {
                                full_path: path_for_copy.clone(),
                            });
                        }),
                    ),
            );

        // 删除（通用）
        let view_delete = view_ref.clone();
        menu = menu.separator().item(
            PopupMenuItem::new(t!("Common.delete").to_string())
                .icon(IconName::Remove)
                .on_click(window.listener_for(&view_delete, move |_this, _, _, cx| {
                    cx.emit(FileListPanelEvent::Delete {
                        name: name_for_delete.clone(),
                        full_path: path_for_delete.clone(),
                    });
                })),
        );

        // 远程面板文件夹：上传文件、上传文件夹
        if is_remote && is_dir {
            let view_upload_file = view_ref.clone();
            let view_upload_folder = view_ref.clone();

            menu = menu
                .separator()
                .item(
                    PopupMenuItem::new(t!("File.upload_file").to_string())
                        .icon(IconName::ArrowUp)
                        .on_click(window.listener_for(
                            &view_upload_file,
                            move |_this, _, _, cx| {
                                cx.emit(FileListPanelEvent::UploadFile);
                            },
                        )),
                )
                .item(
                    PopupMenuItem::new(t!("File.upload_folder").to_string())
                        .icon(IconName::FolderOpen)
                        .on_click(window.listener_for(
                            &view_upload_folder,
                            move |_this, _, _, cx| {
                                cx.emit(FileListPanelEvent::UploadFolder);
                            },
                        )),
                );
        }

        // 刷新和显示隐藏文件（通用）
        let view_refresh = view_ref.clone();
        let view_toggle_hidden = view_ref.clone();
        menu = menu
            .separator()
            .item(
                PopupMenuItem::new(t!("Common.refresh").to_string())
                    .icon(IconName::Refresh)
                    .on_click(window.listener_for(&view_refresh, move |_this, _, _, cx| {
                        cx.emit(FileListPanelEvent::Refresh);
                    })),
            )
            .item(
                PopupMenuItem::new(t!("File.toggle_hidden").to_string())
                    .icon(IconName::Eye)
                    .on_click(
                        window.listener_for(&view_toggle_hidden, move |_this, _, _, cx| {
                            cx.emit(FileListPanelEvent::ToggleHiddenFiles);
                        }),
                    ),
            );

        menu
    }
}

#[derive(Clone, Debug)]
pub enum FileListPanelEvent {
    PathChanged(String),
    ItemDoubleClicked(String),
    SelectionChanged(Vec<String>),
    /// 新建文件
    NewFile,
    /// 新建文件夹
    NewFolder,
    /// 重命名文件/文件夹
    Rename {
        name: String,
        full_path: String,
    },
    /// 下载文件/文件夹
    Download {
        name: String,
        full_path: String,
    },
    /// 修改权限
    ChangePermissions {
        name: String,
        full_path: String,
    },
    /// 在终端中打开当前目录
    OpenInTerminal,
    /// 在终端中打开到文件/文件夹
    OpenInTerminalAt {
        full_path: String,
    },
    /// 复制文件名
    CopyFileName {
        name: String,
    },
    /// 复制绝对路径
    CopyAbsolutePath {
        full_path: String,
    },
    /// 删除文件/文件夹
    Delete {
        name: String,
        full_path: String,
    },
    /// 上传文件
    UploadFile,
    /// 上传文件夹
    UploadFolder,
    /// 刷新列表
    Refresh,
    /// 显示隐藏文件
    ToggleHiddenFiles,
}

#[derive(Clone, Debug)]
pub struct DraggedFileItem {
    pub name: String,
    pub is_dir: bool,
    pub full_path: String,
    pub is_remote: bool,
}

/// 支持多文件拖拽的结构体
#[derive(Clone, Debug)]
pub struct DraggedFileItems {
    pub items: Vec<DraggedFileItem>,
    pub is_remote: bool,
}

impl DraggedFileItems {
    pub fn single(item: DraggedFileItem) -> Self {
        let is_remote = item.is_remote;
        Self {
            items: vec![item],
            is_remote,
        }
    }

    pub fn multiple(items: Vec<DraggedFileItem>, is_remote: bool) -> Self {
        Self { items, is_remote }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Render for DraggedFileItems {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.items.len();

        if count == 1 {
            // 单个文件显示详细信息
            let item = &self.items[0];
            h_flex()
                .id("dragged-file-items")
                .cursor_grab()
                .py_1()
                .px_3()
                .gap_2()
                .items_center()
                .bg(cx.theme().background)
                .border_1()
                .border_color(cx.theme().border)
                .rounded_md()
                .shadow_md()
                .child(
                    Icon::new(if item.is_dir {
                        IconName::Folder
                    } else {
                        IconName::File
                    })
                    .text_color(if item.is_dir {
                        cx.theme().link
                    } else {
                        cx.theme().muted_foreground
                    }),
                )
                .child(div().text_sm().child(item.name.clone()))
                .into_any_element()
        } else {
            // 多个文件显示数量
            h_flex()
                .id("dragged-file-items")
                .cursor_grab()
                .py_1()
                .px_3()
                .gap_2()
                .items_center()
                .bg(cx.theme().background)
                .border_1()
                .border_color(cx.theme().border)
                .rounded_md()
                .shadow_md()
                .child(Icon::new(IconName::Folder1).text_color(cx.theme().link))
                .child(
                    div()
                        .text_sm()
                        .child(t!("File.items_count", count = count).to_string()),
                )
                .into_any_element()
        }
    }
}

impl Render for DraggedFileItem {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .id("dragged-file-item")
            .cursor_grab()
            .py_1()
            .px_3()
            .gap_2()
            .items_center()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .shadow_md()
            .child(
                Icon::new(if self.is_dir {
                    IconName::Folder
                } else {
                    IconName::File
                })
                .text_color(if self.is_dir {
                    cx.theme().link
                } else {
                    cx.theme().muted_foreground
                }),
            )
            .child(div().text_sm().child(self.name.clone()))
    }
}

impl gpui::EventEmitter<FileListPanelEvent> for FileListPanel {}

impl Focusable for FileListPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FileListPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let filtered_count = self.filtered_indices.len();
        let show_parent = !self.is_at_root();
        let total_count = if show_parent {
            filtered_count + 1
        } else {
            filtered_count
        };

        v_flex()
            .size_full()
            .child(self.render_search_bar(cx))
            .child(self.render_header(cx))
            .child(
                uniform_list("file-list", total_count, {
                    cx.processor(move |state: &mut Self, range: Range<usize>, _window, cx| {
                        let current_path = state.current_path.clone();
                        let is_remote = state.is_remote;
                        let has_parent = !state.is_at_root();
                        let view = cx.entity();
                        range
                            .map(|list_ix| {
                                if has_parent && list_ix == 0 {
                                    return div()
                                        .id(list_ix)
                                        .cursor_pointer()
                                        .on_double_click(cx.listener(
                                            move |_this, _, _window, cx| {
                                                cx.emit(FileListPanelEvent::ItemDoubleClicked(
                                                    "..".to_string(),
                                                ));
                                            },
                                        ))
                                        .child(state.render_parent_row(cx))
                                        .into_any_element();
                                }

                                let filtered_ix = if has_parent { list_ix - 1 } else { list_ix };
                                let real_ix = state.filtered_indices[filtered_ix];
                                let item = &state.items[real_ix];
                                let is_selected = state.selected_indices.contains(&filtered_ix);
                                let item_name = item.name.clone();
                                let is_dir = item.is_dir;
                                let full_path = if is_remote {
                                    if current_path.ends_with('/') {
                                        format!("{}{}", current_path, item_name)
                                    } else {
                                        format!("{}/{}", current_path, item_name)
                                    }
                                } else {
                                    std::path::Path::new(&current_path)
                                        .join(&item_name)
                                        .to_string_lossy()
                                        .to_string()
                                };

                                // 构建拖拽项目列表
                                // 如果当前文件在选中列表中且有多个选中项，则拖拽所有选中项
                                // 否则只拖拽当前文件
                                let drag_items = if state.selected_indices.contains(&filtered_ix)
                                    && state.selected_indices.len() > 1
                                {
                                    // 拖拽所有选中的文件
                                    let items: Vec<DraggedFileItem> = state
                                        .selected_indices
                                        .iter()
                                        .filter_map(|&idx| {
                                            state.filtered_indices.get(idx).and_then(|&real_ix| {
                                                state.items.get(real_ix).map(|item| {
                                                    let item_path = if is_remote {
                                                        if current_path.ends_with('/') {
                                                            format!("{}{}", current_path, item.name)
                                                        } else {
                                                            format!(
                                                                "{}/{}",
                                                                current_path, item.name
                                                            )
                                                        }
                                                    } else {
                                                        std::path::Path::new(&current_path)
                                                            .join(&item.name)
                                                            .to_string_lossy()
                                                            .to_string()
                                                    };
                                                    DraggedFileItem {
                                                        name: item.name.clone(),
                                                        is_dir: item.is_dir,
                                                        full_path: item_path,
                                                        is_remote,
                                                    }
                                                })
                                            })
                                        })
                                        .collect();
                                    DraggedFileItems::multiple(items, is_remote)
                                } else {
                                    // 只拖拽当前文件
                                    DraggedFileItems::single(DraggedFileItem {
                                        name: item_name.clone(),
                                        is_dir,
                                        full_path: full_path.clone(),
                                        is_remote,
                                    })
                                };

                                // 右键菜单需要的变量
                                let ctx_name = item_name.clone();
                                let ctx_full_path = full_path.clone();
                                let ctx_is_dir = is_dir;
                                let ctx_is_remote = is_remote;
                                let ctx_view = view.clone();

                                div()
                                    .id(list_ix)
                                    .cursor_pointer()
                                    .on_drag(drag_items, |drag, _, _, cx| cx.new(|_| drag.clone()))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            move |this, event: &MouseDownEvent, _window, cx| {
                                                let multi_select = event.modifiers.secondary();
                                                this.toggle_selection(filtered_ix, multi_select);
                                                cx.notify();
                                            },
                                        ),
                                    )
                                    .on_double_click(cx.listener({
                                        let name = item_name.clone();
                                        move |_this, _, _window, cx| {
                                            if is_dir {
                                                cx.emit(FileListPanelEvent::ItemDoubleClicked(
                                                    name.clone(),
                                                ));
                                            }
                                        }
                                    }))
                                    .context_menu(move |menu, window, cx| {
                                        Self::build_file_context_menu(
                                            menu,
                                            &ctx_name,
                                            &ctx_full_path,
                                            ctx_is_dir,
                                            ctx_is_remote,
                                            &ctx_view,
                                            window,
                                            cx,
                                        )
                                    })
                                    .child(state.render_file_row(
                                        filtered_ix,
                                        item,
                                        is_selected,
                                        cx,
                                    ))
                                    .into_any_element()
                            })
                            .collect()
                    })
                })
                .flex_1()
                .size_full()
                .track_scroll(&self.scroll_handle)
                .with_sizing_behavior(ListSizingBehavior::Auto),
            )
    }
}
