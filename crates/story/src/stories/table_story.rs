use std::{
    ops::Range,
    sync::LazyLock,
    time::{self, Duration},
};

use fake::Fake;
use gpui::{
    Action, AnyElement, App, AppContext, ClickEvent, Context, Div, Entity, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Subscription, Task, TextAlign, Timer, Window, div,
    prelude::FluentBuilder as _,
};
use gpui_component::{
    ActiveTheme as _, Selectable, Sizable as _, Size, StyleSized as _, StyledExt,
    button::Button,
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
    label::Label,
    menu::{DropdownMenu, PopupMenu},
    spinner::Spinner,
    table::{Column, ColumnFixed, ColumnSort, Table, TableDelegate, TableEvent, TableState},
    v_flex,
};
use serde::{Deserialize, Serialize};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = table_story, no_json)]
struct ChangeSize(Size);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = table_story, no_json)]
struct OpenDetail(usize);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Counter {
    symbol: SharedString,
    market: SharedString,
    name: SharedString,
}

static ALL_COUNTERS: LazyLock<Vec<Counter>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../fixtures/counters.json")).unwrap());
static INCREMENT_ID: LazyLock<std::sync::Mutex<usize>> = LazyLock::new(|| std::sync::Mutex::new(0));

impl Counter {
    fn random() -> Self {
        let len = ALL_COUNTERS.len();
        let ix = rand::random::<usize>() % len;
        ALL_COUNTERS[ix].clone()
    }

    fn symbol_code(&self) -> SharedString {
        format!("{}.{}", self.symbol, self.market).into()
    }
}

#[derive(Clone, Debug, Default)]
struct Stock {
    id: usize,
    counter: Counter,
    price: f64,
    change: f64,
    change_percent: f64,
    volume: f64,
    turnover: f64,
    market_cap: f64,
    ttm: f64,
    five_mins_ranking: f64,
    th60_days_ranking: f64,
    year_change_percent: f64,
    bid: f64,
    bid_volume: f64,
    ask: f64,
    ask_volume: f64,
    open: f64,
    prev_close: f64,
    high: f64,
    low: f64,
    turnover_rate: f64,
    rise_rate: f64,
    amplitude: f64,
    pe_status: f64,
    pb_status: f64,
    volume_ratio: f64,
    bid_ask_ratio: f64,
    latest_pre_close: f64,
    latest_post_close: f64,
    pre_market_cap: f64,
    pre_market_percent: f64,
    pre_market_change: f64,
    post_market_cap: f64,
    post_market_percent: f64,
    post_market_change: f64,
    float_cap: f64,
    shares: i64,
    shares_float: i64,
    day_5_ranking: f64,
    day_10_ranking: f64,
    day_30_ranking: f64,
    day_120_ranking: f64,
    day_250_ranking: f64,
}

impl Stock {
    fn random_update(&mut self) {
        self.price = (-300.0..999.999).fake::<f64>();
        self.change = (-0.1..5.0).fake::<f64>();
        self.change_percent = (-0.1..0.1).fake::<f64>();
        self.volume = (-300.0..999.999).fake::<f64>();
        self.turnover = (-300.0..999.999).fake::<f64>();
        self.market_cap = (-1000.0..9999.999).fake::<f64>();
        self.ttm = (-1000.0..9999.999).fake::<f64>();
        self.five_mins_ranking = self.five_mins_ranking * (1.0 + (-0.2..0.2).fake::<f64>());
        self.bid = self.price * (1.0 + (-0.2..0.2).fake::<f64>());
        self.bid_volume = (100.0..1000.0).fake::<f64>();
        self.ask = self.price * (1.0 + (-0.2..0.2).fake::<f64>());
        self.ask_volume = (100.0..1000.0).fake::<f64>();
        self.bid_ask_ratio = self.bid / self.ask;
        self.volume_ratio = self.volume / self.turnover;
        self.high = self.price * (1.0 + (0.0..1.5).fake::<f64>());
        self.low = self.price * (1.0 + (-1.5..0.0).fake::<f64>());
    }
}

