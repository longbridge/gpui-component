//! One window, two languages, one ticking quote board.
//!
//! The left panel is ordinary Rust built from `gpui-component`. The right panel
//! is a `gpui-shell` script view whose JavaScript lives in
//! `crates/story/js/quotes/` and is read from disk when the story opens.
//! Neither half owns the data: a single `Entity<Market>` does, and the script
//! reaches it through a **native module** this story registers before the
//! script runtime starts.
//!
//! ```text
//!   Rust panel ──┐                                  ┌── main.js
//!   (rows drawn  │                                  │   (rows drawn
//!    with        ▼                                  ▼    with div/text)
//!    Label)   Entity<Market>  ◀── native("market") ──┐
//!                    │            quotes / ticks / watch
//!                    │ cx.notify()
//!                    ▼
//!              cx.observe(...) ──▶ re-renders both halves
//! ```
//!
//! # Why a quote board
//!
//! Because it is the load that decides whether a scripting layer is viable. A
//! feed arrives on its own, several times a second, while the window is already
//! repainting for reasons of its own — and the question the runtime has to
//! answer is which of those two frequencies the script pays for.
//!
//! The board ticks every 50 ms out of the box, and the counters under it show
//! both numbers as live rates. Switch the feed to **Repaint only** and watch
//! them come apart: frames keep climbing, script renders drop to zero, because
//! nothing the script reads has changed and the description it already
//! published is simply drawn again.
//!
//! Nothing but plain data crosses. `quotes()` returns an array of records;
//! `watch(symbol)` takes a string and answers a boolean, or fails with a
//! sentence the script sees as an exception. The script cannot hand Rust a
//! callback and Rust cannot hand the script a handle — which is also why the
//! script's colors arrive as hex strings from a second module, `theme`: the host
//! answers "what is `success` in the current theme?", and the script decides
//! what to paint with the answer. Switch the gallery's theme or radius and the
//! script half follows.

use std::{path::PathBuf, rc::Rc, time::Duration};

use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, Hsla, InteractiveElement as _,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement as _, Styled,
    Window, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Colorize as _, Disableable as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    label::Label,
    v_flex,
};
use gpui_shell::{
    RuntimeMetrics, ScriptView, ShellRuntime,
    native::{NativeError, NativeModules, NativeObject, NativeValue},
};

use crate::section;

/// One instrument on the board. The only data either half of the window reads.
#[derive(Clone)]
struct Quote {
    symbol: SharedString,
    name: SharedString,
    /// The session's opening price, so a change is a fact rather than a memory
    /// of the previous frame.
    open: f32,
    last: f32,
    volume: u64,
    watched: bool,
}

impl Quote {
    fn change(&self) -> f32 {
        self.last - self.open
    }

    fn change_percent(&self) -> f32 {
        if self.open == 0. {
            0.
        } else {
            self.change() / self.open * 100.
        }
    }

    /// Up, down, or flat. Both halves colour from this rather than each deciding
    /// what "unchanged" means, because the two panels sitting side by side is
    /// the whole point of the story.
    fn direction(&self) -> i32 {
        match self.change() {
            change if change > 0.0005 => 1,
            change if change < -0.0005 => -1,
            _ => 0,
        }
    }
}

/// The shared state, owned by GPUI and reachable from both languages.
///
/// It is an `Entity` rather than a field on the story so the native module can
/// hold it: a native function is a plain closure with no access to the story's
/// `&mut self`, and an entity handle is the one way to reach host state from
/// inside a script call and still notify observers afterwards.
pub struct Market {
    quotes: Vec<Quote>,
    /// How many feed ticks have landed. The script paints it, which is what
    /// makes "the script did not run" visible rather than merely asserted.
    ticks: u64,
    /// Deterministic, so the board moves the same way on every run and a test
    /// can assert on it. A real feed is not random either — it is just not ours.
    seed: u64,
}

