mod accordion_story;
mod alert_story;
pub mod app_title_bar;
mod assets;
mod badge_story;
mod button_story;
mod calendar_story;
mod chart_story;
mod checkbox_story;
mod clipboard_story;
mod color_picker_story;
mod date_picker_story;
mod drawer_story;
mod dropdown_story;
mod form_story;
mod icon_story;
mod image_story;
mod input_story;
mod kbd_story;
mod label_story;
mod list_story;
mod menu_story;
mod modal_story;
mod notification_story;
mod number_input_story;
mod otp_input_story;
mod popover_story;
mod progress_story;
mod radio_story;
mod resizable_story;
mod scrollable_story;
mod sidebar_story;
mod slider_story;
mod switch_story;
mod table_story;
mod tabs_story;
mod tag_story;
mod textarea_story;
mod title_bar;
mod toggle_story;
mod tooltip_story;
mod webview_story;
mod welcome_story;

pub use assets::Assets;
use gpui::{
    actions, div, prelude::FluentBuilder as _, px, rems, size, Action, AnyElement, AnyView, App,
    AppContext, Bounds, Context, Div, Entity, EventEmitter, Focusable, Global, Hsla,
    InteractiveElement, IntoElement, KeyBinding, Menu, MenuItem, ParentElement, Render, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, WindowBounds, WindowKind,
    WindowOptions,
};

pub use accordion_story::AccordionStory;
pub use alert_story::AlertStory;
pub use badge_story::BadgeStory;
pub use button_story::ButtonStory;
pub use calendar_story::CalendarStory;
pub use chart_story::ChartStory;
pub use checkbox_story::CheckboxStory;
pub use clipboard_story::ClipboardStory;
pub use color_picker_story::ColorPickerStory;
pub use date_picker_story::DatePickerStory;
pub use drawer_story::DrawerStory;
pub use dropdown_story::DropdownStory;
pub use form_story::FormStory;
pub use icon_story::IconStory;
pub use image_story::ImageStory;
pub use input_story::InputStory;
pub use kbd_story::KbdStory;
pub use label_story::LabelStory;
pub use list_story::ListStory;
pub use menu_story::MenuStory;
pub use modal_story::ModalStory;
pub use notification_story::NotificationStory;
pub use number_input_story::NumberInputStory;
pub use otp_input_story::OtpInputStory;
pub use popover_story::PopoverStory;
pub use progress_story::ProgressStory;
pub use radio_story::RadioStory;
pub use resizable_story::ResizableStory;
pub use scrollable_story::ScrollableStory;
use serde::{Deserialize, Serialize};
pub use sidebar_story::SidebarStory;
pub use slider_story::SliderStory;
pub use switch_story::SwitchStory;
pub use table_story::TableStory;
pub use tabs_story::TabsStory;
pub use tag_story::TagStory;
pub use textarea_story::TextareaStory;
pub use title_bar::AppTitleBar;
pub use toggle_story::ToggleStory;
pub use tooltip_story::TooltipStory;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
pub use webview_story::WebViewStory;
pub use welcome_story::WelcomeStory;

use gpui_component::{
    button::Button,
    context_menu::ContextMenuExt,
    dock::{register_panel, Panel, PanelControl, PanelEvent, PanelInfo, PanelState, TitleStyle},
    h_flex,
    notification::Notification,
    popup_menu::PopupMenu,
    scroll::ScrollbarShow,
    v_flex, ActiveTheme, ContextModal, IconName, Root, TitleBar,
};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectScrollbarShow(ScrollbarShow);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectLocale(SharedString);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectFont(usize);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectRadius(usize);

actions!(story, [Quit, Open, CloseWindow, ToggleSearch]);

/// 面板名称常量
const PANEL_NAME: &str = "StoryContainer";

/// 应用程序状态，管理全局状态
pub struct AppState {
    /// 不可见面板的列表
    pub invisible_panels: Entity<Vec<SharedString>>,
}

