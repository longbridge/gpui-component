use gpui::{
    Anchor, App, AppContext as _, Context, Entity, Focusable, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, Styled as _, Window,
};

use crate::{ChangeStorySize, section, story_toolbar};
use gpui_component::{
    ActiveTheme as _, Disableable as _, IconName, Selectable as _, Sizable as _, Size,
    button::{Button, ButtonGroup, ButtonVariants as _, DropdownButton},
    h_flex,
    menu::{DropdownMenu as _, PopupMenuItem},
    separator::Separator,
    switch::Switch,
    v_flex,
};

/// Column sets a results table can switch between.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Columns {
    Essentials,
    Fundamentals,
    Technicals,
}

impl Columns {
    const ALL: [Self; 3] = [Self::Essentials, Self::Fundamentals, Self::Technicals];

    fn label(&self) -> &'static str {
        match self {
            Self::Essentials => "Essentials",
            Self::Fundamentals => "Fundamentals",
            Self::Technicals => "Technicals",
        }
    }
}

/// Ranges of a chart's time axis, the way a toolbar offers them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TimeRange {
    Day,
    Week,
    Month,
    Year,
}

impl TimeRange {
    const ALL: [Self; 4] = [Self::Day, Self::Week, Self::Month, Self::Year];

    fn short(&self) -> &'static str {
        match self {
            Self::Day => "1D",
            Self::Week => "1W",
            Self::Month => "1M",
            Self::Year => "1Y",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Day => "1 day",
            Self::Week => "1 week",
            Self::Month => "1 month",
            Self::Year => "1 year",
        }
    }
}

/// Shapes a chart's drawing tool can place.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    TrendLine,
    Rectangle,
    Ray,
}

impl Shape {
    const ALL: [Self; 3] = [Self::TrendLine, Self::Rectangle, Self::Ray];

    fn label(&self) -> &'static str {
        match self {
            Self::TrendLine => "Trend line",
            Self::Rectangle => "Rectangle",
            Self::Ray => "Ray",
        }
    }
}

/// Studies a chart can overlay, more than one at a time.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Indicator {
    MovingAverage,
    Bollinger,
    Macd,
}

impl Indicator {
    const ALL: [Self; 3] = [Self::MovingAverage, Self::Bollinger, Self::Macd];

    fn label(&self) -> &'static str {
        match self {
            Self::MovingAverage => "MA",
            Self::Bollinger => "BOLL",
            Self::Macd => "MACD",
        }
    }

    fn index(&self) -> usize {
        match self {
            Self::MovingAverage => 0,
            Self::Bollinger => 1,
            Self::Macd => 2,
        }
    }
}

/// Order types a quick-trade button can send.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OrderType {
    Market,
    Limit,
}

impl OrderType {
    const ALL: [Self; 2] = [Self::Market, Self::Limit];

    fn label(&self) -> &'static str {
        match self {
            Self::Market => "Market",
            Self::Limit => "Limit",
        }
    }
}

/// How long an order stays live, the way an order pad offers it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TimeInForce {
    Day,
    GoodTillCanceled,
    ImmediateOrCancel,
}

impl TimeInForce {
    const ALL: [Self; 3] = [Self::Day, Self::GoodTillCanceled, Self::ImmediateOrCancel];

    fn short(&self) -> &'static str {
        match self {
            Self::Day => "Day",
            Self::GoodTillCanceled => "GTC",
            Self::ImmediateOrCancel => "IOC",
        }
    }
}

const QUANTITIES: [u32; 3] = [100, 500, 1000];

/// One line of muted text under a control, never beside it.
fn status(text: impl Into<SharedString>, cx: &App) -> impl IntoElement {
    h_flex()
        .justify_center()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(text.into())
}

pub struct DropdownButtonStory {
    focus_handle: gpui::FocusHandle,
    size: Size,
    /// What the last click did, so the action half and the menu half can be told
    /// apart while clicking around.
    last_action: SharedString,
    columns: Columns,
    time_range: TimeRange,
    /// The drawing tool is down until a shape is picked up.
    shape: Option<Shape>,
    indicators: [bool; 3],
    quantity: u32,
    order_type: OrderType,
    market_open: bool,
    sending: bool,
    time_in_force: TimeInForce,
}

