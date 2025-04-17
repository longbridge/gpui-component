use std::{
    ops::Range,
    time::{self, Duration},
};

use fake::{Fake, Faker};
use gpui::{
    div, impl_internal_actions, prelude::FluentBuilder as _, px, AnyElement, App, AppContext,
    ClickEvent, Context, Edges, Entity, Focusable, InteractiveElement, IntoElement, ParentElement,
    Pixels, Render, SharedString, StatefulInteractiveElement, Styled, Timer, Window,
};
use gpui_component::{
    button::Button,
    checkbox::Checkbox,
    green, h_flex,
    indicator::Indicator,
    input::{InputEvent, TextInput},
    label::Label,
    popup_menu::{PopupMenu, PopupMenuExt},
    red,
    table::{self, ColFixed, ColSort, Table, TableDelegate, TableEvent},
    v_flex, ActiveTheme as _, Selectable, Sizable as _, Size, StyleSized as _,
};
use serde::Deserialize;

#[derive(Clone, PartialEq, Eq, Deserialize)]
struct ChangeSize(Size);

#[derive(Clone, PartialEq, Eq, Deserialize)]
struct OpenDetail(usize);

impl_internal_actions!(table_story, [ChangeSize, OpenDetail]);

#[derive(Clone, Debug, Default)]
struct Stock {
    id: usize,
    symbol: SharedString,
    name: SharedString,
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
    (0..size)
        .map(|id| Stock {
            id,
            symbol: Faker.fake::<String>().into(),
            name: Faker.fake::<String>().into(),
            change: (-100.0..100.0).fake(),
            change_percent: (-1.0..1.0).fake(),
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

struct Column {
    id: SharedString,
    name: SharedString,
    sort: Option<ColSort>,
}

impl Column {
    fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        sort: Option<ColSort>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            sort,
        }
    }
}

struct StockTableDelegate {
    stocks: Vec<Stock>,
    columns: Vec<Column>,
    size: Size,
    loop_selection: bool,
    col_resize: bool,
    col_order: bool,
    col_sort: bool,
    col_selection: bool,
    loading: bool,
    full_loading: bool,
    fixed_cols: bool,
    eof: bool,
    visible_rows: Range<usize>,
    visible_cols: Range<usize>,
}

impl StockTableDelegate {
    fn new(size: usize) -> Self {
        Self {
            size: Size::default(),
            stocks: random_stocks(size),
            columns: vec![
                Column::new("id", "ID", None),
                Column::new("symbol", "Symbol", Some(ColSort::Default)),
                Column::new("name", "Name", None),
                Column::new("price", "Price", Some(ColSort::Default)),
                Column::new("change", "Chg", Some(ColSort::Default)),
                Column::new("change_percent", "Chg%", Some(ColSort::Default)),
                Column::new("volume", "Volume", None),
                Column::new("turnover", "Turnover", None),
                Column::new("market_cap", "Market Cap", None),
                Column::new("ttm", "TTM", None),
                Column::new("five_mins_ranking", "5m Ranking", None),
                Column::new("th60_days_ranking", "60d Ranking", None),
                Column::new("year_change_percent", "Year Chg%", None),
                Column::new("bid", "Bid", None),
                Column::new("bid_volume", "Bid Vol", None),
                Column::new("ask", "Ask", None),
                Column::new("ask_volume", "Ask Vol", None),
                Column::new("open", "Open", None),
                Column::new("prev_close", "Prev Close", None),
                Column::new("high", "High", None),
                Column::new("low", "Low", None),
                Column::new("turnover_rate", "Turnover Rate", None),
                Column::new("rise_rate", "Rise Rate", None),
                Column::new("amplitude", "Amplitude", None),
                Column::new("pe_status", "P/E", None),
                Column::new("pb_status", "P/B", None),
                Column::new("volume_ratio", "Volume Ratio", None),
                Column::new("bid_ask_ratio", "Bid Ask Ratio", None),
                Column::new("latest_pre_close", "Latest Pre Close", None),
                Column::new("latest_post_close", "Latest Post Close", None),
                Column::new("pre_market_cap", "Pre Mkt Cap", None),
                Column::new("pre_market_percent", "Pre Mkt%", None),
                Column::new("pre_market_change", "Pre Mkt Chg", None),
                Column::new("post_market_cap", "Post Mkt Cap", None),
                Column::new("post_market_percent", "Post Mkt%", None),
                Column::new("post_market_change", "Post Mkt Chg", None),
                Column::new("float_cap", "Float Cap", None),
                Column::new("shares", "Shares", None),
                Column::new("shares_float", "Float Shares", None),
                Column::new("day_5_ranking", "5d Ranking", None),
                Column::new("day_10_ranking", "10d Ranking", None),
                Column::new("day_30_ranking", "30d Ranking", None),
                Column::new("day_120_ranking", "120d Ranking", None),
                Column::new("day_250_ranking", "250d Ranking", None),
            ],
            loop_selection: true,
            col_resize: true,
            col_order: true,
            col_sort: true,
            col_selection: true,
            fixed_cols: false,
            loading: false,
            full_loading: false,
            eof: false,
            visible_cols: Range::default(),
            visible_rows: Range::default(),
        }
    }