/// The board. A watchlist-sized twenty rows, which is around three hundred
/// description nodes once the cells and the wrappers are counted — a real
/// description to rebuild, and still a panel a reader can take in at a glance.
///
/// There is no virtualization here yet, so a thousand-row board would be an
/// honest measurement of something this runtime does not claim to do well. A
/// watchlist is what it does claim.
const BOARD: [(&str, &str, f32); 20] = [
    ("700.HK", "Tencent", 372.40),
    ("9988.HK", "Alibaba", 78.15),
    ("3690.HK", "Meituan", 112.60),
    ("1810.HK", "Xiaomi", 17.86),
    ("0005.HK", "HSBC", 62.05),
    ("0388.HK", "HKEX", 268.80),
    ("1299.HK", "AIA", 54.35),
    ("2318.HK", "Ping An", 38.72),
    ("AAPL.US", "Apple", 214.29),
    ("NVDA.US", "NVIDIA", 118.11),
    ("MSFT.US", "Microsoft", 421.53),
    ("TSLA.US", "Tesla", 249.83),
    ("AMZN.US", "Amazon", 186.34),
    ("GOOGL.US", "Alphabet", 165.27),
    ("META.US", "Meta", 502.18),
    ("AVGO.US", "Broadcom", 168.44),
    ("AMD.US", "AMD", 152.61),
    ("NFLX.US", "Netflix", 678.90),
    ("COIN.US", "Coinbase", 214.75),
    ("ARM.US", "Arm", 138.02),
];

impl Market {
    fn open() -> Self {
        Self {
            quotes: BOARD
                .into_iter()
                .map(|(symbol, name, open)| Quote {
                    symbol: symbol.into(),
                    name: name.into(),
                    open,
                    last: open,
                    volume: 0,
                    watched: false,
                })
                .collect(),
            ticks: 0,
            seed: 0x2545_f491_4f6c_dd1d,
        }
    }

    /// One tick of the feed: every price moves a little, every volume grows.
    ///
    /// This is deliberately a *whole-board* update. A feed that moved one row
    /// would let a future subtree memoization hide the cost this story exists to
    /// show.
    fn tick(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
        for index in 0..self.quotes.len() {
            let drift = self.next_signed();
            let traded = self.next_unsigned() % 4_000;
            let quote = &mut self.quotes[index];
            // Proportional, so a 400-dollar name and a 17-dollar one move by
            // amounts that look alike on screen.
            quote.last = (quote.last * (1. + drift * 0.0012)).max(0.01);
            quote.volume = quote.volume.wrapping_add(traded);
        }
    }

    /// xorshift64. A dependency-free generator is worth more here than a good
    /// one: the board only has to move plausibly, and it has to move the same
    /// way twice.
    fn next_unsigned(&mut self) -> u64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed
    }

    /// Roughly -1.0 to 1.0.
    fn next_signed(&mut self) -> f32 {
        (self.next_unsigned() % 2_001) as f32 / 1_000. - 1.
    }

    fn watched_count(&self) -> usize {
        self.quotes.iter().filter(|quote| quote.watched).count()
    }

    /// Sets one row, for the Rust button, which already knows the value it
    /// wants.
    fn set_watched(&mut self, symbol: &str, watched: bool) {
        if let Some(quote) = self.quotes.iter_mut().find(|quote| quote.symbol == symbol) {
            quote.watched = watched;
        }
    }

    /// Flips one row, for the script, which asks by symbol only.
    ///
    /// An unknown symbol is the script's mistake, so it gets a sentence naming
    /// what does exist rather than a silent no-op.
    fn watch(&mut self, symbol: &str) -> Result<bool, NativeError> {
        match self
            .quotes
            .iter_mut()
            .find(|quote| quote.symbol.as_ref() == symbol)
        {
            Some(quote) => {
                quote.watched = !quote.watched;
                Ok(quote.watched)
            }
            None => {
                let known = self
                    .quotes
                    .iter()
                    .map(|quote| quote.symbol.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(NativeError::new(format!(
                    "no quote for `{symbol}`; the board holds {known}"
                )))
            }
        }
    }

    /// Returns how many rows actually moved, so the script can report a no-op as
    /// a no-op.
    fn watch_all(&mut self, watched: bool) -> usize {
        let mut changed = 0;
        for quote in &mut self.quotes {
            if quote.watched != watched {
                quote.watched = watched;
                changed += 1;
            }
        }
        changed
    }

    /// The board as it crosses the boundary: an array of records.
    ///
    /// The numbers are formatted here rather than in the script, so both halves
    /// round the same way. A price that reads 372.40 on the left and 372.4 on
    /// the right would make the comparison about formatting instead of about
    /// rendering.
    fn to_native(&self) -> NativeValue {
        NativeValue::Array(
            self.quotes
                .iter()
                .map(|quote| {
                    NativeValue::from(
                        NativeObject::new()
                            .field("symbol", quote.symbol.to_string())
                            .field("name", quote.name.to_string())
                            .field("last", format!("{:.2}", quote.last))
                            .field("change", format!("{:+.2}", quote.change()))
                            .field("percent", format!("{:+.2}%", quote.change_percent()))
                            .field("volume", thousands(quote.volume))
                            .field("direction", quote.direction())
                            .field("watched", quote.watched),
                    )
                })
                .collect(),
        )
    }
}

