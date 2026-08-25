//! One window, two languages.
//!
//! The left panel is ordinary Rust built from `gpui-component`. The right panel
//! is a `gpui-shell` script view whose JavaScript lives in
//! `crates/story/js/checklist/` and is read from disk when the story opens.
//! Neither half owns the data: a single `Entity<Checklist>` does, and the script
//! reaches it through a **native module** this story registers before the
//! script runtime starts.
//!
//! ```text
//!   Rust panel ──┐                                  ┌── main.js
//!   (Checkbox,   │                                  │   (div, Button,
//!    Button)     ▼                                  ▼    text)
//!             Entity<Checklist>  ◀── native("checklist") ──┐
//!                    │                steps / toggle / set_all
//!                    │ cx.notify()
//!                    ▼
//!              cx.observe(...) ──▶ re-renders both halves
//! ```
//!
//! Nothing but plain data crosses. `steps()` returns an array of records;
//! `toggle(id)` takes a number and answers a boolean, or fails with a sentence
//! the script sees as an exception. The script cannot hand Rust a callback and
//! Rust cannot hand the script a handle — which is also why the script's colors
//! arrive as hex strings from a second module, `theme`: the host answers "what
//! is `primary` in the current theme?", and the script decides what to paint
//! with the answer. Switch the gallery's theme or radius and the script half
//! follows.

use std::{path::PathBuf, rc::Rc, time::Duration};

use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, SharedString, Styled, Window, div, prelude::FluentBuilder as _,
};
use gpui_component::{
    ActiveTheme as _, Colorize as _, Disableable as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    label::Label,
    v_flex,
};
use gpui_shell::{
    RuntimeMetrics, ScriptView, ShellRuntime,
    native::{NativeError, NativeModules, NativeObject, NativeValue},
};

use crate::section;

/// One line of the checklist. The only data either half of the window reads.
#[derive(Clone)]
struct Step {
    id: u32,
    title: SharedString,
    owner: SharedString,
    done: bool,
}

/// The shared state, owned by GPUI and reachable from both languages.
///
/// It is an `Entity` rather than a field on the story so the native module can
/// hold it: a native function is a plain closure with no access to the story's
/// `&mut self`, and an entity handle is the one way to reach host state from
/// inside a script call and still notify observers afterwards.
pub struct Checklist {
    steps: Vec<Step>,
    /// A number a live feed moves, so there is something changing that nobody
    /// clicked. It stands in for the case that actually stresses a scripting
    /// runtime: data arriving on its own, several times a second, while the
    /// window is repainting for its own reasons.
    reading: u32,
}

impl Checklist {
    fn seed() -> Self {
        let steps = [
            (1, "Freeze the changelog", "Jason"),
            (2, "Tag the release commit", "Ana"),
            (3, "Publish the crate", "Wei"),
            (4, "Update the documentation site", "Ana"),
            (5, "Announce in the release channel", "Jason"),
        ];

        Self {
            steps: steps
                .into_iter()
                .map(|(id, title, owner)| Step {
                    id,
                    title: title.into(),
                    owner: owner.into(),
                    done: id == 1,
                })
                .collect(),
            reading: 0,
        }
    }

    /// Moves the live reading. One tick of a feed.
    fn advance(&mut self) {
        self.reading = self.reading.wrapping_add(1);
    }

    fn done_count(&self) -> usize {
        self.steps.iter().filter(|step| step.done).count()
    }

    /// Sets one step, for the Rust checkbox, which already knows the value it
    /// wants.
    fn set(&mut self, id: u32, done: bool) {
        if let Some(step) = self.steps.iter_mut().find(|step| step.id == id) {
            step.done = done;
        }
    }

    /// Flips one step, for the script, which asks by identifier only.
    ///
    /// An unknown identifier is the script's mistake, so it gets a sentence
    /// naming what does exist rather than a silent no-op.
    fn toggle(&mut self, id: u32) -> Result<bool, NativeError> {
        let known = self
            .steps
            .iter()
            .map(|step| step.id.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        match self.steps.iter_mut().find(|step| step.id == id) {
            Some(step) => {
                step.done = !step.done;
                Ok(step.done)
            }
            None => Err(NativeError::new(format!(
                "no step with id {id}; the checklist holds {known}"
            ))),
        }
    }

    /// Returns how many steps actually moved, so the script can report a
    /// no-op as a no-op.
    fn set_all(&mut self, done: bool) -> usize {
        let mut changed = 0;
        for step in &mut self.steps {
            if step.done != done {
                step.done = done;
                changed += 1;
            }
        }
        changed
    }

    /// The checklist as it crosses the boundary: an array of records.
    fn to_native(&self) -> NativeValue {
        NativeValue::Array(
            self.steps
                .iter()
                .map(|step| {
                    NativeValue::from(
                        NativeObject::new()
                            .field("id", step.id)
                            .field("title", step.title.to_string())
                            .field("owner", step.owner.to_string())
                            .field("done", step.done),
                    )
                })
                .collect(),
        )
    }
}

/// What is driving the script view right now.
///
/// Two feeds rather than one, because they separate the two frequencies this
/// runtime keeps apart. A **data** feed moves state the script reads, so the
/// script re-renders; a **repaint** feed only tells GPUI the view needs drawing
/// again, which is what a hover, a scroll, a cursor blink or an animation does.
///
/// Run the second one and watch the readout: frames climb, script renders stay
/// at zero. That is the architecture, live, rather than in a test.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Feed {
    Idle,
    /// Changes the checklist's reading, which invalidates the script view.
    Data(u32),
    /// Notifies the script view without changing anything it reads.
    Repaint(u32),
}

