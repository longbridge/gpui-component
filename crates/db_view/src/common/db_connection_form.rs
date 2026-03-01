use anyhow::Error;
use db::{GlobalDbState, oracle};
use gpui::prelude::FluentBuilder;
use gpui::{
    App, AsyncApp, Axis, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, PathPromptOptions, Render, SharedString, Styled, Window, div,
    prelude::*, px,
};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, IndexPath, Sizable, Size,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    clipboard::Clipboard,
    form::{field, v_form},
    h_flex,
    input::{Input, InputEvent, InputState},
    popover::Popover,
    scroll::ScrollableElement,
    select::{Select, SelectEvent, SelectItem, SelectState},
    tab::{Tab, TabBar},
    v_flex,
};
use one_core::gpui_tokio::Tokio;
use one_core::storage::traits::Repository;
use one_core::storage::{
    ConnectionRepository, DatabaseType, DbConnectionConfig, GlobalStorageState, StoredConnection,
    Workspace, get_config_dir,
};
use rust_i18n::t;

/// Form select item for dropdown fields
#[derive(Clone, Debug)]
pub struct FormSelectItem {
    pub value: String,
    pub label: String,
}

impl FormSelectItem {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

impl SelectItem for FormSelectItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.label.clone().into()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

/// Workspace select item for dropdown
#[derive(Clone, Debug)]
pub struct WorkspaceSelectItem {
    pub id: Option<i64>,
    pub name: String,
}

impl WorkspaceSelectItem {
    pub fn none() -> Self {
        Self {
            id: None,
            name: t!("Common.none").to_string(),
        }
    }

    pub fn from_workspace(ws: &Workspace) -> Self {
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

/// Represents a tab group containing multiple fields
#[derive(Clone, Debug)]
pub struct TabGroup {
    pub name: String,
    pub label: String,
    pub fields: Vec<FormField>,
}

impl TabGroup {
    pub fn new(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            fields: Vec::new(),
        }
    }

    pub fn field(mut self, field: FormField) -> Self {
        self.fields.push(field);
        self
    }

    pub fn fields(mut self, fields: Vec<FormField>) -> Self {
        self.fields = fields;
        self
    }
}

/// Represents a field in the connection form
#[derive(Clone, Debug)]
pub struct FormField {
    pub name: String,
    pub label: String,
    pub placeholder: String,
    pub field_type: FormFieldType,
    pub rows: usize,
    pub required: bool,
    pub default_value: String,
    pub options: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FormFieldType {
    Text,
    Number,
    Password,
    TextArea,
    Select,
}

impl FormField {
    pub fn new(
        name: impl Into<String>,
        label: impl Into<String>,
        field_type: FormFieldType,
    ) -> Self {
        let name = name.into();
        Self {
            placeholder: format!("Enter {}", name.to_lowercase()),
            name,
            label: label.into(),
            field_type,
            rows: 5,
            required: true,
            default_value: String::new(),
            options: Vec::new(),
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn default(mut self, value: impl Into<String>) -> Self {
        self.default_value = value.into();
        self
    }

    pub fn options(mut self, options: Vec<(String, String)>) -> Self {
        self.options = options;
        self
    }
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows;
        self
    }
}

/// Database connection form configuration for different database types
pub struct DbFormConfig {
    pub db_type: DatabaseType,
    pub title: String,
    pub tab_groups: Vec<TabGroup>,
}

impl DbFormConfig {
    /// MySQL form configuration
    pub fn mysql() -> Self {
        Self {
            db_type: DatabaseType::MySQL,
            title: format!("{} (MySQL)", t!("Common.new")),
            tab_groups: vec![
                TabGroup::new("general", t!("ConnectionForm.general")).fields(vec![
                    FormField::new(
                        "name",
                        t!("ConnectionForm.connection_name"),
                        FormFieldType::Text,
                    )
                    .placeholder("My MySQL Database")
                    .default("Local MySQL"),
                    FormField::new("host", t!("ConnectionForm.host"), FormFieldType::Text)
                        .placeholder("localhost")
                        .default("localhost"),
                    FormField::new("port", t!("ConnectionForm.port"), FormFieldType::Number)
                        .placeholder("3306")
                        .default("3306"),
                    FormField::new(
                        "username",
                        t!("ConnectionForm.username"),
                        FormFieldType::Text,
                    )
                    .placeholder("root")
                    .default("root"),
                    FormField::new(
                        "password",
                        t!("ConnectionForm.password"),
                        FormFieldType::Password,
                    )
                    .placeholder("Enter password"),
                    FormField::new(
                        "database",
                        t!("ConnectionForm.database"),
                        FormFieldType::Text,
                    )
                    .optional()
                    .placeholder("database name (optional)")
                    .default("ai_app"),
                ]),
                TabGroup::new("advanced", t!("ConnectionForm.advanced")).fields(vec![
                    FormField::new(
                        "connect_timeout",
                        t!("ConnectionForm.connect_timeout"),
                        FormFieldType::Number,
                    )
                    .optional()
                    .placeholder("30")
                    .default("30"),
                    FormField::new(
                        "read_timeout",
                        t!("ConnectionForm.read_timeout"),
                        FormFieldType::Number,
                    )
                    .optional()
                    .placeholder("28800"),
                ]),
                TabGroup::new("ssl", t!("ConnectionForm.ssl")),
                TabGroup::new("ssh", t!("ConnectionForm.ssh")),
                TabGroup::new("notes", t!("ConnectionForm.notes")).fields(vec![
                    FormField::new(
                        "remark",
                        t!("ConnectionForm.remark"),
                        FormFieldType::TextArea,
                    )
                    .rows(14)
                    .optional()
                    .placeholder(t!("ConnectionForm.enter_remark"))
                    .default(""),
                ]),
            ],
        }
    }

