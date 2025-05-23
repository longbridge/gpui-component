mod accordion_story;
mod alert_story;
mod assets;
mod badge_story;
mod button_story;
mod calendar_story;
mod checkbox_story;
mod clipboard_story;
mod color_picker_story;
mod date_picker_story;
mod drawer_story;
mod dropdown_story;
mod form_story;
mod icon_story;
mod image_story;
mod input_story;
mod kbd_story;
mod label_story;
mod list_story;
mod menu_story;
mod modal_story;
mod notification_story;
mod number_input_story;
mod otp_input_story;
mod plot_story;
mod popover_story;
mod progress_story;
mod radio_story;
mod resizable_story;
mod scrollable_story;
mod sidebar_story;
mod slider_story;
mod switch_story;
mod table_story;
mod tabs_story;
mod tag_story;
mod textarea_story;
mod title_bar;
mod toggle_story;
mod tooltip_story;
mod webview_story;
mod welcome_story;

pub use assets::Assets;
use gpui::{
    actions, div, impl_internal_actions, prelude::FluentBuilder as _, px, rems, size, AnyElement,
    AnyView, App, AppContext, Bounds, Context, Div, Entity, EventEmitter, Focusable, Global, Hsla,
    InteractiveElement, IntoElement, KeyBinding, Menu, MenuItem, ParentElement, Render, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, WindowBounds, WindowKind,
    WindowOptions,
};

pub use accordion_story::AccordionStory;
pub use alert_story::AlertStory;
pub use badge_story::BadgeStory;
pub use button_story::ButtonStory;
pub use calendar_story::CalendarStory;
pub use checkbox_story::CheckboxStory;
pub use clipboard_story::ClipboardStory;
pub use color_picker_story::ColorPickerStory;
pub use date_picker_story::DatePickerStory;
pub use drawer_story::DrawerStory;
pub use dropdown_story::DropdownStory;
pub use form_story::FormStory;
pub use icon_story::IconStory;
pub use image_story::ImageStory;
pub use input_story::InputStory;
pub use kbd_story::KbdStory;
pub use label_story::LabelStory;
pub use list_story::ListStory;
pub use menu_story::MenuStory;
pub use modal_story::ModalStory;
pub use notification_story::NotificationStory;
pub use number_input_story::NumberInputStory;
pub use otp_input_story::OtpInputStory;
pub use plot_story::PlotStory;
pub use popover_story::PopoverStory;
pub use progress_story::ProgressStory;
pub use radio_story::RadioStory;
pub use resizable_story::ResizableStory;
pub use scrollable_story::ScrollableStory;
use serde::{Deserialize, Serialize};
pub use sidebar_story::SidebarStory;
pub use slider_story::SliderStory;
pub use switch_story::SwitchStory;
pub use table_story::TableStory;
pub use tabs_story::TabsStory;
pub use tag_story::TagStory;
pub use textarea_story::TextareaStory;
pub use title_bar::AppTitleBar;
pub use toggle_story::ToggleStory;
pub use tooltip_story::TooltipStory;
pub use webview_story::WebViewStory;
pub use welcome_story::WelcomeStory;

use gpui_component::{
    button::Button,
    context_menu::ContextMenuExt,
    dock::{register_panel, Panel, PanelControl, PanelEvent, PanelInfo, PanelState, TitleStyle},
    h_flex,
    notification::Notification,
    popup_menu::PopupMenu,
    scroll::ScrollbarShow,
    v_flex, ActiveTheme, ContextModal, IconName, Root, TitleBar,
};

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

const PANEL_NAME: &str = "StoryContainer";

pub struct AppState {
    pub invisible_panels: Entity<Vec<SharedString>>,
}
impl AppState {
    fn init(cx: &mut App) {
        let state = Self {
            invisible_panels: cx.new(|_| Vec::new()),
        };
        cx.set_global::<AppState>(state);
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }
}

