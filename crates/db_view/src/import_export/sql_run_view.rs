// 1. 标准库导入
use std::path::PathBuf;
use std::time::Instant;

// 2. 外部 crate 导入（按字母顺序）
use gpui::{
    App, AppContext, AsyncApp, ClickEvent, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, PathPromptOptions, Render, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Disableable, Sizable, VirtualListScrollHandle,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    switch::Switch,
    v_flex, v_virtual_list,
};
use rust_i18n::t;
use std::rc::Rc;

// 3. 当前 crate 导入（按模块分组）
use db::{ExecOptions, GlobalDbState, SqlResult, SqlSource};

#[derive(Debug, Clone)]
struct LogEntry {
    file: String,
    message: String,
    is_error: bool,
}

pub struct SqlRunView {
    connection_id: String,
    database: Option<String>,
    schema: Option<String>,
    file_path: Entity<InputState>,
    pending_file_path: Entity<Option<String>>,
    stop_on_error: Entity<bool>,

    logs: Entity<Vec<LogEntry>>,
    scroll_handle: VirtualListScrollHandle,

    total_statements: Entity<u64>,
    success_count: Entity<u64>,
    error_count: Entity<u32>,
    elapsed_time: Entity<String>,
    progress: Entity<f32>,

    is_running: Entity<bool>,
    is_finished: Entity<bool>,
    start_time: Option<Instant>,

    focus_handle: FocusHandle,
}

