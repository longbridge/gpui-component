use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};

use gpui_component::{
    IconName, Selectable as _, Sizable, Size,
    button::{Button, ButtonGroup, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    tab::{Tab, TabBar},
    v_flex,
};

use crate::section;

pub struct TabsStory {
    focus_handle: FocusHandle,
    active_tab_ix: usize,
    size: Size,
    menu: bool,
}

impl super::Story for TabsStory {
    fn title() -> &'static str {
        "Tabs"
    }

    fn description() -> &'static str {
        "A set of layered sections of content—known as tab panels—that are displayed one at a time."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl TabsStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            active_tab_ix: 0,
            size: Size::default(),
            menu: false,
        }
    }

    fn set_active_tab(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) {
        self.active_tab_ix = ix;
        cx.notify();
    }

    fn set_size(&mut self, size: Size, _: &mut Window, cx: &mut Context<Self>) {
        self.size = size;
        cx.notify();
    }
}

impl Focusable for TabsStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TabsStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                h_flex()
                    .gap_3()
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
                        Checkbox::new("show-menu")
                            .label("More menu")
                            .checked(self.menu)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.menu = !this.menu;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("Tabs").max_w_md().child(
                    TabBar::new("tabs")
                        .w_full()
                        .with_size(self.size)
                        .menu(self.menu)
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .prefix(
                            h_flex()
                                .mx_1()
                                .child(
                                    Button::new("back")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::ArrowLeft),
                                )
                                .child(
                                    Button::new("forward")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::ArrowRight),
                                ),
                        )
                        .child(Tab::new().label("Account"))
                        .child(Tab::new().label("Profile").disabled(true))
                        .child(Tab::new().label("Documents"))
                        .child(Tab::new().label("Mail"))
                        .child(Tab::new().label("Appearance"))
                        .child(Tab::new().label("Settings"))
                        .child(Tab::new().label("About"))
                        .child(Tab::new().label("License"))
                        .suffix(
                            h_flex()
                                .mx_1()
                                .child(Button::new("inbox").ghost().xsmall().icon(IconName::Inbox))
                                .child(
                                    Button::new("more")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Ellipsis),
                                ),
                        ),
                ),
            )
            .child(
                section("Underline Tabs").max_w_md().child(
                    TabBar::new("underline")
                        .w_full()
                        .underline()
                        .with_size(self.size)
                        .menu(self.menu)
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child("Account")
                        .child("Profile")
                        .child("Documents")
                        .child("Mail")
                        .child("Appearance")
                        .child("Settings")
                        .child("About")
                        .child("License"),
                ),
            )
            .child(
                section("Pill Tabs").max_w_md().child(
                    TabBar::new("pill")
                        .w_full()
                        .pill()
                        .with_size(self.size)
                        .menu(self.menu)
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child(Tab::new().label("Account"))
                        .child(Tab::new().label("Profile").disabled(true))
                        .child(Tab::new().label("Documents & Files"))
                        .child(Tab::new().label("Mail"))
                        .child(Tab::new().label("Appearance"))
                        .child(Tab::new().label("Settings"))
                        .child(Tab::new().label("About"))
                        .child(Tab::new().label("License")),
                ),
            )
            .child(
                section("Outline Tabs").max_w_md().child(
                    TabBar::new("outline")
                        .w_full()
                        .outline()
                        .with_size(self.size)
                        .menu(self.menu)
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child(Tab::new().label("Account"))
                        .child(Tab::new().label("Profile").disabled(true))
                        .child(Tab::new().label("Documents & Files"))
                        .child(Tab::new().label("Mail"))
                        .child(Tab::new().label("Appearance"))
                        .child(Tab::new().label("Settings"))
                        .child(Tab::new().label("About"))
                        .child(Tab::new().label("License")),
                ),
            )
            .child(
                section("Segmented Tabs").max_w_md().child(
                    TabBar::new("segmented")
                        .w_full()
                        .segmented()
                        .with_size(self.size)
                        .menu(self.menu)
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child(IconName::Bot)
                        .child(IconName::Calendar)
                        .child(IconName::Map)
                        .children(vec!["Appearance", "Settings", "About", "License"]),
                ),
            )
            .child(
                section("Segmented Tabs (With filling space)")
                    .max_w_md()
                    .child(
                        TabBar::new("flex tabs")
                            .w_full()
                            .segmented()
                            .with_size(self.size)
                            .selected_index(self.active_tab_ix)
                            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                                this.set_active_tab(*ix, window, cx);
                            }))
                            .child(Tab::new().flex_1().label("About"))
                            .child(Tab::new().flex_1().label("Profile")),
                    ),
            )
    }
}
