use crate::{ActiveTheme, Sizable, Size};
use gpui::{
    AnyElement, App, AppContext, Context, Entity, Hsla, IntoElement, ParentElement, Radians,
    Render, RenderOnce, SharedString, StyleRefinement, Styled, Svg, Transformation, Window, div,
    img, prelude::FluentBuilder as _, svg,
};

/// Types implementing this trait can automatically be converted to [`Icon`].
///
/// This allows you to implement a custom version of [`IconName`] that functions as a drop-in
/// replacement for other UI components.
pub trait IconNamed {
    /// Returns the embedded path of the icon.
    fn path(self) -> SharedString;
}

impl<T: IconNamed> From<T> for Icon {
    fn from(value: T) -> Self {
        Icon::build(value)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IconColorMode {
    /// Monochrome mode: uses SVG with text_color tinting (default)
    #[default]
    Mono,
    /// Color mode: renders the original SVG/image colors
    Color,
}

/// The name of an icon in the asset bundle.
#[derive(IntoElement, Clone)]
pub enum IconName {
    ALargeSmall,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Asterisk,
    Battery,
    BatteryCharging,
    BatteryFull,
    BatteryLow,
    BatteryMedium,
    BatteryWarning,
    Bell,
    BookOpen,
    Bot,
    Building2,
    Calendar,
    CaseSensitive,
    ChartPie,
    Check,
    ChevronDown,
    ChevronLeft,
    ChevronRight,
    ChevronsUpDown,
    ChevronUp,
    CircleCheck,
    CircleUser,
    CircleX,
    Close,
    Copy,
    Paste,
    Cpu,
    Dash,
    Delete,
    Ellipsis,
    EllipsisVertical,
    ExternalLink,
    Eye,
    EyeOff,
    File,
    Folder,
    FolderClosed,
    FolderOpen,
    Frame,
    GalleryVerticalEnd,
    GitHub,
    Globe,
    HardDrive,
    Heart,
    HeartOff,
    Inbox,
    Info,
    Inspector,
    LayoutDashboard,
    Loader,
    LoaderCircle,
    Map,
    Maximize,
    MemoryStick,
    Menu,
    Minimize,
    Minus,
    Moon,
    Network,
    Palette,
    PanelBottom,
    PanelBottomOpen,
    PanelLeft,
    PanelLeftClose,
    PanelLeftOpen,
    PanelRight,
    PanelRightClose,
    PanelRightOpen,
    Pause,
    Play,
    Plus,
    Redo,
    Redo2,
    Replace,
    ResizeCorner,
    Search,
    Settings,
    Settings2,
    SortAscending,
    SortDescending,
    SquareTerminal,
    Star,
    StarFill,
    StarOff,
    Sun,
    ThumbsDown,
    ThumbsUp,
    TriangleAlert,
    Undo,
    Undo2,
    User,
    WindowClose,
    WindowMaximize,
    WindowMinimize,
    WindowRestore,
    Database,
    Table,
    Column,
    Key,
    View,
    Function,
    Schema,
    GoldKey,
    PrimaryKey,
    Procedure,
    Trigger,
    FolderViews,
    FolderQueries,
    FolderFunctions,
    FolderIndexes,
    FolderTables,
    FolderSchema,
    FolderColumns,
    FolderTriggers,
    FolderProcedures,
    FolderForeignKeys,
    FolderCheckConstraints,
    FolderSequences,
    CheckConstraint,
    Sequence,
    Query,
    Index,
    Redis,
    Terminal,
    TerminalColor,
    Apps,
    AppsColor,
    MongoDB,
    MySQLColor,
    MySQLLineColor,
    SQLiteColor,
    SQLiteLineColor,
    PostgreSQLColor,
    PostgreSQLLineColor,
    MSSQLColor,
    MSSQLLineColor,
    OracleColor,
    OracleLineColor,
    ClickHouseColor,
    ClickHouseLineColor,
    Workspace,
    RedisColor,
    All,
    Edit,
    Filter,
    Refresh,
    Sync,
    Upload,
    NewFolder,
    EditBorder,
    Folder1,
    FolderOpen1,
    Remove,
    TableData,
    TableDesign,
    Server,
    Export,
    AI,
    Home,
    SettingColor,
    SerialPort,
    Monitor,
    DuckDB,
}

impl IconName {
    /// Return the icon as a Entity<Icon>
    pub fn view(self, cx: &mut App) -> Entity<Icon> {
        Icon::build(self).view(cx)
    }

    /// Return the icon in color mode (renders original colors)
    pub fn color(self) -> Icon {
        Icon::build(self).color()
    }

    pub fn mono(self) -> Icon {
        Icon::build(self).mono()
    }
}

impl IconNamed for IconName {
    fn path(self) -> SharedString {
        match self {
            Self::ALargeSmall => "icons/a-large-small.svg",
            Self::ArrowDown => "icons/arrow-down.svg",
            Self::ArrowLeft => "icons/arrow-left.svg",
            Self::ArrowRight => "icons/arrow-right.svg",
            Self::ArrowUp => "icons/arrow-up.svg",
            Self::Asterisk => "icons/asterisk.svg",
            Self::Battery => "icons/battery.svg",
            Self::BatteryCharging => "icons/battery-charging.svg",
            Self::BatteryFull => "icons/battery-full.svg",
            Self::BatteryLow => "icons/battery-low.svg",
            Self::BatteryMedium => "icons/battery-medium.svg",
            Self::BatteryWarning => "icons/battery-warning.svg",
            Self::Bell => "icons/bell.svg",
            Self::BookOpen => "icons/book-open.svg",
            Self::Bot => "icons/bot.svg",
            Self::Building2 => "icons/building-2.svg",
            Self::Calendar => "icons/calendar.svg",
            Self::CaseSensitive => "icons/case-sensitive.svg",
            Self::ChartPie => "icons/chart-pie.svg",
            Self::Check => "icons/check.svg",
            Self::ChevronDown => "icons/chevron-down.svg",
            Self::ChevronLeft => "icons/chevron-left.svg",
            Self::ChevronRight => "icons/chevron-right.svg",
            Self::ChevronsUpDown => "icons/chevrons-up-down.svg",
            Self::ChevronUp => "icons/chevron-up.svg",
            Self::CircleCheck => "icons/circle-check.svg",
            Self::CircleUser => "icons/circle-user.svg",
            Self::CircleX => "icons/circle-x.svg",
            Self::Close => "icons/close.svg",
            Self::Copy => "icons/copy.svg",
            Self::Paste => "icons/paste.svg",
            Self::Cpu => "icons/cpu.svg",
            Self::Dash => "icons/dash.svg",
            Self::Delete => "icons/delete.svg",
            Self::Ellipsis => "icons/ellipsis.svg",
            Self::EllipsisVertical => "icons/ellipsis-vertical.svg",
            Self::ExternalLink => "icons/external-link.svg",
            Self::Eye => "icons/eye.svg",
            Self::EyeOff => "icons/eye-off.svg",
            Self::File => "icons/file.svg",
            Self::Folder => "icons/folder.svg",
            Self::FolderClosed => "icons/folder-closed.svg",
            Self::FolderOpen => "icons/folder-open.svg",
            Self::Frame => "icons/frame.svg",
            Self::GalleryVerticalEnd => "icons/gallery-vertical-end.svg",
            Self::GitHub => "icons/github.svg",
            Self::Globe => "icons/globe.svg",
            Self::HardDrive => "icons/hard-drive.svg",
            Self::Heart => "icons/heart.svg",
            Self::HeartOff => "icons/heart-off.svg",
            Self::Inbox => "icons/inbox.svg",
            Self::Info => "icons/info.svg",
            Self::Inspector => "icons/inspector.svg",
            Self::LayoutDashboard => "icons/layout-dashboard.svg",
            Self::Loader => "icons/loader.svg",
            Self::LoaderCircle => "icons/loader-circle.svg",
            Self::Map => "icons/map.svg",
            Self::Maximize => "icons/maximize.svg",
            Self::MemoryStick => "icons/memory-stick.svg",
            Self::Menu => "icons/menu.svg",
            Self::Minimize => "icons/minimize.svg",
            Self::Minus => "icons/minus.svg",
            Self::Moon => "icons/moon.svg",
            Self::Network => "icons/network.svg",
            Self::Palette => "icons/palette.svg",
            Self::PanelBottom => "icons/panel-bottom.svg",
            Self::PanelBottomOpen => "icons/panel-bottom-open.svg",
            Self::PanelLeft => "icons/panel-left.svg",
            Self::PanelLeftClose => "icons/panel-left-close.svg",
            Self::PanelLeftOpen => "icons/panel-left-open.svg",
            Self::PanelRight => "icons/panel-right.svg",
            Self::PanelRightClose => "icons/panel-right-close.svg",
            Self::PanelRightOpen => "icons/panel-right-open.svg",
            Self::Pause => "icons/pause.svg",
            Self::Play => "icons/play.svg",
            Self::Plus => "icons/plus.svg",
            Self::Redo => "icons/redo.svg",
            Self::Redo2 => "icons/redo-2.svg",
            Self::Replace => "icons/replace.svg",
            Self::ResizeCorner => "icons/resize-corner.svg",
            Self::Search => "icons/search.svg",
            Self::Settings => "icons/settings.svg",
            Self::Settings2 => "icons/settings-2.svg",
            Self::SortAscending => "icons/sort-ascending.svg",
            Self::SortDescending => "icons/sort-descending.svg",
            Self::SquareTerminal => "icons/square-terminal.svg",
            Self::Star => "icons/star.svg",
            Self::StarFill => "icons/star-fill.svg",
            Self::StarOff => "icons/star-off.svg",
            Self::Sun => "icons/sun.svg",
            Self::ThumbsDown => "icons/thumbs-down.svg",
            Self::ThumbsUp => "icons/thumbs-up.svg",
            Self::TriangleAlert => "icons/triangle-alert.svg",
            Self::Undo => "icons/undo.svg",
            Self::Undo2 => "icons/undo-2.svg",
            Self::User => "icons/user.svg",
            Self::WindowClose => "icons/window-close.svg",
            Self::WindowMaximize => "icons/window-maximize.svg",
            Self::WindowMinimize => "icons/window-minimize.svg",
            Self::WindowRestore => "icons/window-restore.svg",
            Self::Database => "icons/db.svg",
            Self::Schema => "icons/schema.svg",
            Self::Table => "icons/table.svg",
            Self::Folder1 => "icons/folder-1.svg",
            Self::FolderOpen1 => "icons/folder-open-1.svg",
            Self::View => "icons/view.svg",
            Self::Function => "icons/function.svg",
            Self::Column => "icons/column.svg",
            Self::Key => "icons/key.svg",
            Self::GoldKey => "icons/gold_key.svg",
            Self::PrimaryKey => "icons/primary-key.svg",
            Self::Procedure => "icons/procedure.svg",
            Self::Trigger => "icons/trigger.svg",
            Self::FolderViews => "icons/folder-views.svg",
            Self::FolderQueries => "icons/folder-queries.svg",
            Self::FolderFunctions => "icons/folder-functions.svg",
            Self::FolderIndexes => "icons/folder-indexes.svg",
            Self::FolderTables => "icons/folder-tables.svg",
            Self::FolderSchema => "icons/folder-schema.svg",
            Self::FolderColumns => "icons/folder-columns.svg",
            Self::FolderTriggers => "icons/folder-triggers.svg",
            Self::FolderProcedures => "icons/folder-procedures.svg",
            Self::FolderForeignKeys => "icons/folder-foreign-keys.svg",
            Self::FolderCheckConstraints => "icons/folder-check-constraints.svg",
            Self::FolderSequences => "icons/folder-sequences.svg",
            Self::CheckConstraint => "icons/check-constraint.svg",
            Self::Sequence => "icons/sequence.svg",
            Self::Query => "icons/query.svg",
            Self::Index => "icons/index.svg",
            Self::Redis => "icons/redis.svg",
            Self::Terminal => "icons/terminal.svg",
            Self::TerminalColor => "icons/terminal_color.svg",
            Self::Apps => "icons/apps.svg",
            Self::AppsColor => "icons/apps_color.svg",
            Self::MongoDB => "icons/mongodb.svg",
            Self::MySQLColor => "icons/mysql_color.svg",
            Self::SQLiteColor => "icons/sqlite_color.svg",
            Self::PostgreSQLColor => "icons/postgresql_color.svg",
            Self::PostgreSQLLineColor => "icons/postgresql_line_color.svg",
            Self::MSSQLColor => "icons/mssql_color.svg",
            Self::MySQLLineColor => "icons/mysql_line_color.svg",
            Self::SQLiteLineColor => "icons/sqlite_line_color.svg",
            Self::OracleColor => "icons/oracle_color.svg",
            Self::Workspace => "icons/workspace.svg",
            Self::RedisColor => "icons/redis_color.svg",
            Self::All => "icons/all.svg",
            Self::Edit => "icons/edit.svg",
            Self::Filter => "icons/filter.svg",
            Self::Refresh => "icons/refresh.svg",
            Self::Sync => "icons/sync.svg",
            Self::Upload => "icons/upload.svg",
            Self::NewFolder => "icons/new_folder.svg",
            Self::EditBorder => "icons/edit_border.svg",
            Self::MSSQLLineColor => "icons/mssql_line_color.svg",
            Self::OracleLineColor => "icons/oracle_line_color.svg",
            Self::ClickHouseColor => "icons/clickhouse_color.svg",
            Self::ClickHouseLineColor => "icons/clickhouse_line_color.svg",
            Self::Remove => "icons/remove.svg",
            Self::TableData => "icons/table-data.svg",
            Self::TableDesign => "icons/table-design.svg",
            Self::Server => "icons/server.svg",
            Self::Export => "icons/export.svg",
            Self::AI => "icons/ai.svg",
            Self::Home => "icons/home.svg",
            Self::SettingColor => "icons/setting_color.svg",
            Self::SerialPort => "icons/serial_port.svg",
            Self::Monitor => "icons/monitor.svg",
            Self::DuckDB => "icons/duckdb.svg",
        }
        .into()
    }
}

impl From<IconName> for AnyElement {
    fn from(val: IconName) -> Self {
        Icon::build(val).into_any_element()
    }
}

impl RenderOnce for IconName {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        Icon::build(self)
    }
}

#[derive(IntoElement)]
pub struct Icon {
    base: Svg,
    style: StyleRefinement,
    path: SharedString,
    text_color: Option<Hsla>,
    size: Option<Size>,
    rotation: Option<Radians>,
    color_mode: IconColorMode,
}

impl Default for Icon {
    fn default() -> Self {
        Self {
            base: svg().flex_none().size_4(),
            style: StyleRefinement::default(),
            path: "".into(),
            text_color: None,
            size: None,
            rotation: None,
            color_mode: IconColorMode::default(),
        }
    }
}

impl Clone for Icon {
    fn clone(&self) -> Self {
        let mut this = Self::default().path(self.path.clone());
        this.style = self.style.clone();
        this.rotation = self.rotation;
        this.size = self.size;
        this.text_color = self.text_color;
        this.color_mode = self.color_mode;
        this
    }
}

impl Icon {
    pub fn new(icon: impl Into<Icon>) -> Self {
        icon.into()
    }

