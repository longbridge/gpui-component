//! `gpui-shell` — runs a script application directory.
//!
//! ```text
//! gpui-shell <directory> [--watch] [--dev] [--help] [--version]
//! ```
//!
//! The binary is the thin host the library documents: it parses a command line,
//! installs a log sink, builds one runtime, opens one window, and — when asked —
//! drives the source watcher from a GPUI timer. Every decision that outlives a
//! single invocation lives in the library instead.

use std::{
    cell::RefCell,
    fmt::{self, Write as _},
    path::{Path, PathBuf},
    rc::Rc,
};

use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, Context, Entity, IntoElement, Render,
    TitlebarOptions, Window, WindowBounds, WindowHandle, WindowOptions, px, size,
};
use gpui_shell::{AppAssets, Capabilities, ShellRoot, ShellRuntime};
use tracing::{
    Event, Level, Metadata, Subscriber,
    field::{Field, Visit},
    level_filters::LevelFilter,
    span,
};

/// The entry file an application directory must contain. Duplicated from the
/// engine because the host resolves the application root itself: it needs the
/// resolved directory for the window title and for the watcher, both of which
/// are decided before any script is loaded.
const ENTRY: &str = "main.js";

/// The exit code for a command line this binary could not act on. Distinct from
/// 1, which is a runtime that failed to start.
const EXIT_USAGE: i32 = 2;

