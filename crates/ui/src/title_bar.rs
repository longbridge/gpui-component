use std::rc::Rc;

use crate::{h_flex, ActiveTheme, Icon, IconName, InteractiveElementExt as _, Sizable as _};
use gpui::{
    div, prelude::FluentBuilder as _, px, relative, AnyElement, ClickEvent, Div, Element, Hsla,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, Stateful,
    StatefulInteractiveElement as _, Style, Styled, WindowContext,
};

pub const TITLE_BAR_HEIGHT: Pixels = px(35.);
#[cfg(target_os = "macos")]
const TITLE_BAR_LEFT_PADDING: Pixels = px(80.);
#[cfg(not(target_os = "macos"))]
const TITLE_BAR_LEFT_PADDING: Pixels = px(12.);

/// TitleBar used to customize the appearance of the title bar.
///
/// We can put some elements inside the title bar.
#[derive(IntoElement)]
pub struct TitleBar {
    base: Stateful<Div>,
    children: Vec<AnyElement>,
    on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut WindowContext)>>>,
}

impl TitleBar {
    pub fn new() -> Self {
        Self {
            base: div().id("title-bar").pl(TITLE_BAR_LEFT_PADDING),
            children: Vec::new(),
            on_close_window: None,
        }
    }

    /// Add custom for close window event, default is None, then click X button will call `cx.remove_window()`.
    /// Linux only, this will do nothing on other platforms.
    pub fn on_close_window(
        mut self,
        f: impl Fn(&ClickEvent, &mut WindowContext) + 'static,
    ) -> Self {
        if cfg!(target_os = "linux") {
            self.on_close_window = Some(Rc::new(Box::new(f)));
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
        on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut WindowContext)>>>,
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

    fn close(on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut WindowContext)>>>) -> Self {
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

    fn fg(&self, cx: &WindowContext) -> Hsla {
        if cx.theme().mode.is_dark() {
            crate::white()
        } else {
            crate::black()
        }
    }

    fn hover_fg(&self, cx: &WindowContext) -> Hsla {
        if self.is_close() || cx.theme().mode.is_dark() {
            crate::white()
        } else {
            crate::black()
        }
    }

    fn hover_bg(&self, cx: &WindowContext) -> Hsla {
        if self.is_close() {
            if cx.theme().mode.is_dark() {
                crate::red_800()
            } else {
                crate::red_600()
            }
        } else if cx.theme().mode.is_dark() {
            crate::stone_700()
        } else {
            crate::stone_200()
        }
    }
}

impl RenderOnce for ControlIcon {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let fg = self.fg(cx);
        let hover_fg = self.hover_fg(cx);
        let hover_bg = self.hover_bg(cx);
        let icon = self.clone();
        let is_linux = cfg!(target_os = "linux");
        let on_close_window = match &icon {
            ControlIcon::Close { on_close_window } => on_close_window.clone(),
            _ => None,
        };

        div()
            .id(self.id())
            .flex()
            .cursor_pointer()
            .w(TITLE_BAR_HEIGHT)
            .h_full()
            .justify_center()
            .content_center()
            .items_center()
            .text_color(fg)
            .when(is_linux, |this| {
                this.on_mouse_down(MouseButton::Left, move |_, cx| {
                    cx.prevent_default();
                    cx.stop_propagation();
                })
                .on_click(move |_, cx| match icon {
                    Self::Minimize => cx.minimize_window(),
                    Self::Restore => cx.zoom_window(),
                    Self::Maximize => cx.zoom_window(),
                    Self::Close { .. } => {
                        if let Some(f) = on_close_window.clone() {
                            f(&ClickEvent::default(), cx);
                        } else {
                            cx.remove_window();
                        }
                    }
                })
            })
            .hover(|style| style.bg(hover_bg).text_color(hover_fg))
            .active(|style| style.bg(hover_bg.opacity(0.7)))
            .child(Icon::new(self.icon()).small())
    }
}

#[derive(IntoElement)]
struct WindowControls {
    on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut WindowContext)>>>,
}

impl RenderOnce for WindowControls {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        if cfg!(target_os = "macos") {
            return div().id("window-controls");
        }

        h_flex()
            .id("window-controls")
            .items_center()
            .flex_shrink_0()
            .h_full()
            .child(
                h_flex()
                    .justify_center()
                    .content_stretch()
                    .h_full()
                    .child(ControlIcon::minimize())
                    .child(if cx.is_maximized() {
                        ControlIcon::restore()
                    } else {
                        ControlIcon::maximize()
                    }),
            )
            .child(ControlIcon::close(self.on_close_window))
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
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let is_linux = cfg!(target_os = "linux");

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
                .when(cx.is_fullscreen(), |this| this.pl(px(12.)))
                .on_double_click(|_, cx| cx.zoom_window())
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
                .child(WindowControls {
                    on_close_window: self.on_close_window,
                }),
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

    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        cx: &mut WindowContext,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.flex_grow = 1.0;
        style.flex_shrink = 1.0;
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();

        let id = cx.request_layout(style, []);
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut WindowContext,
    ) -> Self::PrepaintState {
    }

    #[allow(unused_variables)]
    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        bounds: gpui::Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        cx: &mut WindowContext,
    ) {
        use gpui::{MouseButton, MouseMoveEvent, MouseUpEvent};
        cx.on_mouse_event(move |ev: &MouseMoveEvent, _, cx: &mut WindowContext| {
            if bounds.contains(&ev.position) && ev.pressed_button == Some(MouseButton::Left) {
                cx.start_window_move();
            }
        });

        cx.on_mouse_event(move |ev: &MouseUpEvent, _, cx: &mut WindowContext| {
            if bounds.contains(&ev.position) && ev.button == MouseButton::Right {
                cx.show_window_menu(ev.position);
            }
        });
    }
}