/// `1234567` as `1,234,567`. Both halves call it, for the same reason the prices
/// are formatted in Rust.
fn thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// What is driving the script view right now.
///
/// Two feeds rather than one, because they separate the two frequencies this
/// runtime keeps apart. A **quotes** feed moves prices the script reads, so the
/// script re-renders; a **repaint** feed only tells GPUI the view needs drawing
/// again, which is what a hover, a scroll, a cursor blink or an animation does.
///
/// Run the second one and watch the readout: frames climb, script renders stay
/// at zero. That is the architecture, live, rather than in a test.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Feed {
    Idle,
    /// Ticks the board, which invalidates the script view. The number is the
    /// interval in milliseconds — a feed is described by how often it arrives,
    /// not by a frequency nobody quotes.
    Quotes(u64),
    /// Notifies the script view without changing anything it reads.
    Repaint(u64),
}

impl Feed {
    fn interval(self) -> Option<Duration> {
        match self {
            Feed::Idle => None,
            Feed::Quotes(ms) | Feed::Repaint(ms) => Some(Duration::from_millis(ms.max(1))),
        }
    }

    fn detail(self) -> String {
        match self {
            Feed::Idle => "nothing is driving the board".to_owned(),
            Feed::Quotes(ms) => format!("every price moves every {ms} ms"),
            Feed::Repaint(ms) => format!("the view is redrawn every {ms} ms"),
        }
    }

    fn caption(self) -> String {
        match self {
            Feed::Idle => "Off".to_owned(),
            Feed::Quotes(ms) => format!("Quotes · {ms} ms"),
            Feed::Repaint(ms) => format!("Repaint only · {ms} ms"),
        }
    }
}

/// The feed the story opens with.
///
/// Running rather than idle on purpose: a board that has to be switched on
/// before it does anything is a board most readers will look at once, in its
/// resting state, and conclude nothing from.
const OPENING_FEED: Feed = Feed::Quotes(50);

/// The choices offered, in the order they answer the question.
const FEEDS: [(&str, Feed); 4] = [
    ("feed-off", Feed::Idle),
    ("feed-quotes-50", Feed::Quotes(50)),
    ("feed-quotes-16", Feed::Quotes(16)),
    ("feed-repaint-16", Feed::Repaint(16)),
];

/// How often the readout re-reads the counters. One second, because the numbers
/// it shows are rates and a rate over a shorter window is mostly noise.
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// The entry file the script application directory must contain.
const ENTRY: &str = "main.js";

/// Where the script lives.
///
/// Resolved against the crate rather than the process working directory, so
/// `cargo run` finds it from anywhere — and so editing the file is enough to
/// change the panel, with no rebuild.
fn script_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("js/quotes")
}

/// Grants the script the two modules it may reach, and nothing else.
///
/// This is the whole extension surface: a script cannot load native code, so
/// what the host registers here is exactly what it can call (design doc §17.6).
/// Registering an empty set — the default — would leave `native("market")`
/// failing with a message saying this host granted none.
fn install_native_modules(market: &Entity<Market>) {
    let mut modules = NativeModules::new();

    modules.register("market", |module| {
        let read = market.clone();
        module.function("quotes", move |_| with_app(|cx| read.read(cx).to_native()));

        // Read separately from `quotes()` so the script can paint how many ticks
        // it has actually seen. When the feed is only asking for repaints this
        // number stops moving on screen, which is the counters' claim made
        // visible in the panel itself.
        let ticks = market.clone();
        module.function("ticks", move |_| {
            with_app(|cx| NativeValue::from(ticks.read(cx).ticks as f64))
        });

        let flip = market.clone();
        module.function("watch", move |arguments| {
            let symbol = arguments.string(0)?;
            with_app(|cx| {
                flip.update(cx, |market, cx| {
                    let watched = market.watch(&symbol)?;
                    // The notification is what keeps the two halves in step:
                    // the story observes this entity and re-renders itself and
                    // the script view together. It is delivered after this call
                    // unwinds, so it cannot re-enter the script engine.
                    cx.notify();
                    Ok(NativeValue::from(watched))
                })
            })?
        });

        let bulk = market.clone();
        module.function("watch_all", move |arguments| {
            let watched = arguments.boolean(0)?;
            with_app(|cx| {
                bulk.update(cx, |market, cx| {
                    let changed = market.watch_all(watched);
                    if changed > 0 {
                        cx.notify();
                    }
                    NativeValue::from(changed as f64)
                })
            })
        });
    });

    modules.register("theme", |module| {
        module.function("palette", |_| with_app(palette));
    });

    gpui_shell::set_native_modules(modules);
}