impl Feed {
    fn hz(self) -> Option<u32> {
        match self {
            Feed::Idle => None,
            Feed::Data(hz) | Feed::Repaint(hz) => Some(hz),
        }
    }

    fn interval(self) -> Option<Duration> {
        self.hz()
            .filter(|hz| *hz > 0)
            .map(|hz| Duration::from_secs_f64(1.0 / f64::from(hz)))
    }

    fn label(self) -> &'static str {
        match self {
            Feed::Idle => "Idle",
            Feed::Data(_) => "Data",
            Feed::Repaint(_) => "Repaint",
        }
    }
}

/// The choices offered, in the order they answer the question.
const FEEDS: [(&str, &str, Feed); 4] = [
    ("feed-off", "Off", Feed::Idle),
    ("feed-data-10", "Data · 10 Hz", Feed::Data(10)),
    ("feed-data-60", "Data · 60 Hz", Feed::Data(60)),
    ("feed-repaint-60", "Repaint · 60 Hz", Feed::Repaint(60)),
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("js/checklist")
}

/// Grants the script the two modules it may reach, and nothing else.
///
/// This is the whole extension surface: a script cannot load native code, so
/// what the host registers here is exactly what it can call (design doc §17.6).
/// Registering an empty set — the default — would leave `native("checklist")`
/// failing with a message saying this host granted none.
fn install_native_modules(checklist: &Entity<Checklist>) {
    let mut modules = NativeModules::new();

    modules.register("checklist", |module| {
        let read = checklist.clone();
        module.function("steps", move |_| with_app(|cx| read.read(cx).to_native()));

        // Read separately from `steps()` so the script can paint the feed
        // without the checklist itself having to change: the two move for
        // different reasons and at very different rates.
        let reading = checklist.clone();
        module.function("reading", move |_| {
            with_app(|cx| NativeValue::from(reading.read(cx).reading))
        });

        let flip = checklist.clone();
        module.function("toggle", move |arguments| {
            let id = arguments.integer(0)? as u32;
            with_app(|cx| {
                flip.update(cx, |checklist, cx| {
                    let done = checklist.toggle(id)?;
                    // The notification is what keeps the two halves in step:
                    // the story observes this entity and re-renders itself and
                    // the script view together. It is delivered after this call
                    // unwinds, so it cannot re-enter the script engine.
                    cx.notify();
                    Ok(NativeValue::from(done))
                })
            })?
        });

        let bulk = checklist.clone();
        module.function("set_all", move |arguments| {
            let done = arguments.boolean(0)?;
            with_app(|cx| {
                bulk.update(cx, |checklist, cx| {
                    let changed = checklist.set_all(done);
                    if changed > 0 {
                        cx.notify();
                    }
                    NativeValue::from(changed)
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
        NativeError::new("the checklist is only reachable while a script call is in progress")
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
    checklist: Entity<Checklist>,
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
        "Run a JavaScript view beside a Rust panel, sharing state through a native module."
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

        let checklist = cx.new(|_| Checklist::seed());
        install_native_modules(&checklist);

        // The single place a change becomes two re-renders. Whoever moved the
        // checklist — a Rust checkbox or a script button — both halves are
        // looking at the same entity, so both are told.
        cx.observe(&checklist, |this, _, cx| {
            if let Some(script) = &this.script {
                // `refresh`, not `notify`: the checklist is state the script
                // reads over a native call, so its description is now stale.
                // A bare notify would redraw the panel from the snapshot it
                // already published — correct for a repaint, wrong here.
                script.update(cx, |view, cx| view.refresh(cx));
            }
            cx.notify();
        })
        .detach();

        let mut story = Self {
            focus_handle: cx.focus_handle(),
            checklist,
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
            // Changing the entity notifies its observers, which invalidates the
            // script view: the script has to run, because what it reads moved.
            Feed::Data(_) => self.checklist.update(cx, |checklist, cx| {
                checklist.advance();
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
    /// checklist keeps its state: only what the script produced is replaced.
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

    fn rust_panel(&self, steps: &[Step], cx: &Context<Self>) -> impl IntoElement {
        v_flex().w_full().gap_2().children(steps.iter().map(|step| {
            let id = step.id;

            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    Checkbox::new(SharedString::from(format!("step-{id}")))
                        .label(step.title.clone())
                        .checked(step.done)
                        .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                            this.checklist.update(cx, |checklist, cx| {
                                checklist.set(id, *checked);
                                cx.notify();
                            });
                        })),
                )
                .child(
                    Label::new(step.owner.clone())
                        .text_xs()
                        .text_color(cx.theme().muted_foreground),
                )
        }))
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
                    .child(reading(
                        "Feed",
                        match self.feed.hz() {
                            Some(hz) => format!("{} · {hz} Hz", self.feed.label()),
                            None => self.feed.label().to_owned(),
                        },
                        match self.feed {
                            Feed::Idle => "nothing is driving the panel".to_owned(),
                            Feed::Data(hz) => format!("the reading moves {hz} times a second"),
                            Feed::Repaint(hz) => {
                                format!("the view is redrawn {hz} times a second")
                            }
                        },
                        cx,
                    )),
            )
            .child(
                Label::new(match self.feed {
                    Feed::Idle => {
                        "Counters are cleared when the feed changes. With no feed running, \
                         hovering the script panel still draws frames — and still runs no script."
                    }
                    Feed::Data(_) => {
                        "The reading is state the script reads, so every tick invalidates its \
                         snapshot: script renders track the feed rate."
                    }
                    Feed::Repaint(_) => {
                        "Nothing the script reads has changed, so every tick is a repaint of the \
                         snapshot it already published: frames climb, script renders stay at zero."
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

/// The native modules this story installed capture its `Entity<Checklist>`, and
/// the registry they live in is process-wide. Leaving them there would keep the
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
        let steps = self.checklist.read(cx).steps.clone();
        let done = self.checklist.read(cx).done_count();
        let total = steps.len();

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
                                    "Checkbox, Button and Label from crates/ui, driving the \
                                     Entity<Checklist> both halves read.",
                                )
                                .sub_title(
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            Button::new("mark-all")
                                                .xsmall()
                                                .primary()
                                                .label("Mark all done")
                                                .disabled(done == total)
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.checklist.update(cx, |checklist, cx| {
                                                        checklist.set_all(true);
                                                        cx.notify();
                                                    });
                                                })),
                                        )
                                        .child(
                                            Button::new("clear-all")
                                                .xsmall()
                                                .outline()
                                                .label("Clear all")
                                                .disabled(done == 0)
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.checklist.update(cx, |checklist, cx| {
                                                        checklist.set_all(false);
                                                        cx.notify();
                                                    });
                                                })),
                                        ),
                                )
                                .v_flex()
                                .child(self.rust_panel(&steps, cx)),
                        ),
                    )
                    .child(
                        div().flex_1().child(
                            section("JavaScript · gpui-shell")
                                .description(
                                    "A ScriptView rendering crates/story/js/checklist/main.js, \
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
                        "A script render and a GPUI frame are not the same event. Drive the \
                         panel and watch the two counters come apart.",
                    )
                    .sub_title(h_flex().gap_2().children(FEEDS.map(|(id, label, feed)| {
                        Button::new(id)
                            .xsmall()
                            .label(label)
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
                                "native(\"checklist\")",
                                "steps() · toggle(id) · set_all(done)",
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

    /// The claim the readout under the panel makes, checked without a person
    /// having to watch two counters.
    ///
    /// It is worth an end-to-end test rather than a unit one because it spans
    /// every part that has to agree: the entity, the native module, the script's
    /// `reading()` call, and the difference between `refresh` and `notify`.
    #[gpui::test]
    fn a_data_feed_re_runs_the_script_and_a_repaint_feed_does_not(cx: &mut TestAppContext) {
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
            description(&mut context, &script).contains("reading 0"),
            "the script should be painting the live reading"
        );

        let baseline = runtime.metrics().read().script_renders();

        // A data tick moves state the script reads, so the description is stale.
        tick(&mut context, &story, Feed::Data(10));
        draw(&mut context, &script);
        assert_eq!(
            runtime.metrics().read().script_renders(),
            baseline + 1,
            "a data tick must re-run the script"
        );
        assert!(description(&mut context, &script).contains("reading 1"));

        // A repaint tick changes nothing the script can see.
        for _ in 0..8 {
            tick(&mut context, &story, Feed::Repaint(60));
            draw(&mut context, &script);
        }
        assert_eq!(
            runtime.metrics().read().script_renders(),
            baseline + 1,
            "eight repaints must not enter the VM"
        );
        assert!(
            description(&mut context, &script).contains("reading 1"),
            "and the description must be the one already published"
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
            gpui::size(gpui::px(480.), gpui::px(360.)),
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
