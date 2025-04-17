use std::{sync::Arc, time::Duration};

use fake::Fake;
use gpui::{
    actions, div, prelude::FluentBuilder as _, px, App, AppContext, Context, Entity, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, ParentElement, Render, SharedString, Styled,
    Task, Timer, WeakEntity, Window,
};
use raw_window_handle::HasWindowHandle;

use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    date_picker::DatePicker,
    h_flex,
    input::TextInput,
    list::{List, ListDelegate, ListItem},
    v_flex,
    webview::WebView,
    wry, ActiveTheme as _, ContextModal as _, Icon, IconName, Placement,
};

use crate::section;

actions!(modal_story, [TestAction]);

pub struct ListItemDeletegate {
    story: WeakEntity<DrawerStory>,
    confirmed_index: Option<usize>,
    selected_index: Option<usize>,
    items: Vec<Arc<String>>,
    matches: Vec<Arc<String>>,
}

impl ListDelegate for ListItemDeletegate {
    type Item = ListItem;

    fn items_count(&self, _: &App) -> usize {
        self.matches.len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> Task<()> {
        let query = query.to_string();
        cx.spawn(async move |this, cx| {
            // Simulate a slow search.
            let sleep = (0.05..0.1).fake();
            Timer::after(Duration::from_secs_f64(sleep)).await;

            this.update(cx, |this, cx| {
                this.delegate_mut().matches = this
                    .delegate()
                    .items
                    .iter()
                    .filter(|item| item.to_lowercase().contains(&query.to_lowercase()))
                    .cloned()
                    .collect();
                cx.notify();
            })
            .ok();
        })
    }

    fn render_item(
        &self,
        ix: usize,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let confirmed = Some(ix) == self.confirmed_index;
        let selected = Some(ix) == self.selected_index;

        if let Some(item) = self.matches.get(ix) {
            let list_item = ListItem::new(("item", ix))
                .check_icon(IconName::Check)
                .confirmed(confirmed)
                .selected(selected)
                .py_1()
                .px_3()
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(item.to_string()),
                )
                .suffix(|_, _| {
                    Button::new("like")
                        .icon(IconName::Heart)
                        .with_variant(ButtonVariant::Ghost)
                        .size(px(18.))
                        .on_click(move |_, window, cx| {
                            cx.stop_propagation();
                            window.prevent_default();

                            println!("You have clicked like.");
                        })
                });
            Some(list_item)
        } else {
            None
        }
    }

    fn render_empty(&self, _: &mut Window, cx: &mut Context<List<Self>>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                Icon::new(IconName::Inbox)
                    .size(px(50.))
                    .text_color(cx.theme().muted_foreground),
            )
            .child("No matches found")
            .items_center()
            .justify_center()
            .p_3()
            .bg(cx.theme().muted)
            .text_color(cx.theme().muted_foreground)
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<List<Self>>) {
        _ = self.story.update(cx, |this, cx| {
            this.close_drawer(window, cx);
        });
    }

    fn confirm(&mut self, _secondary: bool, window: &mut Window, cx: &mut Context<List<Self>>) {
        _ = self.story.update(cx, |this, cx| {
            self.confirmed_index = self.selected_index;
            if let Some(ix) = self.confirmed_index {
                if let Some(item) = self.matches.get(ix) {
                    this.selected_value = Some(SharedString::from(item.to_string()));
                }
            }

            window.close_drawer(cx);
        });
    }

    fn set_selected_index(
        &mut self,
        ix: Option<usize>,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        self.selected_index = ix;

        if let Some(_) = ix {
            cx.notify();
        }
    }
}

pub struct DrawerStory {
    focus_handle: FocusHandle,
    drawer_placement: Option<Placement>,
    selected_value: Option<SharedString>,
    list: Entity<List<ListItemDeletegate>>,
    input1: Entity<TextInput>,
    input2: Entity<TextInput>,
    date_picker: Entity<DatePicker>,
    modal_overlay: bool,
    model_show_close: bool,
    model_padding: bool,
    model_keyboard: bool,
    overlay_closable: bool,
}