impl AppState {
    /// 初始化应用程序状态
    fn init(cx: &mut App) {
        let state = Self {
            invisible_panels: cx.new(|_| Vec::new()),
        };
        cx.set_global::<AppState>(state);
    }

    /// 获取全局应用程序状态的不可变引用
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// 获取全局应用程序状态的可变引用
    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }
}

/// 创建新窗口的通用函数
///
/// # 参数
/// - `title`: 窗口标题
/// - `crate_view_fn`: 创建视图的函数
/// - `cx`: 应用程序上下文
pub fn create_new_window<F, E>(title: &str, crate_view_fn: F, cx: &mut App)
where
    E: Into<AnyView>,
    F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
{
    // 设置默认窗口大小
    let mut window_size = size(px(1600.0), px(1200.0));

    // 根据主显示器大小调整窗口大小
    if let Some(display) = cx.primary_display() {
        let display_size = display.bounds().size;
        window_size.width = window_size.width.min(display_size.width * 0.85);
        window_size.height = window_size.height.min(display_size.height * 0.85);
    }

    // 计算窗口居中位置
    let window_bounds = Bounds::centered(None, window_size, cx);
    let title = SharedString::from(title.to_string());

    // 异步创建窗口
    cx.spawn(async move |cx| {
        // 配置窗口选项
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_min_size: Some(gpui::Size {
                width: px(640.),
                height: px(480.),
            }),
            kind: WindowKind::Normal,
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };

        // 打开新窗口
        let window = cx
            .open_window(options, |window, cx| {
                let view = crate_view_fn(window, cx);
                let root = cx.new(|cx| StoryRoot::new(title.clone(), view, window, cx));

                cx.new(|cx| Root::new(root.into(), window, cx))
            })
            .expect("failed to open window");

        // 激活窗口并设置标题
        window
            .update(cx, |_, window, _| {
                window.activate_window();
                window.set_window_title(&title);
            })
            .expect("failed to update window");

        Ok::<_, anyhow::Error>(())
    })
    .detach();
}

/// 使用自定义选项创建新窗口
///
/// # 参数
/// - `title`: 窗口标题
/// - `options`: 窗口选项配置
/// - `crate_view_fn`: 创建视图的函数
/// - `cx`: 应用程序上下文
pub fn create_new_window_options<F, E>(
    title: &str,
    options: WindowOptions,
    crate_view_fn: F,
    cx: &mut App,
) where
    E: Into<AnyView>,
    F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
{
    let title = SharedString::from(title.to_string());

    cx.spawn(async move |cx| {
        let window = cx
            .open_window(options, |window, cx| {
                let view = crate_view_fn(window, cx);
                let root = cx.new(|cx| StoryRoot::new(title.clone(), view, window, cx));

                cx.new(|cx| Root::new(root.into(), window, cx))
            })
            .expect("failed to open window");

        window
            .update(cx, |_, window, _| {
                window.activate_window();
                window.set_window_title(&title);
            })
            .expect("failed to update window");

        Ok::<_, anyhow::Error>(())
    })
    .detach();
}

/// 故事根组件，包含标题栏和主视图
struct StoryRoot {
    title_bar: Entity<AppTitleBar>, // 应用程序标题栏
    view: AnyView,                  // 主视图
}

impl StoryRoot {
    /// 创建新的故事根组件
    pub fn new(
        title: impl Into<SharedString>,
        view: impl Into<AnyView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_bar = cx.new(|cx| AppTitleBar::new(title, window, cx));
        Self {
            title_bar,
            view: view.into(),
        }
    }
}

impl Render for StoryRoot {
    /// 渲染故事根组件
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 渲染各种覆盖层
        let drawer_layer = Root::render_drawer_layer(window, cx); // 抽屉层
        let modal_layer = Root::render_modal_layer(window, cx); // 模态层
        let notification_layer = Root::render_notification_layer(window, cx); // 通知层

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(self.title_bar.clone()) // 标题栏
                    .child(div().flex_1().overflow_hidden().child(self.view.clone())), // 主视图
            )
            .children(drawer_layer) // 添加抽屉层
            .children(modal_layer) // 添加模态层
            .child(div().absolute().top_8().children(notification_layer)) // 添加通知层
    }
}