pub fn create_new_window<F, E>(title: &str, crate_view_fn: F, cx: &mut App)
where
    E: Into<AnyView>,
    F: FnOnce(&mut Window, &mut App) -> E + Send + 'static,
{
    let mut window_size = size(px(1600.0), px(1200.0));
    if let Some(display) = cx.primary_display() {
        let display_size = display.bounds().size;
        window_size.width = window_size.width.min(display_size.width * 0.85);
        window_size.height = window_size.height.min(display_size.height * 0.85);
    }
    let window_bounds = Bounds::centered(None, window_size, cx);
    let title = SharedString::from(title.to_string());

    cx.spawn(async move |cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_min_size: Some(gpui::Size {
                width: px(640.),
                height: px(480.),
            }),
            kind: WindowKind::Normal,
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };

        let window = cx
            .open_window(options, |window, cx| {
                let view = crate_view_fn(window, cx);
                let root = cx.new(|cx| StoryRoot::new(title.clone(), view, window, cx));

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

struct StoryRoot {
    title_bar: Entity<AppTitleBar>,
    view: AnyView,
}

impl StoryRoot {
    pub fn new(
        title: impl Into<SharedString>,
        view: impl Into<AnyView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_bar = cx.new(|cx| AppTitleBar::new(title, window, cx));
        Self {
            title_bar,
            view: view.into(),
        }
    }
}

impl Render for StoryRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let drawer_layer = Root::render_drawer_layer(window, cx);
        let modal_layer = Root::render_modal_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(self.title_bar.clone())
                    .child(div().flex_1().overflow_hidden().child(self.view.clone())),
            )
            .children(drawer_layer)
            .children(modal_layer)
            .child(div().absolute().top_8().children(notification_layer))
    }
}

impl Global for AppState {}

pub fn init(cx: &mut App) {
    gpui_component::init(cx);
    AppState::init(cx);
    input_story::init(cx);
    number_input_story::init(cx);
    textarea_story::init(cx);
    dropdown_story::init(cx);
    popover_story::init(cx);
    menu_story::init(cx);
    webview_story::init(cx);
    tooltip_story::init(cx);
    otp_input_story::init(cx);

    let http_client = std::sync::Arc::new(
        reqwest_client::ReqwestClient::user_agent("gpui-component/story").unwrap(),
    );
    cx.set_http_client(http_client);

    cx.bind_keys([
        KeyBinding::new("/", ToggleSearch, None),
        KeyBinding::new("cmd-q", Quit, None),
    ]);

    cx.on_action(|_: &Quit, cx: &mut App| {
        cx.quit();
    });

    register_panel(cx, PANEL_NAME, |_, _, info, window, cx| {
        let story_state = match info {
            PanelInfo::Panel(value) => StoryState::from_value(value.clone()),
            _ => {
                unreachable!("Invalid PanelInfo: {:?}", info)
            }
        };

        let view = cx.new(|cx| {
            let (title, description, closable, zoomable, story, on_active) =
                story_state.to_story(window, cx);
            let mut container = StoryContainer::new(window, cx)
                .story(story, story_state.story_klass)
                .on_active(on_active);

            cx.on_focus_in(
                &container.focus_handle,
                window,
                |this: &mut StoryContainer, _, _| {
                    println!("StoryContainer focus in: {}", this.name);
                },
            )
            .detach();

            container.name = title.into();
            container.description = description.into();
            container.closable = closable;
            container.zoomable = zoomable;
            container
        });
        Box::new(view)
    });

    use gpui_component::input::{Copy, Cut, Paste, Redo, Undo};
    cx.set_menus(vec![
        Menu {
            name: "GPUI App".into(),
            items: vec![MenuItem::action("Quit", Quit)],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", Undo, gpui::OsAction::Undo),
                MenuItem::os_action("Redo", Redo, gpui::OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", Cut, gpui::OsAction::Cut),
                MenuItem::os_action("Copy", Copy, gpui::OsAction::Copy),
                MenuItem::os_action("Paste", Paste, gpui::OsAction::Paste),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![],
        },
    ]);
    cx.activate(true);
}

actions!(story, [ShowPanelInfo]);

#[derive(IntoElement)]
struct StorySection {
    base: Div,
    title: AnyElement,
    children: Vec<AnyElement>,
}

impl StorySection {
    #[allow(unused)]
    fn max_w_md(mut self) -> Self {
        self.base = self.base.max_w(rems(48.));
        self
    }

    #[allow(unused)]
    fn max_w_lg(mut self) -> Self {
        self.base = self.base.max_w(rems(64.));
        self
    }

    #[allow(unused)]
    fn max_w_xl(mut self) -> Self {
        self.base = self.base.max_w(rems(80.));
        self
    }

    #[allow(unused)]
    fn max_w_2xl(mut self) -> Self {
        self.base = self.base.max_w(rems(96.));
        self
    }
}

impl ParentElement for StorySection {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for StorySection {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for StorySection {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .gap_2()
            .mb_5()
            .w_full()
            .child(
                h_flex()
                    .justify_between()
                    .w_full()
                    .gap_4()
                    .child(self.title),
            )
            .child(
                v_flex()
                    .p_4()
                    .overflow_x_hidden()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_lg()
                    .items_center()
                    .justify_center()
                    .child(self.base.children(self.children)),
            )
    }
}

impl ContextMenuExt for StorySection {}

pub(crate) fn section(title: impl IntoElement) -> StorySection {
    StorySection {
        title: title.into_any_element(),
        base: h_flex()
            .flex_wrap()
            .justify_center()
            .items_center()
            .w_full()
            .gap_4(),
        children: vec![],
    }
}

pub struct StoryContainer {
    focus_handle: gpui::FocusHandle,
    pub name: SharedString,
    pub title_bg: Option<Hsla>,
    pub description: SharedString,
    width: Option<gpui::Pixels>,
    height: Option<gpui::Pixels>,
    story: Option<AnyView>,
    story_klass: Option<SharedString>,
    closable: bool,
    zoomable: Option<PanelControl>,
    on_active: Option<fn(AnyView, bool, &mut Window, &mut App)>,
}

#[derive(Debug)]
pub enum ContainerEvent {
    Close,
}

pub trait Story: Focusable + Render + Sized {
    fn klass() -> &'static str {
        std::any::type_name::<Self>().split("::").last().unwrap()
    }

    fn title() -> &'static str;
    fn description() -> &'static str {
        ""
    }
    fn closable() -> bool {
        true
    }
    fn zoomable() -> Option<PanelControl> {
        Some(PanelControl::default())
    }
    fn title_bg() -> Option<Hsla> {
        None
    }
    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable>;

    fn on_active(&mut self, active: bool, window: &mut Window, cx: &mut App) {
        let _ = active;
        let _ = window;
        let _ = cx;
    }
    fn on_active_any(view: AnyView, active: bool, window: &mut Window, cx: &mut App)
    where
        Self: 'static,
    {
        if let Some(story) = view.downcast::<Self>().ok() {
            cx.update_entity(&story, |story, cx| {
                story.on_active(active, window, cx);
            });
        }
    }
}

