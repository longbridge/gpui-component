pub(crate) mod assets;
pub(crate) mod components;
pub(crate) mod main_window;
pub(crate) mod views;

use crate::ui::components::{appbar::AppTitleBar, appbar::NormalTitleBar};
use gpui::{prelude::FluentBuilder, Window, *};
use gpui_component::{scroll::ScrollbarShow, v_flex, Root};
use raw_window_handle::HasWindowHandle;
use raw_window_handle::RawWindowHandle;
use serde::Deserialize;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

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
                    #[cfg(target_os = "windows")]
                    {
                        use windows::Win32::UI::WindowsAndMessaging::{
                            WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SIZEBOX, WS_SYSMENU,
                        };
                        // window.set_display_affinity(0x00000011);
                        // let mut style = window.style();
                        // style &= !(WS_SIZEBOX.0 as i32
                        //     | WS_MINIMIZEBOX.0 as i32
                        //     | WS_MAXIMIZEBOX.0 as i32
                        //     | WS_SYSMENU.0 as i32);
                        // window.set_style(style);
                    }
                    let view = crate_view_fn(window, cx);
                    let root = cx.new(|cx| TodoRoot::with_no_title_bar(view));

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
                    {
                        use windows::Win32::UI::WindowsAndMessaging::{
                            WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SIZEBOX, WS_SYSMENU,
                        };
                        //window.set_display_affinity(0x00000011);
                        // let mut style = window.style();
                        // style &= !(WS_SIZEBOX.0 as i32
                        //     | WS_MINIMIZEBOX.0 as i32
                        //     | WS_MAXIMIZEBOX.0 as i32
                        //     | WS_SYSMENU.0 as i32);
                        // window.set_style(style);
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
                    {
                        use windows::Win32::UI::WindowsAndMessaging::{
                            WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SIZEBOX, WS_SYSMENU,
                        };
                        // window.set_display_affinity(0x00000011);
                        // let mut style = window.style();
                        // style &= !(WS_SIZEBOX.0 as i32
                        //     | WS_MINIMIZEBOX.0 as i32
                        //     | WS_MAXIMIZEBOX.0 as i32
                        //     | WS_SYSMENU.0 as i32);
                        // window.set_style(style);
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

#[cfg(target_os = "windows")]
pub trait WindowExt {
    fn hwnd(&self) -> Option<HWND> {
        None
    }

    fn style(&self) -> i32 {
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongW, GWL_STYLE};
        self.hwnd()
            .map_or(0, |hwnd| unsafe { GetWindowLongW(hwnd, GWL_STYLE) })
    }

    fn set_style(&self, style: i32) {
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowLongW, GWL_STYLE};
        self.hwnd().map(|hwnd| unsafe {
            SetWindowLongW(hwnd, GWL_STYLE, style);
        });
    }

    fn set_display_affinity(&self, dwaffinity: u32) {
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowDisplayAffinity, WINDOW_DISPLAY_AFFINITY,
        };
        self.hwnd().map(|hwnd| unsafe {
            SetWindowDisplayAffinity(hwnd, WINDOW_DISPLAY_AFFINITY(dwaffinity)).ok();
        });
    }

    fn enable_window(&self, benable: bool) {
        use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
        if let Some(hwnd) = self.hwnd() {
            unsafe {
                let _ = EnableWindow(hwnd, benable);
            }
        }
    }
}
#[cfg(target_os = "windows")]
impl WindowExt for Window {
    fn hwnd(&self) -> Option<HWND> {
        if let Ok(any_window_handle) = HasWindowHandle::window_handle(self) {
            match any_window_handle.as_raw() {
                RawWindowHandle::Win32(hwnd) => {
                    return Some(HWND(hwnd.hwnd.get() as _));
                }
                RawWindowHandle::WinRt(hwnd) => {
                    let hwnd = hwnd.core_window.as_ptr();

                    return Some(HWND(hwnd));
                }
                _ => return None,
            }
        }
        None
    }
}
