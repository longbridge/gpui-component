use std::time::Duration;

use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Subscription, Task, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, IconName, WindowExt as _,
    button::Button,
    command::{Command, CommandEntry, CommandEvent, CommandGroup, CommandItem, CommandState},
    h_flex, v_flex,
};

use crate::section;

pub struct CommandStory {
    focus_handle: FocusHandle,
    inline: Entity<CommandState>,
    dialog: Entity<CommandState>,
    scrollable: Entity<CommandState>,
    search: Entity<CommandState>,
    /// Held so that a query that arrives while the last one is still in flight
    /// cancels it, instead of racing it.
    _search_task: Option<Task<()>>,
    last_command: Option<gpui::SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for CommandStory {
    fn title() -> &'static str {
        "Command"
    }

    fn description() -> &'static str {
        "A searchable list of commands and quick actions."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

/// The palette used by the inline and dialog examples: two groups of commands,
/// one of them disabled, the second group carrying shortcut hints.
fn suggestions(state: CommandState) -> CommandState {
    state
        .group(
            CommandGroup::new("Suggestions")
                .item(
                    CommandItem::new("calendar")
                        .label("Calendar")
                        .icon(IconName::Calendar),
                )
                .item(
                    CommandItem::new("search-emoji")
                        .label("Search Emoji")
                        .icon(IconName::Search)
                        .checked(true)
                        .keywords(["smile", "icon"]),
                )
                .item(
                    CommandItem::new("calculator")
                        .label("Calculator")
                        .icon(IconName::Frame)
                        .disabled(true),
                ),
        )
        .separator()
        .group(
            CommandGroup::new("Settings")
                .item(
                    CommandItem::new("profile")
                        .label("Profile")
                        .icon(IconName::User)
                        .shortcut("⌘P"),
                )
                .item(
                    CommandItem::new("billing")
                        .label("Billing")
                        .icon(IconName::CircleUser)
                        .shortcut("⌘B"),
                )
                .item(
                    CommandItem::new("settings")
                        .label("Settings")
                        .icon(IconName::Settings)
                        .shortcut("⌘S"),
                ),
        )
}

fn scrollable(state: CommandState) -> CommandState {
    state
        .group(
            CommandGroup::new("Navigation")
                .item(
                    CommandItem::new("Home")
                        .icon(IconName::LayoutDashboard)
                        .shortcut("⌘H"),
                )
                .item(
                    CommandItem::new("Inbox")
                        .icon(IconName::Inbox)
                        .shortcut("⌘I"),
                )
                .item(
                    CommandItem::new("Documents")
                        .icon(IconName::File)
                        .shortcut("⌘D"),
                )
                .item(
                    CommandItem::new("Folders")
                        .icon(IconName::Folder)
                        .shortcut("⌘F"),
                ),
        )
        .separator()
        .group(
            CommandGroup::new("Actions")
                .item(
                    CommandItem::new("New File")
                        .icon(IconName::Plus)
                        .shortcut("⌘N"),
                )
                .item(CommandItem::new("Copy").icon(IconName::Copy).shortcut("⌘C"))
                .item(
                    CommandItem::new("Delete")
                        .icon(IconName::Delete)
                        .shortcut("⌫"),
                ),
        )
        .separator()
        .group(
            CommandGroup::new("Account")
                .item(CommandItem::new("Profile").icon(IconName::User))
                .item(CommandItem::new("Notifications").icon(IconName::Bell))
                .item(CommandItem::new("Help & Support").icon(IconName::Info)),
        )
        .separator()
        .group(
            CommandGroup::new("Tools")
                .item(CommandItem::new("Palette").icon(IconName::Palette))
                .item(CommandItem::new("Terminal").icon(IconName::SquareTerminal))
                .item(CommandItem::new("Globe").icon(IconName::Globe)),
        )
}

/// The stock universe the search panel queries.
///
/// Stands in for whatever a real application would go and fetch.
const STOCKS: [(&str, &str, &str, f32); 10] = [
    ("AAPL.US", "Apple Inc.", "228.52", 1.24),
    ("NVDA.US", "NVIDIA Corporation", "134.81", -0.62),
    ("TSLA.US", "Tesla, Inc.", "251.44", 3.18),
    ("MSFT.US", "Microsoft Corporation", "428.02", 0.41),
    ("AMZN.US", "Amazon.com, Inc.", "186.33", -1.07),
    ("700.HK", "Tencent Holdings Ltd.", "412.60", 0.87),
    ("9988.HK", "Alibaba Group Holding Ltd.", "82.15", -2.31),
    ("3690.HK", "Meituan", "128.90", 1.66),
    ("600519.SH", "Kweichow Moutai Co., Ltd.", "1482.00", -0.34),
    ("000858.SZ", "Wuliangye Yibin Co., Ltd.", "142.77", 0.19),
];

impl CommandStory {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let inline = cx.new(|cx| suggestions(CommandState::new(window, cx)));
        let dialog = cx.new(|cx| suggestions(CommandState::new(window, cx)));
        let scrollable_state = cx.new(|cx| scrollable(CommandState::new(window, cx)));
        let search = cx.new(|cx| CommandState::new(window, cx));