impl EventEmitter<ContainerEvent> for StoryContainer {}

impl StoryContainer {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();

        Self {
            focus_handle,
            name: "".into(),
            title_bg: None,
            description: "".into(),
            width: None,
            height: None,
            story: None,
            story_klass: None,
            closable: true,
            zoomable: Some(PanelControl::default()),
            on_active: None,
        }
    }

    pub fn panel<S: Story>(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let name = S::title();
        let description = S::description();
        let story = S::new_view(window, cx);
        let story_klass = S::klass();
        let focus_handle = story.focus_handle(cx);

        let view = cx.new(|cx| {
            let mut story = Self::new(window, cx)
                .story(story.into(), story_klass)
                .on_active(S::on_active_any);
            story.focus_handle = focus_handle;
            story.closable = S::closable();
            story.zoomable = S::zoomable();
            story.name = name.into();
            story.description = description.into();
            story.title_bg = S::title_bg();
            story
        });

        view
    }

    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: gpui::Pixels) -> Self {
        self.height = Some(height);
        self
    }

    pub fn story(mut self, story: AnyView, story_klass: impl Into<SharedString>) -> Self {
        self.story = Some(story);
        self.story_klass = Some(story_klass.into());
        self
    }

    pub fn on_active(mut self, on_active: fn(AnyView, bool, &mut Window, &mut App)) -> Self {
        self.on_active = Some(on_active);
        self
    }

    fn on_action_panel_info(
        &mut self,
        _: &ShowPanelInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        struct Info;
        let note = Notification::new(format!("You have clicked panel info on: {}", self.name))
            .id::<Info>();
        window.push_notification(note, cx);
    }

    fn on_action_toggle_search(
        &mut self,
        _: &ToggleSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.propagate();
        if window.has_focused_input(cx) {
            return;
        }

        struct Search;
        let note =
            Notification::new(format!("You have toggled search on: {}", self.name)).id::<Search>();
        window.push_notification(note, cx);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoryState {
    pub story_klass: SharedString,
}

impl StoryState {
    fn to_value(&self) -> serde_json::Value {
        serde_json::json!({
            "story_klass": self.story_klass,
        })
    }

    fn from_value(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap()
    }

    fn to_story(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> (
        &'static str,
        &'static str,
        bool,
        Option<PanelControl>,
        AnyView,
        fn(AnyView, bool, &mut Window, &mut App),
    ) {
        macro_rules! story {
            ($klass:tt) => {
                (
                    $klass::title(),
                    $klass::description(),
                    $klass::closable(),
                    $klass::zoomable(),
                    $klass::view(window, cx).into(),
                    $klass::on_active_any,
                )
            };
        }

        match self.story_klass.to_string().as_str() {
            "ButtonStory" => story!(ButtonStory),
            "CalendarStory" => story!(CalendarStory),
            "DropdownStory" => story!(DropdownStory),
            "IconStory" => story!(IconStory),
            "ImageStory" => story!(ImageStory),
            "InputStory" => story!(InputStory),
            "ListStory" => story!(ListStory),
            "ModalStory" => story!(ModalStory),
            "PopoverStory" => story!(PopoverStory),
            "ProgressStory" => story!(ProgressStory),
            "ResizableStory" => story!(ResizableStory),
            "ScrollableStory" => story!(ScrollableStory),
            "SwitchStory" => story!(SwitchStory),
            "TableStory" => story!(TableStory),
            "LabelStory" => story!(LabelStory),
            "TooltipStory" => story!(TooltipStory),
            "WebViewStory" => story!(WebViewStory),
            "AccordionStory" => story!(AccordionStory),
            "SidebarStory" => story!(SidebarStory),
            "FormStory" => story!(FormStory),
            _ => {
                unreachable!("Invalid story klass: {}", self.story_klass)
            }
        }
    }
}

impl Panel for StoryContainer {
    fn panel_name(&self) -> &'static str {
        "StoryContainer"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        self.name.clone().into_any_element()
    }

    fn title_style(&self, cx: &App) -> Option<TitleStyle> {
        if let Some(bg) = self.title_bg {
            Some(TitleStyle {
                background: bg,
                foreground: cx.theme().foreground,
            })
        } else {
            None
        }
    }

    fn closable(&self, _cx: &App) -> bool {
        self.closable
    }

    fn zoomable(&self, _cx: &App) -> Option<PanelControl> {
        self.zoomable
    }

    fn visible(&self, cx: &App) -> bool {
        !AppState::global(cx)
            .invisible_panels
            .read(cx)
            .contains(&self.name)
    }

    fn set_zoomed(&mut self, zoomed: bool, _window: &mut Window, _cx: &mut App) {
        println!("panel: {} zoomed: {}", self.name, zoomed);
    }

    fn set_active(&mut self, active: bool, _window: &mut Window, cx: &mut App) {
        println!("panel: {} active: {}", self.name, active);
        if let Some(on_active) = self.on_active {
            if let Some(story) = self.story.clone() {
                on_active(story, active, _window, cx);
            }
        }
    }

    fn popup_menu(&self, menu: PopupMenu, _window: &Window, _cx: &App) -> PopupMenu {
        menu.menu("Info", Box::new(ShowPanelInfo))
    }

    fn toolbar_buttons(&self, _window: &mut Window, _cx: &mut App) -> Option<Vec<Button>> {
        Some(vec![
            Button::new("info")
                .icon(IconName::Info)
                .on_click(|_, window, cx| {
                    window.push_notification("You have clicked info button", cx);
                }),
            Button::new("search")
                .icon(IconName::Search)
                .on_click(|_, window, cx| {
                    window.push_notification("You have clicked search button", cx);
                }),
        ])
    }

    fn dump(&self, _cx: &App) -> PanelState {
        let mut state = PanelState::new(self);
        let story_state = StoryState {
            story_klass: self.story_klass.clone().unwrap(),
        };
        state.info = PanelInfo::panel(story_state.to_value());
        state
    }
}

impl EventEmitter<PanelEvent> for StoryContainer {}
impl Focusable for StoryContainer {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for StoryContainer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("story-container")
            .size_full()
            .overflow_y_scroll()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_panel_info))
            .on_action(cx.listener(Self::on_action_toggle_search))
            .when_some(self.story.clone(), |this, story| {
                this.child(
                    v_flex()
                        .id("story-children")
                        .w_full()
                        .flex_1()
                        .p_4()
                        .child(story),
                )
            })
    }
}