    fn update_stocks(&mut self, size: usize) {
        self.stocks = random_stocks(size);
        self.eof = size <= 50;
        self.loading = false;
        self.full_loading = false;
    }

    fn render_value_cell(&self, val: f64, cx: &mut Context<Table<Self>>) -> AnyElement {
        let (fg_scale, bg_scale, opacity) = match cx.theme().mode.is_dark() {
            true => (200, 950, 0.3),
            false => (600, 50, 0.6),
        };

        let this = div()
            .h_full()
            .table_cell_size(self.size)
            .child(format!("{:.3}", val));
        // Val is a 0.0 .. n.0
        // 30% to red, 30% to green, others to default
        let right_num = ((val - val.floor()) * 1000.).floor() as i32;

        let this = if right_num % 3 == 0 {
            this.text_color(red(fg_scale))
                .bg(red(bg_scale).opacity(opacity))
        } else if right_num % 3 == 1 {
            this.text_color(green(fg_scale))
                .bg(green(bg_scale).opacity(opacity))
        } else {
            this
        };

        this.into_any_element()
    }
}

impl TableDelegate for StockTableDelegate {
    fn cols_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.stocks.len()
    }

    fn col_name(&self, col_ix: usize, _: &App) -> SharedString {
        if let Some(col) = self.columns.get(col_ix) {
            col.name.clone()
        } else {
            "--".into()
        }
    }

    fn col_width(&self, col_ix: usize, _: &App) -> Pixels {
        if col_ix < 10 {
            120.0.into()
        } else if col_ix < 20 {
            80.0.into()
        } else {
            130.0.into()
        }
    }

    fn col_padding(&self, col_ix: usize, _: &App) -> Option<Edges<Pixels>> {
        if col_ix >= 3 && col_ix <= 10 {
            Some(Edges::all(px(0.)))
        } else {
            None
        }
    }

    fn col_fixed(&self, col_ix: usize, _: &App) -> Option<table::ColFixed> {
        if !self.fixed_cols {
            return None;
        }

        if col_ix < 4 {
            Some(ColFixed::Left)
        } else {
            None
        }
    }

    fn can_resize_col(&self, col_ix: usize, _: &App) -> bool {
        return self.col_resize && col_ix > 1;
    }

    fn can_select_col(&self, _: usize, _: &App) -> bool {
        return self.col_selection;
    }

    fn render_th(
        &self,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<Table<Self>>,
    ) -> impl IntoElement {
        let th = div().child(self.col_name(col_ix, cx));

        if col_ix >= 3 && col_ix <= 10 {
            th.table_cell_size(self.size)
        } else {
            th
        }
    }