/// Reaches the ambient `App` from inside a native call.
///
/// A native function receives arguments and nothing else; the host context it
/// runs in comes from the shell's call scope, which is live for exactly as long
/// as the script call that is on the stack. Outside one there is no honest
/// answer, so this says so rather than reaching for a stale pointer.
fn with_app<R>(read: impl FnOnce(&mut App) -> R) -> Result<R, NativeError> {
    gpui_shell::scope::with_current_app(read).ok_or_else(|| {
        NativeError::new("the board is only reachable while a script call is in progress")
    })
}

/// The host's theme, as the script can consume it.
///
/// Colors leave as `#rrggbb` literals and lengths as numbers, because those are
/// the two things the script's style methods accept. The script never learns
/// which theme is installed — it asks for a role and paints what comes back.
fn palette(cx: &mut App) -> NativeValue {
    let theme = cx.theme();

    NativeValue::from(
        NativeObject::new()
            .field("background", theme.background.to_hex())
            .field("foreground", theme.foreground.to_hex())
            .field("muted", theme.muted.to_hex())
            .field("muted_foreground", theme.muted_foreground.to_hex())
            .field("border", theme.border.to_hex())
            .field("primary", theme.primary.to_hex())
            .field("primary_hover", theme.primary_hover.to_hex())
            .field("primary_foreground", theme.primary_foreground.to_hex())
            .field("secondary", theme.secondary.to_hex())
            .field("accent", theme.accent.to_hex())
            .field("success", theme.success.to_hex())
            .field("danger", theme.danger.to_hex())
            .field("radius", f32::from(theme.radius))
            .field("font_size", f32::from(theme.font_size)),
    )
}

pub struct ShellStory {
    focus_handle: FocusHandle,
    market: Entity<Market>,
    /// Held for as long as the script view is mounted: the view renders through
    /// it, and dropping it would tear the JavaScript context down underneath.
    runtime: Option<Rc<ShellRuntime>>,
    script: Option<Entity<ScriptView>>,
    /// The last load failure, kept visible instead of thrown away — a story
    /// that silently shows the previous script after a syntax error is worse
    /// than one that says what broke.
    script_error: Option<SharedString>,
    feed: Feed,
    /// Bumped whenever the feed changes, so a loop started for an older feed
    /// stops on its next tick instead of racing the new one.
    feed_generation: u64,
    /// The counters as of the last sample, and what the second before it added.
    /// A rate is the difference of two readings; the runtime does not know what
    /// a second is, and should not have to.
    sampled: RuntimeMetrics,
    rate: RuntimeMetrics,
}

