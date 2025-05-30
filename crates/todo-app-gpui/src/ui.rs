mod assets;
mod components;
pub(crate) mod main_window;
mod views;

use gpui::*;
use serde::Deserialize;
use gpui_component::{scroll::ScrollbarShow, v_flex, Root};
use crate::ui::components::{appbar::AppTitleBar, titlebar::NormalTitleBar};

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct SelectScrollbarShow(ScrollbarShow);

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct SelectLocale(SharedString);

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct SelectFont(usize);

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct SelectRadius(usize);

impl_internal_actions!(
    story,
    [SelectLocale, SelectFont, SelectRadius, SelectScrollbarShow]
);

actions!(story, [Quit, Open, CloseWindow, ToggleSearch]);

/// 使用自定义选项创建新窗口
///
/// # 参数
/// - `title`: 窗口标题
/// - `options`: 窗口选项配置
/// - `crate_view_fn`: 创建视图的函数
/// - `cx`: 应用程序上下文
pub fn create_todo_window_options<F, E>(options: WindowOptions, crate_view_fn: F, cx: &mut App)
where
    E: Into<AnyView>,
    F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
{
    cx.spawn(async move |cx| {
        let window = cx
            .open_window(options, |window, cx| {
                let view = crate_view_fn(window, cx);
                let root = cx.new(|cx| TodoRoot::new(view, window, cx));

                cx.new(|cx| Root::new(root.into(), window, cx))
            })
            .expect("failed to open window");

        window
            .update(cx, |_, window, _| {
                window.activate_window();
                window.set_window_title("X-Todo Utility");
            })
            .expect("failed to update window");

        Ok::<_, anyhow::Error>(())
    })
    .detach();
}

/// 故事根组件，包含标题栏和主视图
struct TodoRoot {
    title_bar: Entity<AppTitleBar>, // 应用程序标题栏
    view: AnyView,                  // 主视图
}

impl TodoRoot {
    /// 创建新的故事根组件
    pub fn new(view: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let title_bar = cx.new(|cx| AppTitleBar::new(window, cx));
        Self {
            title_bar,
            view: view.into(),
        }
    }
}

impl Render for TodoRoot {
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

pub fn create_normal_window_options<F, E>(
    title: impl Into<SharedString>,
    options: WindowOptions,
    crate_view_fn: F,
    cx: &mut App,
) where
    E: Into<AnyView>,
    F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
{
    let title = title.into();

    cx.spawn(async move |cx| {
        let window = cx
            .open_window(options, |window, cx| {
                let view = crate_view_fn(window, cx);
                let root = cx.new(|cx| NormalRoot::new(title.clone(), view, window, cx));

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
struct NormalRoot {
    title_bar: Entity<NormalTitleBar>, // 应用程序标题栏
    view: AnyView,                     // 主视图
}

impl NormalRoot {
    /// 创建新的故事根组件
    pub fn new(
        title: impl Into<SharedString>,
        view: impl Into<AnyView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_bar = cx.new(|cx| NormalTitleBar::new(title, window, cx));
        Self {
            title_bar,
            view: view.into(),
        }
    }
}

impl Render for NormalRoot {
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
