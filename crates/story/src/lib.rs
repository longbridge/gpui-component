mod accordion_story;
mod assets;
mod button_story;
mod calendar_story;
mod dropdown_story;
mod icon_story;
mod image_story;
mod input_story;
mod list_story;
mod modal_story;
mod popup_story;
mod progress_story;
mod resizable_story;
mod scrollable_story;
mod sidebar_story;
mod switch_story;
mod table_story;
mod text_story;
mod tooltip_story;
mod webview_story;

pub use assets::Assets;

pub use accordion_story::AccordionStory;
pub use button_story::ButtonStory;
pub use calendar_story::CalendarStory;
pub use dropdown_story::DropdownStory;
pub use icon_story::IconStory;
pub use image_story::ImageStory;
pub use input_story::InputStory;
pub use list_story::ListStory;
pub use modal_story::ModalStory;
pub use popup_story::PopupStory;
pub use progress_story::ProgressStory;
pub use resizable_story::ResizableStory;
pub use scrollable_story::ScrollableStory;
use serde::{Deserialize, Serialize};
pub use sidebar_story::SidebarStory;
pub use switch_story::SwitchStory;
pub use table_story::TableStory;
pub use text_story::TextStory;
pub use tooltip_story::TooltipStory;
pub use webview_story::WebViewStory;

use gpui::{
    actions, div, prelude::FluentBuilder as _, px, AnyElement, AnyView, AppContext, Context as _,
    Div, EventEmitter, FocusableView, Global, Hsla, InteractiveElement, IntoElement, Model,
    ParentElement, Render, SharedString, Styled as _, View, ViewContext, VisualContext,
    WindowContext,
};

use ui::{
    button::Button,
    divider::Divider,
    dock::{register_panel, Panel, PanelEvent, PanelInfo, PanelState, TitleStyle},
    h_flex,
    label::Label,
    notification::Notification,
    popup_menu::PopupMenu,
    theme::ActiveTheme,
    v_flex, ContextModal, IconName,
};

const PANEL_NAME: &str = "StoryContainer";

pub struct AppState {
    pub invisible_panels: Model<Vec<SharedString>>,
}
impl AppState {
    fn init(cx: &mut AppContext) {
        let state = Self {
            invisible_panels: cx.new_model(|_| Vec::new()),
        };
        cx.set_global::<AppState>(state);
    }

    pub fn global(cx: &AppContext) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut AppContext) -> &mut Self {
        cx.global_mut::<Self>()
    }
}

impl Global for AppState {}

pub fn init(cx: &mut AppContext) {
    AppState::init(cx);
    input_story::init(cx);
    dropdown_story::init(cx);
    popup_story::init(cx);

    register_panel(cx, PANEL_NAME, |_, _, info, cx| {
        let story_state = match info {
            PanelInfo::Panel(value) => StoryState::from_value(value.clone()),
            _ => {
                unreachable!("Invalid PanelInfo: {:?}", info)
            }
        };

        let view = cx.new_view(|cx| {
            let (title, description, closable, zoomable, story) = story_state.to_story(cx);
            let mut container = StoryContainer::new(cx).story(story, story_state.story_klass);

            cx.on_focus_in(&container.focus_handle, |this: &mut StoryContainer, _| {
                println!("StoryContainer focus in: {}", this.name);
            })
            .detach();

            container.name = title.into();
            container.description = description.into();
            container.closable = closable;
            container.zoomable = zoomable;
            container
        });
        Box::new(view)
    });
}

actions!(story, [ShowPanelInfo]);

pub fn section(title: impl IntoElement, cx: &WindowContext) -> Div {
    use ui::theme::ActiveTheme;
    let theme = cx.theme();

    h_flex()
        .items_center()
        .gap_4()
        .p_4()
        .w_full()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .flex_wrap()
        .justify_around()
        .child(div().flex_none().w_full().child(title))
}

pub struct StoryContainer {
    focus_handle: gpui::FocusHandle,
    name: SharedString,
    title_bg: Option<Hsla>,
    description: SharedString,
    width: Option<gpui::Pixels>,
    height: Option<gpui::Pixels>,
    story: Option<AnyView>,
    story_klass: Option<SharedString>,
    closable: bool,
    zoomable: bool,
}

#[derive(Debug)]
pub enum ContainerEvent {
    Close,
}

