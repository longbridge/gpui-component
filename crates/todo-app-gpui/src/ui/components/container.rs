use crate::app::{AppState, ToggleSearch, ViewKitState};

use super::ViewKit;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    button::Button,
    dock::{Panel, PanelControl, PanelEvent, PanelInfo, PanelState, TitleStyle},
    notification::Notification,
    popup_menu::PopupMenu,
    *,
};

pub struct Container {
    pub focus_handle: gpui::FocusHandle,   // 焦点处理句柄
    pub name: SharedString,                // 容器名称
    pub title_bg: Option<Hsla>,            // 标题背景色
    pub description: SharedString,         // 描述
    pub width: Option<gpui::Pixels>,       // 宽度
    pub height: Option<gpui::Pixels>,      // 高度
    pub story: Option<AnyView>,            // 故事视图
    pub story_klass: Option<SharedString>, // 故事类名
    pub closable: bool,                    // 是否可关闭
    pub zoomable: Option<PanelControl>,    // 是否可缩放
    pub icon: Option<IconName>,            // 图标
    pub on_active: Option<fn(AnyView, bool, &mut Window, &mut App)>, // 激活回调
}

/// 容器事件枚举
#[derive(Debug)]
pub enum ContainerEvent {
    Close, // 关闭事件
}
// 实现事件发射器
impl EventEmitter<ContainerEvent> for Container {}

// 定义显示面板信息的操作
actions!(story, [ShowPanelInfo]);

impl Container {
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
            icon: None,
            on_active: None,
        }
    }

    /// 为特定故事创建面板
    pub fn panel<S: ViewKit>(window: &mut Window, cx: &mut App) -> Entity<Self> {
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
            story.icon = S::icon();
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
            .id::<Info>()
            .message(format!("You have clicked panel info on: {}", self.name));
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
            .id::<Search>()
            .message(format!("You have toggled search on: {}", self.name));
        window.push_notification(note, cx);
    }
}

// 实现面板特征
impl Panel for Container {
    /// 面板名称
    fn panel_name(&self) -> &'static str {
        "Container"
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
        let story_state = ViewKitState {
            story_klass: self.story_klass.clone().unwrap(),
        };

        state.info = PanelInfo::panel(story_state.to_value());
        state
    }
}

// 实现事件发射器
impl EventEmitter<PanelEvent> for Container {}

// 实现焦点管理
impl Focusable for Container {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

// 实现渲染
impl Render for Container {
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