// 实现全局状态特征
impl Global for AppState {}

/// 初始化应用程序
pub fn init(cx: &mut App) {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("gpui_component=trace".parse().unwrap()),
        )
        .init();

    gpui_component::init(cx);

    // 初始化应用程序状态
    AppState::init(cx);

    // 初始化各种故事组件
    input_story::init(cx);
    number_input_story::init(cx);
    textarea_story::init(cx);
    dropdown_story::init(cx);
    popover_story::init(cx);
    menu_story::init(cx);
    webview_story::init(cx);
    tooltip_story::init(cx);
    otp_input_story::init(cx);

    // 设置 HTTP 客户端
    let http_client = std::sync::Arc::new(
        reqwest_client::ReqwestClient::user_agent("gpui-component/story").unwrap(),
    );
    cx.set_http_client(http_client);

    // 绑定键盘快捷键
    cx.bind_keys([
        KeyBinding::new("/", ToggleSearch, None), // 斜杠键切换搜索
        KeyBinding::new("cmd-q", Quit, None),     // Cmd+Q 退出
    ]);

    // 处理退出操作
    cx.on_action(|_: &Quit, cx: &mut App| {
        cx.quit();
    });

    // 注册面板
    register_panel(cx, PANEL_NAME, |_, _, info, window, cx| {
        let story_state = match info {
            PanelInfo::Panel(value) => StoryState::from_value(value.clone()),
            _ => {
                unreachable!("Invalid PanelInfo: {:?}", info)
            }
        };

        let view = cx.new(|cx| {
            let (title, description, closable, zoomable, story, on_active) =
                story_state.to_story(window, cx);
            let mut container = StoryContainer::new(window, cx)
                .story(story, story_state.story_klass)
                .on_active(on_active);

            // 监听焦点变化
            cx.on_focus_in(
                &container.focus_handle,
                window,
                |this: &mut StoryContainer, _, _| {
                    println!("StoryContainer focus in: {}", this.name);
                },
            )
            .detach();

            container.name = title.into();
            container.description = description.into();
            container.closable = closable;
            container.zoomable = zoomable;
            container
        });
        Box::new(view)
    });

    // 设置应用程序菜单
    use gpui_component::input::{Copy, Cut, Paste, Redo, Undo};
    cx.set_menus(vec![
        Menu {
            name: "GPUI App".into(),
            items: vec![MenuItem::action("Quit", Quit)],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", Undo, gpui::OsAction::Undo),
                MenuItem::os_action("Redo", Redo, gpui::OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", Cut, gpui::OsAction::Cut),
                MenuItem::os_action("Copy", Copy, gpui::OsAction::Copy),
                MenuItem::os_action("Paste", Paste, gpui::OsAction::Paste),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![],
        },
    ]);
    cx.activate(true);
}

// 定义显示面板信息的操作
actions!(story, [ShowPanelInfo]);

/// 故事章节组件，用于组织和展示相关的故事内容
#[derive(IntoElement)]
struct StorySection {
    base: Div,                 // 基础 div 元素
    title: AnyElement,         // 标题元素
    children: Vec<AnyElement>, // 子元素列表
}

impl StorySection {
    /// 设置最大宽度为中等（48rem）
    #[allow(unused)]
    fn max_w_md(mut self) -> Self {
        self.base = self.base.max_w(rems(48.));
        self
    }

    /// 设置最大宽度为大（64rem）
    #[allow(unused)]
    fn max_w_lg(mut self) -> Self {
        self.base = self.base.max_w(rems(64.));
        self
    }