    /// PostgreSQL form configuration
    pub fn postgres() -> Self {
        Self {
            db_type: DatabaseType::PostgreSQL,
            title: format!("{} (PostgreSQL)", t!("Common.new")),
            tab_groups: vec![
                TabGroup::new("general", t!("ConnectionForm.general")).fields(vec![
                    FormField::new(
                        "name",
                        t!("ConnectionForm.connection_name"),
                        FormFieldType::Text,
                    )
                    .placeholder("My PostgreSQL Database")
                    .default("Local PostgreSQL"),
                    FormField::new("host", t!("ConnectionForm.host"), FormFieldType::Text)
                        .placeholder("localhost")
                        .default("localhost"),
                    FormField::new("port", t!("ConnectionForm.port"), FormFieldType::Number)
                        .placeholder("5432")
                        .default("5432"),
                    FormField::new(
                        "username",
                        t!("ConnectionForm.username"),
                        FormFieldType::Text,
                    )
                    .placeholder("postgres")
                    .default("postgres"),
                    FormField::new(
                        "password",
                        t!("ConnectionForm.password"),
                        FormFieldType::Password,
                    )
                    .placeholder("Enter password"),
                    FormField::new(
                        "database",
                        t!("ConnectionForm.database"),
                        FormFieldType::Text,
                    )
                    .optional()
                    .placeholder("database name (optional)"),
                ]),
                TabGroup::new("advanced", t!("ConnectionForm.advanced")).fields(vec![
                    FormField::new(
                        "connect_timeout",
                        t!("ConnectionForm.connect_timeout"),
                        FormFieldType::Number,
                    )
                    .optional()
                    .placeholder("30")
                    .default("30"),
                    FormField::new(
                        "application_name",
                        t!("ConnectionForm.application_name"),
                        FormFieldType::Text,
                    )
                    .optional()
                    .placeholder("Application Name"),
                ]),
                TabGroup::new("ssl", t!("ConnectionForm.ssl")),
                TabGroup::new("ssh", t!("ConnectionForm.ssh")),
                TabGroup::new("notes", t!("ConnectionForm.notes")).fields(vec![
                    FormField::new(
                        "remark",
                        t!("ConnectionForm.remark"),
                        FormFieldType::TextArea,
                    )
                    .rows(14)
                    .optional()
                    .placeholder(t!("ConnectionForm.enter_remark"))
                    .default(""),
                ]),
            ],
        }
    }

    /// MSSQL (SQL Server) form configuration
    pub fn mssql() -> Self {
        Self {
            db_type: DatabaseType::MSSQL,
            title: format!("{} (SQL Server)", t!("Common.new")),
            tab_groups: vec![
                TabGroup::new("general", t!("ConnectionForm.general")).fields(vec![
                    FormField::new(
                        "name",
                        t!("ConnectionForm.connection_name"),
                        FormFieldType::Text,
                    )
                    .placeholder("My SQL Server Database")
                    .default("Local SQL Server"),
                    FormField::new("host", t!("ConnectionForm.host"), FormFieldType::Text)
                        .placeholder("localhost")
                        .default("localhost"),
                    FormField::new("port", t!("ConnectionForm.port"), FormFieldType::Number)
                        .placeholder("1433")
                        .default("1433"),
                    FormField::new(
                        "username",
                        t!("ConnectionForm.username"),
                        FormFieldType::Text,
                    )
                    .placeholder("sa")
                    .default("sa"),
                    FormField::new(
                        "password",
                        t!("ConnectionForm.password"),
                        FormFieldType::Password,
                    )
                    .placeholder("Enter password"),
                    FormField::new(
                        "database",
                        t!("ConnectionForm.database"),
                        FormFieldType::Text,
                    )
                    .optional()
                    .placeholder("database name (optional)"),
                ]),
                TabGroup::new("advanced", t!("ConnectionForm.advanced")).fields(vec![
                    FormField::new(
                        "connect_timeout",
                        t!("ConnectionForm.connect_timeout"),
                        FormFieldType::Number,
                    )
                    .optional()
                    .placeholder("30")
                    .default("30"),
                    FormField::new(
                        "encrypt",
                        t!("ConnectionForm.encrypt"),
                        FormFieldType::Select,
                    )
                    .optional()
                    .default("off")
                    .options(vec![
                        (
                            "off".to_string(),
                            t!("ConnectionForm.encrypt_off").to_string(),
                        ),
                        (
                            "on".to_string(),
                            t!("ConnectionForm.encrypt_on").to_string(),
                        ),
                        (
                            "required".to_string(),
                            t!("ConnectionForm.encrypt_strict").to_string(),
                        ),
                    ]),
                    FormField::new(
                        "trust_cert",
                        t!("ConnectionForm.trust_certificate"),
                        FormFieldType::Select,
                    )
                    .optional()
                    .default("true")
                    .options(vec![
                        ("true".to_string(), t!("Common.yes").to_string()),
                        ("false".to_string(), t!("Common.no").to_string()),
                    ]),
                    FormField::new(
                        "application_name",
                        t!("ConnectionForm.application_name"),
                        FormFieldType::Text,
                    )
                    .optional()
                    .placeholder("Application Name"),
                ]),
                TabGroup::new("ssl", t!("ConnectionForm.ssl")),
                TabGroup::new("ssh", t!("ConnectionForm.ssh")),
                TabGroup::new("notes", t!("ConnectionForm.notes")).fields(vec![
                    FormField::new(
                        "remark",
                        t!("ConnectionForm.remark"),
                        FormFieldType::TextArea,
                    )
                    .rows(14)
                    .optional()
                    .placeholder(t!("ConnectionForm.enter_remark"))
                    .default(""),
                ]),
            ],
        }
    }