fn random_stocks(size: usize) -> Vec<Stock> {
    // Incremental ID with size.
    let start = {
        let mut id_lock = INCREMENT_ID.lock().unwrap();
        let start = *id_lock;
        *id_lock += size + 1;
        start
    };

    (start..start + size)
        .map(|id| Stock {
            id,
            counter: Counter::random(),
            change: (-100.0..100.0).fake(),
            change_percent: (-0.1..0.1).fake(),
            volume: (0.0..1000.0).fake(),
            turnover: (0.0..1000.0).fake(),
            market_cap: (0.0..1000.0).fake(),
            ttm: (0.0..1000.0).fake(),
            five_mins_ranking: (0.0..1000.0).fake(),
            th60_days_ranking: (0.0..1000.0).fake(),
            year_change_percent: (-1.0..1.0).fake(),
            bid: (0.0..1000.0).fake(),
            bid_volume: (0.0..1000.0).fake(),
            ask: (0.0..1000.0).fake(),
            ask_volume: (0.0..1000.0).fake(),
            open: (0.0..1000.0).fake(),
            prev_close: (0.0..1000.0).fake(),
            high: (0.0..1000.0).fake(),
            low: (0.0..1000.0).fake(),
            turnover_rate: (0.0..1.0).fake(),
            rise_rate: (0.0..1.0).fake(),
            amplitude: (0.0..1000.0).fake(),
            pe_status: (0.0..1000.0).fake(),
            pb_status: (0.0..1000.0).fake(),
            volume_ratio: (0.0..1.0).fake(),
            bid_ask_ratio: (0.0..1.0).fake(),
            latest_pre_close: (0.0..1000.0).fake(),
            latest_post_close: (0.0..1000.0).fake(),
            pre_market_cap: (0.0..1000.0).fake(),
            pre_market_percent: (-1.0..1.0).fake(),
            pre_market_change: (-100.0..100.0).fake(),
            post_market_cap: (0.0..1000.0).fake(),
            post_market_percent: (-1.0..1.0).fake(),
            post_market_change: (-100.0..100.0).fake(),
            float_cap: (0.0..1000.0).fake(),
            shares: (100000..9999999).fake(),
            shares_float: (100000..9999999).fake(),
            day_5_ranking: (0.0..1000.0).fake(),
            day_10_ranking: (0.0..1000.0).fake(),
            day_30_ranking: (0.0..1000.0).fake(),
            day_120_ranking: (0.0..1000.0).fake(),
            day_250_ranking: (0.0..1000.0).fake(),
            ..Default::default()
        })
        .collect()
}

struct StockTableDelegate {
    stocks: Vec<Stock>,
    columns: Vec<Column>,
    size: Size,
    loading: bool,
    lazy_load: bool,
    full_loading: bool,
    clicked_row: Option<usize>,
    eof: bool,
    visible_rows: Range<usize>,
    visible_cols: Range<usize>,

    _load_task: Task<()>,
}