fn main() {
    let arguments = match parse(std::env::args().skip(1)) {
        Ok(Invocation::Run(arguments)) => arguments,
        Ok(Invocation::Types(directory)) => {
            // The root always, whether or not anything there imports the module:
            // this command was asked for explicitly, and an empty directory is a
            // reasonable place to start an application. Every other directory
            // that imports `gpui` comes along with it.
            match gpui_shell::write_type_declarations(&directory) {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("gpui-shell: {error}");
                    std::process::exit(1);
                }
            }
            // The declarations describe the runtime; the manifest's Git
            // dependencies are the rest of what the editor has to resolve, and
            // a command that wrote half of it and said nothing would leave the
            // import underlined with no explanation.
            match gpui_shell::write_dependency_links(&directory) {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("gpui-shell: {error:#}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(Invocation::Check(arguments)) => {
            // A check reports diagnostics, not progress, so only warnings and
            // errors reach the terminal.
            install_log_sink(Level::WARN);
            check(arguments);
        }
        Ok(Invocation::Print(text)) => {
            println!("{text}");
            return;
        }
        Err(message) => {
            eprintln!("gpui-shell: {message}");
            eprintln!("Try `gpui-shell --help` for the accepted arguments.");
            std::process::exit(EXIT_USAGE);
        }
    };

    // Installed before anything else: the runtime reports script errors,
    // unhandled promise rejections and illegal-phase calls through `tracing`,
    // and until a subscriber exists every one of them is discarded silently.
    install_log_sink(if arguments.is_development() {
        Level::DEBUG
    } else {
        Level::INFO
    });

    run(arguments);
}

// ---------------------------------------------------------------------------
// Command line
// ---------------------------------------------------------------------------

/// What a successfully parsed command line asks the host to do.
///
/// `--help` and `--version` answer without starting a runtime, so they are a
/// separate outcome rather than a flag on [`Arguments`]: nothing downstream
/// should have to remember to check them.
#[derive(Debug, PartialEq, Eq)]
enum Invocation {
    Run(Arguments),
    /// Load and render once, report, and exit with a status.
    Check(CheckArguments),
    /// Write TypeScript declarations next to an application.
    Types(PathBuf),
    /// Text for stdout, followed by a successful exit.
    Print(String),
}

/// The command line, parsed.
///
/// A value rather than a set of locals so the parsing can be tested without
/// opening a window, which is the only part of this binary that has to run on a
/// desktop.
#[derive(Debug, PartialEq, Eq)]
struct Arguments {
    directory: PathBuf,
    watch: bool,
    development: bool,
}

impl Arguments {
    /// Whether sources are polled for changes. Always true in development mode:
    /// a REPL that cannot reload is half a workflow.
    fn is_watching(&self) -> bool {
        self.watch || self.development
    }

    /// Whether the sandbox relaxations are on.
    fn is_development(&self) -> bool {
        self.development
    }
}

/// Parses the arguments after the program name.
///
/// Hand-rolled because the whole surface is three flags and one path: a parser
/// dependency would be larger than the thing it parses. The error is the exact
/// sentence printed to stderr, so the caller decides nothing about wording.
fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Invocation, String> {
    let arguments: Vec<String> = arguments.into_iter().collect();

    // Answered before anything else can fail. A caller who mistyped one flag is
    // exactly the caller who needs `--help` to still work.
    if arguments.iter().any(|it| it == "--help" || it == "-h") {
        return Ok(Invocation::Print(help()));
    }
    if arguments.iter().any(|it| it == "--version" || it == "-V") {
        return Ok(Invocation::Print(version()));
    }

    let mut directory: Option<PathBuf> = None;
    let mut watch = false;
    let mut development = false;
    let mut check = false;
    let mut types = false;
    let mut print_spec = false;

    for argument in arguments {
        match argument.as_str() {
            // A subcommand rather than a flag, because it does something else
            // entirely: it never shows a window and it exits with a status.
            "check" if directory.is_none() && !check && !types => check = true,
            "types" if directory.is_none() && !check && !types => types = true,
            "--watch" => watch = true,
            "--dev" => development = true,
            "--print-spec" => print_spec = true,
            // A path is never mistaken for a flag, and an unknown flag is never
            // mistaken for a path: silently treating a mistyped `--watch` as a directory
            // would report a missing entry file instead of the typo.
            other if other.starts_with('-') => {
                return Err(format!("unknown flag `{other}`"));
            }
            other if directory.is_none() => directory = Some(PathBuf::from(other)),
            other => {
                return Err(format!(
                    "unexpected argument `{other}`; gpui-shell runs one application directory"
                ));
            }
        }
    }

    let Some(directory) = directory else {
        return Err("expected an application directory".to_owned());
    };

    if check {
        return Ok(Invocation::Check(CheckArguments {
            directory,
            print_spec,
        }));
    }

    if types {
        return Ok(Invocation::Types(directory));
    }

    Ok(Invocation::Run(Arguments {
        directory,
        watch,
        development,
    }))
}

/// What `check` was asked to do.
#[derive(Debug, PartialEq, Eq)]
struct CheckArguments {
    directory: PathBuf,
    print_spec: bool,
}

/// One line per flag, no decoration — the tone the repository's documentation
/// uses everywhere else.
fn help() -> String {
    format!(
        "\
{}

Usage: gpui-shell <directory> [options]
       gpui-shell check <directory> [--print-spec]
       gpui-shell types <directory>

Arguments:
  <directory>  The application root, or the {ENTRY} inside it.

Commands:
  types        Write gpui.d.ts next to the application and link the manifest's
               Git dependencies into node_modules, so an editor — or a model
               writing the code — sees the whole API, the packages it imports,
               and catches a mistyped style method before it runs.
  check        Load and render the application once without showing a window,
               then exit 0 if it worked and 1 if it did not. JavaScript has no
               compiler, so this is what takes its place: it reports syntax
               errors, unresolved imports, a missing or malformed default
               export, unknown style methods with a suggestion, wrongly typed
               style arguments, and an element used twice.

Options:
  --watch      Reload the application when its sources change.
  --dev        Development mode: implies --watch, and relaxes the sandbox.
  --print-spec With check, also print the element description that was built.
  --help       Print this message and exit.
  --version    Print the version and exit.",
        version()
    )
}

fn version() -> String {
    format!("gpui-shell {}", env!("CARGO_PKG_VERSION"))
}

// ---------------------------------------------------------------------------
// The host
// ---------------------------------------------------------------------------

fn run(arguments: Arguments) {
    // Assets are served from the application directory, so the source has to be
    // installed on the `Application` before the loop starts. Resolving the root
    // here rather than inside the loop is what makes that possible; a path that
    // does not resolve falls back to the argument and fails later with the
    // message that explains why.
    let asset_root = gpui_shell::resolve_app_root(&arguments.directory, ENTRY)
        .unwrap_or_else(|_| arguments.directory.clone());

    gpui_platform::application()
        .with_assets(AppAssets::new(asset_root))
        .run(move |cx| {
            gpui_shell::init(cx);

            if arguments.is_development() {
                // Before `ShellRuntime::new`: the policy is read when the
                // context is created.
                gpui_shell::set_development_mode(true);
                tracing::debug!("development mode: eval and the built-in prototypes are open");
            }

            let runtime = match ShellRuntime::new(cx) {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("failed to start the script runtime: {error}");
                    std::process::exit(1);
                }
            };

            // What `process.exit(code)` means here. The runtime never decides
            // this — one plugin must not be able to end an application somebody
            // is working in — but this host *is* the process, so ending it is
            // the honest answer. An embedded host installs something else:
            // closing a panel, closing a window, or refusing.
            gpui_shell::on_exit_request(|request, _, _| {
                let code = request.code();
                tracing::info!("the application asked to exit with {code}");
                std::process::exit(code);
            });

            // Resolving here rather than leaving it to `load_app` gives the window
            // title and the watcher the real application root even when the command
            // line named `main.js`. A path that does not resolve is not reported
            // twice: the load below fails with the message that explains why.
            let root = gpui_shell::resolve_app_root(&arguments.directory, ENTRY)
                .unwrap_or_else(|_| arguments.directory.clone());

            // A view's `init` can create GPUI entities, so the facade loads the
            // application inside the window builder.
            let manifest = read_local_manifest(&root);
            if let Ok(manifest) = &manifest {
                grant_local_access(&root, manifest.as_ref());
            }

            // The declarations describe the runtime that is about to run this
            // script, which is the only version worth editing against. Doing it
            // here rather than asking the author to remember a command is what
            // keeps a stale `gpui.d.ts` from being possible at all.
            //
            // Every launch rather than development mode only: this binary runs an
            // application from its source directory, which is where somebody is
            // editing. Nothing is written when the file already matches, and a
            // directory that refuses the write is logged rather than fatal.
            let manifest_error = manifest.err();
            let built: Rc<RefCell<Option<Entity<ShellRoot>>>> = Rc::new(RefCell::new(None));
            let sink = built.clone();
            let builder_runtime = runtime.clone();
            let application_root = root.clone();

            let window = cx
                .open_window(window_options(&root, cx), move |window, cx| {
                    let loaded = match &manifest_error {
                        Some(message) => Err(anyhow::anyhow!(message.clone())),
                        None => builder_runtime.try_load(&application_root, window, cx),
                    };
                    match loaded {
                        Ok(root) => {
                            *sink.borrow_mut() = Some(root.clone());
                            root
                        }
                        Err(error) => {
                            eprintln!("{error:#}");
                            let content = cx.new(|_| LoadFailure(format!("{error:#}")));
                            cx.new(|cx| ShellRoot::new(content.into(), window, cx))
                        }
                    }
                })
                .expect("failed to open window");

            let loaded = built.borrow_mut().take().ok_or(());

            if arguments.is_watching() {
                match loaded {
                    Ok(root) => watch_sources(runtime, root, window, cx),
                    // A failed first load never produced a mounted application.
                    // Saying so is better than a `--watch` that looks armed and
                    // never fires.
                    Err(()) => eprintln!(
                        "--watch is inactive: the application did not load, so there is \
                        no view to reload into. Fix the declared entry and start gpui-shell again."
                    ),
                }
            }
        });
}