    /// 设置最大宽度为超大（80rem）
    #[allow(unused)]
    fn max_w_xl(mut self) -> Self {
        self.base = self.base.max_w(rems(80.));
        self
    }

    /// 设置最大宽度为 2 倍超大（96rem）
    #[allow(unused)]
    fn max_w_2xl(mut self) -> Self {
        self.base = self.base.max_w(rems(96.));
        self
    }
}

// 实现父元素特征，允许添加子元素
impl ParentElement for StorySection {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

// 实现样式特征，允许应用样式
impl Styled for StorySection {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for StorySection {
    /// 渲染故事章节
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .gap_2() // 间距 2 单位
            .mb_5() // 底部外边距 5 单位
            .w_full() // 全宽
            .child(
                h_flex()
                    .justify_between() // 两端对齐
                    .w_full() // 全宽
                    .gap_4() // 间距 4 单位
                    .child(self.title), // 标题
            )
            .child(
                v_flex()
                    .p_4() // 内边距 4 单位
                    .overflow_x_hidden() // 隐藏水平溢出
                    .border_1() // 1 像素边框
                    .border_color(cx.theme().border) // 主题边框颜色
                    .rounded_lg() // 大圆角
                    .items_center() // 垂直居中
                    .justify_center() // 水平居中
                    .child(self.base.children(self.children)), // 内容
            )
    }
}

// 实现上下文菜单扩展
impl ContextMenuExt for StorySection {}

/// 创建新的故事章节
pub(crate) fn section(title: impl IntoElement) -> StorySection {
    StorySection {
        title: title.into_any_element(),
        base: h_flex()
            .flex_wrap() // 允许换行
            .justify_center() // 水平居中
            .items_center() // 垂直居中
            .w_full() // 全宽
            .gap_4(), // 间距 4 单位
        children: vec![],
    }
}

/// 故事容器组件，用于包装和展示单个故事
pub struct StoryContainer {
    focus_handle: gpui::FocusHandle,   // 焦点处理句柄
    pub name: SharedString,            // 容器名称
    pub title_bg: Option<Hsla>,        // 标题背景色
    pub description: SharedString,     // 描述
    width: Option<gpui::Pixels>,       // 宽度
    height: Option<gpui::Pixels>,      // 高度
    story: Option<AnyView>,            // 故事视图
    story_klass: Option<SharedString>, // 故事类名
    closable: bool,                    // 是否可关闭
    zoomable: Option<PanelControl>,    // 是否可缩放
    on_active: Option<fn(AnyView, bool, &mut Window, &mut App)>, // 激活回调
}

/// 容器事件枚举
#[derive(Debug)]
pub enum ContainerEvent {
    Close, // 关闭事件
}

/// 故事特征，定义故事组件的基本行为
pub trait Story: Focusable + Render + Sized {
    /// 获取故事类名
    fn klass() -> &'static str {
        std::any::type_name::<Self>().split("::").last().unwrap()
    }

    /// 故事标题
    fn title() -> &'static str;

    /// 故事描述
    fn description() -> &'static str {
        ""
    }

    /// 是否可关闭
    fn closable() -> bool {
        true
    }

    /// 是否可缩放
    fn zoomable() -> Option<PanelControl> {
        Some(PanelControl::default())
    }

    /// 标题背景色
    fn title_bg() -> Option<Hsla> {
        None
    }

    /// 创建新视图
    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable>;

    /// 激活状态改变回调
    fn on_active(&mut self, active: bool, window: &mut Window, cx: &mut App) {
        let _ = active;
        let _ = window;
        let _ = cx;
    }

    /// 任意视图的激活状态改变回调
    fn on_active_any(view: AnyView, active: bool, window: &mut Window, cx: &mut App)
    where
        Self: 'static,
    {
        if let Some(story) = view.downcast::<Self>().ok() {
            cx.update_entity(&story, |story, cx| {
                story.on_active(active, window, cx);
            });
        }
    }
}

// 实现事件发射器
impl EventEmitter<ContainerEvent> for StoryContainer {}

