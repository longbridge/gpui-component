use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, IntoElement,
    ParentElement as _, Render, StyleRefinement, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Selectable, Sizable, Size, StyledExt as _,
    accordion::{Accordion, AccordionItem},
    button::{Button, ButtonGroup},
    checkbox::Checkbox,
    h_flex,
    switch::Switch,
    tag::Tag,
    v_flex,
};

use crate::section;

/// A settings row: the icon sits in a rounded square, and the content lines up
/// with the title rather than with the icon.
fn settings_item(
    item: AccordionItem,
    icon: IconName,
    title: &'static str,
    tag: Option<Tag>,
    body: &'static str,
    icon_bg: Hsla,
    muted: Hsla,
) -> AccordionItem {
    item.title(
        h_flex()
            .gap_2()
            .items_center()
            .child(
                h_flex()
                    .flex_none()
                    .size(px(32.))
                    .items_center()
                    .justify_center()
                    .rounded(px(8.))
                    .bg(icon_bg)
                    .child(Icon::new(icon).small().text_color(muted)),
            )
            .child(div().font_semibold().child(title))
            .children(tag.map(|tag| tag.small())),
    )
    .title_style({
        let mut style = StyleRefinement::default();
        style.padding.top = Some(px(8.).into());
        style.padding.bottom = Some(px(8.).into());
        style
    })
    .content_style({
        let mut style = StyleRefinement::default();
        style.text.color = Some(muted);
        // Past the icon square, so the text starts under the title.
        style.padding.left = Some(px(52.).into());
        style.padding.top = Some(px(0.).into());
        style.padding.bottom = Some(px(12.).into());
        style
    })
    .child(body)
}

pub struct AccordionStory {
    open_ixs: Vec<usize>,
    styled_open_ixs: Vec<usize>,
    size: Size,
    bordered: bool,
    disabled: bool,
    multiple: bool,
    show_icon: bool,
    focus_handle: FocusHandle,
}

impl super::Story for AccordionStory {
    fn title() -> &'static str {
        "Accordion"
    }

    fn description() -> &'static str {
        "The accordion uses collapse internally to make it collapsible."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl AccordionStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            bordered: false,
            open_ixs: vec![0],
            styled_open_ixs: vec![0],
            size: Size::default(),
            disabled: false,
            multiple: true,
            show_icon: false,
            focus_handle: cx.focus_handle(),
        }
    }

    fn toggle_accordion(&mut self, open_ixs: Vec<usize>, _: &mut Window, cx: &mut Context<Self>) {
        self.open_ixs = open_ixs;
        cx.notify();
    }

    fn set_size(&mut self, size: Size, _: &mut Window, cx: &mut Context<Self>) {
        self.size = size;
        cx.notify();
    }
}