/// Loads and renders the application once, without showing anything.
///
/// This is what a compiler would do for a language that had one. The script
/// surface is dynamic — an unknown style method, a wrongly typed argument or a
/// reused element are all runtime facts — so the only honest way to check an
/// application is to build it and render one frame. The window is real but
/// never shown, because rendering is where those facts surface.
fn check(arguments: CheckArguments) -> ! {
    // The exit status has to survive the app's own event loop, which does not
    // return a value, so it is stashed and read after `run` unwinds.
    let outcome = Rc::new(RefCell::new(CheckOutcome::default()));
    let sink = outcome.clone();
    let directory = arguments.directory.clone();

    // Assets are served from the application directory, so the source has to be
    // installed on the `Application` before the loop starts. Resolving the root
    // here rather than inside the loop is what makes that possible; a path that
    // does not resolve falls back to the argument and fails later with the
    // message that explains why.
    let asset_root = gpui_shell::resolve_app_root(&arguments.directory, ENTRY)
        .unwrap_or_else(|_| arguments.directory.clone());

    gpui_platform::application()
        .with_assets(AppAssets::new(asset_root))
        .run(move |cx| {
            gpui_shell::init(cx);

            let runtime = match ShellRuntime::new(cx) {
                Ok(runtime) => runtime,
                Err(error) => {
                    sink.borrow_mut().fail(format!("{error:#}"));
                    cx.quit();
                    return;
                }
            };

            let root = match gpui_shell::resolve_app_root(&arguments.directory, ENTRY) {
                Ok(root) => root,
                Err(error) => {
                    sink.borrow_mut().fail(format!("{error:#}"));
                    cx.quit();
                    return;
                }
            };

            let manifest = match read_local_manifest(&root) {
                Ok(manifest) => manifest,
                Err(error) => {
                    sink.borrow_mut().fail(error);
                    cx.quit();
                    return;
                }
            };
            grant_local_access(&root, manifest.as_ref());

            // A check is the closest thing this runtime has to a compiler, so it
            // leaves the editor correct on its way past: whatever it reports, the
            // declarations beside the source are the ones it just checked
            // against.
            let window_sink = sink.clone();
            let print_spec = arguments.print_spec;

            let opened = cx.open_window(hidden_window_options(cx), move |window, cx| {
                let result = runtime.check(&root, window, cx);

                match result {
                    Ok(spec) => window_sink.borrow_mut().succeed(spec, print_spec),
                    Err(error) => window_sink.borrow_mut().fail(format!("{error:#}")),
                }

                cx.new(|_| LoadFailure(String::new()))
            });

            if let Err(error) = opened {
                sink.borrow_mut().fail(format!("{error:#}"));
            }

            // Reporting and exiting happen here rather than after `run` returns:
            // an application loop that has opened a window does not unwind just
            // because nothing is shown, and a check that never terminates is worse
            // than one that reports nothing.
            let outcome = sink.borrow();
            outcome.report(&directory);
            std::process::exit(outcome.status());
        });

    unreachable!("the check exits from inside the application loop")
}

