use crate::{ActiveTheme, Sizable, Size};
use gpui::{
    prelude::FluentBuilder as _, svg, AnyElement, App, AppContext, Context, Entity, Hsla,
    IntoElement, Radians, Render, RenderOnce, SharedString, StyleRefinement, Styled, Svg,
    Transformation, Window,
};

#[derive(IntoElement, Clone)]
pub enum IconName {
    ALargeSmall,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Asterisk,
    Bell,
    BookOpen,
    Bot,
    Calendar,
    ChartPie,
    Check,
    ChevronDown,
    ChevronLeft,
    ChevronRight,
    ChevronUp,
    ChevronsUpDown,
    CircleCheck,
    CircleUser,
    CircleX,
    Close,
    Copy,
    Dash,
    Delete,
    Ellipsis,
    EllipsisVertical,
    ExternalLink,
    Eye,
    EyeOff,
    Frame,
    GalleryVerticalEnd,
    GitHub,
    Globe,
    Heart,
    HeartOff,
    Inbox,
    Info,
    LayoutDashboard,
    Loader,
    LoaderCircle,
    Map,
    Maximize,
    Menu,
    Minimize,
    Minus,
    Moon,
    Palette,
    PanelBottom,
    PanelBottomOpen,
    PanelLeft,
    PanelLeftClose,
    PanelLeftOpen,
    PanelRight,
    PanelRightClose,
    PanelRightOpen,
    Plus,
    ResizeCorner,
    Search,
    Settings,
    Settings2,
    SortAscending,
    SortDescending,
    SquareTerminal,
    Star,
    StarOff,
    Sun,
    ThumbsDown,
    ThumbsUp,
    TriangleAlert,
    WindowClose,
    WindowMaximize,
    WindowMinimize,
    WindowRestore,
}

impl IconName {
    pub fn path(self) -> SharedString {
        match self {
            Self::ALargeSmall => "icons/a-large-small.svg",
            Self::ArrowDown => "icons/arrow-down.svg",
            Self::ArrowLeft => "icons/arrow-left.svg",
            Self::ArrowRight => "icons/arrow-right.svg",
            Self::ArrowUp => "icons/arrow-up.svg",
            Self::Asterisk => "icons/asterisk.svg",
            Self::Bell => "icons/bell.svg",
            Self::BookOpen => "icons/book-open.svg",
            Self::Bot => "icons/bot.svg",
            Self::Calendar => "icons/calendar.svg",
            Self::ChartPie => "icons/chart-pie.svg",
            Self::Check => "icons/check.svg",
            Self::ChevronDown => "icons/chevron-down.svg",
            Self::ChevronLeft => "icons/chevron-left.svg",
            Self::ChevronRight => "icons/chevron-right.svg",
            Self::ChevronUp => "icons/chevron-up.svg",
            Self::ChevronsUpDown => "icons/chevrons-up-down.svg",
            Self::CircleCheck => "icons/circle-check.svg",
            Self::CircleUser => "icons/circle-user.svg",
            Self::CircleX => "icons/circle-x.svg",
            Self::Close => "icons/close.svg",
            Self::Copy => "icons/copy.svg",
            Self::Dash => "icons/dash.svg",
            Self::Delete => "icons/delete.svg",
            Self::Ellipsis => "icons/ellipsis.svg",
            Self::EllipsisVertical => "icons/ellipsis-vertical.svg",
            Self::ExternalLink => "icons/external-link.svg",
            Self::Eye => "icons/eye.svg",
            Self::EyeOff => "icons/eye-off.svg",
            Self::Frame => "icons/frame.svg",
            Self::GalleryVerticalEnd => "icons/gallery-vertical-end.svg",
            Self::GitHub => "icons/github.svg",
            Self::Globe => "icons/globe.svg",
            Self::Heart => "icons/heart.svg",
            Self::HeartOff => "icons/heart-off.svg",
            Self::Inbox => "icons/inbox.svg",
            Self::Info => "icons/info.svg",
            Self::LayoutDashboard => "icons/layout-dashboard.svg",
            Self::Loader => "icons/loader.svg",
            Self::LoaderCircle => "icons/loader-circle.svg",
            Self::Map => "icons/map.svg",
            Self::Maximize => "icons/maximize.svg",
            Self::Menu => "icons/menu.svg",
            Self::Minimize => "icons/minimize.svg",
            Self::Minus => "icons/minus.svg",
            Self::Moon => "icons/moon.svg",
            Self::Palette => "icons/palette.svg",
            Self::PanelBottom => "icons/panel-bottom.svg",
            Self::PanelBottomOpen => "icons/panel-bottom-open.svg",
            Self::PanelLeft => "icons/panel-left.svg",
            Self::PanelLeftClose => "icons/panel-left-close.svg",
            Self::PanelLeftOpen => "icons/panel-left-open.svg",
            Self::PanelRight => "icons/panel-right.svg",
            Self::PanelRightClose => "icons/panel-right-close.svg",
            Self::PanelRightOpen => "icons/panel-right-open.svg",
            Self::Plus => "icons/plus.svg",
            Self::ResizeCorner => "icons/resize-corner.svg",
            Self::Search => "icons/search.svg",
            Self::Settings => "icons/settings.svg",
            Self::Settings2 => "icons/settings-2.svg",
            Self::SortAscending => "icons/sort-ascending.svg",
            Self::SortDescending => "icons/sort-descending.svg",
            Self::SquareTerminal => "icons/square-terminal.svg",
            Self::Star => "icons/star.svg",
            Self::StarOff => "icons/star-off.svg",
            Self::Sun => "icons/sun.svg",
            Self::ThumbsDown => "icons/thumbs-down.svg",
            Self::ThumbsUp => "icons/thumbs-up.svg",
            Self::TriangleAlert => "icons/triangle-alert.svg",
            Self::WindowClose => "icons/window-close.svg",
            Self::WindowMaximize => "icons/window-maximize.svg",
            Self::WindowMinimize => "icons/window-minimize.svg",
            Self::WindowRestore => "icons/window-restore.svg",
        }
        .into()
    }

    /// Return the icon as a Entity<Icon>
    pub fn view(self, cx: &mut App) -> Entity<Icon> {
        Icon::build(self).view(cx)
    }
}

impl From<IconName> for Icon {
    fn from(val: IconName) -> Self {
        Icon::build(val)
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
        this
    }
}

pub trait IconNamed {
    fn path(&self) -> SharedString;
}

impl Icon {
    pub fn new(icon: impl Into<Icon>) -> Self {
        icon.into()
    }

    fn build(name: IconName) -> Self {
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

    pub fn transform(mut self, transformation: gpui::Transformation) -> Self {
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
        let text_color = self.text_color.unwrap_or_else(|| window.text_style().color);
        let text_size = window.text_style().font_size.to_pixels(window.rem_size());
        let has_base_size = self.style.size.width.is_some() || self.style.size.height.is_some();

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
    }
}

impl From<Icon> for AnyElement {
    fn from(val: Icon) -> Self {
        val.into_any_element()
    }
}

impl Render for Icon {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text_color = self.text_color.unwrap_or_else(|| cx.theme().foreground);
        let text_size = window.text_style().font_size.to_pixels(window.rem_size());
        let has_base_size = self.style.size.width.is_some() || self.style.size.height.is_some();

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
    }
}