impl DropdownButtonStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            size: Size::Medium,
            last_action: "Nothing yet".into(),
            columns: Columns::Essentials,
            time_range: TimeRange::Day,
            shape: None,
            indicators: [true, false, false],
            quantity: 100,
            order_type: OrderType::Market,
            market_open: true,
            sending: false,
            time_in_force: TimeInForce::Day,
        })
    }

    fn record(&mut self, action: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.last_action = action.into();
        cx.notify();
    }

    fn indicator_summary(&self) -> String {
        let on: Vec<&str> = Indicator::ALL
            .iter()
            .filter(|indicator| self.indicators[indicator.index()])
            .map(|indicator| indicator.label())
            .collect();

        if on.is_empty() {
            "no studies".to_string()
        } else {
            on.join(", ")
        }
    }
}

impl super::Story for DropdownButtonStory {
    fn title() -> &'static str {
        "DropdownButton"
    }

    fn description() -> &'static str {
        "An action on the left, its alternatives behind the caret."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for DropdownButtonStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DropdownButtonStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();

        v_flex()
            .gap_6()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(status(format!("Last action: {}", self.last_action), cx))
            .child(self.render_table_toolbar(&view, cx))
            .child(self.render_chart_toolbar(&view, cx))
            .child(self.render_quick_trade(&view, cx))
            .child(self.render_place_order(&view, cx))
            .child(self.render_composed(&view, cx))
    }
}