/// A window is needed to render, but nothing should appear on screen for a
/// check: it runs in editors, in CI, and in an agent's loop.
fn hidden_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        show: false,
        focus: false,
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(320.), px(240.)),
            cx,
        ))),
        ..Default::default()
    }
}

/// What the check found. Defaults to "nothing ran", which is itself a failure:
/// a check that silently does nothing must not report success.
#[derive(Default)]
struct CheckOutcome {
    error: Option<String>,
    spec: Option<String>,
    ran: bool,
}

impl CheckOutcome {
    fn fail(&mut self, error: String) {
        self.ran = true;
        self.error = Some(error);
    }

    fn succeed(&mut self, spec: String, keep: bool) {
        self.ran = true;
        if keep {
            self.spec = Some(spec);
        }
    }

    fn status(&self) -> i32 {
        if self.ran && self.error.is_none() {
            0
        } else {
            1
        }
    }

    fn report(&self, directory: &Path) {
        match &self.error {
            Some(error) => {
                eprintln!("{error}");
                eprintln!("\ncheck failed: {}", directory.display());
            }
            None if !self.ran => {
                eprintln!("check did not run: the window never opened");
            }
            None => {
                if let Some(spec) = &self.spec {
                    println!("{spec}");
                }
                println!("check passed: {}", directory.display());
            }
        }
    }
}

