mod assets;
pub(crate) mod components;
pub(crate) mod main_window;
mod views;

use crate::ui::components::{titlebar::AppTitleBar, titlebar::NormalTitleBar};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::{scroll::ScrollbarShow, v_flex, Root};
use raw_window_handle::HasWindowHandle;
use raw_window_handle::RawWindowHandle;
use serde::Deserialize;


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

/// 故事根组件，包含标题栏和主视图
struct TodoRoot {
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

    pub fn with_no_title_bar(
        view: impl Into<AnyView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
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

pub trait AppExt {
    /// 创建一个新的窗口，使用默认的标题栏和主视图
    fn create_todo_window<F, E>(&mut self, options: WindowOptions, crate_view_fn: F)
    where
        E: Into<AnyView>,
        F: FnOnce(&mut Window, &mut App) -> E + Send + 'static;

    fn create_normal_window<F, E>(
        &mut self,
        title: impl Into<SharedString>,
        options: WindowOptions,
        crate_view_fn: F,
    ) where
        E: Into<AnyView>,
        F: FnOnce(&mut Window, &mut App) -> E + Send + 'static;

    fn create_window<F, E>(&mut self, options: WindowOptions, crate_view_fn: F)
    where
        E: Into<AnyView>,
        F: FnOnce(&mut Window, &mut App) -> E + Send + 'static;
}

impl AppExt for App {
    fn create_window<F, E>(&mut self, options: WindowOptions, crate_view_fn: F)
    where
        E: Into<AnyView>,
        F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
    {
        self.spawn(async move |cx| {
            let window = cx
                .open_window(options, |window, cx| {
                    // #[cfg(target_os = "windows")]
                    // if let Ok(hwnd) = window.window_handle() {
                    //     match hwnd.as_raw() {
                    //         RawWindowHandle::Win32(hwnd) => {
                    //             use windows::Win32::UI::WindowsAndMessaging::{
                    //                 GetWindowLongW, SetWindowLongW, GWL_STYLE, WS_MAXIMIZEBOX,
                    //                 WS_MINIMIZEBOX, WS_SIZEBOX, WS_SYSMENU,
                    //             };
                    //             use windows::Win32::Foundation::HWND;
                    //             let hwnd = HWND(hwnd.hwnd.get() as _);
                    //             unsafe {
                    //                 let mut style = GetWindowLongW(hwnd, GWL_STYLE);
                    //                 style ^= WS_SIZEBOX.0 as i32; //设置窗体不可调整大小
                    //                 style ^= WS_MINIMIZEBOX.0 as i32; //设置窗体取消最小化按钮
                    //                 style ^= WS_SYSMENU.0 as i32; //设置窗体取消系统菜单
                    //                 style ^= WS_MAXIMIZEBOX.0 as i32; //设置窗体取消最大化按钮
                    //                 SetWindowLongW(hwnd, GWL_STYLE, style);
                    //             }
                    //             window.refresh();
                    //         }
                    //         RawWindowHandle::WinRt(hwnd) => {
                    //             let hwnd = hwnd.core_window.as_ptr();
                    //             use windows::Win32::UI::WindowsAndMessaging::{
                    //                 GetWindowLongW, SetWindowLongW, GWL_STYLE, WS_MAXIMIZEBOX,
                    //                 WS_MINIMIZEBOX, WS_SIZEBOX,
                    //             };
                    //             use windows::Win32::Foundation::HWND;
                    //             let hwnd = HWND(hwnd);
                    //             unsafe {
                    //                 let mut style = GetWindowLongW(hwnd, GWL_STYLE);
                    //                 style ^= WS_SIZEBOX.0 as i32; //设置窗体不可调整大小
                    //                 style ^= WS_MINIMIZEBOX.0 as i32; //设置窗体取消最小化按钮
                    //                 style ^= WS_MAXIMIZEBOX.0 as i32; //设置窗体取消最大化按钮
                    //                 SetWindowLongW(hwnd, GWL_STYLE, style);
                    //             }
                    //         }
                    //         _ => {}
                    //     }
                    // }
                    let view = crate_view_fn(window, cx);
                    let root = cx.new(|cx| TodoRoot::with_no_title_bar(view, window, cx));

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
    fn create_todo_window<F, E>(&mut self, options: WindowOptions, crate_view_fn: F)
    where
        E: Into<AnyView>,
        F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
    {
        self.spawn(async move |cx| {
            let window = cx
                .open_window(options, |window, cx| {
                    #[cfg(target_os = "windows")]
                    if let Ok(hwnd) = window.window_handle() {
                        match hwnd.as_raw() {
                            RawWindowHandle::Win32(hwnd) => {
                                use windows::Win32::UI::WindowsAndMessaging::{
                                    GetWindowLongW, SetWindowLongW, GWL_STYLE, WS_MAXIMIZEBOX,
                                    WS_SIZEBOX,
                                };
                                use windows::Win32::Foundation::HWND;
                                let hwnd = HWND(hwnd.hwnd.get() as _);
                                unsafe {
                                    let mut style = GetWindowLongW(hwnd, GWL_STYLE);
                                    style ^= WS_SIZEBOX.0 as i32; //设置窗体不可调整大小
                                    style ^= WS_MAXIMIZEBOX.0 as i32; //设置窗体取消最大化按钮
                                    SetWindowLongW(hwnd, GWL_STYLE, style);
                                }
                                window.refresh();
                            }
                            RawWindowHandle::WinRt(hwnd) => {
                                let hwnd = hwnd.core_window.as_ptr();
                                use windows::Win32::UI::WindowsAndMessaging::{
                                    GetWindowLongW, SetWindowLongW, GWL_STYLE, WS_MAXIMIZEBOX,
                                    WS_MINIMIZEBOX, WS_SIZEBOX,
                                };
                                use windows::Win32::Foundation::HWND;
                                let hwnd = HWND(hwnd);
                                unsafe {
                                    let mut style = GetWindowLongW(hwnd, GWL_STYLE);
                                    style ^= WS_SIZEBOX.0 as i32; //设置窗体不可调整大小
                                    style ^= WS_MAXIMIZEBOX.0 as i32; //设置窗体取消最大化按钮
                                    SetWindowLongW(hwnd, GWL_STYLE, style);
                                }
                            }
                            _ => {}
                        }
                    }
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

    fn create_normal_window<F, E>(
        &mut self,
        title: impl Into<SharedString>,
        options: WindowOptions,
        crate_view_fn: F,
    ) where
        E: Into<AnyView>,
        F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
    {
        let title = title.into();

        self.spawn(async move |cx| {
            let window = cx
                .open_window(options, |window, cx| {
                    #[cfg(target_os = "windows")]
                    if let Ok(hwnd) = window.window_handle() {
                        match hwnd.as_raw() {
                            RawWindowHandle::Win32(hwnd) => {
                                use windows::Win32::UI::WindowsAndMessaging::{
                                    GetWindowLongW, SetWindowLongW, GWL_STYLE, WS_MAXIMIZEBOX,
                                    WS_MINIMIZEBOX, WS_SIZEBOX,
                                };
                                use windows::Win32::Foundation::HWND;
                                let hwnd = HWND(hwnd.hwnd.get() as _);
                                unsafe {
                                    let mut style = GetWindowLongW(hwnd, GWL_STYLE);
                                    style ^= WS_SIZEBOX.0 as i32; //设置窗体不可调整大小
                                    style ^= WS_MINIMIZEBOX.0 as i32; //设置窗体取消最小化按钮
                                    style ^= WS_MAXIMIZEBOX.0 as i32; //设置窗体取消最大化按钮
                                    SetWindowLongW(hwnd, GWL_STYLE, style);
                                }
                            }
                            RawWindowHandle::WinRt(hwnd) => {
                                let hwnd = hwnd.core_window.as_ptr();
                                use windows::Win32::UI::WindowsAndMessaging::{
                                    GetWindowLongW, SetWindowLongW, GWL_STYLE, WS_MAXIMIZEBOX,
                                    WS_MINIMIZEBOX, WS_SIZEBOX,
                                };
                                use windows::Win32::Foundation::HWND;
                                let hwnd = HWND(hwnd);
                                unsafe {
                                    let mut style = GetWindowLongW(hwnd, GWL_STYLE);
                                    style ^= WS_SIZEBOX.0 as i32; //设置窗体不可调整大小
                                    style ^= WS_MINIMIZEBOX.0 as i32; //设置窗体取消最小化按钮
                                    style ^= WS_MAXIMIZEBOX.0 as i32; //设置窗体取消最大化按钮
                                    SetWindowLongW(hwnd, GWL_STYLE, style);
                                }
                            }
                            _ => {}
                        }
                    }
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
}