impl super::Story for DrawerStory {
    fn title() -> &'static str {
        "Drawer"
    }

    fn description() -> &'static str {
        "Drawer for open a popup in the edge of the window"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl DrawerStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items: Vec<Arc<String>> = [
            "Baguette (France)",
            "Baklava (Turkey)",
            "Beef Wellington (UK)",
            "Biryani (India)",
            "Borscht (Ukraine)",
            "Bratwurst (Germany)",
            "Bulgogi (Korea)",
            "Burrito (USA)",
            "Ceviche (Peru)",
            "Chicken Tikka Masala (India)",
            "Churrasco (Brazil)",
            "Couscous (North Africa)",
            "Croissant (France)",
            "Dim Sum (China)",
            "Empanada (Argentina)",
            "Fajitas (Mexico)",
            "Falafel (Middle East)",
            "Feijoada (Brazil)",
            "Fish and Chips (UK)",
            "Fondue (Switzerland)",
            "Goulash (Hungary)",
            "Haggis (Scotland)",
            "Kebab (Middle East)",
            "Kimchi (Korea)",
            "Lasagna (Italy)",
            "Maple Syrup Pancakes (Canada)",
            "Moussaka (Greece)",
            "Pad Thai (Thailand)",
            "Paella (Spain)",
            "Pancakes (USA)",
            "Pasta Carbonara (Italy)",
            "Pavlova (Australia)",
            "Peking Duck (China)",
            "Pho (Vietnam)",
            "Pierogi (Poland)",
            "Pizza (Italy)",
            "Poutine (Canada)",
            "Pretzel (Germany)",
            "Ramen (Japan)",
            "Rendang (Indonesia)",
            "Sashimi (Japan)",
            "Satay (Indonesia)",
            "Shepherd's Pie (Ireland)",
            "Sushi (Japan)",
            "Tacos (Mexico)",
            "Tandoori Chicken (India)",
            "Tortilla (Spain)",
            "Tzatziki (Greece)",
            "Wiener Schnitzel (Austria)",
        ]
        .iter()
        .map(|s| Arc::new(s.to_string()))
        .collect();

        let story = cx.entity().downgrade();
        let delegate = ListItemDeletegate {
            story,
            selected_index: None,
            confirmed_index: None,
            items: items.clone(),
            matches: items.clone(),
        };
        let list = cx.new(|cx| {
            let mut list = List::new(delegate, window, cx);
            list.focus(window, cx);
            if let Some(query_input) = list.query_input() {
                query_input.update(cx, |input, cx| {
                    input.set_placeholder("Pickup your country...", window, cx);
                })
            }
            list
        });

        let input1 = cx.new(|cx| TextInput::new(window, cx).placeholder("Your Name"));
        let input2 = cx.new(|cx| {
            TextInput::new(window, cx).placeholder("For test focus back on modal close.")
        });
        let date_picker = cx
            .new(|cx| DatePicker::new("birthday-picker", window, cx).placeholder("Date of Birth"));

        Self {
            focus_handle: cx.focus_handle(),
            drawer_placement: None,
            selected_value: None,
            list,
            input1,
            input2,
            date_picker,
            modal_overlay: true,
            model_show_close: true,
            model_padding: true,
            model_keyboard: true,
            overlay_closable: true,
        }
    }

    fn open_drawer_at(
        &mut self,
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = self.input1.clone();
        let date_picker = self.date_picker.clone();
        let list = self.list.clone();

        let list_h = match placement {
            Placement::Left | Placement::Right => px(400.),
            Placement::Top | Placement::Bottom => px(160.),
        };

        let overlay = self.modal_overlay;
        window.open_drawer_at(placement, cx, move |this, _, cx| {
            this.overlay(overlay)
                .size(px(400.))
                .title("Drawer Title")
                .gap_4()
                .child(input.clone())
                .child(date_picker.clone())
                .child(
                    Button::new("send-notification")
                        .child("Test Notification")
                        .on_click(|_, window, cx| {
                            window.push_notification("Hello this is message from Drawer.", cx)
                        }),
                )
                .child(
                    div()
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded(cx.theme().radius)
                        .size_full()
                        .flex_1()
                        .h(list_h)
                        .child(list.clone()),
                )
                .footer(
                    h_flex()
                        .gap_6()
                        .items_center()
                        .child(Button::new("confirm").primary().label("Confirm").on_click(
                            |_, window, cx| {
                                window.close_drawer(cx);
                            },
                        ))
                        .child(
                            Button::new("cancel")
                                .label("Cancel")
                                .on_click(|_, window, cx| {
                                    window.close_drawer(cx);
                                }),
                        ),
                )
        });
    }

    fn close_drawer(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.drawer_placement = None;
        cx.notify();
    }

    fn on_action_test_action(
        &mut self,
        _: &TestAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.push_notification("You have clicked the TestAction.", cx);
    }
}

