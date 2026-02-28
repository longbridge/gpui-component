use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Pixels, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, div, px,
};
use gpui_component::{ActiveTheme, Icon, IconName, Sizable, Size, v_flex};
use one_core::ai_chat::ask_ai::{AskAiEvent, get_ask_ai_notifier};
use one_core::ai_chat::{AiChatPanel, AiChatPanelEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarPanel {
    AiChat,
}

impl SidebarPanel {
    pub fn icon(&self) -> IconName {
        match self {
            SidebarPanel::AiChat => IconName::Bot,
        }
    }
}

pub const SIDEBAR_DEFAULT_WIDTH: Pixels = px(400.0);
pub const SIDEBAR_MIN_WIDTH: Pixels = px(250.0);
pub const SIDEBAR_MAX_WIDTH: Pixels = px(600.0);
pub const TOOLBAR_WIDTH: Pixels = px(48.0);

#[derive(Clone, Debug)]
pub enum MongoSidebarEvent {
    PanelChanged,
    AskAi,
}

pub struct MongoSidebar {
    active_panel: Option<SidebarPanel>,
    ai_chat_panel: Entity<AiChatPanel>,
    focus_handle: FocusHandle,
    is_active: bool,
    _subs: Vec<Subscription>,
}

impl MongoSidebar {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let ai_chat_panel = cx.new(|cx| AiChatPanel::new(window, cx));

        let mut subs = Vec::new();

        subs.push(
            cx.subscribe(&ai_chat_panel, |this, _, event: &AiChatPanelEvent, cx| {
                if let AiChatPanelEvent::Close = event {
                    this.active_panel = None;
                    cx.emit(MongoSidebarEvent::PanelChanged);
                    cx.notify();
                }
            }),
        );

        if let Some(notifier) = get_ask_ai_notifier(cx) {
            subs.push(
                cx.subscribe(&notifier, move |this, _, event: &AskAiEvent, cx| {
                    if this.is_active {
                        let AskAiEvent::Request(message) = event;
                        this.ask_ai(message.clone(), cx);
                    }
                }),
            );
        }

        Self {
            active_panel: None,
            ai_chat_panel,
            focus_handle: cx.focus_handle(),
            is_active: false,
            _subs: subs,
        }
    }

    pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        self.is_active = active;
        cx.notify();
    }

    pub fn set_active_panel(&mut self, panel: Option<SidebarPanel>, cx: &mut Context<Self>) {
        if self.active_panel != panel {
            self.active_panel = panel;
            cx.emit(MongoSidebarEvent::PanelChanged);
            cx.notify();
        }
    }

    pub fn toggle_panel(&mut self, panel: SidebarPanel, cx: &mut Context<Self>) {
        if self.active_panel == Some(panel) {
            self.set_active_panel(None, cx);
        } else {
            self.set_active_panel(Some(panel), cx);
        }
    }

    pub fn is_panel_visible(&self) -> bool {
        self.active_panel.is_some()
    }

    pub fn ask_ai(&mut self, message: String, cx: &mut Context<Self>) {
        if self.active_panel != Some(SidebarPanel::AiChat) {
            self.active_panel = Some(SidebarPanel::AiChat);
        }

        self.ai_chat_panel.update(cx, |panel, cx| {
            panel.send_external_message(message, cx);
        });

        cx.emit(MongoSidebarEvent::AskAi);
        cx.notify();
    }

    fn render_toolbar_button(
        &self,
        panel: SidebarPanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = self.active_panel == Some(panel);
        let accent_color = cx.theme().accent;
        let accent_fg = cx.theme().accent_foreground;
        let muted_fg = cx.theme().muted_foreground;
        let muted_bg = cx.theme().muted;

        div()
            .id(SharedString::from(format!(
                "mongodb-sidebar-btn-{:?}",
                panel
            )))
            .w(px(36.0))
            .h(px(36.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .cursor_pointer()
            .when(is_active, |this| this.bg(accent_color))
            .when(!is_active, |this| this.hover(|s| s.bg(muted_bg)))
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.toggle_panel(panel, cx);
            }))
            .child(
                Icon::new(panel.icon())
                    .with_size(Size::Medium)
                    .text_color(if is_active { accent_fg } else { muted_fg }),
            )
    }

    pub fn render_toolbar(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let border_color = cx.theme().border;
        let muted_bg = cx.theme().muted;

        v_flex()
            .flex_shrink_0()
            .w(TOOLBAR_WIDTH)
            .h_full()
            .bg(muted_bg)
            .border_l_1()
            .border_color(border_color)
            .items_center()
            .py_2()
            .gap_1()
            .child(self.render_toolbar_button(SidebarPanel::AiChat, window, cx))
            .into_any_element()
    }

    pub fn render_panel_content(
        &self,
        panel: SidebarPanel,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        match panel {
            SidebarPanel::AiChat => self.ai_chat_panel.clone().into_any_element(),
        }
    }
}

impl EventEmitter<MongoSidebarEvent> for MongoSidebar {}

impl Focusable for MongoSidebar {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MongoSidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let border_color = cx.theme().border;
        let bg_color = cx.theme().background;

        div()
            .h_full()
            .flex_shrink_0()
            .when_some(self.active_panel, |this, panel| {
                this.w_full().child(
                    v_flex()
                        .size_full()
                        .border_l_1()
                        .border_color(border_color)
                        .bg(bg_color)
                        .child(self.render_panel_content(panel, window, cx)),
                )
            })
            .when(!self.is_panel_visible(), |this| {
                this.child(self.render_toolbar(window, cx))
            })
    }
}
