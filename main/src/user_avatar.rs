//! 用户头像组件
//!
//! 显示用户头像，已登录时显示头像和用户信息，未登录时显示登录按钮。

use gpui::{
    AnyElement, App, Context, Entity, InteractiveElement, IntoElement, ParentElement, SharedString,
    Styled, Window, div,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable, Size,
    avatar::Avatar,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use one_core::cloud_sync::UserInfo;
use rust_i18n::t;

/// 渲染用户头像区域
///
/// - 已登录：显示头像、用户名、邮箱
/// - 未登录：显示登录按钮
///
/// 参数：
/// - user: 当前用户信息
/// - view: 父视图实体，用于事件处理
/// - on_click: 点击回调，接收 (this, window, cx)
pub fn render_user_avatar<V: 'static>(
    user: Option<&UserInfo>,
    view: Entity<V>,
    on_click: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
    cx: &App,
) -> AnyElement {
    if let Some(user) = user {
        let email: SharedString = user.email.clone().into();
        let display_name: SharedString = user
            .username
            .clone()
            .unwrap_or_else(|| {
                // 从邮箱提取用户名
                user.email.split('@').next().unwrap_or(&user.email).to_string()
            })
            .into();
        let avatar_url = user.avatar_url.clone();

        let avatar = if let Some(url) = &avatar_url {
            Avatar::new()
                .src(url.as_str())
                .with_size(Size::Small)
        } else {
            Avatar::new()
                .name(display_name.clone())
                .with_size(Size::Small)
        };

        let bg_color = cx.theme().secondary;

        h_flex()
            .id("user-avatar-area")
            .w_full()
            .gap_2()
            .items_center()
            .p_1()
            .rounded_md()
            .cursor_pointer()
            .hover(move |this| this.bg(bg_color))
            .child(avatar)
            .child(
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        div()
                            .text_sm()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .max_w_full()
                            .overflow_hidden()
                            .child(display_name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .max_w_full()
                            .overflow_hidden()
                            .child(email),
                    ),
            )
            .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                view.update(cx, |this, cx| {
                    on_click(this, window, cx);
                });
            })
            .into_any_element()
    } else {
        let view_clone = view.clone();
        Button::new("login-button")
            .icon(IconName::User)
            .label(t!("Auth.login"))
            .ghost()
            .w_full()
            .on_click(move |_, window, cx| {
                view_clone.update(cx, |this, cx| {
                    on_click(this, window, cx);
                });
            })
            .into_any_element()
    }
}