impl super::Story for ShellStory {
    fn title() -> &'static str {
        "Shell"
    }

    fn description() -> &'static str {
        "Run a ticking JavaScript quote board beside a Rust one, sharing state \
         through a native module."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ShellStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Only the style reflection table. `gpui_shell::init` would also
        // install the shell's own palette over the Base tokens this gallery
        // projects from its `gpui-component` theme, and the script has no need
        // of them: it reads colors from the host through `native("theme")`.
        gpui_shell::style::init();

        let market = cx.new(|_| Market::open());
        install_native_modules(&market);

        // The single place a change becomes two re-renders. Whoever moved the
        // board — the feed, a Rust button or a script button — both halves are
        // looking at the same entity, so both are told.
        cx.observe(&market, |this, _, cx| {
            if let Some(script) = &this.script {
                // `refresh`, not `notify`: the board is state the script reads
                // over a native call, so its description is now stale. A bare
                // notify would redraw the panel from the snapshot it already
                // published — correct for a repaint, wrong here.
                script.update(cx, |view, cx| view.refresh(cx));
            }
            cx.notify();
        })
        .detach();

        let mut story = Self {
            focus_handle: cx.focus_handle(),
            market,
            runtime: None,
            script: None,
            script_error: None,
            feed: Feed::Idle,
            feed_generation: 0,
            sampled: RuntimeMetrics::default(),
            rate: RuntimeMetrics::default(),
        };

        match ShellRuntime::new() {
            Ok(runtime) => {
                runtime.set_global(cx);
                story.runtime = Some(runtime);
                story.reload(window, cx);
            }
            Err(error) => story.script_error = Some(error.to_string().into()),
        }

        story.sample_metrics(cx);
        story.set_feed(OPENING_FEED, cx);
        story
    }

    /// Re-reads the runtime counters once a second, forever.
    ///
    /// Separate from the feed on purpose: the readout has to keep working when
    /// the feed is off, because "zero script renders while the window is busy"
    /// is one of the readings worth seeing.
    fn sample_metrics(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(SAMPLE_INTERVAL).await;
                let alive = this.update(cx, |this, cx| {
                    let Some(runtime) = &this.runtime else {
                        return;
                    };
                    let reading = runtime.metrics().read();
                    this.rate = reading.since(&this.sampled);
                    this.sampled = reading;
                    cx.notify();
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// Switches the feed, and starts the loop that drives it.
    ///
    /// The counters are reset here rather than accumulated, because the readout
    /// answers "what is this feed costing", not "what has this window done since
    /// it opened".
    fn set_feed(&mut self, feed: Feed, cx: &mut Context<Self>) {
        self.feed_generation += 1;
        self.feed = feed;

        if let Some(runtime) = &self.runtime {
            runtime.metrics().reset();
        }
        self.sampled = RuntimeMetrics::default();
        self.rate = RuntimeMetrics::default();
        cx.notify();

        let Some(interval) = feed.interval() else {
            return;
        };
        let generation = self.feed_generation;

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(interval).await;
                let running = this.update(cx, |this, cx| {
                    if this.feed_generation != generation {
                        return false;
                    }
                    this.tick(cx);
                    true
                });
                if !matches!(running, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    /// One tick of whichever feed is running.
    fn tick(&mut self, cx: &mut Context<Self>) {
        match self.feed {
            Feed::Idle => {}
            // Moving the board notifies its observers, which invalidates the
            // script view: the script has to run, because what it reads moved.
            Feed::Quotes(_) => self.market.update(cx, |market, cx| {
                market.tick();
                cx.notify();
            }),
            // Nothing the script reads has changed, so this is a repaint and
            // nothing more. The view materializes its existing snapshot and the
            // VM is never entered.
            Feed::Repaint(_) => {
                if let Some(script) = &self.script {
                    // A bare notify on purpose. This is exactly the case
                    // `refresh` exists to be distinguished from.
                    script.update(cx, |_, cx| cx.notify());
                }
            }
        }
    }

    /// Re-reads the script from disk and swaps it into the live view.
    ///
    /// The entity survives, so the panel keeps its place in the window and the
    /// board keeps its state: only what the script produced is replaced.
    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(runtime) = self.runtime.clone() else {
            return;
        };

        let loaded = runtime
            .load_app(&script_directory(), ENTRY)
            .and_then(|view_type| runtime.instantiate(&view_type, window, cx));

        match loaded {
            Ok(object) => {
                match self.script.clone() {
                    Some(view) => view.update(cx, |view, cx| {
                        view.replace_object(object);
                        cx.notify();
                    }),
                    None => self.script = Some(cx.new(|_| ScriptView::new(runtime, object))),
                }
                self.script_error = None;
            }
            Err(error) => self.script_error = Some(error.to_string().into()),
        }

        cx.notify();
    }
}

/// The column widths both halves lay out to.
///
/// Shared as constants because the two panels sit side by side and a reader is
/// comparing them: a column that is 72 wide on the left and 70 on the right
/// would make the comparison about alignment instead of about rendering.
const PRICE_COLUMN: f32 = 68.;
const PERCENT_COLUMN: f32 = 66.;
const VOLUME_COLUMN: f32 = 82.;

impl ShellStory {
    fn rust_panel(&self, quotes: &[Quote], cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_1()
            .child(self.rust_header(cx))
            .children(quotes.iter().map(|quote| self.rust_row(quote, cx)))
    }

    fn rust_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let caption = |value: &'static str, width: f32, right: bool| {
            div()
                .w(px(width))
                .flex_none()
                .when(right, |this| this.text_right())
                .child(
                    Label::new(value)
                        .text_xs()
                        .text_color(cx.theme().muted_foreground),
                )
        };

        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .px_2()
            .pb_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(caption("Symbol", 78., false))
            .child(div().flex_1())
            .child(caption("Last", PRICE_COLUMN, true))
            .child(caption("Change", PERCENT_COLUMN, true))
            .child(caption("Volume", VOLUME_COLUMN, true))
    }

    fn rust_row(&self, quote: &Quote, cx: &Context<Self>) -> impl IntoElement {
        let symbol = quote.symbol.clone();
        let watched = quote.watched;
        let moved = direction_color(quote.direction(), cx);

        h_flex()
            .id(SharedString::from(format!("quote-{symbol}")))
            .w_full()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .rounded(cx.theme().radius)
            .hover(|this| this.bg(cx.theme().muted))
            .on_click(cx.listener(move |this, _, _, cx| {
                let symbol = symbol.clone();
                this.market.update(cx, |market, cx| {
                    market.set_watched(&symbol, !watched);
                    cx.notify();
                });
            }))
            .child(
                div()
                    .w(px(78.))
                    .flex_none()
                    .child(Label::new(quote.symbol.clone()).text_xs().font_medium()),
            )
            .child(
                div().flex_1().truncate().child(
                    Label::new(quote.name.clone())
                        .text_xs()
                        .text_color(cx.theme().muted_foreground),
                ),
            )
            .child(
                div().w(px(PRICE_COLUMN)).flex_none().text_right().child(
                    Label::new(format!("{:.2}", quote.last))
                        .text_xs()
                        .text_color(moved),
                ),
            )
            .child(
                div().w(px(PERCENT_COLUMN)).flex_none().text_right().child(
                    Label::new(format!("{:+.2}%", quote.change_percent()))
                        .text_xs()
                        .text_color(moved),
                ),
            )
            .child(
                div().w(px(VOLUME_COLUMN)).flex_none().text_right().child(
                    Label::new(thousands(quote.volume))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground),
                ),
            )
            .child(
                div()
                    .w(px(6.))
                    .h(px(6.))
                    .flex_none()
                    .rounded_full()
                    .when(watched, |this| this.bg(cx.theme().primary)),
            )
    }

    /// The two counters, side by side, with what they mean underneath.
    ///
    /// Rates rather than totals: a total answers "how much work has this window
    /// ever done", and the question here is "what is this costing right now".
    fn readout(&self, cx: &Context<Self>) -> impl IntoElement {
        let script = self.rate.script_renders();
        let frames = self.rate.materializations();

        v_flex()
            .w_full()
            .gap_2()
            .child(
                h_flex()
                    .w_full()
                    .gap_6()
                    .child(reading(
                        "Script renders",
                        format!("{script}/s"),
                        format!("{:.2} ms each", millis(self.rate.mean_script_render())),
                        cx,
                    ))
                    .child(reading(
                        "Frames drawn",
                        format!("{frames}/s"),
                        format!("{:.2} ms each", millis(self.rate.mean_materialize())),
                        cx,
                    ))
                    .child(reading("Feed", self.feed.caption(), self.feed.detail(), cx)),
            )
            .child(
                Label::new(match self.feed {
                    Feed::Idle => {
                        "Counters are cleared when the feed changes. With no feed running, \
                         hovering the script panel still draws frames — and still runs no script."
                    }
                    Feed::Quotes(_) => {
                        "Prices are state the script reads, so every tick invalidates its \
                         snapshot: script renders track the feed, whatever the frame rate is."
                    }
                    Feed::Repaint(_) => {
                        "Nothing the script reads has changed, so every tick is a repaint of the \
                         snapshot it already published: frames climb, script renders stay at zero, \
                         and the tick count in the panel stops moving."
                    }
                })
                .text_xs()
                .text_color(cx.theme().muted_foreground),
            )
    }

    fn script_panel(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_2()
            .when_some(self.script_error.clone(), |this, message| {
                this.child(
                    v_flex()
                        .w_full()
                        .gap_1()
                        .p_2()
                        .rounded(cx.theme().radius)
                        .border_1()
                        .border_color(cx.theme().danger)
                        .child(Label::new("The script did not load").text_xs())
                        .child(
                            Label::new(message)
                                .text_xs()
                                .text_color(cx.theme().muted_foreground),
                        ),
                )
            })
            .children(self.script.clone())
    }
}