    /// Oracle form configuration
    pub fn oracle() -> Self {
        Self {
            db_type: DatabaseType::Oracle,
            title: format!("{} (Oracle)", t!("Common.new")),
            tab_groups: vec![
                TabGroup::new("general", t!("ConnectionForm.general")).fields(vec![
                    FormField::new(
                        "name",
                        t!("ConnectionForm.connection_name"),
                        FormFieldType::Text,
                    )
                    .placeholder("My Oracle Database")
                    .default("Local Oracle"),
                    FormField::new("host", t!("ConnectionForm.host"), FormFieldType::Text)
                        .placeholder("localhost")
                        .default("localhost"),
                    FormField::new("port", t!("ConnectionForm.port"), FormFieldType::Number)
                        .placeholder("1521")
                        .default("1521"),
                    FormField::new(
                        "username",
                        t!("ConnectionForm.username"),
                        FormFieldType::Text,
                    )
                    .placeholder("system")
                    .default("system"),
                    FormField::new(
                        "password",
                        t!("ConnectionForm.password"),
                        FormFieldType::Password,
                    )
                    .placeholder("Enter password"),
                    FormField::new("service_name", "Service Name", FormFieldType::Text)
                        .optional()
                        .placeholder("ORCL (or use SID)"),
                    FormField::new("sid", "SID", FormFieldType::Text)
                        .optional()
                        .placeholder("orcl (or use Service Name)"),
                ]),
                TabGroup::new("advanced", t!("ConnectionForm.advanced")).fields(vec![
                    FormField::new(
                        "connect_timeout",
                        t!("ConnectionForm.connect_timeout"),
                        FormFieldType::Number,
                    )
                    .optional()
                    .placeholder("30")
                    .default("30"),
                ]),
                TabGroup::new("ssl", t!("ConnectionForm.ssl")),
                TabGroup::new("ssh", t!("ConnectionForm.ssh")),
                TabGroup::new("notes", t!("ConnectionForm.notes")).fields(vec![
                    FormField::new(
                        "remark",
                        t!("ConnectionForm.remark"),
                        FormFieldType::TextArea,
                    )
                    .rows(14)
                    .optional()
                    .placeholder(t!("ConnectionForm.enter_remark"))
                    .default(""),
                ]),
            ],
        }
    }

    /// ClickHouse form configuration
    pub fn clickhouse() -> Self {
        Self {
            db_type: DatabaseType::ClickHouse,
            title: format!("{} (ClickHouse)", t!("Common.new")),
            tab_groups: vec![
                TabGroup::new("general", t!("ConnectionForm.general")).fields(vec![
                    FormField::new(
                        "name",
                        t!("ConnectionForm.connection_name"),
                        FormFieldType::Text,
                    )
                    .placeholder("My ClickHouse Database")
                    .default("Local ClickHouse"),
                    FormField::new("host", t!("ConnectionForm.host"), FormFieldType::Text)
                        .placeholder("localhost")
                        .default("localhost"),
                    FormField::new("port", t!("ConnectionForm.port"), FormFieldType::Number)
                        .placeholder("8123 (HTTP port)")
                        .default("8123"),
                    FormField::new(
                        "username",
                        t!("ConnectionForm.username"),
                        FormFieldType::Text,
                    )
                    .placeholder("default")
                    .default("default"),
                    FormField::new(
                        "password",
                        t!("ConnectionForm.password"),
                        FormFieldType::Password,
                    )
                    .placeholder("Enter password"),
                    FormField::new(
                        "database",
                        t!("ConnectionForm.database"),
                        FormFieldType::Text,
                    )
                    .optional()
                    .placeholder("database name (optional)"),
                ]),
                TabGroup::new("advanced", t!("ConnectionForm.advanced")).fields(vec![
                    FormField::new(
                        "schema",
                        t!("ConnectionForm.schema"),
                        FormFieldType::Select,
                    )
                    .optional()
                    .default("http")
                    .options(vec![
                        (
                            "http".to_string(),
                            t!("ConnectionForm.schema_http").to_string(),
                        ),
                        (
                            "https".to_string(),
                            t!("ConnectionForm.schema_https").to_string(),
                        ),
                    ]),
                    FormField::new(
                        "connect_timeout",
                        t!("ConnectionForm.connect_timeout"),
                        FormFieldType::Number,
                    )
                    .optional()
                    .placeholder("30")
                    .default("30"),
                    FormField::new(
                        "compression",
                        t!("ConnectionForm.compression"),
                        FormFieldType::Select,
                    )
                    .optional()
                    .default("lz4")
                    .options(vec![
                        ("none".to_string(), t!("Common.none").to_string()),
                        ("lz4".to_string(), "LZ4".to_string()),
                    ]),
                ]),
                TabGroup::new("ssl", t!("ConnectionForm.ssl")),
                TabGroup::new("ssh", t!("ConnectionForm.ssh")),
                TabGroup::new("notes", t!("ConnectionForm.notes")).fields(vec![
                    FormField::new(
                        "remark",
                        t!("ConnectionForm.remark"),
                        FormFieldType::TextArea,
                    )
                    .rows(14)
                    .optional()
                    .placeholder(t!("ConnectionForm.enter_remark"))
                    .default(""),
                ]),
            ],
        }
    }