impl Focusable for DrawerStory {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DrawerStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("drawer-story")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_test_action))
            .size_full()
            .child(
                v_flex()
                    .gap_6()
                    .child(
                        h_flex()
                            .id("state")
                            .items_center()
                            .gap_3()
                            .child(
                                Checkbox::new("overlay")
                                    .label("Overlay")
                                    .checked(self.modal_overlay)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.modal_overlay = !view.modal_overlay;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("closable")
                                    .label("Overlay Closable")
                                    .checked(self.overlay_closable)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.overlay_closable = !view.overlay_closable;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("show-close")
                                    .label("Close Button")
                                    .checked(self.model_show_close)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.model_show_close = !view.model_show_close;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("padding")
                                    .label("Inner Padding")
                                    .checked(self.model_padding)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.model_padding = !view.model_padding;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("keyboard")
                                    .label("Keyboard")
                                    .checked(self.model_keyboard)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.model_keyboard = !view.model_keyboard;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        section("Normal Drawer")
                            .child(
                                Button::new("show-drawer-left")
                                    .label("Left Drawer...")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_drawer_at(Placement::Left, window, cx)
                                    })),
                            )
                            .child(
                                Button::new("show-drawer-top")
                                    .label("Top Drawer...")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_drawer_at(Placement::Top, window, cx)
                                    })),
                            )
                            .child(
                                Button::new("show-drawer-right")
                                    .label("Right Drawer...")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_drawer_at(Placement::Right, window, cx)
                                    })),
                            )
                            .child(
                                Button::new("show-drawer-bottom")
                                    .label("Bottom Drawer...")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_drawer_at(Placement::Bottom, window, cx)
                                    })),
                            ),
                    )
                    .child(
                        section("Focus back test")
                            .max_w_md()
                            .child(self.input2.clone())
                            .child(
                                Button::new("test-action")
                                    .label("Test Action")
                                    .flex_shrink_0()
                                    .on_click(|_, window, cx| {
                                        window.dispatch_action(Box::new(TestAction), cx);
                                    })
                                    .tooltip(
                                        "This button for test dispatch action, \
                                        to make sure when Modal close,\
                                        \nthis still can handle the action.",
                                    ),
                            ),
                    )
                    .child(
                        section("WebView in Drawer").child(
                            Button::new("webview")
                                .label("Open WebView")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    let webview = cx.new(|cx| {
                                        let webview = wry::WebViewBuilder::new()
                                            .build_as_child(
                                                &window.window_handle().expect("No window handle"),
                                            )
                                            .unwrap();

                                        WebView::new(webview, window, cx)
                                    });
                                    webview.update(cx, |webview, _| {
                                        webview.load_url("https://github.com/explore");
                                    });
                                    window.open_drawer(cx, move |drawer, window, cx| {
                                        let height =
                                            window.window_bounds().get_bounds().size.height;
                                        let webview_bounds = webview.read(cx).bounds();

                                        drawer.title("WebView Title").p_0().child(
                                            div()
                                                .h(height - webview_bounds.origin.y)
                                                .child(webview.clone()),
                                        )
                                    });
                                })),
                        ),
                    )
                    .when_some(self.selected_value.clone(), |this, selected_value| {
                        this.child(
                            h_flex().gap_1().child("You have selected:").child(
                                div()
                                    .child(selected_value.to_string())
                                    .text_color(gpui::red()),
                            ),
                        )
                    }),
            )
    }
}