/// Up is `success`, down is `danger`, flat is ordinary text. Both halves ask
/// this question of the same theme, which is why the two panels agree.
fn direction_color(direction: i32, cx: &Context<ShellStory>) -> Hsla {
    match direction {
        1 => cx.theme().success,
        -1 => cx.theme().danger,
        _ => cx.theme().foreground,
    }
}

/// The native modules this story installed capture its `Entity<Market>`, and the
/// registry they live in is process-wide. Leaving them there would keep the
/// entity alive after the story is gone — which GPUI reports as a leaked handle,
/// and which is exactly the shape of leak a plugin host would hit on unload.
impl Drop for ShellStory {
    fn drop(&mut self) {
        gpui_shell::clear_native_modules();
    }
}

impl Focusable for ShellStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ShellStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let quotes = self.market.read(cx).quotes.clone();
        let watched = self.market.read(cx).watched_count();
        let total = quotes.len();

        v_flex()
            .size_full()
            .gap_3()
            .child(
                h_flex()
                    .w_full()
                    .items_start()
                    .gap_4()
                    .child(
                        div().flex_1().child(
                            section("Rust · gpui-component")
                                .description(
                                    "Rows built from crates/ui, reading the Entity<Market> both \
                                     halves share.",
                                )
                                .sub_title(
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            Button::new("watch-all")
                                                .xsmall()
                                                .primary()
                                                .label("Watch all")
                                                .disabled(watched == total)
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.market.update(cx, |market, cx| {
                                                        market.watch_all(true);
                                                        cx.notify();
                                                    });
                                                })),
                                        )
                                        .child(
                                            Button::new("watch-none")
                                                .xsmall()
                                                .outline()
                                                .label("Clear")
                                                .disabled(watched == 0)
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.market.update(cx, |market, cx| {
                                                        market.watch_all(false);
                                                        cx.notify();
                                                    });
                                                })),
                                        ),
                                )
                                .v_flex()
                                .child(self.rust_panel(&quotes, cx)),
                        ),
                    )
                    .child(
                        div().flex_1().child(
                            section("JavaScript · gpui-shell")
                                .description(
                                    "The same board, drawn by crates/story/js/quotes/main.js, \
                                     read from disk at run time.",
                                )
                                .sub_title(
                                    Button::new("reload-script")
                                        .xsmall()
                                        .outline()
                                        .label("Reload script")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.reload(window, cx);
                                        })),
                                )
                                .v_flex()
                                .child(self.script_panel(cx)),
                        ),
                    ),
            )
            .child(
                section("Render frequency")
                    .description(
                        "A script render and a GPUI frame are not the same event. Change the feed \
                         and watch the two counters come apart.",
                    )
                    .sub_title(h_flex().gap_2().children(FEEDS.map(|(id, feed)| {
                        Button::new(id)
                            .xsmall()
                            .label(feed.caption())
                            .when(self.feed == feed, |this| this.primary())
                            .when(self.feed != feed, |this| this.outline())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_feed(feed, cx);
                            }))
                    })))
                    .v_flex()
                    .child(self.readout(cx)),
            )
            .child(
                section("Where the boundary is")
                    .description(
                        "The script holds no state and no host object. It calls two native \
                         modules this story registered before the runtime started, and every \
                         argument and result is plain data.",
                    )
                    .v_flex()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_1()
                            .child(boundary_line(
                                "native(\"market\")",
                                "quotes() · ticks() · watch(symbol) · watch_all(on)",
                                cx,
                            ))
                            .child(boundary_line(
                                "native(\"theme\")",
                                "palette() — the gallery's own colors and radius, as data",
                                cx,
                            ))
                            .child(boundary_line(
                                "Editing main.js",
                                "needs no rebuild: press Reload script above",
                                cx,
                            )),
                    ),
            )
    }
}

