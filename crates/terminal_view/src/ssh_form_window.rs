use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, AsyncApp, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, WeakEntity, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    radio::Radio,
    select::{Select, SelectItem, SelectState},
    tab::{Tab, TabBar},
    v_flex, ActiveTheme, Disableable, Sizable, Size, TitleBar,
};
use one_core::cloud_sync::{GlobalCloudUser, TeamOption};
use one_core::connection_notifier::{get_notifier, ConnectionDataEvent};
use one_core::gpui_tokio::Tokio;
use one_core::storage::traits::Repository;
use one_core::storage::{
    JumpServerConfig, ProxyConfig, ProxyType as StorageProxyType, SshAuthMethod, SshParams,
    StoredConnection, Workspace,
};
use rust_i18n::t;
use ssh::{
    JumpServerConnectConfig, ProxyConnectConfig, ProxyType, RusshClient, SshAuth, SshClient,
    SshConnectConfig,
};
use std::time::Duration;

pub struct SshFormWindowConfig {
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

pub struct SshFormWindow {
    focus_handle: FocusHandle,
    title: SharedString,
    is_editing: bool,
    editing_id: Option<i64>,
    editing_cloud_id: Option<String>,
    editing_last_synced_at: Option<i64>,

    // 当前活动标签页索引
    active_tab: usize,

    // 基本信息
    name_input: Entity<InputState>,
    host_input: Entity<InputState>,
    port_input: Entity<InputState>,
    username_input: Entity<InputState>,
    password_input: Entity<InputState>,
    key_path_input: Entity<InputState>,
    passphrase_input: Entity<InputState>,

    auth_method: AuthMethodSelection,
    workspace_select: Entity<SelectState<Vec<WorkspaceSelectItem>>>,
    team_select: Entity<SelectState<Vec<TeamSelectItem>>>,

    // 跳板机设置
    enable_jump_server: bool,
    jump_host_input: Entity<InputState>,
    jump_port_input: Entity<InputState>,
    jump_username_input: Entity<InputState>,
    jump_password_input: Entity<InputState>,

    // 代理设置
    enable_proxy: bool,
    proxy_type: ProxyTypeSelection,
    proxy_host_input: Entity<InputState>,
    proxy_port_input: Entity<InputState>,
    proxy_username_input: Entity<InputState>,
    proxy_password_input: Entity<InputState>,

    // 高级设置
    connect_timeout_input: Entity<InputState>,
    keepalive_interval_input: Entity<InputState>,
    keepalive_max_input: Entity<InputState>,

    // 初始化
    init_script_input: Entity<InputState>,
    default_directory_input: Entity<InputState>,

    // 其他设置
    remark_input: Entity<InputState>,

    last_tested_signature: Option<String>,

    // 云同步开关
    sync_enabled: bool,