impl StockTableDelegate {
    fn new(size: usize) -> Self {
        Self {
            size: Size::default(),
            stocks: random_stocks(size),
            lazy_load: false,
            clicked_row: None,
            columns: vec![
                Column::new("id", "ID")
                    .width(60.)
                    .fixed(ColumnFixed::Left)
                    .resizable(true)
                    .min_width(40.)
                    .max_width(100.),
                Column::new("market", "Market")
                    .width(60.)
                    .fixed(ColumnFixed::Left)
                    .resizable(true)
                    .min_width(50.),
                Column::new("name", "Name")
                    .width(180.)
                    .fixed(ColumnFixed::Left)
                    .max_width(300.),
                Column::new("symbol", "Symbol")
                    .width(100.)
                    .fixed(ColumnFixed::Left)
                    .sortable(),
                Column::new("price", "Price").sortable().text_right().p_0(),
                Column::new("change", "Chg").sortable().text_right().p_0(),
                Column::new("change_percent", "Chg%")
                    .sortable()
                    .text_right()
                    .p_0(),
                Column::new("volume", "Volume").p_0(),
                Column::new("turnover", "Turnover").p_0(),
                Column::new("market_cap", "Market Cap").p_0(),
                Column::new("ttm", "TTM").p_0(),
                Column::new("five_mins_ranking", "5m Ranking")
                    .text_right()
                    .p_0(),
                Column::new("th60_days_ranking", "60d Ranking"),
                Column::new("year_change_percent", "Year Chg%"),
                Column::new("bid", "Bid").text_right().p_0(),
                Column::new("bid_volume", "Bid Vol").text_right().p_0(),
                Column::new("ask", "Ask").text_right().p_0(),
                Column::new("ask_volume", "Ask Vol").text_right().p_0(),
                Column::new("open", "Open").text_right().p_0(),
                Column::new("prev_close", "Prev Close").text_right().p_0(),
                Column::new("high", "High").text_right().p_0(),
                Column::new("low", "Low").text_right().p_0(),
                Column::new("turnover_rate", "Turnover Rate"),
                Column::new("rise_rate", "Rise Rate"),
                Column::new("amplitude", "Amplitude"),
                Column::new("pe_status", "P/E"),
                Column::new("pb_status", "P/B"),
                Column::new("volume_ratio", "Volume Ratio")
                    .text_right()
                    .p_0(),
                Column::new("bid_ask_ratio", "Bid Ask Ratio")
                    .text_right()
                    .p_0(),
                Column::new("latest_pre_close", "Latest Pre Close"),
                Column::new("latest_post_close", "Latest Post Close"),
                Column::new("pre_market_cap", "Pre Mkt Cap"),
                Column::new("pre_market_percent", "Pre Mkt%"),
                Column::new("pre_market_change", "Pre Mkt Chg"),
                Column::new("post_market_cap", "Post Mkt Cap"),
                Column::new("post_market_percent", "Post Mkt%"),
                Column::new("post_market_change", "Post Mkt Chg"),
                Column::new("float_cap", "Float Cap"),
                Column::new("shares", "Shares"),
                Column::new("shares_float", "Float Shares"),
                Column::new("day_5_ranking", "5d Ranking"),
                Column::new("day_10_ranking", "10d Ranking"),
                Column::new("day_30_ranking", "30d Ranking"),
                Column::new("day_120_ranking", "120d Ranking"),
                Column::new("day_250_ranking", "250d Ranking"),
            ],
            loading: false,
            full_loading: false,
            eof: false,
            visible_cols: Range::default(),
            visible_rows: Range::default(),
            _load_task: Task::ready(()),
        }
    }

    fn update_stocks(&mut self, size: usize) {
        // Reset incremental ID
        {
            let mut id_lock = INCREMENT_ID.lock().unwrap();
            *id_lock = 0;
        }

        self.stocks = random_stocks(size);
        self.eof = size <= 50;
        self.loading = false;
        self.full_loading = false;
    }

    fn render_percent(&self, col: &Column, val: f64, cx: &mut App) -> AnyElement {
        let right_num = ((val - val.floor()) * 1000.).floor() as i32;

        div()
            .h_full()
            .table_cell_size(self.size)
            .when(col.align == TextAlign::Right, |this| {
                this.h_flex().justify_end()
            })
            .map(|this| {
                if right_num % 3 == 0 {
                    this.text_color(cx.theme().red)
                        .bg(cx.theme().red_light.alpha(0.05))
                } else if right_num % 3 == 1 {
                    this.text_color(cx.theme().green)
                        .bg(cx.theme().green_light.alpha(0.05))
                } else {
                    this
                }
            })
            .child(format!("{:.2}%", val * 100.))
            .into_any_element()
    }

    fn render_value_cell(&self, col: &Column, val: f64, cx: &mut App) -> AnyElement {
        let this = div()
            .h_full()
            .table_cell_size(self.size)
            .child(format!("{:.3}", val));
        // Val is a 0.0 .. n.0
        // 30% to red, 30% to green, others to default
        let right_num = ((val - val.floor()) * 1000.).floor() as i32;

        let this = if right_num % 3 == 0 {
            this.text_color(cx.theme().red)
                .bg(cx.theme().red_light.alpha(0.05))
        } else if right_num % 3 == 1 {
            this.text_color(cx.theme().green)
                .bg(cx.theme().green_light.alpha(0.05))
        } else {
            this
        };

        this.when(col.align == TextAlign::Right, |this| {
            this.h_flex().justify_end()
        })
        .into_any_element()
    }
}

