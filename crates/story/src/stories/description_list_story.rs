use gpui::*;
use gpui::{
    Action, App, AppContext, Axis, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, Styled, Window,
};
use gpui_component::{AxisExt, h_flex, menu::DropdownMenu as _};
use gpui_component::{
    Sizable as _, Size,
    button::Button,
    checkbox::Checkbox,
    description_list::{DescriptionItem, DescriptionList},
    dock::PanelControl,
    text::TextView,
    v_flex,
};
use serde::Deserialize;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = description_list_story, no_json)]
struct ChangeSize(Size);

pub struct DescriptionListStory {
    focus_handle: FocusHandle,
    layout: Axis,
    bordered: bool,
    size: Size,
    items: Vec<(&'static str, &'static str, usize)>,
}

impl DescriptionListStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let items = vec![
            ("Name", "GPUI Component", 1),
            (
                "Description",
                "UI components for building fantastic desktop application by using [GPUI](https://gpui.rs).\
                \n\n \
                Contains a lot of useful UI components, such as **Button**, **Input**, **Table**, **List**, **Select**, **DatePicker** ... \
                \n\n \
                You can easily create your native desktop application by using GPUI Component.
                ",
                3,
            ),
            ("Version", "0.1.0", 1),
            ("License", "Apache-2.0", 1),
            ("Author", "Longbridge", 1),
            ("--", "--", 1),
            (
                "Repository",
                "https://github.com/longbridge/gpui-component",
                2,
            ),
            (
                "Category",
                "UI, Desktop, Framework",
                1,
            ),
            (
                "This is a long label for Platform",
                "macOS, Windows, Linux",
                1,
            ),
        ];

        Self {
            items,
            bordered: true,
            size: Size::default(),
            layout: Axis::Horizontal,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn set_layout(&mut self, layout: Axis, cx: &mut Context<Self>) {
        self.layout = layout;
        cx.notify();
    }

    fn set_bordered(&mut self, bordered: bool, cx: &mut Context<Self>) {
        self.bordered = bordered;
        cx.notify();
    }

    fn on_change_size(&mut self, a: &ChangeSize, _: &mut Window, cx: &mut Context<Self>) {
        self.size = a.0;
        cx.notify();
    }
}

impl super::Story for DescriptionListStory {
    fn title() -> &'static str {
        "DescriptionList"
    }

    fn description() -> &'static str {
        "Use to display details with a tidy layout."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for DescriptionListStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DescriptionListStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("example")
            .on_action(cx.listener(Self::on_change_size))
            .p_4()
            .size_full()
            .gap_2()
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Checkbox::new("layout")
                            .checked(self.layout.is_vertical())
                            .label("Vertical Layout")
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                let new_layout = if *checked {
                                    Axis::Vertical
                                } else {
                                    Axis::Horizontal
                                };
                                this.set_layout(new_layout, cx);
                            })),
                    )
                    .child(
                        Checkbox::new("bordered")
                            .checked(self.bordered)
                            .label("Bordered")
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.set_bordered(*checked, cx);
                            })),
                    )
                    .child(
                        Button::new("size")
                            .small()
                            .outline()
                            .label(format!("size: {:?}", self.size))
                            .dropdown_menu({
                                let size = self.size;
                                move |menu, _, _| {
                                    menu.menu_with_check(
                                        "Large",
                                        size == Size::Large,
                                        Box::new(ChangeSize(Size::Large)),
                                    )
                                    .menu_with_check(
                                        "Medium",
                                        size == Size::Medium,
                                        Box::new(ChangeSize(Size::Medium)),
                                    )
                                    .menu_with_check(
                                        "Small",
                                        size == Size::Small,
                                        Box::new(ChangeSize(Size::Small)),
                                    )
                                }
                            }),
                    ),
            )
            .child(
                DescriptionList::new()
                    .columns(3)
                    .layout(self.layout)
                    .bordered(self.bordered)
                    .with_size(self.size)
                    .children(self.items.clone().into_iter().enumerate().map(
                        |(ix, (label, value, span))| {
                            if label == "--" {
                                return DescriptionItem::Divider;
                            }

                            DescriptionItem::new(label)
                                .value(TextView::markdown(ix, value).into_any_element())
                                .span(span)
                        },
                    )),
            )
    }
}