    fn build(name: impl IconNamed) -> Self {
        Self::default().path(name.path())
    }

    /// Set the icon path of the Assets bundle
    ///
    /// For example: `icons/foo.svg`
    pub fn path(mut self, path: impl Into<SharedString>) -> Self {
        self.path = path.into();
        self
    }

    /// Create a new view for the icon
    pub fn view(self, cx: &mut App) -> Entity<Icon> {
        cx.new(|_| self)
    }

    pub fn transform(mut self, transformation: Transformation) -> Self {
        self.base = self.base.with_transformation(transformation);
        self
    }

    pub fn empty() -> Self {
        Self::default()
    }

    /// Rotate the icon by the given angle
    pub fn rotate(mut self, radians: impl Into<Radians>) -> Self {
        self.base = self
            .base
            .with_transformation(Transformation::rotate(radians));
        self
    }

    /// Set the icon color mode
    pub fn color_mode(mut self, mode: IconColorMode) -> Self {
        self.color_mode = mode;
        self
    }

    /// Set the icon to color mode (renders original colors)
    pub fn color(mut self) -> Self {
        self.color_mode = IconColorMode::Color;
        self
    }

    /// Set the icon to mono mode (uses text_color tinting)
    pub fn mono(mut self) -> Self {
        self.color_mode = IconColorMode::Mono;
        self
    }
}

impl Styled for Icon {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }

    fn text_color(mut self, color: impl Into<Hsla>) -> Self {
        self.text_color = Some(color.into());
        self
    }
}

impl Sizable for Icon {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let text_size = window.text_style().font_size.to_pixels(window.rem_size());
        let has_base_size = self.style.size.width.is_some() || self.style.size.height.is_some();

        match self.color_mode {
            IconColorMode::Mono => {
                // Monochrome mode: use SVG with text_color tinting
                let text_color = self.text_color.unwrap_or_else(|| window.text_style().color);

                let mut base = self.base;
                *base.style() = self.style;

                base.flex_shrink_0()
                    .text_color(text_color)
                    .when(!has_base_size, |this| this.size(text_size))
                    .when_some(self.size, |this, size| match size {
                        Size::Size(px) => this.size(px),
                        Size::XSmall => this.size_3(),
                        Size::Small => this.size_3p5(),
                        Size::Medium => this.size_4(),
                        Size::Large => this.size_6(),
                    })
                    .path(self.path)
                    .into_any_element()
            }
            IconColorMode::Color => {
                // Color mode: use img to render original colors
                let size = self.size.unwrap_or(Size::Medium);
                let (w, h) = match size {
                    Size::Size(px) => (px, px),
                    Size::XSmall => (gpui::px(12.), gpui::px(12.)),
                    Size::Small => (gpui::px(14.), gpui::px(14.)),
                    Size::Medium => (gpui::px(16.), gpui::px(16.)),
                    Size::Large => (gpui::px(24.), gpui::px(24.)),
                };

                div()
                    .flex_shrink_0()
                    .w(w)
                    .h(h)
                    .child(img(self.path.clone()).size_full())
                    .into_any_element()
            }
        }
    }
}

