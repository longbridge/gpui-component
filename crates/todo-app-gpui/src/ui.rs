pub(crate) mod assets;
pub(crate) mod components;
pub(crate) mod main_window;
pub(crate) mod views;

use crate::ui::components::{appbar::AppTitleBar, appbar::NormalTitleBar};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::{scroll::ScrollbarShow, v_flex, Root};
use serde::Deserialize;

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = todo, no_json)]
pub struct SelectScrollbarShow(ScrollbarShow);

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = todo, no_json)]
pub struct SelectLocale(SharedString);

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = todo, no_json)]
pub struct SelectFont(usize);

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = todo, no_json)]
pub struct SelectRadius(usize);

/// 故事根组件，包含标题栏和主视图
pub(crate) struct TodoRoot {
    title_bar: Option<Entity<AppTitleBar>>, // 应用程序标题栏
    view: AnyView,                          // 主视图
}

impl TodoRoot {
    /// 创建新的故事根组件
    pub fn new(view: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let title_bar = cx.new(|cx| AppTitleBar::new(window, cx));
        Self {
            title_bar: Some(title_bar),
            view: view.into(),
        }
    }

    pub fn with_no_title_bar(view: impl Into<AnyView>) -> Self {
        Self {
            title_bar: None, // 不使用标题栏
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
                    .when_some(self.title_bar.clone(), |div, app_title_bar| {
                        div.child(app_title_bar)
                    })
                    .child(div().flex_1().overflow_hidden().child(self.view.clone())), // 主视图
            )
            .children(drawer_layer) // 添加抽屉层
            .children(modal_layer) // 添加模态层
            .child(div().absolute().top_8().children(notification_layer)) // 添加通知层
    }
}

/// 故事根组件，包含标题栏和主视图
pub(crate) struct NormalRoot {
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
