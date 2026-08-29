use gpui::{
    App, AppContext as _, Axis, Context, Entity, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, SharedString, Styled as _, Subscription, Window, div,
    px,
};

use gpui_component::{
    Sizable as _, Size, StyledExt as _,
    button::Button,
    carousel::{
        Carousel, CarouselContent, CarouselEvent, CarouselItem, CarouselNext, CarouselPagination,
        CarouselPaginationItem, CarouselPrevious, CarouselState,
    },
    h_flex, v_flex,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct CarouselStory {
    focus_handle: FocusHandle,
    horizontal: Entity<CarouselState>,
    vertical: Entity<CarouselState>,
    looped: Entity<CarouselState>,
    controlled: Entity<CarouselState>,
    keyboard: Entity<CarouselState>,
    controlled_index: usize,
    size: Size,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for CarouselStory {
    fn title() -> &'static str {
        "Carousel"
    }

    fn description() -> &'static str {
        "A carousel for browsing a set of related items with keyboard and pointer navigation."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl CarouselStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let horizontal = cx.new(|_| CarouselState::new(3));
            let vertical = cx.new(|_| CarouselState::new(3).with_axis(Axis::Vertical));
            let looped = cx.new(|_| CarouselState::new(4).with_looping(true));
            let controlled = cx.new(|_| CarouselState::new(3).with_selected_index(1));
            let keyboard = cx.new(|_| CarouselState::new(3));

            let subscription = cx.subscribe(
                &controlled,
                move |this: &mut Self, _, event: &CarouselEvent, cx| {
                    let CarouselEvent::Change(index) = event;
                    this.controlled_index = *index;
                    cx.notify();
                },
            );

            Self {
                focus_handle: cx.focus_handle(),
                horizontal,
                vertical,
                looped,
                controlled,
                keyboard,
                controlled_index: 1,
                size: Size::default(),
                _subscriptions: vec![subscription],
            }
        })
    }

    fn slide(label: impl Into<SharedString>) -> impl IntoElement {
        div()
            .h(px(160.))
            .w_full()
            .flex()
            .items_center()
            .justify_center()
            .rounded_lg()
            .bg(gpui::hsla(0.61, 0.42, 0.45, 1.0))
            .text_color(gpui::white())
            .text_lg()
            .child(label.into())
    }

    fn items(state: &Entity<CarouselState>, prefix: &'static str, count: usize) -> CarouselContent {
        (0..count).fold(CarouselContent::new(state), |content, index| {
            let label = format!("{prefix} · {}", index + 1);
            content
                .child(CarouselItem::new((prefix, index), index, state).child(Self::slide(label)))
        })
    }
}

impl Focusable for CarouselStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CarouselStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_4()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Horizontal")
                    .description("Use Left and Right to move between slides.")
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-horizontal", &self.horizontal)
                            .child(Self::items(&self.horizontal, "Horizontal", 3))
                            .child(CarouselPrevious::new(&self.horizontal).with_size(self.size))
                            .child(CarouselNext::new(&self.horizontal).with_size(self.size))
                            .child(CarouselPagination::new().children((0..3).map(|index| {
                                CarouselPaginationItem::new(
                                    ("horizontal-pagination", index),
                                    index,
                                    &self.horizontal,
                                )
                                .with_size(self.size)
                                .child((index + 1).to_string())
                            }))),
                    ),
            )
            .child(
                section("Vertical")
                    .description("Use Up and Down to navigate a vertical carousel.")
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-vertical", &self.vertical)
                            .child(Self::items(&self.vertical, "Vertical", 3).h(px(160.)))
                            .child(CarouselPrevious::new(&self.vertical).with_size(self.size))
                            .child(CarouselNext::new(&self.vertical).with_size(self.size)),
                    ),
            )
            .child(
                section("Looping")
                    .description("Looping navigation wraps from the last slide to the first.")
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-looped", &self.looped)
                            .child(Self::items(&self.looped, "Looped", 4))
                            .child(CarouselPrevious::new(&self.looped).with_size(self.size))
                            .child(CarouselNext::new(&self.looped).with_size(self.size)),
                    ),
            )
            .child(
                section("Controlled / Programmatic")
                    .description("The selected index is owned by application state and can be changed programmatically.")
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-controlled", &self.controlled)
                            .child(Self::items(&self.controlled, "Controlled", 3))
                            .child(CarouselPrevious::new(&self.controlled).with_size(self.size))
                            .child(CarouselNext::new(&self.controlled).with_size(self.size)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().child(format!("Selected slide: {}", self.controlled_index + 1)))
                            .child(
                                Button::new("controlled-first")
                                    .label("Go to first")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.controlled_index = 0;
                                        let controlled = this.controlled.clone();
                                        controlled.update(cx, |state, cx| {
                                            state.set_selected_index(0, cx);
                                        });
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("controlled-last")
                                    .label("Go to last")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.controlled_index = 2;
                                        let controlled = this.controlled.clone();
                                        controlled.update(cx, |state, cx| {
                                            state.set_selected_index(2, cx);
                                        });
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                section("Keyboard navigation")
                    .description(
                        "Tab to the Carousel or either navigation button, then use Left, Right, Home, and End.",
                    )
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-keyboard", &self.keyboard)
                            .child(Self::items(&self.keyboard, "Keyboard", 3))
                            .child(CarouselPrevious::new(&self.keyboard).with_size(self.size))
                            .child(CarouselNext::new(&self.keyboard).with_size(self.size)),
                    ),
            )
    }
}