impl SqlRunView {
    pub fn new(
        connection_id: impl Into<String>,
        database: Option<String>,
        schema: Option<String>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| Self {
            connection_id: connection_id.into(),
            database,
            schema,
            file_path: cx.new(|cx| InputState::new(window, cx)),
            pending_file_path: cx.new(|_| None),
            stop_on_error: cx.new(|_| false),

            logs: cx.new(|_| Vec::new()),
            scroll_handle: VirtualListScrollHandle::new(),

            total_statements: cx.new(|_| 0),
            success_count: cx.new(|_| 0),
            error_count: cx.new(|_| 0),
            elapsed_time: cx.new(|_| "0.00s".to_string()),
            progress: cx.new(|_| 0.0),

            is_running: cx.new(|_| false),
            is_finished: cx.new(|_| false),
            start_time: None,

            focus_handle: cx.focus_handle(),
        })
    }

    fn add_log(
        cx: &AsyncApp,
        logs: &Entity<Vec<LogEntry>>,
        scroll_handle: &VirtualListScrollHandle,
        file: String,
        message: String,
        is_error: bool,
    ) {
        let logs_clone = logs.clone();
        let scroll_handle_clone = scroll_handle.clone();
        let _ = cx.update(|cx| {
            logs_clone.update(cx, |l, cx| {
                l.push(LogEntry {
                    file,
                    message,
                    is_error,
                });
                cx.notify();
            });
            scroll_handle_clone.scroll_to_bottom();
        });
    }

    fn select_file(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let pending = self.pending_file_path.clone();
        let logs = self.logs.clone();
        let scroll_handle = self.scroll_handle.clone();
        let future = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            multiple: true,
            directories: false,
            prompt: Some(t!("SqlRun.select_sql_file").into()),
        });

        cx.spawn(async move |_, cx: &mut AsyncApp| {
            if let Ok(Ok(Some(paths))) = future.await {
                let mut path = String::new();
                for (i, path_buf) in paths.iter().enumerate() {
                    path.push_str(path_buf.to_str().unwrap_or(""));
                    if i < paths.len() - 1 {
                        path.push(';');
                    }
                }
                let _ = cx.update(|cx| {
                    pending.update(cx, |p, cx| {
                        *p = Some(path.clone());
                        cx.notify();
                    });
                });
                Self::add_log(
                    &cx,
                    &logs,
                    &scroll_handle,
                    "".to_string(),
                    t!("SqlRun.selected_file", path = path).to_string(),
                    false,
                );
            }
        })
        .detach();
    }

    fn start_run(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if *self.is_running.read(cx) {
            return;
        }

        self.is_running.update(cx, |r, cx| {
            *r = true;
            cx.notify();
        });

        self.start_time = Some(Instant::now());

        let global_state = cx.global::<GlobalDbState>().clone();
        let connection_id = self.connection_id.clone();
        let database = self.database.clone();
        let schema = self.schema.clone();
        let file_path_str = self.file_path.read(cx).text().to_string();
        let stop_on_error = *self.stop_on_error.read(cx);
        let logs = self.logs.clone();
        let scroll_handle = self.scroll_handle.clone();
        let total_statements = self.total_statements.clone();
        let success_count = self.success_count.clone();
        let error_count = self.error_count.clone();
        let elapsed_time = self.elapsed_time.clone();
        let progress = self.progress.clone();
        let is_running = self.is_running.clone();
        let is_finished = self.is_finished.clone();
        let start_time = self.start_time;

        if file_path_str.is_empty() {
            self.logs.update(cx, |l, cx| {
                l.push(LogEntry {
                    file: "".to_string(),
                    message: t!("SqlRun.select_sql_file_required").to_string(),
                    is_error: true,
                });
                cx.notify();
            });
            self.scroll_handle.scroll_to_bottom();
            self.is_running.update(cx, |r, cx| {
                *r = false;
                cx.notify();
            });
            return;
        }

        cx.spawn(async move |_, cx: &mut AsyncApp| {
            let files: Vec<String> = file_path_str
                .split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let total_files = files.len();
            let mut last_progress_update = Instant::now();

            for (file_index, file_path) in files.iter().enumerate() {
                Self::add_log(
                    &cx,
                    &logs,
                    &scroll_handle,
                    file_path.clone(),
                    t!(
                        "SqlRun.start_processing_file",
                        current = file_index + 1,
                        total = total_files
                    )
                    .to_string(),
                    false,
                );

                let conn_id = connection_id.clone();
                let opts = ExecOptions {
                    stop_on_error,
                    transactional: false,
                    max_rows: None,
                    streaming: false,
                };

                let rx_result = global_state.execute_streaming(
                    cx,
                    conn_id,
                    SqlSource::File(PathBuf::from(file_path)),
                    database.clone(),
                    schema.clone(),
                    Some(opts),
                );

                let mut rx = match rx_result {
                    Ok(rx) => rx,
                    Err(e) => {
                        Self::add_log(
                            &cx,
                            &logs,
                            &scroll_handle,
                            file_path.clone(),
                            t!("SqlRun.execute_failed", error = e).to_string(),
                            true,
                        );
                        let _ = cx.update(|cx| {
                            error_count.update(cx, |e, cx| {
                                *e += 1;
                                cx.notify();
                            });
                        });

                        if stop_on_error {
                            let _ = cx.update(|cx| {
                                is_running.update(cx, |r, cx| {
                                    *r = false;
                                    cx.notify();
                                });
                            });
                            return;
                        }
                        continue;
                    }
                };

                while let Some(streaming_progress) = rx.recv().await {
                    let is_error = streaming_progress.result.is_error();
                    let now = Instant::now();

                    if is_error {
                        let error_msg = if let SqlResult::Error(e) = &streaming_progress.result {
                            e.message.clone()
                        } else {
                            t!("SqlRun.unknown_error").to_string()
                        };

                        Self::add_log(
                            &cx,
                            &logs,
                            &scroll_handle,
                            file_path.clone(),
                            t!(
                                "SqlRun.statement_failed",
                                statement = streaming_progress.current,
                                error = error_msg
                            )
                            .to_string(),
                            true,
                        );

                        let _ = cx.update(|cx| {
                            error_count.update(cx, |e, cx| {
                                *e += 1;
                                cx.notify();
                            });
                        });

                        if stop_on_error {
                            let _ = cx.update(|cx| {
                                is_running.update(cx, |r, cx| {
                                    *r = false;
                                    cx.notify();
                                });
                            });
                            return;
                        }
                    } else {
                        let _ = cx.update(|cx| {
                            success_count.update(cx, |s, cx| {
                                *s += 1;
                                cx.notify();
                            });
                        });
                    }

                    if now.duration_since(last_progress_update).as_millis() >= 200 || is_error {
                        last_progress_update = now;
                        let elapsed = start_time.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);

                        let file_progress = streaming_progress.progress_percent();
                        let overall_progress = ((file_index as f32 * 100.0 + file_progress)
                            / total_files as f32)
                            .min(100.0);

                        if !is_error {
                            Self::add_log(
                                &cx,
                                &logs,
                                &scroll_handle,
                                file_path.clone(),
                                t!(
                                    "SqlRun.executed_statements",
                                    count = streaming_progress.current,
                                    progress = format!("{:.1}", file_progress)
                                )
                                .to_string(),
                                false,
                            );
                        }

                        let _ = cx.update(|cx| {
                            elapsed_time.update(cx, |t, cx| {
                                *t = format!("{:.2}s", elapsed);
                                cx.notify();
                            });

                            total_statements.update(cx, |t, cx| {
                                *t = streaming_progress.current as u64;
                                cx.notify();
                            });

                            progress.update(cx, |pr, cx| {
                                *pr = overall_progress;
                                cx.notify();
                            });
                        });
                    }
                }

                Self::add_log(
                    &cx,
                    &logs,
                    &scroll_handle,
                    file_path.clone(),
                    t!("SqlRun.file_completed").to_string(),
                    false,
                );
            }

            let elapsed = start_time.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);

            Self::add_log(
                &cx,
                &logs,
                &scroll_handle,
                "".to_string(),
                t!("SqlRun.all_completed", elapsed = format!("{:.2}", elapsed)).to_string(),
                false,
            );

            let _ = cx.update(|cx| {
                progress.update(cx, |p, cx| {
                    *p = 100.0;
                    cx.notify();
                });
                is_running.update(cx, |r, cx| {
                    *r = false;
                    cx.notify();
                });
                is_finished.update(cx, |f, cx| {
                    *f = true;
                    cx.notify();
                });
            });
        })
        .detach();
    }
}