    is_testing: bool,
    test_result: Option<Result<(), String>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthMethodSelection {
    #[default]
    Password,
    PrivateKey,
    Agent,
    AutoPublicKey,
}

fn build_connection_test_signature(params: &SshParams) -> String {
    format!("{:?}", params)
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ProxyTypeSelection {
    #[default]
    Socks5,
    Http,
}

impl SshFormWindow {
    pub fn new(config: SshFormWindowConfig, window: &mut Window, cx: &mut Context<Self>) -> Self {
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
            t!("SSH.edit").to_string()
        } else {
            t!("SSH.new").to_string()
        }
        .into();

        // 基本信息
        let name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("SSH.name_placeholder")));
        let host_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("SSH.host_placeholder")));
        let port_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("22");
            state.set_value("22", window, cx);
            state
        });
        let username_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("SSH.username_placeholder")));
        let password_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("SSH.password_placeholder"))
                .masked(true)
        });
        let key_path_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("SSH.key_path_placeholder")));
        let passphrase_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("SSH.passphrase_placeholder"))
                .masked(true)
        });

        // 跳板机设置
        let jump_host_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("SSH.host_placeholder")));
        let jump_port_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("22");
            state.set_value("22", window, cx);
            state
        });
        let jump_username_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("SSH.username_placeholder")));
        let jump_password_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("SSH.password_placeholder"))
                .masked(true)
        });

        // 代理设置
        let proxy_host_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("SSH.proxy_host")));
        let proxy_port_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("1080");
            state.set_value("1080", window, cx);
            state
        });
        let proxy_username_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("SSH.proxy_username")));
        let proxy_password_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("SSH.proxy_password"))
                .masked(true)
        });

        // 高级设置
        let connect_timeout_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("30");
            state.set_value("30", window, cx);
            state
        });
        let keepalive_interval_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("20");
            state.set_value("20", window, cx);
            state
        });
        let keepalive_max_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("3");
            state.set_value("3", window, cx);
            state
        });

        // 初始化
        let init_script_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("SSH.init_script_placeholder"))
                .auto_grow(3, 8)
        });
        let default_directory_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("SSH.default_directory_placeholder"))
        });

        // 其他设置
        let remark_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("SSH.remark_placeholder"))
                .auto_grow(3, 10)
        });

        let mut workspace_items = vec![WorkspaceSelectItem::none()];
        workspace_items.extend(
            config
                .workspaces
                .iter()
                .map(WorkspaceSelectItem::from_workspace),
        );
        let workspace_select =
            cx.new(|cx| SelectState::new(workspace_items, Some(Default::default()), window, cx));

        let mut team_items = vec![TeamSelectItem::personal()];
        team_items.extend(config.teams.iter().map(TeamSelectItem::from_team));
        let team_select =
            cx.new(|cx| SelectState::new(team_items, Some(Default::default()), window, cx));

        let mut auth_method = AuthMethodSelection::Password;
        let mut workspace_id: Option<i64> = None;
        let mut enable_jump_server = false;
        let mut enable_proxy = false;
        let mut proxy_type = ProxyTypeSelection::default();
        let mut sync_enabled = true; // 默认启用云同步

        if let Some(ref conn) = config.editing_connection {
            // 加载同步状态
            sync_enabled = conn.sync_enabled;

            if let Ok(params) = conn.to_ssh_params() {
                name_input.update(cx, |s, cx| s.set_value(&conn.name, window, cx));
                host_input.update(cx, |s, cx| s.set_value(&params.host, window, cx));
                port_input.update(cx, |s, cx| {
                    s.set_value(&params.port.to_string(), window, cx)
                });
                username_input.update(cx, |s, cx| s.set_value(&params.username, window, cx));

                match params.auth_method {
                    SshAuthMethod::Password { ref password } => {
                        auth_method = AuthMethodSelection::Password;
                        password_input.update(cx, |s, cx| s.set_value(password, window, cx));
                    }
                    SshAuthMethod::PrivateKey {
                        ref key_path,
                        ref passphrase,
                    } => {
                        auth_method = AuthMethodSelection::PrivateKey;
                        key_path_input.update(cx, |s, cx| s.set_value(key_path, window, cx));
                        if let Some(ref pass) = passphrase {
                            passphrase_input.update(cx, |s, cx| s.set_value(pass, window, cx));
                        }
                    }
                    SshAuthMethod::Agent => {
                        auth_method = AuthMethodSelection::Agent;
                    }
                    SshAuthMethod::AutoPublicKey => {
                        auth_method = AuthMethodSelection::AutoPublicKey;
                    }
                }

                // 加载高级设置
                if let Some(timeout) = params.connect_timeout {
                    connect_timeout_input
                        .update(cx, |s, cx| s.set_value(&timeout.to_string(), window, cx));
                }
                if let Some(interval) = params.keepalive_interval {
                    keepalive_interval_input
                        .update(cx, |s, cx| s.set_value(&interval.to_string(), window, cx));
                }
                if let Some(max) = params.keepalive_max {
                    keepalive_max_input
                        .update(cx, |s, cx| s.set_value(&max.to_string(), window, cx));
                }

                // 加载初始化设置
                if let Some(ref dir) = params.default_directory {
                    default_directory_input.update(cx, |s, cx| s.set_value(dir, window, cx));
                }
                if let Some(ref script) = params.init_script {
                    init_script_input.update(cx, |s, cx| s.set_value(script, window, cx));
                }

                // 加载跳板机设置
                if let Some(ref jump) = params.jump_server {
                    enable_jump_server = true;
                    jump_host_input.update(cx, |s, cx| s.set_value(&jump.host, window, cx));
                    jump_port_input
                        .update(cx, |s, cx| s.set_value(&jump.port.to_string(), window, cx));
                    jump_username_input.update(cx, |s, cx| s.set_value(&jump.username, window, cx));
                    if let SshAuthMethod::Password { ref password } = jump.auth_method {
                        jump_password_input.update(cx, |s, cx| s.set_value(password, window, cx));
                    }
                }

                // 加载代理设置
                if let Some(ref proxy) = params.proxy {
                    enable_proxy = true;
                    proxy_type = match proxy.proxy_type {
                        StorageProxyType::Socks5 => ProxyTypeSelection::Socks5,
                        StorageProxyType::Http => ProxyTypeSelection::Http,
                    };
                    proxy_host_input.update(cx, |s, cx| s.set_value(&proxy.host, window, cx));
                    proxy_port_input
                        .update(cx, |s, cx| s.set_value(&proxy.port.to_string(), window, cx));
                    if let Some(ref username) = proxy.username {
                        proxy_username_input.update(cx, |s, cx| s.set_value(username, window, cx));
                    }
                    if let Some(ref password) = proxy.password {
                        proxy_password_input.update(cx, |s, cx| s.set_value(password, window, cx));
                    }
                }
            }
            workspace_id = conn.workspace_id;

            // 加载团队归属
            if let Some(ref team_id) = conn.team_id {
                team_select.update(cx, |select, cx| {
                    select.set_selected_value(&Some(team_id.clone()), window, cx);
                });
            }

            // 加载备注
            if let Some(ref remark) = conn.remark {
                remark_input.update(cx, |s, cx| s.set_value(remark, window, cx));
            }
        }

        if let Some(ws_id) = workspace_id {
            workspace_select.update(cx, |select, cx| {
                select.set_selected_value(&Some(ws_id), window, cx);
            });
        }

        Self {
            focus_handle: cx.focus_handle(),
            title,
            is_editing,
            editing_id,
            editing_cloud_id,
            editing_last_synced_at,
            active_tab: 0,
            name_input,
            host_input,
            port_input,
            username_input,
            password_input,
            key_path_input,
            passphrase_input,
            auth_method,
            workspace_select,
            team_select,
            enable_jump_server,
            jump_host_input,
            jump_port_input,
            jump_username_input,
            jump_password_input,
            enable_proxy,
            proxy_type,
            proxy_host_input,
            proxy_port_input,
            proxy_username_input,
            proxy_password_input,
            connect_timeout_input,
            keepalive_interval_input,
            keepalive_max_input,
            init_script_input,
            default_directory_input,
            remark_input,
            last_tested_signature: None,
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

    fn build_ssh_params(&self, cx: &App) -> Option<SshParams> {
        let host = self.host_input.read(cx).text().to_string();
        let port: u16 = self
            .port_input
            .read(cx)
            .text()
            .to_string()
            .parse()
            .unwrap_or(22);
        let username = self.username_input.read(cx).text().to_string();

        if host.is_empty() || username.is_empty() {
            return None;
        }

        let auth_method = match self.auth_method {
            AuthMethodSelection::Password => {
                let password = self.password_input.read(cx).text().to_string();
                SshAuthMethod::Password { password }
            }
            AuthMethodSelection::PrivateKey => {
                let key_path = self.key_path_input.read(cx).text().to_string();
                let passphrase = {
                    let p = self.passphrase_input.read(cx).text().to_string();
                    if p.is_empty() {
                        None
                    } else {
                        Some(p)
                    }
                };
                SshAuthMethod::PrivateKey {
                    key_path,
                    passphrase,
                }
            }
            AuthMethodSelection::Agent => SshAuthMethod::Agent,
            AuthMethodSelection::AutoPublicKey => SshAuthMethod::AutoPublicKey,
        };

        // 高级设置
        let connect_timeout: Option<u64> = self
            .connect_timeout_input
            .read(cx)
            .text()
            .to_string()
            .parse()
            .ok();
        let keepalive_interval: Option<u64> = self
            .keepalive_interval_input
            .read(cx)
            .text()
            .to_string()
            .parse()
            .ok();
        let keepalive_max: Option<usize> = self
            .keepalive_max_input
            .read(cx)
            .text()
            .to_string()
            .parse()
            .ok();

        // 初始化设置
        let default_directory = {
            let d = self.default_directory_input.read(cx).text().to_string();
            if d.is_empty() {
                None
            } else {
                Some(d)
            }
        };
        let init_script = {
            let s = self.init_script_input.read(cx).text().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        };

        // 跳板机配置
        let jump_server = if self.enable_jump_server {
            let jump_host = self.jump_host_input.read(cx).text().to_string();
            let jump_username = self.jump_username_input.read(cx).text().to_string();
            if !jump_host.is_empty() && !jump_username.is_empty() {
                let jump_port: u16 = self
                    .jump_port_input
                    .read(cx)
                    .text()
                    .to_string()
                    .parse()
                    .unwrap_or(22);
                let jump_password = self.jump_password_input.read(cx).text().to_string();
                Some(JumpServerConfig {
                    host: jump_host,
                    port: jump_port,
                    username: jump_username,
                    auth_method: SshAuthMethod::Password {
                        password: jump_password,
                    },
                })
            } else {
                None
            }
        } else {
            None
        };

        // 代理配置
        let proxy = if self.enable_proxy {
            let proxy_host = self.proxy_host_input.read(cx).text().to_string();
            if !proxy_host.is_empty() {
                let proxy_port: u16 = self
                    .proxy_port_input
                    .read(cx)
                    .text()
                    .to_string()
                    .parse()
                    .unwrap_or(1080);
                let proxy_username = {
                    let u = self.proxy_username_input.read(cx).text().to_string();
                    if u.is_empty() {
                        None
                    } else {
                        Some(u)
                    }
                };
                let proxy_password = {
                    let p = self.proxy_password_input.read(cx).text().to_string();
                    if p.is_empty() {
                        None
                    } else {
                        Some(p)
                    }
                };
                let proxy_type = match self.proxy_type {
                    ProxyTypeSelection::Socks5 => StorageProxyType::Socks5,
                    ProxyTypeSelection::Http => StorageProxyType::Http,
                };
                Some(ProxyConfig {
                    proxy_type,
                    host: proxy_host,
                    port: proxy_port,
                    username: proxy_username,
                    password: proxy_password,
                })
            } else {
                None
            }
        } else {
            None
        };

        Some(SshParams {
            host,
            port,
            username,
            auth_method,
            connect_timeout,
            keepalive_interval,
            keepalive_max,
            default_directory,
            init_script,
            jump_server,
            proxy,
        })
    }

    fn build_ssh_connect_config(&self, params: &SshParams) -> SshConnectConfig {
        let auth = match &params.auth_method {
            SshAuthMethod::Password { password } => SshAuth::Password(password.clone()),
            SshAuthMethod::PrivateKey {
                key_path,
                passphrase,
            } => SshAuth::PrivateKey {
                key_path: key_path.clone(),
                passphrase: passphrase.clone(),
                certificate_path: None,
            },
            SshAuthMethod::Agent => SshAuth::Agent,
            SshAuthMethod::AutoPublicKey => SshAuth::AutoPublicKey,
        };

        // 构建跳板机配置
        let jump_server = params.jump_server.as_ref().map(|jump| {
            let jump_auth = match &jump.auth_method {
                SshAuthMethod::Password { password } => SshAuth::Password(password.clone()),
                SshAuthMethod::PrivateKey {
                    key_path,
                    passphrase,
                } => SshAuth::PrivateKey {
                    key_path: key_path.clone(),
                    passphrase: passphrase.clone(),
                    certificate_path: None,
                },
                SshAuthMethod::Agent => SshAuth::Agent,
                SshAuthMethod::AutoPublicKey => SshAuth::AutoPublicKey,
            };
            JumpServerConnectConfig {
                host: jump.host.clone(),
                port: jump.port,
                username: jump.username.clone(),
                auth: jump_auth,
            }
        });

        // 构建代理配置
        let proxy = params.proxy.as_ref().map(|p| {
            let proxy_type = match p.proxy_type {
                StorageProxyType::Socks5 => ProxyType::Socks5,
                StorageProxyType::Http => ProxyType::Http,
            };
            ProxyConnectConfig {
                proxy_type,
                host: p.host.clone(),
                port: p.port,
                username: p.username.clone(),
                password: p.password.clone(),
            }
        });

        SshConnectConfig {
            host: params.host.clone(),
            port: params.port,
            username: params.username.clone(),
            auth,
            timeout: params.connect_timeout.map(Duration::from_secs),
            keepalive_interval: params.keepalive_interval.map(Duration::from_secs),
            keepalive_max: params.keepalive_max,
            jump_server,
            proxy,
        }
    }

    fn on_test(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(params) = self.build_ssh_params(cx) else {
            self.last_tested_signature = None;
            self.test_result = Some(Err(t!("SSH.validation_error").to_string()));
            cx.notify();
            return;
        };

        self.is_testing = true;
        self.last_tested_signature = None;
        self.test_result = None;
        cx.notify();

        let signature = build_connection_test_signature(&params);
        let config = self.build_ssh_connect_config(&params);

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let spawn_result = Tokio::spawn_result(cx, async move {
                let mut client = RusshClient::connect(config).await?;
                client.disconnect().await?;
                Ok::<(), anyhow::Error>(())
            })
            .await;

            let test_result: Result<(), String> = match spawn_result {
                Ok(task) => Ok(task),
                Err(e) => Err(e.to_string()),
            };

            let _ = this.update(cx, |this, cx| {
                this.is_testing = false;
                this.last_tested_signature = test_result.as_ref().ok().map(|_| signature.clone());
                this.test_result = Some(test_result);
                cx.notify();
            });
        })
        .detach();
    }

    fn on_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_testing {
            self.test_result = Some(Err(t!("SSH.save_while_testing").to_string()));
            cx.notify();
            return;
        }

        let Some(params) = self.build_ssh_params(cx) else {
            self.last_tested_signature = None;
            self.test_result = Some(Err(t!("SSH.validation_error").to_string()));
            cx.notify();
            return;
        };

        let current_signature = build_connection_test_signature(&params);
        if !matches!(self.test_result.as_ref(), Some(Ok(()))) {
            self.test_result = Some(Err(t!("SSH.test_required_before_save").to_string()));
            cx.notify();
            return;
        }

        if self.last_tested_signature.as_deref() != Some(current_signature.as_str()) {
            self.test_result = Some(Err(t!("SSH.retest_after_change").to_string()));
            cx.notify();
            return;
        }

        let name = self.name_input.read(cx).text().to_string();
        let name = if name.is_empty() {
            format!("{}@{}:{}", params.username, params.host, params.port)
        } else {
            name
        };

        let workspace_id = self.get_workspace_id(cx);
        let mut conn = StoredConnection::new_ssh(name, params, workspace_id);
        conn.sync_enabled = self.sync_enabled; // 设置同步状态
        conn.team_id = self.get_team_id(cx);
        if !self.is_editing {
            conn.owner_id = GlobalCloudUser::get_user(cx).map(|u| u.id);
        }
        if self.is_editing {
            conn.id = self.editing_id;
            conn.cloud_id = self.editing_cloud_id.clone();
            conn.last_synced_at = self.editing_last_synced_at;
        }

        // 保存备注
        let remark = self.remark_input.read(cx).text().to_string();
        if !remark.is_empty() {
            conn.remark = Some(remark);
        }

        let storage = cx
            .global::<one_core::storage::GlobalStorageState>()
            .storage
            .clone();
        let is_editing = self.is_editing;

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
                window.remove_window();
            }
            Err(e) => {
                let error_msg = t!("SSH.save_failed", error = e).to_string();
                tracing::error!("{}", error_msg);
                self.test_result = Some(Err(error_msg));
                cx.notify();
            }
        }
    }

    fn on_cancel(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        window.remove_window();
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

    /// 渲染基本信息标签页
    fn render_basic_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let auth_method = self.auth_method;

        v_flex()
            .gap_2()
            .child(self.render_form_row(&t!("SSH.name"), Input::new(&self.name_input)))
            .child(self.render_form_row(&t!("SSH.host"), Input::new(&self.host_input)))
            .child(self.render_form_row(&t!("SSH.port"), Input::new(&self.port_input)))
            .child(self.render_form_row(&t!("SSH.username"), Input::new(&self.username_input)))
            .child(
                self.render_form_row(
                    &t!("SSH.auth_method"),
                    h_flex()
                        .gap_4()
                        .child(
                            Radio::new("password")
                                .label(t!("SSH.password").to_string())
                                .checked(auth_method == AuthMethodSelection::Password)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.auth_method = AuthMethodSelection::Password;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Radio::new("private-key")
                                .label(t!("SSH.private_key").to_string())
                                .checked(auth_method == AuthMethodSelection::PrivateKey)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.auth_method = AuthMethodSelection::PrivateKey;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Radio::new("agent")
                                .label(t!("SSH.agent").to_string())
                                .checked(auth_method == AuthMethodSelection::Agent)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.auth_method = AuthMethodSelection::Agent;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Radio::new("auto-publickey")
                                .label(t!("SSH.auto_publickey").to_string())
                                .checked(auth_method == AuthMethodSelection::AutoPublicKey)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.auth_method = AuthMethodSelection::AutoPublicKey;
                                    cx.notify();
                                })),
                        ),
                ),
            )
            .when(auth_method == AuthMethodSelection::Password, |this| {
                this.child(self.render_form_row(
                    &t!("SSH.password"),
                    Input::new(&self.password_input).mask_toggle(),
                ))
            })
            .when(auth_method == AuthMethodSelection::PrivateKey, |this| {
                this.child(
                    self.render_form_row(&t!("SSH.key_path"), Input::new(&self.key_path_input)),
                )
                .child(self.render_form_row(
                    &t!("SSH.passphrase"),
                    Input::new(&self.passphrase_input).mask_toggle(),
                ))
            })
            .when(auth_method == AuthMethodSelection::AutoPublicKey, |this| {
                this.child(
                    h_flex().justify_center().child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("SSH.auto_publickey_hint").to_string()),
                    ),
                )
            })
            .child(self.render_form_row(
                &t!("SSH.workspace"),
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
                                .child(t!("ConnectionForm.cloud_sync_desc").to_string()),
                        ),
                ),
            )
    }

    /// 渲染初始化标签页
    fn render_init_tab(&self) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(self.render_form_row(
                &t!("SSH.default_directory"),
                Input::new(&self.default_directory_input),
            ))
            .child(
                self.render_form_row(&t!("SSH.init_script"), Input::new(&self.init_script_input)),
            )
    }

    /// 渲染跳板机标签页
    fn render_jump_server_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let enable_jump = self.enable_jump_server;

        v_flex()
            .gap_2()
            .child(
                self.render_form_row(
                    &t!("SSH.enable_jump_server"),
                    Checkbox::new("enable-jump")
                        .checked(enable_jump)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.enable_jump_server = !this.enable_jump_server;
                            cx.notify();
                        })),
                ),
            )
            .when(enable_jump, |this| {
                this.child(
                    self.render_form_row(&t!("SSH.jump_host"), Input::new(&self.jump_host_input)),
                )
                .child(
                    self.render_form_row(&t!("SSH.jump_port"), Input::new(&self.jump_port_input)),
                )
                .child(self.render_form_row(
                    &t!("SSH.jump_username"),
                    Input::new(&self.jump_username_input),
                ))
                .child(self.render_form_row(
                    &t!("SSH.jump_password"),
                    Input::new(&self.jump_password_input).mask_toggle(),
                ))
            })
    }

    /// 渲染代理标签页
    fn render_proxy_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let enable_proxy = self.enable_proxy;
        let proxy_type = self.proxy_type;

        v_flex()
            .gap_2()
            .child(
                self.render_form_row(
                    &t!("SSH.enable_proxy"),
                    Checkbox::new("enable-proxy")
                        .checked(enable_proxy)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.enable_proxy = !this.enable_proxy;
                            cx.notify();
                        })),
                ),
            )
            .when(enable_proxy, |this| {
                this.child(
                    self.render_form_row(
                        &t!("SSH.proxy_type"),
                        h_flex()
                            .gap_4()
                            .child(
                                Radio::new("socks5")
                                    .label("SOCKS5")
                                    .checked(proxy_type == ProxyTypeSelection::Socks5)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.proxy_type = ProxyTypeSelection::Socks5;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Radio::new("http")
                                    .label("HTTP")
                                    .checked(proxy_type == ProxyTypeSelection::Http)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.proxy_type = ProxyTypeSelection::Http;
                                        cx.notify();
                                    })),
                            ),
                    ),
                )
                .child(
                    self.render_form_row(&t!("SSH.proxy_host"), Input::new(&self.proxy_host_input)),
                )
                .child(
                    self.render_form_row(&t!("SSH.proxy_port"), Input::new(&self.proxy_port_input)),
                )
                .child(self.render_form_row(
                    &t!("SSH.proxy_username"),
                    Input::new(&self.proxy_username_input),
                ))
                .child(self.render_form_row(
                    &t!("SSH.proxy_password"),
                    Input::new(&self.proxy_password_input).mask_toggle(),
                ))
            })
    }

    /// 渲染高级设置标签页
    fn render_advanced_tab(&self) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(self.render_form_row(
                &t!("SSH.connect_timeout"),
                Input::new(&self.connect_timeout_input),
            ))
            .child(self.render_form_row(
                &t!("SSH.keepalive_interval"),
                Input::new(&self.keepalive_interval_input),
            ))
            .child(self.render_form_row(
                &t!("SSH.keepalive_max"),
                Input::new(&self.keepalive_max_input),
            ))
    }

    /// 渲染其他设置标签页
    fn render_other_tab(&self) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(self.render_form_row(&t!("SSH.remark"), Input::new(&self.remark_input)))
    }
}