/// Installs the storage location and the capability grant for a local run.
fn grant_local_access(root: &Path, manifest: Option<&gpui_shell::plugin::PluginManifest>) {
    // A directory this command was pointed at has no name of its own, so its
    // path is its identity — which is right while someone is editing it, and is
    // exactly what an installed application should not do. An installed one
    // passes its own bundle id.
    let id = gpui_shell::bundle_id_for_path(root);
    let store = match gpui_shell::set_bundle_id(&id) {
        Ok(store) => store,
        Err(error) => {
            tracing::warn!("storage is unavailable: {error}");
            return;
        }
    };

    if let Err(error) = std::fs::create_dir_all(&store) {
        tracing::warn!(
            "storage is unavailable: cannot create {}: {error}",
            store.display()
        );
        return;
    }

    gpui_shell::set_capabilities(local_capabilities(root, &store, manifest));
    tracing::debug!("storage: {}", store.display());
}

/// Reads and validates inert application metadata before any entry code runs.
fn read_local_manifest(root: &Path) -> Result<Option<gpui_shell::plugin::PluginManifest>, String> {
    let path = root.join(gpui_shell::plugin::MANIFEST_FILE);
    if !path.exists() {
        return Ok(None);
    }

    gpui_shell::plugin::PluginManifest::read(root)
        .map(Some)
        .map_err(|error| format!("invalid {}: {error}", path.display()))
}

/// What a locally run application is allowed to do.
///
/// Running a directory from the command line is an explicit act of trust, the
/// same as `node app.js`: the application may read its own sources and use its
/// own storage. It is deliberately narrower than "everything" — no network, no
/// process execution, no clipboard, and no filesystem access outside those two
/// directories — because an installed plugin will run through the same code
/// path with a manifest deciding instead.
fn local_capabilities(
    root: &Path,
    store: &Path,
    manifest: Option<&gpui_shell::plugin::PluginManifest>,
) -> Capabilities {
    let capabilities = if let Some(manifest) = manifest {
        manifest.capabilities(root, store)
    } else {
        // Preserve the original local-app experience for source directories
        // that are not plugin bundles. Their private store is the only ambient
        // state they receive; network and clipboard stay denied.
        Capabilities::new().storage(true)
    };

    capabilities
        .read_roots([root.to_path_buf(), store.to_path_buf()])
        .write_roots([store.to_path_buf()])
        .exit(true)
}

