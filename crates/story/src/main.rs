use gpui::{prelude::*, *};
use gpui_component::{
    h_flex,
    input::{InputEvent, TextInput},
    sidebar::{Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
    v_flex, ActiveTheme as _, Icon, IconName,
};
use story::*;

pub struct Gallery {
    stories: Vec<(&'static str, Vec<Entity<StoryContainer>>)>,
    active_group_index: Option<usize>,
    active_index: Option<usize>,
    collapsed: bool,
    search_input: Entity<TextInput>,
    _subscriptions: Vec<Subscription>,
}

impl Gallery {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| {
            TextInput::new(window, cx)
                .appearance(false)
                .cleanable()
                .placeholder("Search...")
        });
        let _subscriptions = vec![cx.subscribe(&search_input, |this, _, e, cx| match e {
            InputEvent::Change(_) => {
                this.active_group_index = Some(0);
                this.active_index = Some(0);
                cx.notify()
            }
            _ => {}
        })];
        let stories = vec![
            (
                "Getting Started",
                vec![StoryContainer::panel::<WelcomeStory>(window, cx)],
            ),
            (
                "Components",
                vec![
                    StoryContainer::panel::<AccordionStory>(window, cx),
                    StoryContainer::panel::<AlertStory>(window, cx),
                    StoryContainer::panel::<BadgeStory>(window, cx),
                    StoryContainer::panel::<ButtonStory>(window, cx),
                    StoryContainer::panel::<CalendarStory>(window, cx),
                    StoryContainer::panel::<CheckboxStory>(window, cx),
                    StoryContainer::panel::<ClipboardStory>(window, cx),
                    StoryContainer::panel::<DatePickerStory>(window, cx),
                    StoryContainer::panel::<DropdownStory>(window, cx),
                    StoryContainer::panel::<DrawerStory>(window, cx),
                    StoryContainer::panel::<FormStory>(window, cx),
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
                ],
            ),
        ];

        Self {
            search_input,
            stories,
            active_group_index: Some(0),
            active_index: Some(0),
            collapsed: false,
            _subscriptions,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search_input.read(cx).text().trim().to_lowercase();

        let stories: Vec<_> = self
            .stories
            .iter()
            .filter_map(|(name, items)| {
                let filtered_items: Vec<_> = items
                    .iter()
                    .filter(|story| story.read(cx).name.to_lowercase().contains(&query))
                    .cloned()
                    .collect();

                if !filtered_items.is_empty() {
                    Some((name, filtered_items))
                } else {
                    None
                }
            })
            .collect();

        let active_group = self.active_group_index.and_then(|index| stories.get(index));
        let active_story = self
            .active_index
            .and(active_group)
            .and_then(|group| group.1.get(self.active_index.unwrap()));
        let (story_name, description) =
            if let Some(story) = active_story.as_ref().map(|story| story.read(cx)) {
                (story.name.clone(), story.description.clone())
            } else {
                ("".into(), "".into())
            };

        h_flex()
            .id("gallery-container")
            .size_full()
            .child(
                Sidebar::left()
                    .collapsed(self.collapsed)
                    .header(
                        v_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                SidebarHeader::new()
                                    .w_full()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(cx.theme().radius)
                                            .bg(cx.theme().primary)
                                            .text_color(cx.theme().primary_foreground)
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
                                            })
                                            .rounded_lg(),
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
                                                .child(
                                                    div()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child("Gallery")
                                                        .text_xs(),
                                                ),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .bg(cx.theme().sidebar_border)
                                    .px_1()
                                    .rounded_full()
                                    .flex_1()
                                    .mx_1()
                                    .child(self.search_input.clone()),
                            ),
                    )
                    .children(stories.clone().into_iter().enumerate().map(
                        |(group_ix, (group_name, sub_stories))| {
                            SidebarGroup::new(*group_name).child(SidebarMenu::new().children(
                                sub_stories.iter().enumerate().map(|(ix, story)| {
                                    SidebarMenuItem::new(story.read(cx).name.clone())
                                        .active(
                                            self.active_group_index == Some(group_ix)
                                                && self.active_index == Some(ix),
                                        )
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, _, cx| {
                                                this.active_group_index = Some(group_ix);
                                                this.active_index = Some(ix);
                                                cx.notify();
                                            },
                                        ))
                                }),
                            ))
                        },
                    )),
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
                            .when_some(active_story, |this, active_story| {
                                this.child(active_story.clone())
                            }),
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