impl Focusable for SshFormWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SshFormWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_testing = self.is_testing;
        let active_tab = self.active_tab;

        let test_result_element = match &self.test_result {
            Some(Ok(())) => Some(
                div()
                    .text_sm()
                    .text_color(cx.theme().success)
                    .child(t!("SSH.test_success").to_string()),
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
            // TabBar
            .child(
                div().flex().justify_center().px_3().pt_2().child(
                    TabBar::new("ssh-form-tabs")
                        .with_size(Size::Small)
                        .underline()
                        .selected_index(active_tab)
                        .on_click(cx.listener(|this, ix: &usize, _, cx| {
                            this.active_tab = *ix;
                            cx.notify();
                        }))
                        .child(Tab::new().label(t!("SSH.tab_basic").to_string()))
                        .child(Tab::new().label(t!("SSH.tab_init").to_string()))
                        .child(Tab::new().label(t!("SSH.tab_jump_server").to_string()))
                        .child(Tab::new().label(t!("SSH.tab_proxy").to_string()))
                        .child(Tab::new().label(t!("SSH.tab_advanced").to_string()))
                        .child(Tab::new().label(t!("SSH.tab_other").to_string())),
                ),
            )
            // 标签页内容
            .child(
                div()
                    .id("ssh-form-content")
                    .flex_1()
                    .p_3()
                    .overflow_y_scroll()
                    .child(match active_tab {
                        0 => self.render_basic_tab(cx).into_any_element(),
                        1 => self.render_init_tab().into_any_element(),
                        2 => self.render_jump_server_tab(cx).into_any_element(),
                        3 => self.render_proxy_tab(cx).into_any_element(),
                        4 => self.render_advanced_tab().into_any_element(),
                        5 => self.render_other_tab().into_any_element(),
                        _ => div().into_any_element(),
                    }),
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
                            .disabled(is_testing)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.on_save(window, cx);
                            })),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::build_connection_test_signature;
    use one_core::storage::{SshAuthMethod, SshParams};

    fn sample_params() -> SshParams {
        SshParams {
            host: "127.0.0.1".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_method: SshAuthMethod::Agent,
            connect_timeout: Some(30),
            keepalive_interval: Some(60),
            keepalive_max: Some(3),
            default_directory: Some("/tmp".to_string()),
            init_script: Some("pwd".to_string()),
            jump_server: None,
            proxy: None,
        }
    }

    #[test]
    fn connection_test_signature_changes_when_auth_related_fields_change() {
        let params = sample_params();
        let original = build_connection_test_signature(&params);

        let mut changed = sample_params();
        changed.auth_method = SshAuthMethod::AutoPublicKey;
        assert_ne!(original, build_connection_test_signature(&changed));

        let mut changed_host = sample_params();
        changed_host.host = "example.com".to_string();
        assert_ne!(original, build_connection_test_signature(&changed_host));
    }
}