impl TableDelegate for StockTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.stocks.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let col = self.columns.get(col_ix).unwrap();

        div()
            .child(col.name.clone())
            .when(col_ix >= 3 && col_ix <= 10, |this| {
                this.table_cell_size(self.size)
            })
            .when(col.align == TextAlign::Right, |this| {
                this.h_flex().w_full().justify_end()
            })
    }

    fn context_menu(
        &mut self,
        row_ix: usize,
        menu: PopupMenu,
        _window: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        menu.menu(
            format!("Selected Row: {}", row_ix),
            Box::new(OpenDetail(row_ix)),
        )
        .separator()
        .menu("Size Large", Box::new(ChangeSize(Size::Large)))
        .menu("Size Medium", Box::new(ChangeSize(Size::Medium)))
        .menu("Size Small", Box::new(ChangeSize(Size::Small)))
        .menu("Size XSmall", Box::new(ChangeSize(Size::XSmall)))
    }

    fn render_tr(
        &mut self,
        row_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> Stateful<Div> {
        div()
            .id(row_ix)
            .on_click(cx.listener(move |table, ev: &ClickEvent, _window, cx| {
                println!(
                    "You have clicked row with secondary: {}",
                    ev.modifiers().secondary()
                );

                table.delegate_mut().clicked_row = Some(row_ix);
                cx.notify();
            }))
    }

    /// NOTE: Performance metrics
    ///
    /// last render 561 cells total: 232.745µs, avg: 414ns
    /// frame duration: 8.825083ms
    ///
    /// This is means render the full table cells takes 232.745µs. Then 232.745µs / 8.82ms = 2.6% of the frame duration.
    ///
    /// If we improve the td rendering, we can reduce the time to render the full table cells.
    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let stock = self.stocks.get(row_ix).unwrap();
        let col = self.columns.get(col_ix).unwrap();

        match col.key.as_ref() {
            "id" => stock.id.to_string().into_any_element(),
            "market" => div()
                .map(|this| {
                    if stock.counter.market == "US" {
                        this.text_color(cx.theme().blue)
                    } else {
                        this.text_color(cx.theme().magenta)
                    }
                })
                .child(stock.counter.market.clone())
                .into_any_element(),
            "symbol" => stock.counter.symbol_code().into_any_element(),
            "name" => stock.counter.name.clone().into_any_element(),
            "price" => self.render_value_cell(&col, stock.price, cx),
            "change" => self.render_value_cell(&col, stock.change, cx),
            "change_percent" => self.render_percent(&col, stock.change_percent, cx),
            "volume" => self.render_value_cell(&col, stock.volume, cx),
            "turnover" => self.render_value_cell(&col, stock.turnover, cx),
            "market_cap" => self.render_value_cell(&col, stock.market_cap, cx),
            "ttm" => self.render_value_cell(&col, stock.ttm, cx),
            "five_mins_ranking" => self.render_value_cell(&col, stock.five_mins_ranking, cx),
            "th60_days_ranking" => stock
                .th60_days_ranking
                .floor()
                .to_string()
                .into_any_element(),
            "year_change_percent" => self.render_percent(&col, stock.year_change_percent, cx),
            "bid" => self.render_value_cell(&col, stock.bid, cx),
            "bid_volume" => self.render_value_cell(&col, stock.bid_volume, cx),
            "ask" => self.render_value_cell(&col, stock.ask, cx),
            "ask_volume" => self.render_value_cell(&col, stock.ask_volume, cx),
            "open" => self.render_value_cell(&col, stock.open, cx),
            "prev_close" => self.render_value_cell(&col, stock.prev_close, cx),
            "high" => self.render_value_cell(&col, stock.high, cx),
            "low" => self.render_value_cell(&col, stock.low, cx),
            "turnover_rate" => (stock.turnover_rate * 100.0)
                .floor()
                .to_string()
                .into_any_element(),
            "rise_rate" => (stock.rise_rate * 100.0)
                .floor()
                .to_string()
                .into_any_element(),
            "amplitude" => (stock.amplitude * 100.0)
                .floor()
                .to_string()
                .into_any_element(),
            "pe_status" => stock.pe_status.floor().to_string().into_any_element(),
            "pb_status" => stock.pb_status.floor().to_string().into_any_element(),
            "volume_ratio" => self.render_value_cell(&col, stock.volume_ratio, cx),
            "bid_ask_ratio" => self.render_value_cell(&col, stock.bid_ask_ratio, cx),
            "latest_pre_close" => stock
                .latest_pre_close
                .floor()
                .to_string()
                .into_any_element(),
            "latest_post_close" => stock
                .latest_post_close
                .floor()
                .to_string()
                .into_any_element(),
            "pre_market_cap" => stock.pre_market_cap.floor().to_string().into_any_element(),
            "pre_market_percent" => self.render_percent(&col, stock.pre_market_percent, cx),
            "pre_market_change" => stock
                .pre_market_change
                .floor()
                .to_string()
                .into_any_element(),
            "post_market_cap" => stock.post_market_cap.floor().to_string().into_any_element(),
            "post_market_percent" => self.render_percent(&col, stock.post_market_percent, cx),
            "post_market_change" => stock
                .post_market_change
                .floor()
                .to_string()
                .into_any_element(),
            "float_cap" => stock.float_cap.floor().to_string().into_any_element(),
            "shares" => stock.shares.to_string().into_any_element(),
            "shares_float" => stock.shares_float.to_string().into_any_element(),
            "day_5_ranking" => stock.day_5_ranking.floor().to_string().into_any_element(),
            "day_10_ranking" => stock.day_10_ranking.floor().to_string().into_any_element(),
            "day_30_ranking" => stock.day_30_ranking.floor().to_string().into_any_element(),
            "day_120_ranking" => stock.day_120_ranking.floor().to_string().into_any_element(),
            "day_250_ranking" => stock.day_250_ranking.floor().to_string().into_any_element(),
            _ => "--".to_string().into_any_element(),
        }
    }

    fn move_column(
        &mut self,
        col_ix: usize,
        to_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        let col = self.columns.remove(col_ix);
        self.columns.insert(to_ix, col);
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        if let Some(col) = self.columns.get_mut(col_ix) {
            match col.key.as_ref() {
                "id" => self.stocks.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b.id.cmp(&a.id),
                    _ => a.id.cmp(&b.id),
                }),
                "symbol" => self.stocks.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b.counter.symbol.cmp(&a.counter.symbol),
                    _ => a.id.cmp(&b.id),
                }),
                "change" | "change_percent" => self.stocks.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b
                        .change
                        .partial_cmp(&a.change)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    _ => a.id.cmp(&b.id),
                }),
                _ => {}
            }
        }
    }

    fn loading(&self, _: &App) -> bool {
        self.full_loading
    }

    fn has_more(&self, _: &App) -> bool {
        if !self.lazy_load {
            return false;
        }
        if self.loading {
            return false;
        }

        return !self.eof;
    }

    fn load_more_threshold(&self) -> usize {
        150
    }

    fn load_more(&mut self, _: &mut Window, cx: &mut Context<TableState<Self>>) {
        if !self.lazy_load {
            return;
        }

        self.loading = true;

        self._load_task = cx.spawn(async move |view, cx| {
            // Simulate network request, delay 1s to load data.
            Timer::after(Duration::from_secs(1)).await;

            _ = cx.update(|cx| {
                let _ = view.update(cx, |view, _| {
                    view.delegate_mut().stocks.extend(random_stocks(200));
                    view.delegate_mut().loading = false;
                    view.delegate_mut().eof = view.delegate().stocks.len() >= 6000;
                });
            });
        });
    }

    fn visible_rows_changed(
        &mut self,
        visible_range: Range<usize>,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        self.visible_rows = visible_range;
    }

    fn visible_columns_changed(
        &mut self,
        visible_range: Range<usize>,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        self.visible_cols = visible_range;
    }
}