impl DropdownButtonStory {
    /// A results-table toolbar: two splits and a plain button, the mix a table
    /// header usually carries.
    fn render_table_toolbar(&self, view: &Entity<Self>, cx: &Context<Self>) -> impl IntoElement {
        let columns = self.columns;

        section("Screener results")
            .description("The default action, with its variants behind the caret.")
            .child(
                v_flex()
                    .gap_3()
                    .items_center()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                DropdownButton::new("export")
                                    .with_size(self.size)
                                    .button(
                                        Button::new("export-main")
                                            .icon(IconName::ArrowDown)
                                            .label("Export")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.record("Exported CSV", cx);
                                            })),
                                    )
                                    .dropdown_menu({
                                        let view = view.clone();
                                        move |menu, window, _| {
                                            menu.item(PopupMenuItem::new("Excel").on_click(
                                                window.listener_for(&view, |this, _, _, cx| {
                                                    this.record("Exported Excel", cx);
                                                }),
                                            ))
                                            .item(PopupMenuItem::new("JSON").on_click(
                                                window.listener_for(&view, |this, _, _, cx| {
                                                    this.record("Exported JSON", cx);
                                                }),
                                            ))
                                            .separator()
                                            .item(
                                                PopupMenuItem::new("Copy to clipboard").on_click(
                                                    window.listener_for(&view, |this, _, _, cx| {
                                                        this.record("Copied to the clipboard", cx);
                                                    }),
                                                ),
                                            )
                                        }
                                    }),
                            )
                            .child(
                                DropdownButton::new("columns")
                                    .with_size(self.size)
                                    .button(Button::new("columns-main").label("Columns").on_click(
                                        cx.listener(|this, _, _, cx| {
                                            this.record("Opened the column picker", cx);
                                        }),
                                    ))
                                    .dropdown_menu({
                                        let view = view.clone();
                                        move |mut menu, window, _| {
                                            for preset in Columns::ALL {
                                                menu = menu.item(
                                                    PopupMenuItem::new(preset.label())
                                                        .checked(preset == columns)
                                                        .on_click(window.listener_for(
                                                            &view,
                                                            move |this, _, _, cx| {
                                                                this.columns = preset;
                                                                this.record(
                                                                    format!(
                                                                        "Showing {}",
                                                                        preset.label()
                                                                    ),
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                );
                                            }
                                            menu
                                        }
                                    }),
                            )
                            .child(
                                Button::new("refresh")
                                    .with_size(self.size)
                                    .label("Refresh")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.record("Refreshed the results", cx);
                                    })),
                            ),
                    )
                    .child(status(format!("{} columns", columns.label()), cx)),
            )
    }

    /// A chart toolbar, where ghost splits carry the current selection and sit
    /// next to plain buttons.
    fn render_chart_toolbar(&self, view: &Entity<Self>, cx: &Context<Self>) -> impl IntoElement {
        let time_range = self.time_range;
        let shape = self.shape;
        let indicators = self.indicators;
        let any_indicator = indicators.iter().any(|on| *on);

        section("Chart toolbar")
            .description("Ghost splits join when selected, separate when idle.")
            .child(
                v_flex()
                    .gap_3()
                    .items_center()
                    .child(
                        h_flex()
                            .gap_1()
                            .p_1()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .child(
                                DropdownButton::new("time-range")
                                    .ghost()
                                    .xsmall()
                                    .selected(true)
                                    .button(
                                        Button::new("time-range-main")
                                            .label(time_range.short())
                                            .tooltip(time_range.name()),
                                    )
                                    .dropdown_menu({
                                        let view = view.clone();
                                        move |mut menu, window, _| {
                                            for range in TimeRange::ALL {
                                                menu = menu.item(
                                                    PopupMenuItem::new(range.name())
                                                        .checked(range == time_range)
                                                        .on_click(window.listener_for(
                                                            &view,
                                                            move |this, _, _, cx| {
                                                                this.time_range = range;
                                                                this.record(
                                                                    format!(
                                                                        "Switched to {}",
                                                                        range.name()
                                                                    ),
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                );
                                            }
                                            menu
                                        }
                                    }),
                            )
                            .child(
                                DropdownButton::new("indicators")
                                    .ghost()
                                    .xsmall()
                                    .selected(any_indicator)
                                    .button(
                                        Button::new("indicators-main")
                                            .label("Studies")
                                            .tooltip("Overlays on the price"),
                                    )
                                    .dropdown_menu({
                                        let view = view.clone();
                                        move |mut menu, window, _| {
                                            for indicator in Indicator::ALL {
                                                menu = menu.item(
                                                    PopupMenuItem::new(indicator.label())
                                                        .checked(indicators[indicator.index()])
                                                        .on_click(window.listener_for(
                                                            &view,
                                                            move |this, _, _, cx| {
                                                                let slot = indicator.index();
                                                                this.indicators[slot] =
                                                                    !this.indicators[slot];
                                                                let verb =
                                                                    match this.indicators[slot] {
                                                                        true => "Added",
                                                                        false => "Removed",
                                                                    };
                                                                this.record(
                                                                    format!(
                                                                        "{verb} {}",
                                                                        indicator.label()
                                                                    ),
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                );
                                            }
                                            menu
                                        }
                                    }),
                            )
                            .child(
                                DropdownButton::new("drawing")
                                    .ghost()
                                    .xsmall()
                                    .selected(shape.is_some())
                                    .button(
                                        Button::new("drawing-main")
                                            .label("Draw")
                                            .tooltip(
                                                shape
                                                    .map_or("Drawing tools", |shape| shape.label()),
                                            )
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.shape = match this.shape {
                                                    Some(_) => None,
                                                    None => Some(Shape::TrendLine),
                                                };
                                                match this.shape {
                                                    Some(shape) => this.record(
                                                        format!("Picked up the {}", shape.label()),
                                                        cx,
                                                    ),
                                                    None => {
                                                        this.record("Put the drawing tool down", cx)
                                                    }
                                                }
                                            })),
                                    )
                                    .dropdown_menu({
                                        let view = view.clone();
                                        move |mut menu, window, _| {
                                            for next in Shape::ALL {
                                                menu = menu.item(
                                                    PopupMenuItem::new(next.label())
                                                        .checked(shape == Some(next))
                                                        .on_click(window.listener_for(
                                                            &view,
                                                            move |this, _, _, cx| {
                                                                this.shape = Some(next);
                                                                this.record(
                                                                    format!(
                                                                        "Picked up the {}",
                                                                        next.label()
                                                                    ),
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                );
                                            }
                                            menu
                                        }
                                    }),
                            )
                            .child(Separator::vertical())
                            .child(
                                Button::new("undo")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Undo2)
                                    .tooltip("Undo")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.record("Undid the last edit", cx);
                                    })),
                            ),
                    )
                    .child(status(
                        format!(
                            "{} · {} · {}",
                            time_range.name(),
                            self.indicator_summary(),
                            match shape {
                                Some(shape) => shape.label().to_lowercase(),
                                None => "no drawing tool".to_string(),
                            }
                        ),
                        cx,
                    )),
            )
    }

    /// A quick-trade pad: a segmented group picks the size, the splits send it,
    /// and the market switch takes both halves out at once.
    fn render_quick_trade(&self, view: &Entity<Self>, cx: &Context<Self>) -> impl IntoElement {
        let order_type = self.order_type;
        let quantity = self.quantity;
        let disabled = !self.market_open;
        let sending = self.sending;

        let side = |id: &'static str, label: &'static str| {
            let view = view.clone();
            DropdownButton::new(id)
                .outline()
                .small()
                .disabled(disabled)
                .button(
                    Button::new(SharedString::from(format!("{id}-main")))
                        .label(format!("{label} {quantity} · {}", order_type.label()))
                        .loading(sending)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.record(
                                format!("Sent {label} {quantity} at {}", order_type.label()),
                                cx,
                            );
                        })),
                )
                .dropdown_menu(move |mut menu, window, _| {
                    for next in OrderType::ALL {
                        menu = menu.item(
                            PopupMenuItem::new(next.label())
                                .checked(next == order_type)
                                .on_click(window.listener_for(&view, move |this, _, _, cx| {
                                    this.order_type = next;
                                    this.record(
                                        format!("Sent {label} {quantity} at {}", next.label()),
                                        cx,
                                    );
                                })),
                        );
                    }
                    menu
                })
        };

        section("Quick trade")
            .description("Disabling covers both halves; the menu's choice stays on the action.")
            .child(
                v_flex()
                    .gap_3()
                    .items_center()
                    .child(
                        ButtonGroup::new("quantity")
                            .outline()
                            .small()
                            .children(QUANTITIES.map(|amount| {
                                Button::new(SharedString::from(format!("qty-{amount}")))
                                    .label(amount.to_string())
                                    .selected(amount == quantity)
                            }))
                            .on_click(cx.listener(|this, clicked: &Vec<usize>, _, cx| {
                                if let Some(amount) = clicked.first().map(|ix| QUANTITIES[*ix]) {
                                    this.quantity = amount;
                                    this.record(format!("Order size {amount}"), cx);
                                }
                            })),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(side("buy", "Buy"))
                            .child(side("sell", "Sell")),
                    )
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                Switch::new("market-open")
                                    .label("Market open")
                                    .checked(self.market_open)
                                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                        this.market_open = *checked;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Switch::new("sending")
                                    .label("Order in flight")
                                    .checked(self.sending)
                                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                        this.sending = *checked;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
    }

    /// The one action a pad exists to run, so it takes the primary variant. The
    /// menu holds how long that same order stays live.
    fn render_place_order(&self, view: &Entity<Self>, cx: &Context<Self>) -> impl IntoElement {
        let time_in_force = self.time_in_force;
        let view = view.clone();

        section("Place an order")
            .description("Primary for the one action a screen runs; the menu opens left-aligned.")
            .child(
                DropdownButton::new("place-order")
                    .primary()
                    .with_size(self.size)
                    .button(
                        Button::new("place-order-main")
                            .label(format!("Place order · {}", time_in_force.short()))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.record(
                                    format!("Placed a {} order", time_in_force.short()),
                                    cx,
                                );
                            })),
                    )
                    .dropdown_menu_with_anchor(Anchor::TopLeft, move |mut menu, window, _| {
                        for next in TimeInForce::ALL {
                            menu = menu.item(
                                PopupMenuItem::new(next.short())
                                    .checked(next == time_in_force)
                                    .on_click(window.listener_for(&view, move |this, _, _, cx| {
                                        this.time_in_force = next;
                                        this.record(format!("Placed a {} order", next.short()), cx);
                                    })),
                            );
                        }
                        menu
                    }),
            )
    }

    /// The same shape built from the group directly, which is what a split with
    /// more than two members needs.
    fn render_composed(&self, view: &Entity<Self>, cx: &Context<Self>) -> impl IntoElement {
        let view = view.clone();

        section("Composed with ButtonGroup")
            .description("A group whose last member opens a menu.")
            .child(
                ButtonGroup::new("history")
                    .outline()
                    .small()
                    .child(
                        Button::new("history-undo")
                            .icon(IconName::Undo2)
                            .tooltip("Undo")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.record("Undid the last edit", cx);
                            })),
                    )
                    .child(
                        Button::new("history-redo")
                            .icon(IconName::Redo2)
                            .tooltip("Redo")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.record("Redid the last edit", cx);
                            })),
                    )
                    .child(
                        Button::new("history-more")
                            .icon(IconName::Ellipsis)
                            .dropdown_menu({
                                let view = view.clone();
                                move |menu, window, _| {
                                    menu.item(PopupMenuItem::new("Undo all").on_click(
                                        window.listener_for(&view, |this, _, _, cx| {
                                            this.record("Undid every edit", cx);
                                        }),
                                    ))
                                    .item(
                                        PopupMenuItem::new("Redo all").on_click(
                                            window.listener_for(&view, |this, _, _, cx| {
                                                this.record("Redid every edit", cx);
                                            }),
                                        ),
                                    )
                                }
                            }),
                    ),
            )
    }
}