impl Focusable for AccordionStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AccordionStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let icon_bg = cx.theme().secondary.opacity(0.5);
        let muted_fg = cx.theme().muted_foreground;

        v_flex()
            .gap_5()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .flex_wrap()
                    .child(
                        ButtonGroup::new("toggle-size")
                            .outline()
                            .compact()
                            .child(
                                Button::new("xsmall")
                                    .label("XSmall")
                                    .selected(self.size == Size::XSmall),
                            )
                            .child(
                                Button::new("small")
                                    .label("Small")
                                    .selected(self.size == Size::Small),
                            )
                            .child(
                                Button::new("medium")
                                    .label("Medium")
                                    .selected(self.size == Size::Medium),
                            )
                            .child(
                                Button::new("large")
                                    .label("Large")
                                    .selected(self.size == Size::Large),
                            )
                            .on_click(cx.listener(|this, selecteds: &Vec<usize>, window, cx| {
                                let size = match selecteds[0] {
                                    0 => Size::XSmall,
                                    1 => Size::Small,
                                    2 => Size::Medium,
                                    3 => Size::Large,
                                    _ => unreachable!(),
                                };
                                this.set_size(size, window, cx);
                            })),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Checkbox::new("multiple")
                                    .label("Multiple")
                                    .checked(self.multiple)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.multiple = *checked;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("show_icon")
                                    .label("Icon")
                                    .checked(self.show_icon)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.show_icon = *checked;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("disabled")
                                    .label("Disabled")
                                    .checked(self.disabled)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.disabled = *checked;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("bordered")
                                    .label("Bordered")
                                    .checked(self.bordered)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.bordered = *checked;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                section("Normal").child(
                    div().w(px(480.)).child(
                        Accordion::new("test")
                            .bordered(self.bordered)
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .multiple(self.multiple)
                            .item(|this| {
                                this.open(self.open_ixs.contains(&0))
                                    .when(self.show_icon, |this| this.icon(IconName::Info))
                                    .title("Is it accessible?")
                                    .child(
                                        "Yes. Each item is a button with an aria-expanded state, \
                                    so screen readers announce whether the section is open, \
                                    and the whole group can be reached with the keyboard.",
                                    )
                            })
                            .item(|this| {
                                this.open(self.open_ixs.contains(&1))
                                    .when(self.show_icon, |this| this.icon(IconName::Inbox))
                                    .title("Can it hold any content?")
                                    .child(
                                        v_flex()
                                            .gap_3()
                                            .child(
                                                "An item takes any element as its content, \
                                            not just text. The height animation measures \
                                            whatever you put in it.",
                                            )
                                            .child(
                                                h_flex()
                                                    .gap_4()
                                                    .child(Switch::new("switch1").label("Switch"))
                                                    .child(
                                                        Checkbox::new("checkbox1")
                                                            .label("Or a Checkbox"),
                                                    ),
                                            ),
                                    )
                            })
                            .item(|this| {
                                this.open(self.open_ixs.contains(&2))
                                    .when(self.show_icon, |this| this.icon(IconName::Moon))
                                    .title("Is it animated?")
                                    .child(
                                        "Yes. Expanding and collapsing animates the height of \
                                    the content, and the chevron rotates to follow. \
                                    Items below move along with it.",
                                    )
                            })
                            .on_toggle_click(cx.listener(
                                |this, open_ixs: &[usize], window, cx| {
                                    this.toggle_accordion(open_ixs.to_vec(), window, cx);
                                },
                            )),
                    ),
                ),
            )
            .child(
                section("Custom Style").child(
                    // A tinted frame around the card.
                    div()
                        .w(px(480.))
                        .p(px(4.))
                        .rounded(px(16.))
                        .bg(cx.theme().secondary.opacity(0.5))
                        .border_1()
                        .border_color(cx.theme().border.opacity(0.5))
                        .child(
                            Accordion::new("custom-style")
                                .multiple(self.multiple)
                                .disabled(self.disabled)
                                .item(|this| {
                                    settings_item(
                                        this,
                                        IconName::Settings,
                                        "Account Settings",
                                        Some(Tag::success().outline().child("New")),
                                        "Manage your account preferences, security settings, \
                                    and personal information. You can also configure \
                                    two-factor authentication here.",
                                        icon_bg,
                                        muted_fg,
                                    )
                                    .open(self.styled_open_ixs.contains(&0))
                                })
                                .item(|this| {
                                    settings_item(
                                        this,
                                        IconName::Eye,
                                        "Privacy & Security",
                                        None,
                                        "Control who can see your profile and how your data \
                                    is used.",
                                        icon_bg,
                                        muted_fg,
                                    )
                                    .open(self.styled_open_ixs.contains(&1))
                                })
                                .item(|this| {
                                    settings_item(
                                        this,
                                        IconName::Info,
                                        "Help & Support",
                                        None,
                                        "Browse the documentation, or get in touch with the \
                                    support team.",
                                        icon_bg,
                                        muted_fg,
                                    )
                                    .open(self.styled_open_ixs.contains(&2))
                                })
                                .on_toggle_click(cx.listener(|this, open_ixs: &[usize], _, cx| {
                                    this.styled_open_ixs = open_ixs.to_vec();
                                    cx.notify();
                                })),
                        ),
                ),
            )
    }
}
