use crate::ui::views::settings::Settings;
use crate::ui::AppExt;
use crate::ui::{SelectFont, SelectLocale, SelectRadius, SelectScrollbarShow};
use gpui::*;
use gpui::{
    div, img, prelude::FluentBuilder as _, px, relative, AnyElement, App, AppContext, ClickEvent,
    Context, Corner, Div, Element, Entity, FocusHandle, Hsla, Image, IntoElement, MouseButton,
    ParentElement, Pixels, Render, RenderOnce, SharedString, Stateful, Style, Styled, Subscription,
    TitlebarOptions, Window,
};

use gpui_component::{
    badge::Badge,
    button::{Button, ButtonVariants as _},
    h_flex, locale,
    popup_menu::PopupMenuExt as _,
    scroll::ScrollbarShow,
    set_locale, ActiveTheme, ContextModal as _, Icon, IconName, InteractiveElementExt as _,
    Sizable as _, Theme,
};
use std::rc::Rc;
use std::sync::Arc;

pub const TITLE_BAR_HEIGHT: Pixels = px(34.);
#[cfg(target_os = "macos")]
const TITLE_BAR_LEFT_PADDING: Pixels = px(80.);
#[cfg(not(target_os = "macos"))]
const TITLE_BAR_LEFT_PADDING: Pixels = px(12.);

pub struct AppTitleBar {
    locale_selector: Entity<LocaleSelector>,
    child: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    _subscriptions: Vec<Subscription>,
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
           // .show_maximize(false)
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
                    .child(self.locale_selector.clone())
                    .child(
                        Button::new("setting")
                            .icon(IconName::Settings)
                            .small()
                            .ghost()
                            .on_click(|_, _, cx| {
                                cx.activate(true);

                                let window_size = size(px(800.0), px(800.0));
                                let window_bounds = Bounds::centered(None, window_size, cx);
                                let options = WindowOptions {
                                    app_id: Some("x-todo-app".to_string()),
                                    window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                                    titlebar:None,
                                    // window_min_size: Some(gpui::Size {
                                    //     width: px(800.),
                                    //     height: px(800.),
                                    // }),

                                    kind: WindowKind::PopUp,
                                    #[cfg(target_os = "linux")]
                                    window_background:
                                        gpui::WindowBackgroundAppearance::Transparent,
                                    #[cfg(target_os = "linux")]
                                    window_decorations: Some(gpui::WindowDecorations::Client),
                                    ..Default::default()
                                };
                                cx.create_normal_window("Settings", options, move |window, cx| {
                                    Settings::view(window, cx)
                                });
                            }),
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
            //.show_minimize(false)
        //.show_maximize(false)
            // left side
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

/// TitleBar used to customize the appearance of the title bar.
///
/// We can put some elements inside the title bar.
#[derive(IntoElement)]
pub struct TitleBar {
    base: Stateful<Div>,
    children: Vec<AnyElement>,
    window_controls: WindowControls,
    // on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,
}

impl TitleBar {
    pub fn new() -> Self {
        Self {
            base: div().id("title-bar").pl(TITLE_BAR_LEFT_PADDING),
            children: Vec::new(),
            window_controls: WindowControls::new(),
        }
    }

    /// Returns the default title bar options for compatible with the [`crate::TitleBar`].
    pub fn title_bar_options() -> TitlebarOptions {
        TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: Some(gpui::point(px(9.0), px(9.0))),
        }
    }

    pub fn show_minimize(mut self, show: bool) -> Self {
        self.window_controls = self.window_controls.show_minimize(show);
        self
    }

    pub fn show_maximize(mut self, show: bool) -> Self {
        self.window_controls = self.window_controls.show_maximize(show);
        self
    }

    pub fn show_close(mut self, show: bool) -> Self {
        self.window_controls = self.window_controls.show_close(show);
        self
    }

    /// Add custom for close window event, default is None, then click X button will call `window.remove_window()`.
    /// Linux only, this will do nothing on other platforms.
    pub fn on_close_window(
        mut self,
        f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        if cfg!(target_os = "linux") {
            self.window_controls = self
                .window_controls
                .on_close_window(Some(Rc::new(Box::new(f))));
        }
        self
    }
}

// The Windows control buttons have a fixed width of 35px.
//
// We don't need implementation the click event for the control buttons.
// If user clicked in the bounds, the window event will be triggered.
#[derive(IntoElement, Clone)]
enum ControlIcon {
    Minimize,
    Restore,
    Maximize,
    Close {
        on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,
    },
}

impl ControlIcon {
    fn minimize() -> Self {
        Self::Minimize
    }

    fn restore() -> Self {
        Self::Restore
    }

    fn maximize() -> Self {
        Self::Maximize
    }

    fn close(on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>) -> Self {
        Self::Close { on_close_window }
    }

    fn id(&self) -> &'static str {
        match self {
            Self::Minimize => "minimize",
            Self::Restore => "restore",
            Self::Maximize => "maximize",
            Self::Close { .. } => "close",
        }
    }

    fn icon(&self) -> IconName {
        match self {
            Self::Minimize => IconName::WindowMinimize,
            Self::Restore => IconName::WindowRestore,
            Self::Maximize => IconName::WindowMaximize,
            Self::Close { .. } => IconName::WindowClose,
        }
    }

    fn is_close(&self) -> bool {
        matches!(self, Self::Close { .. })
    }

    fn fg(&self, cx: &App) -> Hsla {
        if cx.theme().mode.is_dark() {
            gpui_component::white()
        } else {
            gpui_component::black()
        }
    }