pub struct TableStory {
    table: Entity<TableState<StockTableDelegate>>,
    num_stocks_input: Entity<InputState>,
    stripe: bool,
    refresh_data: bool,
    size: Size,

    _subscriptions: Vec<Subscription>,
    _load_task: Task<()>,
}

impl super::Story for TableStory {
    fn title() -> &'static str {
        "Table"
    }

    fn description() -> &'static str {
        "A complex data table with selection, sorting, column moving, and loading more."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn closable() -> bool {
        false
    }
}

impl Focusable for TableStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.table.focus_handle(cx)
    }
}

impl TableStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Create the number input field with validation for positive integers
        let num_stocks_input = cx.new(|cx| {
            let mut input = InputState::new(window, cx)
                .placeholder("Enter number of Stocks to display")
                .validate(|s, _| s.parse::<usize>().is_ok());
            input.set_value("5000", window, cx);
            input
        });

        let delegate = StockTableDelegate::new(5000);
        let table = cx.new(|cx| TableState::new(delegate, window, cx));

        let _subscriptions = vec![
            cx.subscribe_in(&table, window, Self::on_table_event),
            cx.subscribe_in(&num_stocks_input, window, Self::on_num_stocks_input_change),
            // Spawn a background to random refresh the list
        ];

        let _load_task = cx.spawn(async move |this, cx| {
            loop {
                Timer::after(time::Duration::from_millis(33)).await;

                this.update(cx, |this, cx| {
                    if !this.refresh_data {
                        return;
                    }

                    this.table.update(cx, |table, _| {
                        table.delegate_mut().stocks.iter_mut().enumerate().for_each(
                            |(i, stock)| {
                                let n = (3..10).fake::<usize>();
                                // update 30% of the stocks
                                if i % n == 0 {
                                    stock.random_update();
                                }
                            },
                        );
                    });
                    cx.notify();
                })
                .ok();
            }
        });

        Self {
            table,
            num_stocks_input,
            stripe: false,
            refresh_data: false,
            size: Size::default(),
            _subscriptions,
            _load_task,
        }
    }

    // Event handler for changes in the number input field
    fn on_num_stocks_input_change(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            // Update when the user presses Enter or the input loses focus
            InputEvent::PressEnter { .. } | InputEvent::Blur => {
                let text = self.num_stocks_input.read(cx).value().to_string();
                if let Ok(total_count) = text.parse::<usize>() {
                    if total_count == self.table.read(cx).delegate().stocks.len() {
                        return;
                    }

                    self.table.update(cx, |table, _| {
                        table.delegate_mut().update_stocks(total_count);
                    });
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    fn toggle_loop_selection(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.loop_selection = *checked;
            cx.notify();
        });
    }

    fn toggle_col_resize(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.col_resizable = *checked;
            cx.notify();
        });
    }

    fn toggle_col_order(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.col_movable = *checked;
            cx.notify();
        });
    }

    fn toggle_col_sort(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.sortable = *checked;
            cx.notify();
        });
    }

    fn toggle_col_fixed(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.col_fixed = *checked;
            cx.notify();
        });
    }

    fn toggle_col_selection(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.col_selectable = *checked;
            cx.notify();
        });
    }

    fn toggle_stripe(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.stripe = *checked;
        cx.notify();
    }

    fn on_change_size(&mut self, a: &ChangeSize, _: &mut Window, cx: &mut Context<Self>) {
        self.size = a.0;
        cx.notify();
    }

    fn toggle_refresh_data(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.refresh_data = *checked;
        cx.notify();
    }

    fn on_table_event(
        &mut self,
        _: &Entity<TableState<StockTableDelegate>>,
        event: &TableEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            TableEvent::ColumnWidthsChanged(col_widths) => {
                println!("Column widths changed: {:?}", col_widths)
            }
            TableEvent::SelectColumn(ix) => println!("Select col: {}", ix),
            TableEvent::DoubleClickedRow(ix) => println!("Double clicked row: {}", ix),
            TableEvent::SelectRow(ix) => println!("Select row: {}", ix),
            TableEvent::MoveColumn(origin_idx, target_idx) => {
                println!("Move col index: {} -> {}", origin_idx, target_idx);
            }
        }
    }
}

