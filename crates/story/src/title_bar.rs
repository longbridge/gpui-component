use std::rc::Rc;

use gpui::{Window, ModelContext, AppContext, Model, 
    div, AnyElement, ClickEvent, Corner, FocusHandle, Hsla, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, Render, SharedString, Styled as _,  
    VisualContext as _, 
};
use ui::{
    badge::Badge,
    button::{Button, ButtonVariants as _},
    color_picker::{ColorPicker, ColorPickerEvent},
    popup_menu::PopupMenuExt as _,
    prelude::FluentBuilder as _,
    scroll::ScrollbarShow,
    ActiveTheme as _, ContextModal as _, IconName, Sizable as _, Theme, TitleBar,
};

use crate::{SelectFont, SelectLocale, SelectScrollbarShow};

pub struct AppTitleBar {
    title: SharedString,
    theme_color: Option<Hsla>,
    locale_selector: Entity<LocaleSelector>,
    font_size_selector: Entity<FontSizeSelector>,
    theme_color_picker: Entity<ColorPicker>,
    child: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
}

impl AppTitleBar {
    pub fn new(title: impl Into<SharedString>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let locale_selector = cx.new(LocaleSelector::new);
        let font_size_selector = cx.new(FontSizeSelector::new);

        let theme_color_picker = cx.new(|cx| {
            let mut picker = ColorPicker::new("theme-color-picker", cx)
                .xsmall()
                .anchor(Corner::TopRight)
                .label("Theme Color");
            picker.set_value(cx.theme().primary, cx);
            picker
        });
        cx.subscribe(
            &theme_color_picker,
            |this, _, ev: &ColorPickerEvent, cx| match ev {
                ColorPickerEvent::Change(color) => {
                    this.set_theme_color(*color, cx);
                }
            },
        )
        .detach();

        Self {
            title: title.into(),
            theme_color: None,
            locale_selector,
            font_size_selector,
            theme_color_picker,
            child: Rc::new(|_| div().into_any_element()),
        }
    }

    pub fn child<F, E>(mut self, f: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.child = Rc::new(move |cx| f(cx).into_any_element());
        self
    }

    fn set_theme_color(&mut self, color: Option<Hsla>, window: &mut Window, cx: &mut Context<Self>) {
        self.theme_color = color;
        if let Some(color) = self.theme_color {
            let theme = cx.global_mut::<Theme>();
            theme.apply_color(color);
            window.refresh();
        }
    }

    fn change_color_mode(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let mode = match cx.theme().mode.is_dark() {
            true => ui::ThemeMode::Light,
            false => ui::ThemeMode::Dark,
        };

        Theme::change(mode, cx);
        self.set_theme_color(self.theme_color, cx);
    }
}

impl Render for AppTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notifications_count = cx.notifications().len();

        TitleBar::new()
            // left side
            .child(div().flex().items_center().child(self.title.clone()))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_2()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, |_, window, cx| cx.stop_propagation())
                    .child(self.theme_color_picker.clone())
                    .child((self.child.clone())(cx))
                    .child(
                        Button::new("theme-mode")
                            .map(|this| {
                                if cx.theme().mode.is_dark() {
                                    this.icon(IconName::Sun)
                                } else {
                                    this.icon(IconName::Moon)
                                }
                            })
                            .small()
                            .ghost()
                            .on_click(cx.listener(Self::change_color_mode)),
                    )
                    .child(self.locale_selector.clone())
                    .child(self.font_size_selector.clone())
                    .child(
                        Badge::new().dot().count(1).child(
                            Button::new("github")
                                .icon(IconName::GitHub)
                                .small()
                                .ghost()
                                .on_click(|_, cx| {
                                    cx.open_url("https://github.com/longbridge/gpui-component")
                                }),
                        ),
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
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_select_locale(&mut self, locale: &SelectLocale, window: &mut Window, cx: &mut Context<Self>) {
        ui::set_locale(&locale.0);
        window.refresh();
    }
}

impl Render for LocaleSelector {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();
        let locale = ui::locale().to_string();

        div()
            .id("locale-selector")
            .track_focus(&focus_handle)
            .on_action(cx.listener(Self::on_select_locale))
            .child(
                Button::new("btn")
                    .small()
                    .ghost()
                    .icon(IconName::Globe)
                    .popup_menu(move |this, _| {
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
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_select_font(&mut self, font_size: &SelectFont, window: &mut Window, cx: &mut Context<Self>) {
        Theme::global_mut(cx).font_size = font_size.0 as f32;
        window.refresh();
    }

    fn on_select_scrollbar_show(&mut self, show: &SelectScrollbarShow, window: &mut Window, cx: &mut Context<Self>) {
        Theme::global_mut(cx).scrollbar_show = show.0;
        window.refresh();
    }
}

impl Render for FontSizeSelector {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();
        let font_size = cx.theme().font_size as i32;
        let scroll_show = cx.theme().scrollbar_show;

        div()
            .id("font-size-selector")
            .track_focus(&focus_handle)
            .on_action(cx.listener(Self::on_select_font))
            .on_action(cx.listener(Self::on_select_scrollbar_show))
            .child(
                Button::new("btn")
                    .small()
                    .ghost()
                    .icon(IconName::Settings2)
                    .popup_menu(move |this, _| {
                        this.menu_with_check(
                            "Font Large",
                            font_size == 18,
                            Box::new(SelectFont(18)),
                        )
                        .menu_with_check("Font Default", font_size == 16, Box::new(SelectFont(16)))
                        .menu_with_check("Font Small", font_size == 14, Box::new(SelectFont(14)))
                        .separator()
                        .menu_with_check(
                            "Scrolling to show Scrollbar",
                            scroll_show == ScrollbarShow::Scrolling,
                            Box::new(SelectScrollbarShow(ScrollbarShow::Scrolling)),
                        )
                        .menu_with_check(
                            "Hover to show Scrollbar",
                            scroll_show == ScrollbarShow::Hover,
                            Box::new(SelectScrollbarShow(ScrollbarShow::Hover)),
                        )
                    })
                    .anchor(Corner::TopRight),
            )
    }
}