impl From<Icon> for AnyElement {
    fn from(val: Icon) -> Self {
        val.into_any_element()
    }
}

impl Render for Icon {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text_size = window.text_style().font_size.to_pixels(window.rem_size());
        let has_base_size = self.style.size.width.is_some() || self.style.size.height.is_some();

        match self.color_mode {
            IconColorMode::Mono => {
                // Monochrome mode: use SVG with text_color tinting
                let text_color = self.text_color.unwrap_or_else(|| cx.theme().foreground);

                let mut base = svg().flex_none();
                *base.style() = self.style.clone();

                base.flex_shrink_0()
                    .text_color(text_color)
                    .when(!has_base_size, |this| this.size(text_size))
                    .when_some(self.size, |this, size| match size {
                        Size::Size(px) => this.size(px),
                        Size::XSmall => this.size_3(),
                        Size::Small => this.size_3p5(),
                        Size::Medium => this.size_4(),
                        Size::Large => this.size_6(),
                    })
                    .path(self.path.clone())
                    .when_some(self.rotation, |this, rotation| {
                        this.with_transformation(Transformation::rotate(rotation))
                    })
                    .into_any_element()
            }
            IconColorMode::Color => {
                // Color mode: use img to render original colors
                let size = self.size.unwrap_or(Size::Medium);
                let (w, h) = match size {
                    Size::Size(px) => (px, px),
                    Size::XSmall => (gpui::px(12.), gpui::px(12.)),
                    Size::Small => (gpui::px(14.), gpui::px(14.)),
                    Size::Medium => (gpui::px(16.), gpui::px(16.)),
                    Size::Large => (gpui::px(24.), gpui::px(24.)),
                };

                div()
                    .flex_shrink_0()
                    .w(w)
                    .h(h)
                    .child(img(self.path.clone()).size_full())
                    .into_any_element()
            }
        }
    }
}