    /// SQLite form configuration
    pub fn sqlite() -> Self {
        let default_db_path = get_config_dir()
            .map(|p| p.join("onetcli_default.db").to_string_lossy().to_string())
            .unwrap_or_else(|_| "onetcli_default.db".to_string());

        Self {
            db_type: DatabaseType::SQLite,
            title: format!("{} (SQLite)", t!("Common.new")),
            tab_groups: vec![
                TabGroup::new("general", t!("ConnectionForm.general")).fields(vec![
                    FormField::new(
                        "name",
                        t!("ConnectionForm.connection_name"),
                        FormFieldType::Text,
                    )
                    .placeholder("My SQLite Database")
                    .default("Local SQLite"),
                    FormField::new(
                        "host",
                        t!("ConnectionForm.database_file_path"),
                        FormFieldType::Text,
                    )
                    .placeholder("/path/to/database.db")
                    .default(default_db_path),
                ]),
                TabGroup::new("notes", t!("ConnectionForm.notes")).fields(vec![
                    FormField::new(
                        "remark",
                        t!("ConnectionForm.remark"),
                        FormFieldType::TextArea,
                    )
                    .rows(14)
                    .optional()
                    .placeholder(t!("ConnectionForm.enter_remark"))
                    .default(""),
                ]),
            ],
        }
    }
}

/// Event emitted when a connection is saved successfully
#[derive(Clone, Debug)]
pub enum DbConnectionFormEvent {
    Saved(StoredConnection),
    SaveError(String),
}

/// Database connection form modal
pub struct DbConnectionForm {
    config: DbFormConfig,
    current_db_type: Entity<DatabaseType>,
    focus_handle: FocusHandle,
    active_tab: usize,
    field_values: Vec<(String, Entity<String>)>,
    field_inputs: Vec<Option<Entity<InputState>>>,
    field_selects: std::collections::HashMap<String, Entity<SelectState<Vec<FormSelectItem>>>>,
    is_testing: Entity<bool>,
    test_result: Entity<Option<Result<bool, String>>>,
    workspace_select: Entity<SelectState<Vec<WorkspaceSelectItem>>>,
    pending_file_path: Entity<Option<String>>,
    editing_connection: Option<StoredConnection>,
    /// 是否启用云同步
    sync_enabled: Entity<bool>,
    /// Oracle 客户端检测状态：Ok(版本) / Err(错误)
    oracle_client_status: Entity<Option<Result<String, String>>>,
    oracle_client_checking: Entity<bool>,
}

impl DbConnectionForm {
    pub fn new(config: DbFormConfig, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let current_db_type = cx.new(|_| config.db_type);

        // Initialize field values, inputs, and selects
        let mut field_values = Vec::new();
        let mut field_inputs = Vec::new();
        let mut field_selects = std::collections::HashMap::new();

        for tab_group in &config.tab_groups {
            for field in &tab_group.fields {
                let value = cx.new(|_| field.default_value.clone());
                field_values.push((field.name.clone(), value.clone()));

                if field.field_type == FormFieldType::Select {
                    // Create SelectState for Select fields
                    let items: Vec<FormSelectItem> = field
                        .options
                        .iter()
                        .map(|(v, l)| FormSelectItem::new(v.clone(), l.clone()))
                        .collect();
                    // Find the index of the default value
                    let selected_index = if field.default_value.is_empty() {
                        Some(IndexPath::new(0))
                    } else {
                        items
                            .iter()
                            .position(|i| i.value == field.default_value)
                            .map(IndexPath::new)
                    };
                    let field_name = field.name.clone();
                    let value_clone = value.clone();
                    let select = cx.new(|cx| SelectState::new(items, selected_index, window, cx));
                    // Subscribe to select changes
                    cx.subscribe_in(
                        &select,
                        window,
                        move |_form,
                              _select,
                              event: &SelectEvent<Vec<FormSelectItem>>,
                              _window,
                              cx| {
                            if let SelectEvent::Confirm(Some(val)) = event {
                                value_clone.update(cx, |v, cx| {
                                    *v = val.clone();
                                    cx.notify();
                                });
                            }
                        },
                    )
                    .detach();
                    field_selects.insert(field_name, select);
                    field_inputs.push(None);
                } else {
                    // Create InputState for other field types
                    let input = cx.new(|cx| {
                        let mut input_state =
                            InputState::new(window, cx).placeholder(&field.placeholder);

                        if field.field_type == FormFieldType::Password {
                            input_state = input_state.masked(true);
                        }

                        if field.field_type == FormFieldType::TextArea {
                            if field.name == "remark" {
                                input_state = input_state.auto_grow(3, 10);
                            } else if field.rows == 14 {
                                input_state = input_state.rows(14);
                            } else {
                                input_state = input_state.auto_grow(5, 14);
                            }
                        }

                        input_state.set_value(field.default_value.clone(), window, cx);
                        input_state
                    });

                    // Subscribe to input changes
                    let value_clone = value.clone();
                    cx.subscribe_in(&input, window, move |_form, _input, event, _window, cx| {
                        if let InputEvent::Change = event {
                            value_clone.update(cx, |v, cx| {
                                *v = _input.read(cx).text().to_string();
                                cx.notify();
                            });
                        }
                    })
                    .detach();

                    field_inputs.push(Some(input));
                }
            }
        }

        let is_testing = cx.new(|_| false);
        let test_result = cx.new(|_| None);

        let workspace_items = vec![WorkspaceSelectItem::none()];
        let workspace_select =
            cx.new(|cx| SelectState::new(workspace_items, Some(Default::default()), window, cx));

        let pending_file_path = cx.new(|_| None);

        // 默认启用云同步
        let sync_enabled = cx.new(|_| true);
        let oracle_client_status = cx.new(|_| None);
        let oracle_client_checking = cx.new(|_| false);

        let form = Self {
            config,
            current_db_type,
            focus_handle,
            active_tab: 0,
            field_values,
            field_inputs,
            field_selects,
            is_testing,
            test_result,
            workspace_select,
            pending_file_path,
            editing_connection: None,
            sync_enabled,
            oracle_client_status,
            oracle_client_checking,
        };

        form.refresh_oracle_client_status(cx);
        form
    }

