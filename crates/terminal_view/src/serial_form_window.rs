use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    select::{Select, SelectItem, SelectState},
    v_flex, ActiveTheme, Disableable, IndexPath, Sizable, TitleBar,
};
use one_core::cloud_sync::{GlobalCloudUser, TeamOption};
use one_core::connection_notifier::{get_notifier, ConnectionDataEvent};
use one_core::storage::traits::Repository;
use one_core::storage::{
    SerialFlowControl, SerialParams, SerialParity, StoredConnection, Workspace,
};
use rust_i18n::t;

pub struct SerialFormWindowConfig {
    pub editing_connection: Option<StoredConnection>,
    pub workspaces: Vec<Workspace>,
    pub teams: Vec<TeamOption>,
}

#[derive(Clone, Default, PartialEq)]
struct WorkspaceSelectItem {
    id: Option<i64>,
    name: String,
}

impl WorkspaceSelectItem {
    fn none() -> Self {
        Self {
            id: None,
            name: t!("Common.none").to_string(),
        }
    }

    fn from_workspace(ws: &Workspace) -> Self {
        Self {
            id: ws.id,
            name: ws.name.clone(),
        }
    }
}

impl SelectItem for WorkspaceSelectItem {
    type Value = Option<i64>;

    fn title(&self) -> SharedString {
        self.name.clone().into()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

#[derive(Clone, Default, PartialEq)]
struct TeamSelectItem {
    id: Option<String>,
    name: String,
}

impl TeamSelectItem {
    fn personal() -> Self {
        Self {
            id: None,
            name: t!("TeamSync.personal").to_string(),
        }
    }

    fn from_team(team: &TeamOption) -> Self {
        Self {
            id: Some(team.id.clone()),
            name: team.name.clone(),
        }
    }
}

impl SelectItem for TeamSelectItem {
    type Value = Option<String>;

    fn title(&self) -> SharedString {
        self.name.clone().into()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

#[derive(Clone, PartialEq)]
struct BaudRateItem {
    rate: u32,
}

impl SelectItem for BaudRateItem {
    type Value = u32;

    fn title(&self) -> SharedString {
        self.rate.to_string().into()
    }

    fn value(&self) -> &Self::Value {
        &self.rate
    }
}

#[derive(Clone, PartialEq)]
struct DataBitsItem {
    bits: u8,
}

impl SelectItem for DataBitsItem {
    type Value = u8;

    fn title(&self) -> SharedString {
        self.bits.to_string().into()
    }

    fn value(&self) -> &Self::Value {
        &self.bits
    }
}

#[derive(Clone, PartialEq)]
struct StopBitsItem {
    bits: u8,
}

impl SelectItem for StopBitsItem {
    type Value = u8;

    fn title(&self) -> SharedString {
        self.bits.to_string().into()
    }

    fn value(&self) -> &Self::Value {
        &self.bits
    }
}

#[derive(Clone, PartialEq)]
struct ParityItem {
    parity: SerialParity,
    label: &'static str,
}

impl SelectItem for ParityItem {
    type Value = SerialParity;

    fn title(&self) -> SharedString {
        self.label.into()
    }

    fn value(&self) -> &Self::Value {
        &self.parity
    }
}

#[derive(Clone, PartialEq)]
struct FlowControlItem {
    flow: SerialFlowControl,
    label: &'static str,
}

impl SelectItem for FlowControlItem {
    type Value = SerialFlowControl;

    fn title(&self) -> SharedString {
        self.label.into()
    }

    fn value(&self) -> &Self::Value {
        &self.flow
    }
}

#[derive(Clone, PartialEq)]
struct PortItem {
    name: String,
}

impl SelectItem for PortItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.name.clone().into()
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }
}

pub struct SerialFormWindow {
    focus_handle: FocusHandle,
    title: SharedString,
    is_editing: bool,
    editing_id: Option<i64>,
    editing_cloud_id: Option<String>,
    editing_last_synced_at: Option<i64>,

    // 基本信息
    name_input: Entity<InputState>,
    port_name_input: Entity<InputState>,
    port_select: Entity<SelectState<Vec<PortItem>>>,
    baud_rate_select: Entity<SelectState<Vec<BaudRateItem>>>,
    data_bits_select: Entity<SelectState<Vec<DataBitsItem>>>,
    stop_bits_select: Entity<SelectState<Vec<StopBitsItem>>>,
    parity_select: Entity<SelectState<Vec<ParityItem>>>,
    flow_control_select: Entity<SelectState<Vec<FlowControlItem>>>,
    workspace_select: Entity<SelectState<Vec<WorkspaceSelectItem>>>,
    team_select: Entity<SelectState<Vec<TeamSelectItem>>>,
    remark_input: Entity<InputState>,
    sync_enabled: bool,

    is_testing: bool,
    test_result: Option<Result<(), String>>,
}

fn enumerate_ports() -> Vec<PortItem> {
    match serialport::available_ports() {
        Ok(ports) => ports
            .into_iter()
            .map(|p| PortItem { name: p.port_name })
            .collect(),
        Err(_) => Vec::new(),
    }
}

const BAUD_RATES: &[u32] = &[
    300, 1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600,
];

impl SerialFormWindow {
    pub fn new(
        config: SerialFormWindowConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let is_editing = config.editing_connection.is_some();
        let editing_id = config.editing_connection.as_ref().and_then(|c| c.id);
        let editing_cloud_id = config
            .editing_connection
            .as_ref()
            .and_then(|c| c.cloud_id.clone());
        let editing_last_synced_at = config
            .editing_connection
            .as_ref()
            .and_then(|c| c.last_synced_at);

        let title: SharedString = if is_editing {
            t!("Serial.edit").to_string()
        } else {
            t!("Serial.new").to_string()
        }
        .into();

        let name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("Serial.name_placeholder")));
        let port_name_input = cx
            .new(|cx| InputState::new(window, cx).placeholder(t!("Serial.port_name_placeholder")));
        let remark_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("Serial.remark_placeholder"))
                .auto_grow(3, 10)
        });

        // 串口选择
        let ports = enumerate_ports();
        let port_select = cx.new(|cx| SelectState::new(ports, None, window, cx));

        // 波特率选择，默认 115200（索引 8）
        let baud_items: Vec<BaudRateItem> = BAUD_RATES
            .iter()
            .map(|&r| BaudRateItem { rate: r })
            .collect();
        let default_baud_index = BAUD_RATES.iter().position(|&r| r == 115200).unwrap_or(8);
        let baud_rate_select = cx.new(|cx| {
            SelectState::new(
                baud_items,
                Some(IndexPath::default().row(default_baud_index)),
                window,
                cx,
            )
        });

        // 数据位选择，默认 8（索引 3）
        let data_bits_items: Vec<DataBitsItem> = vec![
            DataBitsItem { bits: 5 },
            DataBitsItem { bits: 6 },
            DataBitsItem { bits: 7 },
            DataBitsItem { bits: 8 },
        ];
        let data_bits_select = cx.new(|cx| {
            SelectState::new(
                data_bits_items,
                Some(IndexPath::default().row(3)),
                window,
                cx,
            )
        });

        // 停止位选择，默认 1（索引 0）
        let stop_bits_items: Vec<StopBitsItem> =
            vec![StopBitsItem { bits: 1 }, StopBitsItem { bits: 2 }];
        let stop_bits_select = cx.new(|cx| {
            SelectState::new(
                stop_bits_items,
                Some(IndexPath::default().row(0)),
                window,
                cx,
            )
        });

        // 校验位选择，默认 None（索引 0）
        let parity_items: Vec<ParityItem> = SerialParity::all()
            .iter()
            .map(|p| ParityItem {
                parity: *p,
                label: p.label(),
            })
            .collect();
        let parity_select = cx.new(|cx| {
            SelectState::new(parity_items, Some(IndexPath::default().row(0)), window, cx)
        });

        // 流控选择，默认 None（索引 0）
        let flow_items: Vec<FlowControlItem> = SerialFlowControl::all()
            .iter()
            .map(|f| FlowControlItem {
                flow: *f,
                label: f.label(),
            })
            .collect();
        let flow_control_select = cx
            .new(|cx| SelectState::new(flow_items, Some(IndexPath::default().row(0)), window, cx));

        // 工作区选择
        let mut workspace_items = vec![WorkspaceSelectItem::none()];
        workspace_items.extend(
            config
                .workspaces
                .iter()
                .map(WorkspaceSelectItem::from_workspace),
        );
        let workspace_select =
            cx.new(|cx| SelectState::new(workspace_items, Some(Default::default()), window, cx));

        // 团队选择
        let mut team_items = vec![TeamSelectItem::personal()];
        team_items.extend(config.teams.iter().map(TeamSelectItem::from_team));
        let team_select =
            cx.new(|cx| SelectState::new(team_items, Some(Default::default()), window, cx));

        let mut sync_enabled = true;
        let mut workspace_id: Option<i64> = None;
        let mut team_id: Option<String> = None;

        // 编辑模式：加载已有数据
        if let Some(ref conn) = config.editing_connection {
            sync_enabled = conn.sync_enabled;

            if let Ok(params) = conn.to_serial_params() {
                name_input.update(cx, |s, cx| s.set_value(&conn.name, window, cx));
                port_name_input.update(cx, |s, cx| s.set_value(&params.port_name, window, cx));

                port_select.update(cx, |s, cx| {
                    s.set_selected_value(&params.port_name, window, cx);
                });
                baud_rate_select.update(cx, |s, cx| {
                    s.set_selected_value(&params.baud_rate, window, cx);
                });
                data_bits_select.update(cx, |s, cx| {
                    s.set_selected_value(&params.data_bits, window, cx);
                });
                stop_bits_select.update(cx, |s, cx| {
                    s.set_selected_value(&params.stop_bits, window, cx);
                });
                parity_select.update(cx, |s, cx| {
                    s.set_selected_value(&params.parity, window, cx);
                });
                flow_control_select.update(cx, |s, cx| {
                    s.set_selected_value(&params.flow_control, window, cx);
                });
            }
            workspace_id = conn.workspace_id;
            team_id = conn.team_id.clone();

            if let Some(ref remark) = conn.remark {
                remark_input.update(cx, |s, cx| s.set_value(remark, window, cx));
            }
        }

        if let Some(ws_id) = workspace_id {
            workspace_select.update(cx, |select, cx| {
                select.set_selected_value(&Some(ws_id), window, cx);
            });
        }

        if let Some(ref tid) = team_id {
            team_select.update(cx, |select, cx| {
                select.set_selected_value(&Some(tid.clone()), window, cx);
            });
        }

        Self {
            focus_handle: cx.focus_handle(),
            title,
            is_editing,
            editing_id,
            editing_cloud_id,
            editing_last_synced_at,
            name_input,
            port_name_input,
            port_select,
            baud_rate_select,
            data_bits_select,
            stop_bits_select,
            parity_select,
            flow_control_select,
            workspace_select,
            team_select,
            remark_input,
            sync_enabled,
            is_testing: false,
            test_result: None,
        }
    }

    fn get_workspace_id(&self, cx: &App) -> Option<i64> {
        self.workspace_select
            .read(cx)
            .selected_value()
            .cloned()
            .flatten()
    }

    fn get_team_id(&self, cx: &App) -> Option<String> {
        self.team_select
            .read(cx)
            .selected_value()
            .cloned()
            .flatten()
    }

    fn get_port_name(&self, cx: &App) -> String {
        // 优先使用手动输入的端口名
        let manual = self.port_name_input.read(cx).text().to_string();
        if !manual.is_empty() {
            return manual;
        }
        // 否则使用下拉选择的端口
        self.port_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_default()
    }

    fn build_serial_params(&self, cx: &App) -> Option<SerialParams> {
        let port_name = self.get_port_name(cx);
        if port_name.is_empty() {
            return None;
        }

        let baud_rate = self
            .baud_rate_select
            .read(cx)
            .selected_value()
            .copied()
            .unwrap_or(115200);
        let data_bits = self
            .data_bits_select
            .read(cx)
            .selected_value()
            .copied()
            .unwrap_or(8);
        let stop_bits = self
            .stop_bits_select
            .read(cx)
            .selected_value()
            .copied()
            .unwrap_or(1);
        let parity = self
            .parity_select
            .read(cx)
            .selected_value()
            .copied()
            .unwrap_or(SerialParity::None);
        let flow_control = self
            .flow_control_select
            .read(cx)
            .selected_value()
            .copied()
            .unwrap_or(SerialFlowControl::None);

        Some(SerialParams {
            port_name,
            baud_rate,
            data_bits,
            stop_bits,
            parity,
            flow_control,
        })
    }

    fn on_test(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(params) = self.build_serial_params(cx) else {
            self.test_result = Some(Err(t!("Serial.validation_error").to_string()));
            cx.notify();
            return;
        };

        self.is_testing = true;
        self.test_result = None;
        cx.notify();

        // 串口测试：尝试打开然后立即关闭
        let result = (|| {
            use std::time::Duration;

            let data_bits = match params.data_bits {
                5 => serialport::DataBits::Five,
                6 => serialport::DataBits::Six,
                7 => serialport::DataBits::Seven,
                _ => serialport::DataBits::Eight,
            };
            let stop_bits = match params.stop_bits {
                2 => serialport::StopBits::Two,
                _ => serialport::StopBits::One,
            };
            let parity = match params.parity {
                SerialParity::Odd => serialport::Parity::Odd,
                SerialParity::Even => serialport::Parity::Even,
                SerialParity::None => serialport::Parity::None,
            };
            let flow_control = match params.flow_control {
                SerialFlowControl::Software => serialport::FlowControl::Software,
                SerialFlowControl::Hardware => serialport::FlowControl::Hardware,
                SerialFlowControl::None => serialport::FlowControl::None,
            };

            let _port = serialport::new(&params.port_name, params.baud_rate)
                .data_bits(data_bits)
                .stop_bits(stop_bits)
                .parity(parity)
                .flow_control(flow_control)
                .timeout(Duration::from_secs(3))
                .open()
                .map_err(|e| {
                    let msg = e.to_string();
                    // macOS 上对 pty 虚拟串口调用 ioctl(TIOCEXCL) 会报 ENOTTY
                    if msg.contains("Not a typewriter") || msg.contains("ENOTTY") {
                        format!("{}\n{}", msg, t!("Serial.enotty_hint"))
                    } else {
                        msg
                    }
                })?;

            Ok::<(), String>(())
        })();

        self.is_testing = false;
        self.test_result = Some(result);
        cx.notify();
    }

    fn on_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(params) = self.build_serial_params(cx) else {
            self.test_result = Some(Err(t!("Serial.validation_error").to_string()));
            cx.notify();
            return;
        };

        let name = self.name_input.read(cx).text().to_string();
        let name = if name.is_empty() {
            format!("{}@{}", params.baud_rate, params.port_name)
        } else {
            name
        };

        let workspace_id = self.get_workspace_id(cx);
        let mut conn = StoredConnection::new_serial(name, params, workspace_id);
        conn.sync_enabled = self.sync_enabled;
        conn.team_id = self.get_team_id(cx);
        if !self.is_editing {
            conn.owner_id = GlobalCloudUser::get_user(cx).map(|u| u.id);
        }
        if self.is_editing {
            conn.id = self.editing_id;
            conn.cloud_id = self.editing_cloud_id.clone();
            conn.last_synced_at = self.editing_last_synced_at;
        }

        let remark = self.remark_input.read(cx).text().to_string();
        if !remark.is_empty() {
            conn.remark = Some(remark);
        }

        let storage = cx
            .global::<one_core::storage::GlobalStorageState>()
            .storage
            .clone();
        let is_editing = self.is_editing;

        cx.spawn(async move |_this, cx| {
            let result: Result<StoredConnection, anyhow::Error> = (|| {
                let repo = storage
                    .get::<one_core::storage::ConnectionRepository>()
                    .ok_or_else(|| anyhow::anyhow!("ConnectionRepository not found"))?;

                if is_editing {
                    repo.update(&mut conn)?;
                } else {
                    repo.insert(&mut conn)?;
                }
                Ok(conn)
            })();

            match result {
                Ok(saved_conn) => {
                    let _ = cx.update(|cx| {
                        if let Some(notifier) = get_notifier(cx) {
                            let event = if is_editing {
                                ConnectionDataEvent::ConnectionUpdated {
                                    connection: saved_conn,
                                }
                            } else {
                                ConnectionDataEvent::ConnectionCreated {
                                    connection: saved_conn,
                                }
                            };
                            notifier.update(cx, |_, cx| {
                                cx.emit(event);
                            });
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("保存串口连接失败: {}", e);
                }
            }
        })
        .detach();

        window.remove_window();
    }

    fn on_cancel(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        window.remove_window();
    }

    fn on_refresh_ports(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ports = enumerate_ports();
        let selected = self.get_port_name(cx);

        // 找到当前选中端口在新列表中的索引
        let selected_index = if selected.is_empty() {
            None
        } else {
            ports
                .iter()
                .position(|p| p.name == selected)
                .map(|ix| IndexPath::default().row(ix))
        };

        self.port_select.update(cx, |state, cx| {
            *state = SelectState::new(ports, selected_index, window, cx);
        });
        cx.notify();
    }

    fn render_form_row(&self, label: &str, child: impl IntoElement) -> impl IntoElement {
        h_flex()
            .gap_3()
            .items_center()
            .child(
                div()
                    .w(px(100.0))
                    .text_sm()
                    .text_right()
                    .child(label.to_string()),
            )
            .child(div().flex_1().child(child))
    }
}

impl Focusable for SerialFormWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SerialFormWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_testing = self.is_testing;

        let test_result_element = match &self.test_result {
            Some(Ok(())) => Some(
                div()
                    .text_sm()
                    .text_color(cx.theme().success)
                    .child(t!("Serial.test_success").to_string()),
            ),
            Some(Err(e)) => Some(
                div()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(e.clone()),
            ),
            None => None,
        };

        v_flex()
            .justify_center()
            .size_full()
            .bg(cx.theme().background)
            .child(
                TitleBar::new().child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .flex_1()
                        .text_sm()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(self.title.clone()),
                ),
            )
            // 表单内容
            .child(
                div()
                    .id("serial-form-content")
                    .flex_1()
                    .p_3()
                    .overflow_y_scroll()
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                self.render_form_row(
                                    &t!("Serial.name"),
                                    Input::new(&self.name_input),
                                ),
                            )
                            .child(
                                self.render_form_row(
                                    &t!("Serial.port_name"),
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .flex_1()
                                                .child(Select::new(&self.port_select).w_full()),
                                        )
                                        .child(
                                            Button::new("refresh-ports")
                                                .small()
                                                .outline()
                                                .label(t!("Serial.refresh_ports").to_string())
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.on_refresh_ports(window, cx);
                                                })),
                                        ),
                                ),
                            )
                            .child(self.render_form_row("", Input::new(&self.port_name_input)))
                            .child(self.render_form_row(
                                &t!("Serial.baud_rate"),
                                Select::new(&self.baud_rate_select).w_full(),
                            ))
                            .child(self.render_form_row(
                                &t!("Serial.data_bits"),
                                Select::new(&self.data_bits_select).w_full(),
                            ))
                            .child(self.render_form_row(
                                &t!("Serial.stop_bits"),
                                Select::new(&self.stop_bits_select).w_full(),
                            ))
                            .child(self.render_form_row(
                                &t!("Serial.parity"),
                                Select::new(&self.parity_select).w_full(),
                            ))
                            .child(self.render_form_row(
                                &t!("Serial.flow_control"),
                                Select::new(&self.flow_control_select).w_full(),
                            ))
                            .child(self.render_form_row(
                                &t!("Serial.workspace"),
                                Select::new(&self.workspace_select).w_full(),
                            ))
                            .child(self.render_form_row(
                                &t!("TeamSync.team_label"),
                                Select::new(&self.team_select).w_full(),
                            ))
                            .child(
                                self.render_form_row(
                                    &t!("ConnectionForm.cloud_sync"),
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            Checkbox::new("sync-enabled")
                                                .checked(self.sync_enabled)
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.sync_enabled = !this.sync_enabled;
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(
                                                    t!("ConnectionForm.cloud_sync_desc")
                                                        .to_string(),
                                                ),
                                        ),
                                ),
                            )
                            .child(self.render_form_row(
                                &t!("Serial.remark"),
                                Input::new(&self.remark_input),
                            )),
                    ),
            )
            // 测试结果
            .when_some(test_result_element, |this, elem| {
                this.child(h_flex().justify_center().pb_2().child(elem))
            })
            // 底部按钮
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .px_6()
                    .py_4()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("cancel")
                            .small()
                            .label(t!("Common.cancel").to_string())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.on_cancel(window, cx);
                            })),
                    )
                    .child(
                        Button::new("test")
                            .small()
                            .outline()
                            .label(if is_testing {
                                t!("Connection.testing").to_string()
                            } else {
                                t!("Connection.test").to_string()
                            })
                            .disabled(is_testing)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.on_test(window, cx);
                            })),
                    )
                    .child(
                        Button::new("ok")
                            .small()
                            .primary()
                            .label(t!("Common.ok").to_string())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.on_save(window, cx);
                            })),
                    ),
            )
    }
}
