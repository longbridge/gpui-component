use gpui::{prelude::*, *};
use gpui_component::{
    blue_500, h_flex,
    sidebar::{Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
    v_flex, ActiveTheme as _, Icon, IconName,
};
use story::*;

pub struct Gallery {
    stories: Vec<Entity<StoryContainer>>,
    active_index: usize,
    collapsed: bool,
}

impl Gallery {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let stories = vec![
            StoryContainer::panel::<AccordionStory>(window, cx),
            StoryContainer::panel::<ButtonStory>(window, cx),
            StoryContainer::panel::<CheckboxStory>(window, cx),
            StoryContainer::panel::<DrawerStory>(window, cx),
            StoryContainer::panel::<IconStory>(window, cx),
            StoryContainer::panel::<ImageStory>(window, cx),
            StoryContainer::panel::<InputStory>(window, cx),
            StoryContainer::panel::<KbdStory>(window, cx),
            StoryContainer::panel::<LabelStory>(window, cx),
            StoryContainer::panel::<ListStory>(window, cx),
            StoryContainer::panel::<MenuStory>(window, cx),
            StoryContainer::panel::<ModalStory>(window, cx),
            StoryContainer::panel::<NotificationStory>(window, cx),
            StoryContainer::panel::<NumberInputStory>(window, cx),
            StoryContainer::panel::<OtpInputStory>(window, cx),
            StoryContainer::panel::<PopoverStory>(window, cx),
            StoryContainer::panel::<ProgressStory>(window, cx),
            StoryContainer::panel::<RadioStory>(window, cx),
            StoryContainer::panel::<ResizableStory>(window, cx),
            StoryContainer::panel::<ScrollableStory>(window, cx),
            StoryContainer::panel::<SidebarStory>(window, cx),
            StoryContainer::panel::<SliderStory>(window, cx),
            StoryContainer::panel::<SwitchStory>(window, cx),
            StoryContainer::panel::<TableStory>(window, cx),
            StoryContainer::panel::<TabsStory>(window, cx),
            StoryContainer::panel::<TagStory>(window, cx),
            StoryContainer::panel::<TextareaStory>(window, cx),
            StoryContainer::panel::<TooltipStory>(window, cx),
        ];

        Self {
            stories,
            active_index: 0,
            collapsed: false,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_story = self.stories[self.active_index].clone();
        let story_name = active_story.read(cx).name.clone();
        let description = active_story.read(cx).description.clone();

        h_flex()
            .id("gallery-container")
            .size_full()
            .child(
                Sidebar::left()
                    .collapsed(self.collapsed)
                    .header(
                        SidebarHeader::new()
                            .w_full()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(cx.theme().radius)
                                    .bg(blue_500())
                                    .text_color(white())
                                    .size_8()
                                    .flex_shrink_0()
                                    .when(!self.collapsed, |this| {
                                        this.child(Icon::new(IconName::GalleryVerticalEnd))
                                    })
                                    .when(self.collapsed, |this| {
                                        this.size_4()
                                            .bg(cx.theme().transparent)
                                            .text_color(cx.theme().foreground)
                                            .child(Icon::new(IconName::GalleryVerticalEnd))
                                    }),
                            )
                            .when(!self.collapsed, |this| {
                                this.child(
                                    v_flex()
                                        .gap_0()
                                        .text_sm()
                                        .flex_1()
                                        .line_height(relative(1.25))
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child("GPUI Component")
                                        .child(div().child("Gallery").text_xs()),
                                )
                            })
                            .when(!self.collapsed, |this| {
                                this.child(
                                    Icon::new(IconName::ChevronsUpDown).size_4().flex_shrink_0(),
                                )
                            }),
                    )
                    .child(
                        SidebarGroup::new("Components").child(SidebarMenu::new().children(
                            self.stories.iter().enumerate().map(|(ix, story)| {
                                SidebarMenuItem::new(story.read(cx).name.clone())
                                    .active(self.active_index == ix)
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        this.active_index = ix;
                                        cx.notify();
                                    }))
                            }),
                        )),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .overflow_x_hidden()
                    .child(
                        h_flex()
                            .id("header")
                            .p_4()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .justify_between()
                            .items_start()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().text_xl().child(story_name))
                                    .child(
                                        div()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(description),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("story")
                            .flex_1()
                            .overflow_y_scroll()
                            .child(active_story),
                    ),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        story::init(cx);
        cx.activate(true);

        story::create_new_window("Gallery of GPUI Component", Gallery::view, cx);
    });
}