    fn context_menu(
        &self,
        row_ix: usize,
        menu: PopupMenu,
        _window: &Window,
        _cx: &App,
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
        &self,
        row_ix: usize,
        _: &mut Window,
        cx: &mut Context<Table<Self>>,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id(row_ix)
            .on_click(cx.listener(|_, ev: &ClickEvent, _, _| {
                println!(
                    "You have clicked row with secondary: {}",
                    ev.modifiers().secondary()
                )
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
        &self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<Table<Self>>,
    ) -> impl IntoElement {
        let stock = self.stocks.get(row_ix).unwrap();
        let col = self.columns.get(col_ix).unwrap();

        match col.id.as_ref() {
            "id" => stock.id.to_string().into_any_element(),
            "name" => stock.name.clone().into_any_element(),
            "symbol" => stock.symbol.clone().into_any_element(),
            "price" => self.render_value_cell(stock.price, cx),
            "change" => self.render_value_cell(stock.change, cx),
            "change_percent" => self.render_value_cell(stock.change_percent, cx),
            "volume" => self.render_value_cell(stock.volume, cx),
            "turnover" => self.render_value_cell(stock.turnover, cx),
            "market_cap" => self.render_value_cell(stock.market_cap, cx),
            "ttm" => self.render_value_cell(stock.ttm, cx),
            "five_mins_ranking" => self.render_value_cell(stock.five_mins_ranking, cx),
            "th60_days_ranking" => stock.th60_days_ranking.to_string().into_any_element(),
            "year_change_percent" => (stock.year_change_percent * 100.0)
                .to_string()
                .into_any_element(),
            "bid" => self.render_value_cell(stock.bid, cx),
            "bid_volume" => self.render_value_cell(stock.bid_volume, cx),
            "ask" => self.render_value_cell(stock.ask, cx),
            "ask_volume" => self.render_value_cell(stock.ask_volume, cx),
            "open" => stock.open.to_string().into_any_element(),
            "prev_close" => stock.prev_close.to_string().into_any_element(),
            "high" => self.render_value_cell(stock.high, cx),
            "low" => self.render_value_cell(stock.low, cx),
            "turnover_rate" => (stock.turnover_rate * 100.0).to_string().into_any_element(),
            "rise_rate" => (stock.rise_rate * 100.0).to_string().into_any_element(),
            "amplitude" => (stock.amplitude * 100.0).to_string().into_any_element(),
            "pe_status" => stock.pe_status.to_string().into_any_element(),
            "pb_status" => stock.pb_status.to_string().into_any_element(),
            "volume_ratio" => self.render_value_cell(stock.volume_ratio, cx),
            "bid_ask_ratio" => self.render_value_cell(stock.bid_ask_ratio, cx),
            "latest_pre_close" => stock.latest_pre_close.to_string().into_any_element(),
            "latest_post_close" => stock.latest_post_close.to_string().into_any_element(),
            "pre_market_cap" => stock.pre_market_cap.to_string().into_any_element(),
            "pre_market_percent" => (stock.pre_market_percent * 100.0)
                .to_string()
                .into_any_element(),
            "pre_market_change" => stock.pre_market_change.to_string().into_any_element(),
            "post_market_cap" => stock.post_market_cap.to_string().into_any_element(),
            "post_market_percent" => (stock.post_market_percent * 100.0)
                .to_string()
                .into_any_element(),
            "post_market_change" => stock.post_market_change.to_string().into_any_element(),
            "float_cap" => stock.float_cap.to_string().into_any_element(),
            "shares" => stock.shares.to_string().into_any_element(),
            "shares_float" => stock.shares_float.to_string().into_any_element(),
            "day_5_ranking" => stock.day_5_ranking.to_string().into_any_element(),
            "day_10_ranking" => stock.day_10_ranking.to_string().into_any_element(),
            "day_30_ranking" => stock.day_30_ranking.to_string().into_any_element(),
            "day_120_ranking" => stock.day_120_ranking.to_string().into_any_element(),
            "day_250_ranking" => stock.day_250_ranking.to_string().into_any_element(),
            _ => "--".to_string().into_any_element(),
        }
    }

    fn can_loop_select(&self, _: &App) -> bool {
        self.loop_selection
    }

    fn can_move_col(&self, _: usize, _: &App) -> bool {
        self.col_order
    }

    fn move_col(
        &mut self,
        col_ix: usize,
        to_ix: usize,
        _: &mut Window,
        _: &mut Context<Table<Self>>,
    ) {
        let col = self.columns.remove(col_ix);
        self.columns.insert(to_ix, col);
    }

    fn col_sort(&self, col_ix: usize, _: &App) -> Option<ColSort> {
        if !self.col_sort {
            return None;
        }

        self.columns.get(col_ix).and_then(|c| c.sort)
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColSort,
        _: &mut Window,
        _: &mut Context<Table<Self>>,
    ) {
        if !self.col_sort {
            return;
        }

        if let Some(col) = self.columns.get_mut(col_ix) {
            match col.id.as_ref() {
                "id" => self.stocks.sort_by(|a, b| match sort {
                    ColSort::Descending => b.id.cmp(&a.id),
                    _ => a.id.cmp(&b.id),
                }),
                "symbol" => self.stocks.sort_by(|a, b| match sort {
                    ColSort::Descending => b.symbol.cmp(&a.symbol),
                    _ => a.id.cmp(&b.id),
                }),
                "change" | "change_percent" => self.stocks.sort_by(|a, b| match sort {
                    ColSort::Descending => b
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

    fn can_load_more(&self, _: &App) -> bool {
        return !self.loading && !self.eof;
    }

    fn load_more_threshold(&self) -> usize {
        150
    }

    fn load_more(&mut self, _: &mut Window, cx: &mut Context<Table<Self>>) {
        self.loading = true;

        cx.spawn(async move |view, cx| {
            // Simulate network request, delay 1s to load data.
            Timer::after(Duration::from_secs(1)).await;

            cx.update(|cx| {
                let _ = view.update(cx, |view, _| {
                    view.delegate_mut().stocks.extend(random_stocks(200));
                    view.delegate_mut().loading = false;
                    view.delegate_mut().eof = view.delegate().stocks.len() >= 6000;
                });
            })
        })
        .detach();
    }

    fn visible_rows_changed(
        &mut self,
        visible_range: Range<usize>,
        _: &mut Window,
        _: &mut Context<Table<Self>>,
    ) {
        self.visible_rows = visible_range;
    }

    fn visible_cols_changed(
        &mut self,
        visible_range: Range<usize>,
        _: &mut Window,
        _: &mut Context<Table<Self>>,
    ) {
        self.visible_cols = visible_range;
    }
}

pub struct TableStory {
    table: Entity<Table<StockTableDelegate>>,
    num_stocks_input: Entity<TextInput>,
    stripe: bool,
    refresh_data: bool,
    size: Size,
}

impl super::Story for TableStory {
    fn title() -> &'static str {
        "Table"
    }

    fn description() -> &'static str {
        "A complex data table with selection, sorting, column moving, and loading more."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
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
            let mut input = TextInput::new(window, cx)
                .placeholder("Enter number of Stocks to display")
                .validate(|s| s.parse::<usize>().is_ok());
            input.set_text("5000", window, cx);
            input
        });

        let delegate = StockTableDelegate::new(5000);
        let table = cx.new(|cx| Table::new(delegate, window, cx));

        cx.subscribe_in(&table, window, Self::on_table_event)
            .detach();
        cx.subscribe_in(&num_stocks_input, window, Self::on_num_stocks_input_change)
            .detach();

        // Spawn a background to random refresh the list
        cx.spawn(async move |this, cx| {
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
        })
        .detach();

        Self {
            table,
            num_stocks_input,
            stripe: false,
            refresh_data: false,
            size: Size::default(),
        }
    }

    // Event handler for changes in the number input field
    fn on_num_stocks_input_change(
        &mut self,
        _: &Entity<TextInput>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            // Update when the user presses Enter or the input loses focus
            InputEvent::PressEnter { .. } | InputEvent::Blur => {
                let text = self.num_stocks_input.read(cx).text().to_string();
                if let Ok(num) = text.parse::<usize>() {
                    self.table.update(cx, |table, _| {
                        table.delegate_mut().update_stocks(num);
                    });
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    fn toggle_loop_selection(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.delegate_mut().loop_selection = *checked;
            cx.notify();
        });
    }

    fn toggle_col_resize(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.delegate_mut().col_resize = *checked;
            table.refresh(cx);
            cx.notify();
        });
    }

    fn toggle_col_order(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.delegate_mut().col_order = *checked;
            table.refresh(cx);
            cx.notify();
        });
    }

    fn toggle_col_sort(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.delegate_mut().col_sort = *checked;
            table.refresh(cx);
            cx.notify();
        });
    }

    fn toggle_col_selection(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.delegate_mut().col_selection = *checked;
            table.refresh(cx);
            cx.notify();
        });
    }

    fn toggle_stripe(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.stripe = *checked;
        let stripe = self.stripe;
        self.table.update(cx, |table, cx| {
            table.set_stripe(stripe, cx);
            cx.notify();
        });
    }

    fn toggle_fixed_cols(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.delegate_mut().fixed_cols = *checked;
            table.refresh(cx);
            cx.notify();
        });
    }

    fn on_change_size(&mut self, a: &ChangeSize, _: &mut Window, cx: &mut Context<Self>) {
        self.size = a.0;
        self.table.update(cx, |table, cx| {
            table.set_size(a.0, cx);
            table.delegate_mut().size = a.0;
        });
    }

    fn toggle_refresh_data(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.refresh_data = *checked;
        cx.notify();
    }

    fn on_table_event(
        &mut self,
        _: &Entity<Table<StockTableDelegate>>,
        event: &TableEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            TableEvent::ColWidthsChanged(col_widths) => {
                println!("Col widths changed: {:?}", col_widths)
            }
            TableEvent::SelectCol(ix) => println!("Select col: {}", ix),
            TableEvent::DoubleClickedRow(ix) => println!("Double clicked row: {}", ix),
            TableEvent::SelectRow(ix) => println!("Select row: {}", ix),
            TableEvent::MoveCol(origin_idx, target_idx) => {
                println!("Move col index: {} -> {}", origin_idx, target_idx);
            }
        }
    }
}