pub trait Story: FocusableView {
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
    fn zoomable() -> bool {
        true
    }
    fn title_bg() -> Option<Hsla> {
        None
    }
    fn new_view(cx: &mut WindowContext) -> View<impl FocusableView>;
}

impl EventEmitter<ContainerEvent> for StoryContainer {}

impl StoryContainer {
    pub fn new(cx: &mut WindowContext) -> Self {
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
            zoomable: true,
        }
    }

    pub fn panel<S: Story>(cx: &mut WindowContext) -> View<Self> {
        let name = S::title();
        let description = S::description();
        let story = S::new_view(cx);
        let story_klass = S::klass();
        let focus_handle = story.focus_handle(cx);

        let view = cx.new_view(|cx| {
            let mut story = Self::new(cx).story(story.into(), story_klass);
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

    fn on_action_panel_info(&mut self, _: &ShowPanelInfo, cx: &mut ViewContext<Self>) {
        struct Info;
        let note = Notification::new(format!("You have clicked panel info on: {}", self.name))
            .id::<Info>();
        cx.push_notification(note);
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
        cx: &mut WindowContext,
    ) -> (&'static str, &'static str, bool, bool, AnyView) {
        macro_rules! story {
            ($klass:tt) => {
                (
                    $klass::title(),
                    $klass::description(),
                    $klass::closable(),
                    $klass::zoomable(),
                    $klass::view(cx).into(),
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
            "PopupStory" => story!(PopupStory),
            "ProgressStory" => story!(ProgressStory),
            "ResizableStory" => story!(ResizableStory),
            "ScrollableStory" => story!(ScrollableStory),
            "SwitchStory" => story!(SwitchStory),
            "TableStory" => story!(TableStory),
            "TextStory" => story!(TextStory),
            "TooltipStory" => story!(TooltipStory),
            "WebViewStory" => story!(WebViewStory),
            "AccordionStory" => story!(AccordionStory),
            "SidebarStory" => story!(SidebarStory),
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

    fn title(&self, _cx: &WindowContext) -> AnyElement {
        self.name.clone().into_any_element()
    }

    fn title_style(&self, cx: &AppContext) -> Option<TitleStyle> {
        if let Some(bg) = self.title_bg {
            Some(TitleStyle {
                background: bg,
                foreground: cx.theme().foreground,
            })
        } else {
            None
        }
    }

    fn closable(&self, _cx: &AppContext) -> bool {
        self.closable
    }

    fn zoomable(&self, _cx: &AppContext) -> bool {
        self.zoomable
    }

    fn visible(&self, cx: &AppContext) -> bool {
        !AppState::global(cx)
            .invisible_panels
            .read(cx)
            .contains(&self.name)
    }

    fn set_zoomed(&self, zoomed: bool, _cx: &ViewContext<Self>) {
        println!("panel: {} zoomed: {}", self.name, zoomed);
    }

    fn set_active(&self, active: bool, _cx: &ViewContext<Self>) {
        println!("panel: {} active: {}", self.name, active);
    }

    fn popup_menu(&self, menu: PopupMenu, _cx: &WindowContext) -> PopupMenu {
        menu.track_focus(&self.focus_handle)
            .menu("Info", Box::new(ShowPanelInfo))
    }

    fn toolbar_buttons(&self, _cx: &WindowContext) -> Vec<Button> {
        vec![
            Button::new("info").icon(IconName::Info).on_click(|_, cx| {
                cx.push_notification("You have clicked info button");
            }),
            Button::new("search")
                .icon(IconName::Search)
                .on_click(|_, cx| {
                    cx.push_notification("You have clicked search button");
                }),
        ]
    }

    fn dump(&self, _cx: &AppContext) -> PanelState {
        let mut state = PanelState::new(self);
        let story_state = StoryState {
            story_klass: self.story_klass.clone().unwrap(),
        };
        state.info = PanelInfo::panel(story_state.to_value());
        state
    }
}

impl EventEmitter<PanelEvent> for StoryContainer {}
impl FocusableView for StoryContainer {
    fn focus_handle(&self, _: &AppContext) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for StoryContainer {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .id("story-container")
            .size_full()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_panel_info))
            .when(self.description.len() > 0, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .p_4()
                        .child(Label::new(self.description.clone()).text_size(px(16.0)))
                        .child(Divider::horizontal().label("This is a divider")),
                )
            })
            .when_some(self.story.clone(), |this, story| {
                this.child(v_flex().id("story-children").size_full().p_4().child(story))
            })
    }
}