fn window_options(root: &Path, cx: &App) -> WindowOptions {
    let title = root
        .file_name()
        .map(|name| format!("{} — gpui-shell", name.to_string_lossy()))
        .unwrap_or_else(|| "gpui-shell".to_owned());

    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(880.), px(720.)),
            cx,
        ))),
        // A window with no title is unidentifiable in a switcher or a tiling
        // layout, which is how this one first reached a user.
        titlebar: Some(TitlebarOptions {
            title: Some(title.into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Polls the application directory and reloads the view when it changes.
///
/// The loop, the failure toast and the exit conditions all live in
/// `ShellRuntime` — the binary used to carry its own copy of them, which
/// meant an embedded host got a quieter hot-reload than the CLI for no reason
/// anyone had decided. `--watch` is the one thing that is this binary's to
/// answer, because the person running it is the person editing; an embedded
/// host has no such flag and takes the debug build as the answer.
fn watch_sources(
    runtime: Rc<ShellRuntime>,
    root: Entity<ShellRoot>,
    window: WindowHandle<ShellRoot>,
    cx: &mut App,
) {
    // Through the untyped handle on purpose. `WindowHandle::<ShellRoot>::update`
    // leases the root view for the length of the closure, and `watch` reads that
    // same entity to find the mounted application — a second borrow GPUI answers
    // with a panic. All this call wants from the window is a `&mut Window`.
    let started =
        AnyWindowHandle::from(window).update(cx, |_, window, cx| runtime.watch(&root, window, cx));

    match started {
        // Detached on purpose: this watcher lasts as long as the window, and
        // the window is the whole application.
        Ok(Ok(watch)) => watch.forget(),
        Ok(Err(error)) => tracing::error!("cannot watch for changes: {error}"),
        Err(error) => tracing::error!("cannot watch for changes: {error}"),
    }
}

/// What the window shows when the application could not be loaded.
///
/// A failed load still opens a window: the error belongs on screen, not only in
/// a terminal the user may not be watching.
struct LoadFailure(String);

impl Render for LoadFailure {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        gpui_shell::failure_surface(
            "This application could not be loaded",
            &self.0,
            "gpui-shell <directory> expects main.js in that directory, \
             default-exporting a class that extends View.",
            window,
            cx,
        )
    }
}

// ---------------------------------------------------------------------------
// Logging
// ---------------------------------------------------------------------------

/// Sends `tracing` events to stderr.
///
/// The crate depends on `tracing` but not on `tracing-subscriber`, so without
/// this every `tracing::error!` the runtime emits — a throwing event handler, an
/// unhandled promise rejection, an overlay opened from the wrong phase — is
/// dropped on the floor and the author sees a view that simply stopped
/// responding. One line per event on stderr is the whole requirement here;
/// filtering, spans, and formatting are what `tracing-subscriber` is for, and
/// the day this binary wants any of them it should take that dependency rather
/// than grow the subscriber below.
fn install_log_sink(max_level: Level) {
    // An error means something already installed a subscriber, which is a
    // better sink than this one by definition.
    let _ = tracing::subscriber::set_global_default(StderrSubscriber { max_level });
}

struct StderrSubscriber {
    max_level: Level,
}

impl Subscriber for StderrSubscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() <= self.max_level
    }

    /// Lets `tracing` skip the callsite entirely instead of asking on every
    /// event, which is what keeps a disabled `debug!` in a render path free.
    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(LevelFilter::from_level(self.max_level))
    }

    /// Spans are not recorded, so every span shares one id. A subscriber that
    /// prints events and nothing else has no use for span identity.
    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }

    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut fields = EventFields::default();
        event.record(&mut fields);
        eprintln!(
            "{:<5} {}: {}",
            event.metadata().level(),
            event.metadata().target(),
            fields.0
        );
    }

    fn enter(&self, _: &span::Id) {}

    fn exit(&self, _: &span::Id) {}
}

/// One event's fields, flattened to a line.
#[derive(Default)]
struct EventFields(String);