    fn hover_fg(&self, cx: &App) -> Hsla {
        if self.is_close() || cx.theme().mode.is_dark() {
            gpui_component::white()
        } else {
            gpui_component::black()
        }
    }

    fn hover_bg(&self, cx: &App) -> Hsla {
        if self.is_close() {
            if cx.theme().mode.is_dark() {
                gpui_component::red_800()
            } else {
                gpui_component::red_600()
            }
        } else if cx.theme().mode.is_dark() {
            gpui_component::stone_700()
        } else {
            gpui_component::stone_200()
        }
    }
}

impl RenderOnce for ControlIcon {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let fg = self.fg(cx);
        let hover_fg = self.hover_fg(cx);
        let hover_bg = self.hover_bg(cx);
        let icon = self.clone();
        let is_linux = true; //cfg!(target_os = "linux");
        let on_close_window = match &icon {
            ControlIcon::Close { on_close_window } => on_close_window.clone(),
            _ => None,
        };

        div()
            .id(self.id())
            .flex()
            .w(TITLE_BAR_HEIGHT)
            .h_full()
            .justify_center()
            .content_center()
            .items_center()
            .text_color(fg)
            .when(is_linux, |this| {
                this.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_click(move |_, window, cx| match icon {
                    Self::Minimize => {
                        println!("Minimize window");
                      //  window.minimize_window()
                    }
                    Self::Restore => {
                        println!("Restore window");
                       // window.zoom_window()
                    }
                    Self::Maximize => {
                        println!("Maximize window");
                       // window.zoom_window()
                    }
                    Self::Close { .. } => {
                        println!("Close window");
                        // if let Some(f) = on_close_window.clone() {
                        //     f(&ClickEvent::default(), window, cx);
                        // } else {
                        //     window.remove_window();
                        // }
                    }
                })
            })
            .hover(|style| style.bg(hover_bg).text_color(hover_fg))
            .active(|style| style.bg(hover_bg.opacity(0.7)))
            .child(Icon::new(self.icon()).small())
    }
}

#[derive(IntoElement, Clone)]
struct WindowControls {
    show_minimize: bool,
    show_maximize: bool,
    show_close: bool,
    on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,
}

impl WindowControls {
    pub fn new() -> Self {
        Self {
            show_minimize: true,
            show_maximize: true,
            show_close: true,
            on_close_window: None,
        }
    }

    fn show_minimize(mut self, show: bool) -> Self {
        self.show_minimize = show;
        self
    }
    fn show_maximize(mut self, show: bool) -> Self {
        self.show_maximize = show;
        self
    }
    fn show_close(mut self, show: bool) -> Self {
        self.show_close = show;
        self
    }

    fn on_close_window(
        mut self,
        f: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,
    ) -> Self {
        if cfg!(target_os = "linux") {
            self.on_close_window = f;
        }
        self
    }
}

impl RenderOnce for WindowControls {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        if cfg!(target_os = "macos") {
            return div().id("window-controls");
        }
        h_flex()
            .id("window-controls")
            .items_center()
            .flex_shrink_0()
            .h_full()
            .when(self.show_minimize || self.show_maximize, |div| {
                div.child(
                    h_flex()
                        .justify_center()
                        .content_stretch()
                        .h_full()
                        .when(self.show_minimize, |div| div.child(ControlIcon::minimize()))
                        .when(self.show_maximize, |div| {
                            div.child(if window.is_maximized() {
                                ControlIcon::restore()
                            } else {
                                ControlIcon::maximize()
                            })
                        }),
                )
            })
            .when(self.show_close, |div| {
                div.child(ControlIcon::close(self.on_close_window))
            })
    }
}

impl Styled for TitleBar {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for TitleBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for TitleBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_linux = true; //cfg!(target_os = "linux");

        const HEIGHT: Pixels = px(34.);

        div().flex_shrink_0().child(
            self.base
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .h(HEIGHT)
                .border_b_1()
                .border_color(cx.theme().title_bar_border)
                .bg(cx.theme().title_bar)
                .when(window.is_fullscreen(), |this| this.pl(px(12.)))
                //.on_double_click(|_, window, _| window.zoom_window())
                .child(
                    h_flex()
                        .h_full()
                        .justify_between()
                        .flex_shrink_0()
                        .flex_1()
                        .when(is_linux, |this| {
                            this.child(
                                div()
                                    .top_0()
                                    .left_0()
                                    .absolute()
                                    .size_full()
                                    .h_full()
                                    .child(TitleBarElement {}),
                            )
                        })
                        .children(self.children),
                )
                .child(self.window_controls.clone()),
        )
    }
}

/// A TitleBar Element that can be move the window.
pub struct TitleBarElement {}

impl IntoElement for TitleBarElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TitleBarElement {
    type RequestLayoutState = ();

    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.flex_grow = 1.0;
        style.flex_shrink = 1.0;
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();

        let id = window.request_layout(style, [], cx);
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    #[allow(unused_variables)]
    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: gpui::Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui::{MouseButton, MouseMoveEvent, MouseUpEvent};
        window.on_mouse_event(
            move |ev: &MouseMoveEvent, _, window: &mut Window, cx: &mut App| {
                if bounds.contains(&ev.position) && ev.pressed_button == Some(MouseButton::Left) {
                    window.start_window_move();
                }
            },
        );

        window.on_mouse_event(
            move |ev: &MouseUpEvent, _, window: &mut Window, cx: &mut App| {
                if bounds.contains(&ev.position) && ev.button == MouseButton::Right {
                    window.show_window_menu(ev.position);
                }
            },
        );
    }
}