        let _subscriptions = vec![
            cx.subscribe(&inline, Self::on_command_event),
            cx.subscribe(&dialog, Self::on_command_event),
            cx.subscribe(&scrollable_state, Self::on_command_event),
            cx.subscribe_in(&search, window, Self::on_search_event),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            inline,
            dialog,
            scrollable: scrollable_state,
            search,
            _search_task: None,
            last_command: None,
            _subscriptions,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn on_command_event(
        &mut self,
        _: Entity<CommandState>,
        event: &CommandEvent,
        cx: &mut Context<Self>,
    ) {
        if let CommandEvent::Confirm(value) = event {
            self.last_command = Some(value.clone());
            cx.notify();
        }
    }

    /// Open the stock search as a dialog, starting from an empty query.
    ///
    /// The palette keeps a fixed height so that results arriving, and the
    /// query being narrowed, do not make the dialog jump around.
    fn on_open_stock_search(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._search_task = None;
        self.search.update(cx, |search, cx| {
            search.set_query("", window, cx);
            search.set_loading(false, window, cx);
            search.set_entries([], cx);
        });

        window.open_command_dialog(&self.search, cx, |command, _, _| {
            command
                .filterable(false)
                .placeholder("Search stocks...")
                .empty("No stock found.")
                .min_h(px(320.))
                .max_h(px(320.))
        });
    }

    /// Answer the search panel's queries the way a remote search would: spin
    /// the field, wait, then replace the entries with the results.
    fn on_search_event(
        &mut self,
        state: &Entity<CommandState>,
        event: &CommandEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let CommandEvent::Query(query) = event else {
            return self.on_command_event(state.clone(), event, cx);
        };

        let query = query.trim().to_lowercase();
        if query.is_empty() {
            self._search_task = None;
            return state.update(cx, |state, cx| {
                state.set_loading(false, window, cx);
                state.set_entries([], cx);
            });
        }

        state.update(cx, |state, cx| state.set_loading(true, window, cx));

        let state = state.clone();
        self._search_task = Some(cx.spawn_in(window, async move |_, cx| {
            // The round trip a real search would spend on the network.
            cx.background_executor()
                .timer(Duration::from_millis(400))
                .await;

            let entries = STOCKS
                .iter()
                .filter(|(symbol, name, _, _)| {
                    symbol.to_lowercase().contains(&query) || name.to_lowercase().contains(&query)
                })
                .map(|stock| CommandEntry::Item(stock_item(*stock)))
                .collect::<Vec<_>>();

            _ = state.update_in(cx, |state, window, cx| {
                state.set_loading(false, window, cx);
                state.set_entries(entries, cx);
            });
        }));
    }
}

/// A two-line search result: symbol and name on the left, quote on the right.
fn stock_item(stock: (&'static str, &'static str, &'static str, f32)) -> CommandItem {
    let (symbol, name, price, change) = stock;

    CommandItem::new(symbol).element(move |_, cx| {
        let change_color = if change < 0. {
            cx.theme().chart_bearish
        } else {
            cx.theme().chart_bullish
        };

        h_flex()
            .w_full()
            .gap_3()
            .items_center()
            .justify_between()
            .child(
                v_flex()
                    .gap_0p5()
                    .child(div().text_sm().child(symbol))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(name),
                    ),
            )
            .child(
                v_flex()
                    .gap_0p5()
                    .items_end()
                    .child(div().text_sm().child(price))
                    .child(
                        div()
                            .text_xs()
                            .text_color(change_color)
                            .child(format!("{:+.2}%", change)),
                    ),
            )
            .into_any_element()
    })
}

impl Focusable for CommandStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CommandStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_state = self.dialog.clone();

        v_flex()
            .w_full()
            .gap_6()
            .child(
                section("Inline")
                    .description("A palette rendered in place, with groups, icons and shortcuts.")
                    .child(Command::new(&self.inline).w(px(380.))),
            )
            .child(
                section("Dialog")
                    .description("The same palette in a dialog, with its search field focused.")
                    .child(
                        Button::new("open-command-dialog")
                            .outline()
                            .label("Open Menu")
                            .on_click(cx.listener(move |_, _, window, cx| {
                                window.open_command_dialog(&dialog_state, cx, |command, _, _| {
                                    command.placeholder("Type a command or search...")
                                });
                            })),
                    ),
            )
            .child(
                section("Scrollable")
                    .description("More commands than fit, capped at 220px.")
                    .child(Command::new(&self.scrollable).max_h(px(220.)).w(px(380.))),
            )
            .child(
                section("Search panel")
                    .description(
                        "A palette used as a search panel: its own filtering is off, and \
                         every query is answered asynchronously — try \"a\", \"hk\" or \"tesla\".",
                    )
                    .child(
                        Button::new("open-stock-search")
                            .outline()
                            .label("Search Stocks")
                            .on_click(cx.listener(Self::on_open_stock_search)),
                    ),
            )
            .when_some(self.last_command.clone(), |this, value| {
                this.child(
                    section("Last confirmed")
                        .description("The value reported by the last CommandEvent::Confirm.")
                        .child(value),
                )
            })
    }
}