/// One counter: what it is, the number, and what the number means.
///
/// The number is the focal point and gets the size; the label above it and the
/// detail below it are both quiet, because a reader glancing at this is
/// comparing two figures, not reading three lines.
fn reading(
    caption: &'static str,
    value: String,
    detail: String,
    cx: &Context<ShellStory>,
) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(
            Label::new(caption)
                .text_xs()
                .text_color(cx.theme().muted_foreground),
        )
        .child(Label::new(value).font_semibold())
        .child(
            Label::new(detail)
                .text_xs()
                .text_color(cx.theme().muted_foreground),
        )
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// One line of the boundary summary: the call on the left, what it does on the
/// right. Two columns rather than a sentence, because the reader is scanning
/// for a name, not reading prose.
fn boundary_line(
    call: &'static str,
    detail: &'static str,
    cx: &Context<ShellStory>,
) -> impl IntoElement {
    h_flex()
        .w_full()
        .items_start()
        .gap_3()
        .child(Label::new(call).text_xs().font_medium().w_48())
        .child(
            Label::new(detail)
                .text_xs()
                .text_color(cx.theme().muted_foreground),
        )
}

#[cfg(test)]
mod tests {
    use std::ops::Deref as _;

    use gpui::{TestAppContext, VisualTestContext};