impl StoryContainer {
    /// 创建新的故事容器
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();

        Self {
            focus_handle,
            name: "".into(),
            title_bg: None,
            description: "".into(),
            width: None,
            height: None,
            story: None,
            story_klass: None,
            closable: true,
            zoomable: Some(PanelControl::default()),
            on_active: None,
        }
    }

    /// 为特定故事创建面板
    pub fn panel<S: Story>(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let name = S::title();
        let description = S::description();
        let story = S::new_view(window, cx);
        let story_klass = S::klass();
        let focus_handle = story.focus_handle(cx);

        let view = cx.new(|cx| {
            let mut story = Self::new(window, cx)
                .story(story.into(), story_klass)
                .on_active(S::on_active_any);
            story.focus_handle = focus_handle;
            story.closable = S::closable();
            story.zoomable = S::zoomable();
            story.name = name.into();
            story.description = description.into();
            story.title_bg = S::title_bg();
            story
        });

        view
    }

    /// 设置宽度
    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = Some(width);
        self
    }

    /// 设置高度
    pub fn height(mut self, height: gpui::Pixels) -> Self {
        self.height = Some(height);
        self
    }

    /// 设置故事
    pub fn story(mut self, story: AnyView, story_klass: impl Into<SharedString>) -> Self {
        self.story = Some(story);
        self.story_klass = Some(story_klass.into());
        self
    }

    /// 设置激活回调
    pub fn on_active(mut self, on_active: fn(AnyView, bool, &mut Window, &mut App)) -> Self {
        self.on_active = Some(on_active);
        self
    }

    /// 处理显示面板信息操作
    fn on_action_panel_info(
        &mut self,
        _: &ShowPanelInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        struct Info;
        let note = Notification::new()
            .message(format!("You have clicked panel info on: {}", self.name))
            .id::<Info>();
        window.push_notification(note, cx);
    }

    /// 处理切换搜索操作
    fn on_action_toggle_search(
        &mut self,
        _: &ToggleSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.propagate();
        if window.has_focused_input(cx) {
            return;
        }

        struct Search;
        let note = Notification::new()
            .message(format!("You have toggled search on: {}", self.name))
            .id::<Search>();
        window.push_notification(note, cx);
    }
}

/// 故事状态，用于序列化和反序列化故事信息
#[derive(Debug, Serialize, Deserialize)]
pub struct StoryState {
    pub story_klass: SharedString, // 故事类名
}

impl StoryState {
    /// 转换为 JSON 值
    fn to_value(&self) -> serde_json::Value {
        serde_json::json!({
            "story_klass": self.story_klass,
        })
    }

    /// 从 JSON 值创建
    fn from_value(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap()
    }

    /// 转换为故事元组
    fn to_story(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> (
        &'static str,
        &'static str,
        bool,
        Option<PanelControl>,
        AnyView,
        fn(AnyView, bool, &mut Window, &mut App),
    ) {
        // 宏定义：简化故事创建代码
        macro_rules! story {
            ($klass:tt) => {
                (
                    $klass::title(),
                    $klass::description(),
                    $klass::closable(),
                    $klass::zoomable(),
                    $klass::view(window, cx).into(),
                    $klass::on_active_any,
                )
            };
        }

        // 根据故事类名匹配对应的故事
        match self.story_klass.to_string().as_str() {
            "ButtonStory" => story!(ButtonStory),
            "CalendarStory" => story!(CalendarStory),
            "DropdownStory" => story!(DropdownStory),
            "IconStory" => story!(IconStory),
            "ImageStory" => story!(ImageStory),
            "InputStory" => story!(InputStory),
            "ListStory" => story!(ListStory),
            "ModalStory" => story!(ModalStory),
            "PopoverStory" => story!(PopoverStory),
            "ProgressStory" => story!(ProgressStory),
            "ResizableStory" => story!(ResizableStory),
            "ScrollableStory" => story!(ScrollableStory),
            "SwitchStory" => story!(SwitchStory),
            "TableStory" => story!(TableStory),
            "LabelStory" => story!(LabelStory),
            "TooltipStory" => story!(TooltipStory),
            "WebViewStory" => story!(WebViewStory),
            "AccordionStory" => story!(AccordionStory),
            "SidebarStory" => story!(SidebarStory),
            "FormStory" => story!(FormStory),
            _ => {
                unreachable!("Invalid story klass: {}", self.story_klass)
            }
        }
    }
}

// 实现面板特征
impl Panel for StoryContainer {
    /// 面板名称
    fn panel_name(&self) -> &'static str {
        "StoryContainer"
    }