    fn refresh_oracle_client_status(&self, cx: &mut Context<Self>) {
        if *self.current_db_type.read(cx) != DatabaseType::Oracle {
            self.oracle_client_checking.update(cx, |checking, cx| {
                *checking = false;
                cx.notify();
            });
            self.oracle_client_status.update(cx, |status, cx| {
                *status = None;
                cx.notify();
            });
            return;
        }

        self.oracle_client_checking.update(cx, |checking, cx| {
            *checking = true;
            cx.notify();
        });

        let checking_handle = self.oracle_client_checking.clone();
        let status_handle = self.oracle_client_status.clone();

        cx.spawn(async move |_, cx: &mut AsyncApp| {
            let result = oracle::detect_local_client_version();
            let _ = cx.update(|cx| {
                checking_handle.update(cx, |checking, cx| {
                    *checking = false;
                    cx.notify();
                });
                status_handle.update(cx, |status, cx| {
                    *status = Some(result);
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn oracle_client_guide_text(&self, cx: &App) -> Option<String> {
        if *self.current_db_type.read(cx) != DatabaseType::Oracle {
            return None;
        }

        let has_error = matches!(self.oracle_client_status.read(cx).as_ref(), Some(Err(_)));
        if !has_error {
            return None;
        }
        
        #[cfg(target_os = "windows")]
        return Some(t!("ConnectionForm.oracle_client_guide_windows").to_string());
        #[cfg(target_os = "macos")]
        return Some(t!("ConnectionForm.oracle_client_guide_macos").to_string());
        #[cfg(target_os = "linux")]
        return Some(t!("ConnectionForm.oracle_client_guide_linux").to_string());
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        return Some(t!("ConnectionForm.oracle_client_guide_other").to_string());
    }

    fn oracle_client_download_url(&self, cx: &App) -> Option<&'static str> {
        if *self.current_db_type.read(cx) != DatabaseType::Oracle {
            return None;
        }

        let has_error = matches!(self.oracle_client_status.read(cx).as_ref(), Some(Err(_)));
        if !has_error {
            return None;
        }

        Some("https://www.oracle.com/database/technologies/instant-client/downloads.html")
    }

    pub fn set_workspaces(
        &mut self,
        workspaces: Vec<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut items = vec![WorkspaceSelectItem::none()];
        items.extend(workspaces.iter().map(WorkspaceSelectItem::from_workspace));

        self.workspace_select.update(cx, |select, cx| {
            select.set_items(items, window, cx);
        });
        cx.notify();
    }

    pub fn load_connection(
        &mut self,
        connection: &StoredConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.editing_connection = Some(connection.clone());
        self.set_field_value("name", &connection.name, window, cx);

        // 加载同步状态
        self.sync_enabled.update(cx, |sync, cx| {
            *sync = connection.sync_enabled;
            cx.notify();
        });

        if let Ok(params) = connection.to_db_connection() {
            self.set_field_value("host", &params.host, window, cx);
            self.set_field_value("port", &params.port.to_string(), window, cx);
            self.set_field_value("username", &params.username, window, cx);
            self.set_field_value("password", &params.password, window, cx);
            if let Some(db) = &params.database {
                self.set_field_value("database", db, window, cx);
            }
            if let Some(sn) = &params.service_name {
                self.set_field_value("service_name", sn, window, cx);
            }
            if let Some(sid) = &params.sid {
                self.set_field_value("sid", sid, window, cx);
            }
        }

        if let Some(remark) = &connection.remark {
            self.set_field_value("remark", remark, window, cx);
        }

        if let Some(ws_id) = connection.workspace_id {
            self.workspace_select.update(cx, |select, cx| {
                select.set_selected_value(&Some(ws_id), window, cx);
            });
        } else {
            self.workspace_select.update(cx, |select, cx| {
                select.set_selected_value(&None, window, cx);
            });
        }
    }

    fn set_field_value(
        &mut self,
        field_name: &str,
        value: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some((idx, _)) = self
            .field_values
            .iter()
            .enumerate()
            .find(|(_, (name, _))| name == field_name)
        {
            self.field_values[idx].1.update(cx, |v, cx| {
                *v = value.to_string();
                cx.notify();
            });
            // Update input or select based on field type
            if let Some(Some(input)) = self.field_inputs.get(idx) {
                input.update(cx, |input, cx| {
                    input.set_value(value.to_string(), window, cx);
                });
            } else if let Some(select) = self.field_selects.get(field_name) {
                select.update(cx, |select, cx| {
                    select.set_selected_value(&value.to_string(), window, cx);
                });
            }
        }
    }

    fn get_field_value(&self, field_name: &str, cx: &App) -> Option<String> {
        self.field_values
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, value)| value.read(cx).clone())
    }

    fn build_connection(&self, cx: &App) -> DbConnectionConfig {
        let workspace_id = self
            .workspace_select
            .read(cx)
            .selected_value()
            .cloned()
            .flatten();

        // Collect extra params (fields that are not basic connection fields)
        let basic_fields = [
            "name",
            "host",
            "port",
            "username",
            "password",
            "database",
            "remark",
            "service_name",
            "sid",
        ];
        let mut extra_params = std::collections::HashMap::new();

        for (field_name, value_entity) in &self.field_values {
            if !basic_fields.contains(&field_name.as_str()) {
                let value = value_entity.read(cx).clone();
                if !value.is_empty() {
                    extra_params.insert(field_name.clone(), value);
                }
            }
        }

        let db_type = *self.current_db_type.read(cx);

        let port_str = self.get_field_value("port", cx);

        let mut port = 3306;

        if let Some(port_str) = port_str {
            port = port_str.parse().unwrap_or(3306);
        }
        DbConnectionConfig {
            id: String::new(),
            database_type: db_type,
            name: self.get_field_value("name", cx).unwrap_or_default(),
            host: self.get_field_value("host", cx).unwrap_or_default(),
            port,
            username: self.get_field_value("username", cx).unwrap_or_default(),
            password: self.get_field_value("password", cx).unwrap_or_default(),
            database: self.get_field_value("database", cx),
            service_name: self.get_field_value("service_name", cx),
            sid: self.get_field_value("sid", cx),
            workspace_id,
            extra_params,
        }
    }

    fn validate(&self, cx: &App) -> Result<(), String> {
        for tab_group in &self.config.tab_groups {
            for field in &tab_group.fields {
                if field.required {
                    let value = self.get_field_value(&field.name, cx);
                    if value.is_none() {
                        return Err(format!("{} is required", field.label));
                    }
                }
            }
        }

        self.validate_oracle_client(cx)?;
        Ok(())
    }

    fn validate_oracle_client(&self, cx: &App) -> Result<(), String> {
        if *self.current_db_type.read(cx) != DatabaseType::Oracle {
            return Ok(());
        }

        oracle::detect_local_client_version()
            .map(|_| ())
            .map_err(|error| t!("ConnectionForm.oracle_client_required", error = error).to_string())
    }

    fn simplify_connection_error_message(err: &Error) -> String {
        let mut message = err
            .chain()
            .last()
            .map(|e| e.to_string())
            .unwrap_or_else(|| err.to_string());

        // 去掉常见包装前缀，只保留最有价值的底层异常信息。
        let prefixes = [
            "connection error: ",
            "query error: ",
            "transaction error: ",
            "failed to connect: ",
            "failed to switch schema: ",
            "failed to query: ",
        ];

        loop {
            let mut changed = false;
            for prefix in prefixes {
                if let Some(rest) = message.strip_prefix(prefix) {
                    message = rest.trim().to_string();
                    changed = true;
                    break;
                }
            }
            if !changed {
                break;
            }
        }

        if let Some(pos) = message.find("ORA-") {
            return message[pos..].trim().to_string();
        }

        message.trim().to_string()
    }

    pub fn trigger_test_connection(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self.validate(cx) {
            self.test_result.update(cx, |result, cx| {
                *result = Some(Err(e));
                cx.notify();
            });
            return;
        }

        let connection = self.build_connection(cx);
        let db_type = *self.current_db_type.read(cx);

        self.is_testing.update(cx, |testing, cx| {
            *testing = true;
            cx.notify();
        });

        let global_state = cx.global::<GlobalDbState>().clone();
        let test_result_handle = self.test_result.clone();
        let is_testing_handle = self.is_testing.clone();

        cx.spawn(async move |_, cx: &mut AsyncApp| {
            let manager = global_state.db_manager;

            let test_result = Tokio::spawn_result(cx, async move {
                let db_plugin = manager.get_plugin(&db_type)?;
                let conn = db_plugin.create_connection(connection).await?;
                conn.ping().await?;
                Ok::<bool, Error>(true)
            })
            .await;

            let result_msg = match test_result {
                Ok(_) => Ok(true),
                Err(err) => {
                    let detail = Self::simplify_connection_error_message(&err);
                    Err(format!("{}: {}", t!("ConnectionForm.test_failed"), detail))
                }
            };

            let _ = cx.update(|cx| {
                is_testing_handle.update(cx, |testing, cx| {
                    *testing = false;
                    cx.notify();
                });
                test_result_handle.update(cx, |result, cx| {
                    *result = Some(result_msg);
                    cx.notify();
                });
            });
        })
        .detach();
    }

    pub fn build_stored_connection(&self, cx: &App) -> Result<(StoredConnection, bool), String> {
        self.validate(cx)?;

        let connection = self.build_connection(cx);
        let remark = self.get_field_value("remark", cx);
        let is_update = self.editing_connection.is_some();
        let sync_enabled = *self.sync_enabled.read(cx);

        let mut stored = match &self.editing_connection {
            Some(conn) => {
                let mut c = conn.clone();
                c.name = connection.name.clone();
                c.workspace_id = connection.workspace_id;
                c.sync_enabled = sync_enabled;
                c.params = serde_json::to_string(&connection)
                    .map_err(|e| format!("{}: {}", t!("ConnectionForm.serialize_failed"), e))?;
                c
            }
            None => {
                let mut c = StoredConnection::from_db_connection(connection);
                c.sync_enabled = sync_enabled;
                c
            }
        };

        stored.remark = remark;
        Ok((stored, is_update))
    }

    pub fn set_save_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.test_result.update(cx, |result, cx| {
            *result = Some(Err(error));
            cx.notify();
        });
    }

    pub fn trigger_cancel(&mut self, _cx: &mut Context<Self>) {
        self.editing_connection = None;
    }

    pub fn is_testing(&self, cx: &App) -> bool {
        *self.is_testing.read(cx)
    }

    pub fn set_test_result(&mut self, result: Result<bool, String>, cx: &mut Context<Self>) {
        self.is_testing.update(cx, |testing, cx| {
            *testing = false;
            cx.notify();
        });
        self.test_result.update(cx, |test_result, cx| {
            *test_result = Some(result);
            cx.notify();
        });
    }

    pub fn save_connection(&mut self, cx: &mut Context<Self>) {
        let (stored, is_update) = match self.build_stored_connection(cx) {
            Ok(data) => data,
            Err(e) => {
                self.set_save_error(e.clone(), cx);
                cx.emit(DbConnectionFormEvent::SaveError(e));
                return;
            }
        };

        let storage = cx.global::<GlobalStorageState>().storage.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let repo_op = storage.get::<ConnectionRepository>();
            if let Some(repo) = repo_op {
                let mut stored = stored;
                if is_update {
                    let re = repo.update(&stored);
                    match re {
                        Ok(..) => {
                            let _ = this.update(cx, |form, cx| {
                                form.editing_connection = None;
                                cx.emit(DbConnectionFormEvent::Saved(stored));
                            });
                        }
                        Err(e) => {
                            let error_msg = format!("{}: {}", t!("ConnectionForm.save_failed"), e);
                            let _ = this.update(cx, |form, cx| {
                                form.set_save_error(error_msg.clone(), cx);
                                cx.emit(DbConnectionFormEvent::SaveError(error_msg));
                            });
                        }
                    }
                } else {
                    let re = repo.insert(&mut stored);
                    match re {
                        Ok(id) => {
                            let _ = this.update(cx, |form, cx| {
                                form.editing_connection = None;
                                stored.id = Some(id);
                                cx.emit(DbConnectionFormEvent::Saved(stored));
                            });
                        }
                        Err(e) => {
                            let error_msg = format!("{}: {}", t!("ConnectionForm.save_failed"), e);
                            let _ = this.update(cx, |form, cx| {
                                form.set_save_error(error_msg.clone(), cx);
                                cx.emit(DbConnectionFormEvent::SaveError(error_msg));
                            });
                        }
                    }
                }
            }
        })
        .detach();
    }

    fn browse_file_path(&mut self, _window: &mut Window, cx: &mut App) {
        let pending = self.pending_file_path.clone();

        let future = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            multiple: false,
            directories: false,
            prompt: Some(t!("ConnectionForm.select_database_file").into()),
        });

