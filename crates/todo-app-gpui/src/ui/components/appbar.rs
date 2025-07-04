use super::titlebar::TitleBar;
use crate::ui::views::settings::Settings;
use crate::ui::{SelectFont, SelectLocale, SelectRadius, SelectScrollbarShow};
use gpui::*;
use gpui_component::{
    badge::Badge,
    button::{Button, ButtonVariants as _},
    popup_menu::PopupMenuExt as _,
    scroll::ScrollbarShow,
    *,
};
use std::rc::Rc;
use std::sync::Arc;

pub struct AppTitleBar {
    locale_selector: Entity<LocaleSelector>,
    child: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    _subscriptions: Vec<Subscription>,
    setting_window: Option<WindowHandle<Root>>,
}

impl AppTitleBar {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let locale_selector = cx.new(|cx| LocaleSelector::new(window, cx));
        if cx.should_auto_hide_scrollbars() {
            Theme::global_mut(cx).scrollbar_show = ScrollbarShow::Scrolling;
        } else {
            Theme::global_mut(cx).scrollbar_show = ScrollbarShow::Hover;
        }

        let _subscriptions = vec![];

        Self {
            locale_selector,
            child: Rc::new(|_, _| div().into_any_element()),
            _subscriptions,
            setting_window: None,
        }
    }

    pub fn child<F, E>(mut self, f: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.child = Rc::new(move |window, cx| f(window, cx).into_any_element());
        self
    }
}

impl Render for AppTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notifications_count = window.notifications(cx).len();
        TitleBar::new()
            .show_maximize(false)
            // left side
            .child(
                div().flex().items_center().child(
                    img(Arc::new(Image::from_bytes(
                        gpui::ImageFormat::Png,
                        cx.asset_source()
                            .load("logo0.png")
                            .unwrap()
                            .unwrap()
                            .to_vec(),
                    )))
                    .size_12(),
                ), // .child(self.title.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_2()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child((self.child.clone())(window, cx))
                    // .child(self.locale_selector.clone())
                    .child(
                        Button::new("setting")
                            .icon(IconName::Settings)
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this,_ev, window, cx| {
                                if let Some(handle) = this.setting_window.as_ref() {
                                     if handle.is_active(cx).is_some(){
                                        handle
                                            .update(cx, |_, window, cx| {
                                            window.activate_window();
                                            cx.notify();
                        })
                        .ok();
                                     }else{
                                         let handle = Settings::open(Some("服务提供商"), window, cx);
                                this.setting_window = Some(handle);
                                     }
                                }else{
                                let handle = Settings::open(Some("服务提供商"), window, cx);
                                this.setting_window = Some(handle);
                                }
                            })),
                    )
                    .child(
                        div().relative().child(
                            Badge::new().count(notifications_count).max(99).child(
                                Button::new("bell")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::Bell),
                            ),
                        ),
                    ),
            )
    }
}

struct LocaleSelector {
    focus_handle: FocusHandle,
}

impl LocaleSelector {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_select_locale(
        &mut self,
        locale: &SelectLocale,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        set_locale(&locale.0);
        window.refresh();
    }
}

impl Render for LocaleSelector {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();
        let locale = locale().to_string();

        div()
            .id("locale-selector")
            .track_focus(&focus_handle)
            .on_action(cx.listener(Self::on_select_locale))
            .child(
                Button::new("btn")
                    .small()
                    .ghost()
                    .icon(IconName::Globe)
                    .popup_menu(move |this, _, _| {
                        this.menu_with_check(
                            "English",
                            locale == "en",
                            Box::new(SelectLocale("en".into())),
                        )
                        .menu_with_check(
                            "简体中文",
                            locale == "zh-CN",
                            Box::new(SelectLocale("zh-CN".into())),
                        )
                    })
                    .anchor(Corner::TopRight),
            )
    }
}

struct FontSizeSelector {
    focus_handle: FocusHandle,
}

impl FontSizeSelector {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_select_font(
        &mut self,
        font_size: &SelectFont,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::global_mut(cx).font_size = px(font_size.0 as f32);
        window.refresh();
    }

    fn on_select_radius(
        &mut self,
        radius: &SelectRadius,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::global_mut(cx).radius = px(radius.0 as f32);
        window.refresh();
    }

    fn on_select_scrollbar_show(
        &mut self,
        show: &SelectScrollbarShow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::global_mut(cx).scrollbar_show = show.0;
        window.refresh();
    }
}

impl Render for FontSizeSelector {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();
        let font_size = cx.theme().font_size.0 as i32;
        let radius = cx.theme().radius.0 as i32;
        let scroll_show = cx.theme().scrollbar_show;

        div()
            .id("font-size-selector")
            .track_focus(&focus_handle)
            .on_action(cx.listener(Self::on_select_font))
            .on_action(cx.listener(Self::on_select_radius))
            .on_action(cx.listener(Self::on_select_scrollbar_show))
            .child(
                Button::new("btn")
                    .small()
                    .ghost()
                    .icon(IconName::Settings2)
                    .popup_menu(move |this, _, _| {
                        this.scrollable()
                            .max_h(px(480.))
                            .label("Font Size")
                            .menu_with_check("Large", font_size == 18, Box::new(SelectFont(18)))
                            .menu_with_check(
                                "Medium (default)",
                                font_size == 16,
                                Box::new(SelectFont(16)),
                            )
                            .menu_with_check("Small", font_size == 14, Box::new(SelectFont(14)))
                            .separator()
                            .label("Border Radius")
                            .menu_with_check("8px", radius == 8, Box::new(SelectRadius(8)))
                            .menu_with_check(
                                "4px (default)",
                                radius == 4,
                                Box::new(SelectRadius(4)),
                            )
                            .menu_with_check("0px", radius == 0, Box::new(SelectRadius(0)))
                            .separator()
                            .label("Scrollbar")
                            .menu_with_check(
                                "Scrolling to show",
                                scroll_show == ScrollbarShow::Scrolling,
                                Box::new(SelectScrollbarShow(ScrollbarShow::Scrolling)),
                            )
                            .menu_with_check(
                                "Hover to show",
                                scroll_show == ScrollbarShow::Hover,
                                Box::new(SelectScrollbarShow(ScrollbarShow::Hover)),
                            )
                            .menu_with_check(
                                "Always show",
                                scroll_show == ScrollbarShow::Always,
                                Box::new(SelectScrollbarShow(ScrollbarShow::Always)),
                            )
                    })
                    .anchor(Corner::TopRight),
            )
    }
}

pub struct NormalTitleBar {
    title: SharedString,
    child: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
}

impl NormalTitleBar {
    pub fn new(
        title: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        if cx.should_auto_hide_scrollbars() {
            Theme::global_mut(cx).scrollbar_show = ScrollbarShow::Scrolling;
        } else {
            Theme::global_mut(cx).scrollbar_show = ScrollbarShow::Hover;
        }

        Self {
            title: title.into(),
            child: Rc::new(|_, _| div().into_any_element()),
        }
    }

    pub fn child<F, E>(mut self, f: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.child = Rc::new(move |window, cx| f(window, cx).into_any_element());
        self
    }
}

impl Render for NormalTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new()
            .show_minimize(false)
            .show_maximize(false)
            .child(self.title.clone())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_2()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child((self.child.clone())(window, cx)),
            )
    }
}