    /// 面板标题
    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        self.name.clone().into_any_element()
    }

    /// 标题样式
    fn title_style(&self, cx: &App) -> Option<TitleStyle> {
        if let Some(bg) = self.title_bg {
            Some(TitleStyle {
                background: bg,
                foreground: cx.theme().foreground,
            })
        } else {
            None
        }
    }

    /// 是否可关闭
    fn closable(&self, _cx: &App) -> bool {
        self.closable
    }

    /// 是否可缩放
    fn zoomable(&self, _cx: &App) -> Option<PanelControl> {
        self.zoomable
    }

    /// 是否可见
    fn visible(&self, cx: &App) -> bool {
        !AppState::global(cx)
            .invisible_panels
            .read(cx)
            .contains(&self.name)
    }

    /// 设置缩放状态
    fn set_zoomed(&mut self, zoomed: bool, _window: &mut Window, _cx: &mut App) {
        println!("panel: {} zoomed: {}", self.name, zoomed);
    }

    /// 设置激活状态
    fn set_active(&mut self, active: bool, _window: &mut Window, cx: &mut App) {
        println!("panel: {} active: {}", self.name, active);
        if let Some(on_active) = self.on_active {
            if let Some(story) = self.story.clone() {
                on_active(story, active, _window, cx);
            }
        }
    }

    /// 弹出菜单
    fn popup_menu(&self, menu: PopupMenu, _window: &Window, _cx: &App) -> PopupMenu {
        menu.menu("Info", Box::new(ShowPanelInfo))
    }

    /// 工具栏按钮
    fn toolbar_buttons(&self, _window: &mut Window, _cx: &mut App) -> Option<Vec<Button>> {
        Some(vec![
            Button::new("info")
                .icon(IconName::Info)
                .on_click(|_, window, cx| {
                    window.push_notification("You have clicked info button", cx);
                }),
            Button::new("search")
                .icon(IconName::Search)
                .on_click(|_, window, cx| {
                    window.push_notification("You have clicked search button", cx);
                }),
        ])
    }

    /// 转储面板状态
    fn dump(&self, _cx: &App) -> PanelState {
        let mut state = PanelState::new(self);
        let story_state = StoryState {
            story_klass: self.story_klass.clone().unwrap(),
        };
        state.info = PanelInfo::panel(story_state.to_value());
        state
    }
}

// 实现事件发射器
impl EventEmitter<PanelEvent> for StoryContainer {}

// 实现焦点管理
impl Focusable for StoryContainer {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

// 实现渲染
impl Render for StoryContainer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("story-container") // 元素 ID
            .size_full() // 占满容器
            .overflow_y_scroll() // 垂直滚动
            .track_focus(&self.focus_handle) // 跟踪焦点
            .on_action(cx.listener(Self::on_action_panel_info)) // 监听面板信息操作
            .on_action(cx.listener(Self::on_action_toggle_search)) // 监听切换搜索操作
            .when_some(self.story.clone(), |this, story| {
                this.child(
                    v_flex()
                        .id("story-children") // 子元素 ID
                        .w_full() // 全宽
                        .flex_1() // 占满剩余空间
                        .p_4() // 内边距 4 单位
                        .child(story), // 故事内容
                )
            })
    }
}