        cx.spawn(async move |cx| {
            if let Ok(Ok(Some(paths))) = future.await {
                if let Some(path) = paths.first() {
                    let path_str = path.to_string_lossy().to_string();
                    let _ = cx.update(|cx| {
                        pending.update(cx, |p, cx| {
                            *p = Some(path_str);
                            cx.notify();
                        });
                    });
                }
            }
        })
        .detach();
    }

    fn get_input_by_name(&self, field_name: &str) -> Option<Entity<InputState>> {
        let mut idx = 0;
        for tab_group in &self.config.tab_groups {
            for field in &tab_group.fields {
                if field.name == field_name {
                    return self.field_inputs.get(idx).and_then(|opt| opt.clone());
                }
                idx += 1;
            }
        }
        None
    }
}

impl EventEmitter<DbConnectionFormEvent> for DbConnectionForm {}

impl Focusable for DbConnectionForm {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DbConnectionForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Check if there's a pending file path to apply
        if let Some(path) = self.pending_file_path.read(cx).clone() {
            if let Some(host_input) = self.get_input_by_name("host") {
                host_input.update(cx, |state, cx| {
                    state.set_value(path, window, cx);
                });
            }
            self.pending_file_path.update(cx, |p, _| *p = None);
        }

        let test_result_msg = self.test_result.read(cx).as_ref().map(|r| match r {
            Ok(true) => format!("✓ {}", t!("ConnectionForm.test_success")),
            Ok(false) => format!("✗ {}", t!("ConnectionForm.connection_failed")),
            Err(e) => format!("✗ {}", e),
        });