    use super::*;

    /// The claim the counters under the panels make, checked without a person
    /// having to watch two numbers.
    ///
    /// It is worth an end-to-end test rather than a unit one because it spans
    /// every part that has to agree: the entity, the native module, the script's
    /// `ticks()` call, and the difference between `refresh` and `notify`.
    #[gpui::test]
    fn a_quote_tick_re_runs_the_script_and_a_repaint_does_not(cx: &mut TestAppContext) {
        // The story reads the gallery's theme through `native("theme")`, so
        // the theme has to exist before the script's first render.
        cx.update(gpui_component::init);
        let window = cx.add_window(|window, cx| ShellStory::new(window, cx));
        let story = cx.update(|cx| window.entity(cx)).expect("the story");
        let mut context = VisualTestContext::from_window(*window.deref(), cx);

        let (runtime, script) = story.read_with(&mut context, |story, _| {
            assert!(
                story.script_error.is_none(),
                "the story's script did not load: {:?}",
                story.script_error
            );
            (story.runtime.clone(), story.script.clone())
        });
        let runtime = runtime.expect("a runtime");
        let script = script.expect("a script view");

        draw(&mut context, &script);
        assert!(
            description(&mut context, &script).contains("tick 0"),
            "the script should be painting the tick count"
        );

        let baseline = runtime.metrics().read().script_renders();

        // A quote tick moves prices the script reads, so the description is
        // stale and has to be rebuilt.
        tick(&mut context, &story, Feed::Quotes(50));
        draw(&mut context, &script);
        assert_eq!(
            runtime.metrics().read().script_renders(),
            baseline + 1,
            "a quote tick must re-run the script"
        );
        assert!(description(&mut context, &script).contains("tick 1"));

        // A repaint tick changes nothing the script can see.
        for _ in 0..8 {
            tick(&mut context, &story, Feed::Repaint(16));
            draw(&mut context, &script);
        }
        assert_eq!(
            runtime.metrics().read().script_renders(),
            baseline + 1,
            "eight repaints must not enter the VM"
        );
        assert!(
            description(&mut context, &script).contains("tick 1"),
            "and the description must be the one already published"
        );
    }

    /// The board moves the same way twice, so the panel a reader sees on one run
    /// is the panel they saw on the last one.
    #[gpui::test]
    fn the_feed_is_deterministic(_: &mut TestAppContext) {
        let mut first = Market::open();
        let mut second = Market::open();
        for _ in 0..64 {
            first.tick();
            second.tick();
        }

        let prices = |market: &Market| {
            market
                .quotes
                .iter()
                .map(|quote| format!("{:.4}", quote.last))
                .collect::<Vec<_>>()
        };
        assert_eq!(prices(&first), prices(&second));
        assert!(
            prices(&first) != prices(&Market::open()),
            "sixty-four ticks should have moved something"
        );
    }

    fn tick(context: &mut VisualTestContext, story: &Entity<ShellStory>, feed: Feed) {
        story.update(context, |story, cx| {
            story.feed = feed;
            story.tick(cx);
        });
    }

    fn draw(context: &mut VisualTestContext, script: &Entity<ScriptView>) {
        let script = script.clone();
        context.draw(
            gpui::Point::default(),
            gpui::size(gpui::px(520.), gpui::px(420.)),
            move |_, _| script.into_any_element(),
        );
    }

    /// The published description, read without entering the VM.
    fn description(context: &mut VisualTestContext, script: &Entity<ScriptView>) -> String {
        context.update(|_, cx| {
            script
                .read(cx)
                .snapshot()
                .map(gpui_shell::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    }
}