impl Render for TableStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let table = &self.table.read(cx);
        let delegate = table.delegate();
        let rows_count = delegate.rows_count(cx);
        let size = self.size;

        v_flex()
            .on_action(cx.listener(Self::on_change_size))
            .size_full()
            .text_sm()
            .gap_4()
            .child(
                h_flex()
                    .items_center()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        Checkbox::new("loop-selection")
                            .label("Loop Selection")
                            .selected(table.loop_selection)
                            .on_click(cx.listener(Self::toggle_loop_selection)),
                    )
                    .child(
                        Checkbox::new("col-resize")
                            .label("Column Resize")
                            .selected(table.col_resizable)
                            .on_click(cx.listener(Self::toggle_col_resize)),
                    )
                    .child(
                        Checkbox::new("col-order")
                            .label("Column Order")
                            .selected(table.col_movable)
                            .on_click(cx.listener(Self::toggle_col_order)),
                    )
                    .child(
                        Checkbox::new("col-sort")
                            .label("Sortable")
                            .selected(table.sortable)
                            .on_click(cx.listener(Self::toggle_col_sort)),
                    )
                    .child(
                        Checkbox::new("col-selection")
                            .label("Column Selectable")
                            .selected(table.col_selectable)
                            .on_click(cx.listener(Self::toggle_col_selection)),
                    )
                    .child(
                        Checkbox::new("fixed")
                            .label("Column Fixed")
                            .selected(table.col_fixed)
                            .on_click(cx.listener(Self::toggle_col_fixed)),
                    )
                    .child(
                        Checkbox::new("stripe")
                            .label("Stripe")
                            .selected(self.stripe)
                            .on_click(cx.listener(Self::toggle_stripe)),
                    )
                    .child(
                        Checkbox::new("loading")
                            .label("Loading")
                            .checked(self.table.read(cx).delegate().full_loading)
                            .on_click(cx.listener(|this, check: &bool, _, cx| {
                                this.table.update(cx, |this, cx| {
                                    this.delegate_mut().full_loading = *check;
                                    cx.notify();
                                })
                            })),
                    )
                    .child(
                        Checkbox::new("refresh-data")
                            .label("Refresh Data")
                            .selected(self.refresh_data)
                            .on_click(cx.listener(Self::toggle_refresh_data)),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("size")
                            .outline()
                            .small()
                            .label(format!("size: {:?}", self.size))
                            .dropdown_menu(move |menu, _, _| {
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
                                .menu_with_check(
                                    "XSmall",
                                    size == Size::XSmall,
                                    Box::new(ChangeSize(Size::XSmall)),
                                )
                            }),
                    )
                    .child(
                        Button::new("scroll-top")
                            .outline()
                            .small()
                            .child("Scroll to Top")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.table.update(cx, |table, cx| {
                                    table.scroll_to_row(0, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-bottom")
                            .outline()
                            .small()
                            .child("Scroll to Bottom")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.table.update(cx, |table, cx| {
                                    table.scroll_to_row(table.delegate().rows_count(cx) - 1, cx);
                                })
                            })),
                    ), // .child(
                       //     Button::new("scroll-first-col")
                       //         .child("Scroll to First Column")
                       //         .small()
                       //         .on_click(cx.listener(|this, _, window, cx| {
                       //             this.table.update(cx, |table, cx| {
                       //                 table.scroll_to_col(0, cx);
                       //             })
                       //         })),
                       // )
                       // .child(
                       //     Button::new("scroll-last-col")
                       //         .child("Scroll to Last Column")
                       //         .small()
                       //         .on_click(cx.listener(|this, _, window, cx| {
                       //             this.table.update(cx, |table, cx| {
                       //                 table.scroll_to_col(table.delegate().columns_count(cx), cx);
                       //             })
                       //         })),
                       // ),
            )
            .child(
                h_flex().items_center().gap_2().child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            h_flex()
                                .gap_2()
                                .flex_1()
                                .child(Label::new("Number of Stocks:"))
                                .child(
                                    h_flex()
                                        .min_w_32()
                                        .child(Input::new(&self.num_stocks_input).small())
                                        .into_any_element(),
                                )
                                .when(delegate.loading, |this| {
                                    this.child(
                                        h_flex().gap_1().child(Spinner::new()).child("Loading..."),
                                    )
                                })
                                .child(
                                    Checkbox::new("lazy-load")
                                        .label("Lazy Load")
                                        .checked(delegate.lazy_load)
                                        .on_click(cx.listener(|this, check: &bool, _, cx| {
                                            this.table.update(cx, |table, cx| {
                                                table.delegate_mut().lazy_load = *check;
                                                cx.notify();
                                            })
                                        })),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .child(format!("Total Rows: {}", rows_count))
                                .child(format!("Visible Rows: {:?}", delegate.visible_rows))
                                .child(format!("Visible Cols: {:?}", delegate.visible_cols))
                                .when(delegate.eof, |this| this.child("All data loaded.")),
                        ),
                ),
            )
            .child(
                Table::new(&self.table)
                    .with_size(self.size)
                    .stripe(self.stripe),
            )
    }
}