        // Calculate field input indices for current tab
        let mut field_input_offset = 0;
        for (tab_idx, tab_group) in self.config.tab_groups.iter().enumerate() {
            if tab_idx < self.active_tab {
                field_input_offset += tab_group.fields.len();
            }
        }

        let current_tab_fields = &self.config.tab_groups[self.active_tab].fields;

        v_flex()
            .gap_4()
            .size_full()
            .child(
                // Tab bar
                div().flex().justify_center().child(
                    TabBar::new("connection-tabs")
                        .with_size(Size::Large)
                        .underline()
                        .selected_index(self.active_tab)
                        .on_click(cx.listener(|this, ix: &usize, _window, cx| {
                            this.active_tab = *ix;
                            cx.notify();
                        }))
                        .children(
                            self.config
                                .tab_groups
                                .iter()
                                .map(|tab| Tab::new().label(tab.label.clone())),
                        ),
                ),
            )
            .child(
                // Form fields for active tab
                div()
                    .flex_1()
                    .min_h(px(250.))
                    .overflow_y_scrollbar()
                    .when(!current_tab_fields.is_empty(), |this| {
                        let is_general_tab = self.active_tab == 0;
                        let db_type = self.config.db_type;

                        this.child(
                            v_form()
                                .layout(Axis::Horizontal)
                                .with_size(Size::Medium)
                                .columns(1)
                                .label_width(px(100.))
                                .children(current_tab_fields.iter().enumerate().map(
                                    |(i, field_info)| {
                                        let input_idx = field_input_offset + i;
                                        let is_sqlite_path = db_type == DatabaseType::SQLite
                                            && field_info.name == "host";
                                        let is_textarea =
                                            field_info.field_type == FormFieldType::TextArea;
                                        let is_select =
                                            field_info.field_type == FormFieldType::Select;
                                        let is_password =
                                            field_info.field_type == FormFieldType::Password;
                                        let field_name = field_info.name.clone();

                                        field()
                                            .label(field_info.label.clone())
                                            .required(field_info.required)
                                            .when(!is_textarea, |f| f.items_center())
                                            .when(is_textarea, |f| f.items_start())
                                            .label_justify_end()
                                            .child(
                                                h_flex()
                                                    .w_full()
                                                    .gap_2()
                                                    .when(is_textarea, |el| el.items_start())
                                                    .when(is_select, |el| {
                                                        if let Some(select_state) =
                                                            self.field_selects.get(&field_name)
                                                        {
                                                            el.child(
                                                                Select::new(select_state).w_full(),
                                                            )
                                                        } else {
                                                            el
                                                        }
                                                    })
                                                    .when(!is_select, |el| {
                                                        if let Some(Some(input_state)) =
                                                            self.field_inputs.get(input_idx)
                                                        {
                                                            let input =
                                                                Input::new(input_state).w_full();
                                                            let input = if is_password {
                                                                input.mask_toggle()
                                                            } else {
                                                                input
                                                            };
                                                            el.child(input)
                                                        } else {
                                                            el
                                                        }
                                                    })
                                                    .when(is_sqlite_path, |el| {
                                                        el.child(
                                                            Button::new("browse-file")
                                                                .icon(IconName::FolderOpen)
                                                                .ghost()
                                                                .on_click(cx.listener(
                                                                    |this, _, window, cx| {
                                                                        this.browse_file_path(
                                                                            window, cx,
                                                                        );
                                                                    },
                                                                )),
                                                        )
                                                    }),
                                            )
                                    },
                                ))
                                .when(is_general_tab, |form| {
                                    let sync_enabled = self.sync_enabled.clone();
                                    let is_sync_checked = *self.sync_enabled.read(cx);
                                    let is_checking = *self.oracle_client_checking.read(cx);
                                    let oracle_client_status =
                                        self.oracle_client_status.read(cx).clone();
                                    let oracle_client_guide = self.oracle_client_guide_text(cx);
                                    let oracle_client_download_url =
                                        self.oracle_client_download_url(cx);
                                    form.child(
                                        field()
                                            .label(t!("ConnectionForm.workspace").to_string())
                                            .items_center()
                                            .label_justify_end()
                                            .child(Select::new(&self.workspace_select).w_full()),
                                    )
                                    .child(
                                        field()
                                            .label(t!("ConnectionForm.cloud_sync").to_string())
                                            .items_center()
                                            .label_justify_end()
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .child(
                                                        Checkbox::new("sync-enabled")
                                                            .checked(is_sync_checked)
                                                            .on_click(move |_, _, cx| {
                                                                sync_enabled.update(
                                                                    cx,
                                                                    |sync, cx| {
                                                                        *sync = !*sync;
                                                                        cx.notify();
                                                                    },
                                                                );
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(
                                                                t!(
                                                                    "ConnectionForm.cloud_sync_desc"
                                                                )
                                                                .to_string(),
                                                            ),
                                                    ),
                                            ),
                                    )
                                    .when(db_type == DatabaseType::Oracle, |form| {
                                        let has_error = matches!(
                                            &oracle_client_status,
                                            Some(Err(_))
                                        );
                                        let oracle_client_guide = oracle_client_guide.clone();
                                        let oracle_client_download_url = oracle_client_download_url;

                                        form.child(
                                            field()
                                                .label(
                                                    t!("ConnectionForm.oracle_client_status")
                                                        .to_string(),
                                                )
                                                .items_center()
                                                .label_justify_end()
                                                .child(
                                                    h_flex()
                                                        .w_full()
                                                        .items_center()
                                                        .gap_2()
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .overflow_hidden()
                                                                .text_ellipsis()
                                                                .whitespace_nowrap()
                                                                .flex_shrink()
                                                                .min_w_0()
                                                                .when(is_checking, |div| {
                                                                    div.text_color(
                                                                        cx.theme()
                                                                            .muted_foreground,
                                                                    )
                                                                    .child(
                                                                        t!("ConnectionForm.oracle_client_checking")
                                                                            .to_string(),
                                                                    )
                                                                })
                                                                .when(!is_checking, |div| {
                                                                    match &oracle_client_status {
                                                                        Some(Ok(version)) => div
                                                                            .text_color(gpui::rgb(0x166534))
                                                                            .child(
                                                                                t!(
                                                                                    "ConnectionForm.oracle_client_available",
                                                                                    version = version
                                                                                )
                                                                                .to_string(),
                                                                            ),
                                                                        Some(Err(error)) => div
                                                                            .text_color(gpui::rgb(0x991b1b))
                                                                            .child(
                                                                                t!(
                                                                                    "ConnectionForm.oracle_client_unavailable",
                                                                                    error = error
                                                                                )
                                                                                .to_string(),
                                                                            ),
                                                                        None => div
                                                                            .text_color(
                                                                                cx.theme()
                                                                                    .muted_foreground,
                                                                            )
                                                                            .child("-"),
                                                                    }
                                                                }),
                                                        )
                                                        .child(
                                                            div().flex_shrink_0().child(
                                                                Button::new(
                                                                    "oracle-client-status-refresh",
                                                                )
                                                                .small()
                                                                .ghost()
                                                                .icon(IconName::Refresh)
                                                                .disabled(is_checking)
                                                                .on_click(cx.listener(
                                                                    |this, _, _window, cx| {
                                                                        this.refresh_oracle_client_status(cx);
                                                                    },
                                                                )),
                                                            ),
                                                        )
                                                        .when(has_error, |this| {
                                                            let guide = oracle_client_guide.clone();
                                                            let download_url = oracle_client_download_url;
                                                            this.child(
                                                                div().flex_shrink_0().child(
                                                                    Popover::new("oracle-client-guide-popover")
                                                                        .trigger(
                                                                            Button::new("oracle-client-guide-btn")
                                                                                .small()
                                                                                .ghost()
                                                                                .icon(IconName::Info)
                                                                                .label(
                                                                                    t!("ConnectionForm.oracle_client_guide_label")
                                                                                        .to_string(),
                                                                                ),
                                                                        )
                                                                        .content(move |_state, _window, cx| {
                                                                            v_flex()
                                                                                .gap_2()
                                                                                .max_w(px(360.))
                                                                                .child(
                                                                                    h_flex()
                                                                                        .items_center()
                                                                                        .gap_1()
                                                                                        .child(
                                                                                            Icon::new(IconName::Info)
                                                                                                .with_size(Size::Small)
                                                                                                .text_color(cx.theme().muted_foreground),
                                                                                        )
                                                                                        .child(
                                                                                            div()
                                                                                                .text_sm()
                                                                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                                                                .child(
                                                                                                    t!("ConnectionForm.oracle_client_guide_title")
                                                                                                        .to_string(),
                                                                                                ),
                                                                                        ),
                                                                                )
                                                                                .when_some(guide.clone(), |this, guide| {
                                                                                    this.child(
                                                                                        div()
                                                                                            .text_sm()
                                                                                            .text_color(
                                                                                                cx.theme()
                                                                                                    .muted_foreground,
                                                                                            )
                                                                                            .child(guide),
                                                                                    )
                                                                                })
                                                                                .when_some(download_url, |this, url| {
                                                                                    this.child(
                                                                                        h_flex()
                                                                                            .w_full()
                                                                                            .justify_end()
                                                                                            .gap_2()
                                                                                            .child(
                                                                                                Clipboard::new(
                                                                                                    "oracle-client-copy-url",
                                                                                                )
                                                                                                .value(SharedString::from(url)),
                                                                                            )
                                                                                            .child(
                                                                                                Button::new(
                                                                                                    "oracle-client-download-page",
                                                                                                )
                                                                                                .small()
                                                                                                .outline()
                                                                                                .label(
                                                                                                    t!(
                                                                                                        "ConnectionForm.oracle_client_open_download"
                                                                                                    )
                                                                                                    .to_string(),
                                                                                                )
                                                                                                .on_click(
                                                                                                    move |_, _window, cx| {
                                                                                                        cx.open_url(url);
                                                                                                    },
                                                                                                ),
                                                                                            ),
                                                                                    )
                                                                                })
                                                                        }),
                                                                ),
                                                            )
                                                        }),
                                                ),
                                        )
                                    })
                                }),
                        )
                    })
                    .when(current_tab_fields.is_empty(), |this| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .h_full()
                                .text_color(cx.theme().muted_foreground)
                                .child(t!("SqlEditor.no_settings").to_string()),
                        )
                    }),
            )
            .child(
                // Test result message area
                div()
                    .h(px(40.))
                    .flex()
                    .items_center()
                    .when_some(test_result_msg, |this, msg| {
                        let is_success = msg.starts_with("✓");
                        this.child(
                            div()
                                .w_full()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .bg(if is_success {
                                    gpui::rgb(0xdcfce7)
                                } else {
                                    gpui::rgb(0xfee2e2)
                                })
                                .text_color(if is_success {
                                    gpui::rgb(0x166534)
                                } else {
                                    gpui::rgb(0x991b1b)
                                })
                                .child(msg),
                        )
                    }),
            )
    }
}