impl Focusable for SqlRunView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Clone for SqlRunView {
    fn clone(&self) -> Self {
        Self {
            connection_id: self.connection_id.clone(),
            database: self.database.clone(),
            schema: self.schema.clone(),
            file_path: self.file_path.clone(),
            pending_file_path: self.pending_file_path.clone(),
            stop_on_error: self.stop_on_error.clone(),

            logs: self.logs.clone(),
            scroll_handle: self.scroll_handle.clone(),

            total_statements: self.total_statements.clone(),
            success_count: self.success_count.clone(),
            error_count: self.error_count.clone(),
            elapsed_time: self.elapsed_time.clone(),
            progress: self.progress.clone(),

            is_running: self.is_running.clone(),
            is_finished: self.is_finished.clone(),
            start_time: self.start_time,

            focus_handle: self.focus_handle.clone(),
        }
    }
}

impl Render for SqlRunView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(path) = self.pending_file_path.read(cx).clone() {
            self.file_path.update(cx, |state, cx| {
                state.replace(path, window, cx);
            });
            self.pending_file_path.update(cx, |p, _| *p = None);
        }

        let is_running = *self.is_running.read(cx);
        let is_finished = *self.is_finished.read(cx);
        let progress_value = *self.progress.read(cx);
        let total_stmts = *self.total_statements.read(cx);
        let success = *self.success_count.read(cx);
        let errors = *self.error_count.read(cx);
        let elapsed = self.elapsed_time.read(cx).clone();
        let logs = self.logs.read(cx).clone();

        v_flex()
            .w_full()
            .h(px(500.0))
            .gap_3()
            .p_4()
            .pt_8()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .w_24()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("SqlRun.sql_file_label").to_string()),
                    )
                    .child(Input::new(&self.file_path).w_full())
                    .child(
                        Button::new("select_file")
                            .small()
                            .child(t!("SqlRun.browse").to_string())
                            .disabled(is_running)
                            .on_click(window.listener_for(
                                &cx.entity(),
                                |view, _: &ClickEvent, window, cx| {
                                    view.select_file(window, cx);
                                },
                            )),
                    ),
            )
            .child(
                h_flex().gap_4().child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            Switch::new("stop_on_error")
                                .checked(*self.stop_on_error.read(cx))
                                .disabled(is_running)
                                .on_click(cx.listener(|view, checked, _, cx| {
                                    view.stop_on_error.update(cx, |value, cx| {
                                        *value = *checked;
                                        cx.notify();
                                    });
                                })),
                        )
                        .child(t!("SqlRun.stop_on_error").to_string()),
                ),
            )
            .child(div().h_px().bg(cx.theme().border))
            .child(
                h_flex()
                    .gap_6()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("SqlRun.total_statements").to_string()),
                            )
                            .child(div().child(total_stmts.to_string())),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("SqlRun.success_label").to_string()),
                            )
                            .child(div().child(success.to_string())),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("SqlRun.error_label").to_string()),
                            )
                            .child(
                                div()
                                    .text_color(if errors > 0 {
                                        cx.theme().danger
                                    } else {
                                        cx.theme().foreground
                                    })
                                    .child(errors.to_string()),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("SqlRun.time_label").to_string()),
                            )
                            .child(div().child(elapsed)),
                    ),
            )
            .child(div().h_px().bg(cx.theme().border))
            .child({
                let chars_per_line = 100;
                let line_height = 20.0_f32;
                let min_height = line_height;
                let max_height = 80.0_f32;

                let item_sizes = Rc::new(
                    logs.iter()
                        .map(|entry| {
                            let text_len = if entry.file.is_empty() {
                                entry.message.len() + 6
                            } else {
                                let short_file_len = entry
                                    .file
                                    .split('/')
                                    .last()
                                    .map(|s| s.len())
                                    .unwrap_or(entry.file.len());
                                short_file_len + entry.message.len() + 8
                            };
                            let lines =
                                ((text_len as f32 / chars_per_line as f32).ceil() as i32).max(1);
                            let height = (lines as f32 * line_height).clamp(min_height, max_height);
                            gpui::size(px(0.), px(height))
                        })
                        .collect::<Vec<_>>(),
                );

                div()
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_md()
                    .overflow_hidden()
                    .bg(cx.theme().background)
                    .p_2()
                    .child(
                        v_virtual_list(
                            cx.entity().clone(),
                            "logs-virtual-list",
                            item_sizes.clone(),
                            move |view, visible_range, _window, cx| {
                                let logs = view.logs.read(cx);
                                visible_range
                                    .into_iter()
                                    .filter_map(|idx| {
                                        logs.get(idx).map(|entry| {
                                            let text = if entry.file.is_empty() {
                                                format!("[RUN] {}", entry.message)
                                            } else {
                                                let short_file = entry
                                                    .file
                                                    .split('/')
                                                    .last()
                                                    .unwrap_or(&entry.file);
                                                format!("[RUN] {}> {}", short_file, entry.message)
                                            };
                                            let is_error = entry.is_error;
                                            let item_height = item_sizes
                                                .get(idx)
                                                .map(|s| s.height)
                                                .unwrap_or(px(20.));
                                            div()
                                                .id(("log-entry", idx))
                                                .w_full()
                                                .text_xs()
                                                .h(item_height)
                                                .text_color(if is_error {
                                                    cx.theme().danger
                                                } else {
                                                    cx.theme().foreground
                                                })
                                                .child(text)
                                        })
                                    })
                                    .collect()
                            },
                        )
                        .size_full()
                        .track_scroll(&self.scroll_handle),
                    )
            })
            .child(
                div()
                    .h_2()
                    .w_full()
                    .rounded_full()
                    .bg(cx.theme().primary.opacity(0.2))
                    .child(
                        div()
                            .h_full()
                            .rounded_full()
                            .bg(cx.theme().primary)
                            .w(gpui::relative(progress_value / 100.0)),
                    ),
            )
            .child(
                h_flex()
                    .pt_2()
                    .gap_2()
                    .justify_end()
                    .when(!is_running && !is_finished, |this| {
                        this.child(
                            Button::new("start")
                                .primary()
                                .child(t!("SqlRun.execute").to_string())
                                .on_click(window.listener_for(
                                    &cx.entity(),
                                    |view, _: &ClickEvent, window, cx| {
                                        view.start_run(window, cx);
                                    },
                                )),
                        )
                    })
                    .when(is_running, |this| {
                        this.child(
                            Button::new("running")
                                .loading(true)
                                .child(t!("SqlRun.executing").to_string()),
                        )
                    })
                    .when(is_finished, |this| {
                        this.child(
                            Button::new("close")
                                .child(t!("SqlRun.close").to_string())
                                .on_click(|_, window, _cx| {
                                    window.remove_window();
                                }),
                        )
                    }),
            )
    }
}