impl Visit for EventFields {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        // The `message` field is the event's own text and is printed bare;
        // everything else is structured data and keeps its name.
        let _ = if field.name() == "message" {
            write!(self.0, "{value:?}")
        } else {
            write!(self.0, "{}={value:?}", field.name())
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(arguments: &[&str]) -> Result<Invocation, String> {
        parse(arguments.iter().map(|argument| (*argument).to_string()))
    }

    fn run_args(arguments: &[&str]) -> Arguments {
        match parse_args(arguments).expect("the arguments should parse") {
            Invocation::Run(arguments) => arguments,
            Invocation::Check(arguments) => panic!("expected a run, got a check of {arguments:?}"),
            Invocation::Types(directory) => {
                panic!("expected a run, got types for {}", directory.display())
            }
            Invocation::Print(text) => panic!("expected a run, got: {text}"),
        }
    }

    fn check_args(arguments: &[&str]) -> CheckArguments {
        match parse_args(arguments).expect("the arguments should parse") {
            Invocation::Check(arguments) => arguments,
            Invocation::Run(arguments) => panic!("expected a check, got a run of {arguments:?}"),
            Invocation::Types(directory) => {
                panic!("expected a check, got types for {}", directory.display())
            }
            Invocation::Print(text) => panic!("expected a check, got: {text}"),
        }
    }

    #[test]
    fn check_takes_a_directory_and_an_optional_spec_dump() {
        let arguments = check_args(&["check", "examples/js_todolist"]);
        assert_eq!(arguments.directory, PathBuf::from("examples/js_todolist"));
        assert!(!arguments.print_spec);

        let arguments = check_args(&["check", "examples/js_todolist", "--print-spec"]);
        assert!(arguments.print_spec);
    }

    #[test]
    fn check_is_a_command_not_a_directory() {
        // A directory literally named `check` would be ambiguous; the command
        // wins, which is why the error for a missing path has to be clear.
        assert_eq!(
            parse_args(&["check"]).unwrap_err(),
            "expected an application directory"
        );
    }

    #[test]
    fn a_bare_directory_runs_without_watching() {
        let arguments = run_args(&["examples/js_todolist"]);

        assert_eq!(arguments.directory, PathBuf::from("examples/js_todolist"));
        assert!(!arguments.is_watching());
        assert!(!arguments.is_development());
    }

    #[test]
    fn watch_is_accepted_on_either_side_of_the_directory() {
        assert!(run_args(&["app", "--watch"]).is_watching());
        assert!(run_args(&["--watch", "app"]).is_watching());
    }

    #[test]
    fn dev_implies_watch() {
        let arguments = run_args(&["app", "--dev"]);

        assert!(arguments.is_development());
        assert!(
            arguments.is_watching(),
            "development mode without reloading is half a workflow"
        );
    }

    #[test]
    fn an_unknown_flag_is_reported_rather_than_taken_as_a_path() {
        let error = parse_args(&["app", "--wtach"]).expect_err("a typo is not a directory");
        assert!(error.contains("--wtach"), "the message names the flag");
    }

    #[test]
    fn a_second_directory_is_reported() {
        let error = parse_args(&["app", "other"]).expect_err("only one application runs");
        assert!(error.contains("other"));
    }

    #[test]
    fn a_missing_directory_is_reported() {
        let error = parse_args(&[]).expect_err("there is nothing to run");
        assert!(error.contains("application directory"));
    }

    #[test]
    fn help_and_version_answer_before_anything_else() {
        // Reachable even when the rest of the line is unusable, which is when a
        // caller most needs to be told what the accepted arguments are.
        for arguments in [&["--help"][..], &["--nope", "--help"][..]] {
            assert!(matches!(parse_args(arguments), Ok(Invocation::Print(_))));
        }

        let Ok(Invocation::Print(text)) = parse_args(&["--version"]) else {
            panic!("--version prints");
        };
        assert!(text.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn help_lists_every_flag() {
        let help = help();
        for flag in ["--watch", "--dev", "--help", "--version"] {
            assert!(help.contains(flag), "`{flag}` is missing from the help");
        }
    }

    #[test]
    fn a_local_application_manifest_grants_its_declared_network_capabilities() {
        let root = std::env::temp_dir().join(format!(
            "gpui-shell-local-manifest-current-{}",
            std::process::id()
        ));
        let data = root.join("data");
        std::fs::create_dir_all(&root).expect("temporary app directory");
        std::fs::write(
            root.join("gpui-shell.json"),
            r#"{
              "id": "com.example.market",
              "name": "Market",
              "version": "1.0.0",
              "shell-version": "0.1.0",
              "entry": "main.js",
              "capabilities": {
                "network": { "hosts": ["quotes.example.com"] },
                "storage": true,
                "clipboard": { "write": true }
              }
            }"#,
        )
        .expect("manifest");

        let manifest = read_local_manifest(&root)
            .expect("valid manifest")
            .expect("present manifest");
        let capabilities = local_capabilities(&root, &data, Some(&manifest));
        assert!(capabilities.may_reach("quotes.example.com"));
        assert!(capabilities.has_storage());
        assert!(capabilities.is_clipboard_writable());
        assert!(!capabilities.is_clipboard_readable());
        assert!(!capabilities.may_reach("trade.example.com"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn an_incompatible_local_manifest_is_refused_before_loading_script() {
        let root = std::env::temp_dir().join(format!(
            "gpui-shell-local-manifest-version-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("temporary app directory");
        std::fs::write(
            root.join("gpui-shell.json"),
            r#"{
              "id": "com.example.future",
              "name": "Future",
              "version": "1.0.0",
              "shell-version": "99.0.0",
              "entry": "main.js"
            }"#,
        )
        .expect("manifest");

        let error = read_local_manifest(&root).expect_err("future shells are incompatible");
        assert!(error.contains("requires gpui-shell 99.0.0"), "{error}");

        let _ = std::fs::remove_dir_all(root);
    }
}