impl Render for TableStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let delegate = self.table.read(cx).delegate();
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
                            .selected(delegate.loop_selection)
                            .on_click(cx.listener(Self::toggle_loop_selection)),
                    )
                    .child(
                        Checkbox::new("col-resize")
                            .label("Column Resize")
                            .selected(delegate.col_resize)
                            .on_click(cx.listener(Self::toggle_col_resize)),
                    )
                    .child(
                        Checkbox::new("col-order")
                            .label("Column Order")
                            .selected(delegate.col_order)
                            .on_click(cx.listener(Self::toggle_col_order)),
                    )
                    .child(
                        Checkbox::new("col-sort")
                            .label("Column Sort")
                            .selected(delegate.col_sort)
                            .on_click(cx.listener(Self::toggle_col_sort)),
                    )
                    .child(
                        Checkbox::new("col-selection")
                            .label("Column Selection")
                            .selected(delegate.col_selection)
                            .on_click(cx.listener(Self::toggle_col_selection)),
                    )
                    .child(
                        Checkbox::new("stripe")
                            .label("Stripe")
                            .selected(self.stripe)
                            .on_click(cx.listener(Self::toggle_stripe)),
                    )
                    .child(
                        Checkbox::new("fixed-cols")
                            .label("Fixed Columns")
                            .selected(delegate.fixed_cols)
                            .on_click(cx.listener(Self::toggle_fixed_cols)),
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
                            .small()
                            .label(format!("size: {:?}", self.size))
                            .popup_menu(move |menu, _, _| {
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
                            .child("Scroll to Top")
                            .small()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.table.update(cx, |table, cx| {
                                    table.scroll_to_row(0, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-bottom")
                            .child("Scroll to Bottom")
                            .small()
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
                       //                 table.scroll_to_col(table.delegate().cols_count(cx), cx);
                       //             })
                       //         })),
                       // ),
            )
            .child(
                h_flex().items_center().gap_2().child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .gap_1()
                        .child(Label::new("Number of Stocks:"))
                        .child(
                            h_flex()
                                .min_w_32()
                                .child(self.num_stocks_input.clone())
                                .into_any_element(),
                        )
                        .when(delegate.loading, |this| {
                            this.child(h_flex().gap_1().child(Indicator::new()).child("Loading..."))
                        })
                        .child(format!("Total Rows: {}", rows_count))
                        .child(format!("Visible Rows: {:?}", delegate.visible_rows))
                        .child(format!("Visible Cols: {:?}", delegate.visible_cols))
                        .when(delegate.eof, |this| this.child("All data loaded.")),
                ),
            )
            .child(self.table.clone())
    }
}
