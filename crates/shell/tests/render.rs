//! End-to-end tests for the script render protocol.
//!
//! These exercise the whole path — VM, method dispatch, spec arena, event
//! callbacks — without painting a frame, because the element description is
//! plain data. They run against whichever engine is enabled, which is what
//! keeps the fallback engine honest.

use gpui::{AppContext as _, TestAppContext, VisualTestContext};
use gpui_shell::{ScriptView, ShellRuntime};

const COUNTER: &str = r#"
import { View, v_flex, text, Button } from "gpui";

export default class Counter extends View {
  init() {
    this.count = 0;
  }

  render() {
    return v_flex()
      .size_full()
      .items_center()
      .gap_2()
      .p(16)
      .bg("background")
      .child(text(`Count: ${this.count}`).text_color("foreground"))
      .child(
        Button.new("increment")
          .px(12)
          .py(6)
          .rounded(6)
          .bg("primary")
          .on_click((event, cx) => {
            this.count += 1;
            cx.notify();
          })
          .child(text("Increment").text_color("primary_foreground")),
      );
  }
}
"#;

/// The entry name only affects diagnostics, but each engine has its own
/// convention and the tests should read the way real code does.
const ENTRY: &str = "counter.js";

#[gpui::test]
fn a_script_view_produces_an_element_description(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime.load_source(ENTRY, COUNTER).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(tree.starts_with("v_flex"), "unexpected root: {tree}");
    assert!(tree.contains("text \"Count: 0\""), "missing label: {tree}");
    assert!(
        tree.contains("Button \"increment\""),
        "missing button: {tree}"
    );
    assert!(tree.contains(":on_click(fn)"), "missing handler: {tree}");
}

#[gpui::test]
fn an_element_cannot_be_added_to_two_parents(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, v_flex, text } from "gpui";

export default class Broken extends View {
  render() {
    const shared = text("reused");
    return v_flex().child(shared).child(shared);
  }
}
"#;

    let view_type = runtime.load_source("broken", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("reusing an element must fail");

    assert!(
        error.to_string().contains("already added to a parent"),
        "unexpected error: {error}"
    );
}

#[gpui::test]
fn an_unknown_style_method_suggests_the_closest_name(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, div } from "gpui";

export default class Typo extends View {
  render() {
    return div().items_centre();
  }
}
"#;

    let view_type = runtime.load_source("typo", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a typo must fail");

    assert!(
        error.to_string().contains("items_center"),
        "expected a suggestion, got: {error}"
    );
}

#[gpui::test]
fn a_view_renders_through_gpui(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime.load_source(ENTRY, COUNTER).expect("load");

    // The view is constructed inside the window builder, because `init` may
    // create retained state and that needs a live `Window`.
    let runtime_for_view = runtime.clone();
    let window = cx.add_window(|window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view.clone(), object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    // A real paint must not panic: it exercises materialize, not just the
    // description.
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        |_, _| gpui::div(),
    );
    context.run_until_parked();
}

struct Empty;

impl gpui::Render for Empty {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
    }
}

use std::ops::Deref;

#[gpui::test]
fn the_bundled_example_application_loads_and_renders(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    // The example is the contract with users: if it stops rendering, the
    // quickstart in the README is wrong.
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/js_todolist")
        .canonicalize()
        .expect("example directory");

    let view_type = runtime
        .load_app(&directory, "main.js")
        .expect("load example");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(tree.contains("Button"), "example has no button: {tree}");
    assert!(tree.contains("text"), "example has no text: {tree}");
}

#[gpui::test]
fn state_styles_reuse_the_ordinary_style_methods(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, div, Button, text } from "gpui";

export default class Styled extends View {
  render() {
    return div()
      .hover((el) => el.bg("accent"))
      .child(
        Button.new("go")
          .bg("primary")
          .hover((el) => el.opacity(0.9))
          .active((el) => el.opacity(0.8))
          .child(text("Go")),
      );
  }
}
"#;

    let view_type = runtime.load_source("styled", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(tree.contains(":hover(.bg"), "hover not recorded: {tree}");
    assert!(
        tree.contains(":active(.opacity"),
        "active not recorded: {tree}"
    );
}

#[gpui::test]
fn theme_tokens_resolve_outside_a_call_scope(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    // Materialization happens after the call scope closes, so a palette that
    // could only be read through the scope resolved every color to `None` and
    // painted an unstyled black window. This is that regression.
    assert!(
        gpui_shell::theme::token_color("background").is_some(),
        "semantic tokens must resolve without an open call scope"
    );
    assert!(gpui_shell::theme::token_color("primary").is_some());
    assert!(gpui_shell::theme::token_color("not_a_token").is_none());
}

/// The todo list exists to exercise the whole runtime at once: retained input
/// state, controlled checkboxes, a dialog, a toast, capability-gated storage,
/// and a filter that must survive every mutation. If a subsystem regresses,
/// this is the test that notices.
#[gpui::test]
fn the_todolist_example_exercises_the_runtime(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/js_todolist")
        .canonicalize()
        .expect("example directory");

    let view_type = runtime
        .load_app(&directory, "main.js")
        .expect("load example");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    // `init` creates retained state, so instantiation needs a live host call.
    let object = context.update(|window, cx| runtime.instantiate(&view_type, window, cx));
    let object = object.expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    for expected in [
        "Todo",             // the object
        "Input",            // retained state reached the description
        "\"Add\"",          // the action that creates work
        "No items yet",     // the empty state explains the next step
        "Clear completed…", // an ellipsis, because it opens a dialog
    ] {
        assert!(
            tree.contains(expected),
            "todolist is missing `{expected}`:\n{tree}"
        );
    }
}

#[gpui::test]
fn an_unknown_input_event_names_the_valid_ones(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, div, InputState } from "gpui";

export default class Bad extends View {
  init() {
    this.field = InputState.new({});
    this.field.on("entered", () => {});
  }
  render() {
    return div();
  }
}
"#;

    let view_type = runtime.load_source("bad", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let error = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect_err("an unknown event name must fail");

    assert!(
        error.to_string().contains("submit"),
        "the error should list the valid events, got: {error}"
    );
}

/// Hot reload has to pick up a change in an imported module, not only in the
/// entry point. QuickJS caches an evaluated module by name and an ES module
/// cannot be unloaded, so a naive reload re-evaluates `main.js` against the
/// first version of everything it imports — and looks like it worked.
#[gpui::test]
fn a_reload_picks_up_a_change_in_an_imported_module(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory = std::env::temp_dir().join(format!("gpui-shell-reload-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("temp directory");

    std::fs::write(
        directory.join("main.js"),
        r#"
import { View, v_flex, text } from "gpui";
import { caption } from "./caption.js";

export default class Reloading extends View {
  render() {
    return v_flex().child(text(caption()));
  }
}
"#,
    )
    .expect("write main");
    std::fs::write(
        directory.join("caption.js"),
        "export const caption = () => \"before\";\n",
    )
    .expect("write caption");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let render = |context: &mut VisualTestContext| {
        let view_type = runtime.load_app(&directory, "main.js").expect("load");
        let object = context
            .update(|window, cx| runtime.instantiate(&view_type, window, cx))
            .expect("instantiate");
        context.update(|window, cx| {
            runtime
                .render_to_spec(&object, None, window, cx)
                .expect("render")
        })
    };

    assert!(render(&mut context).contains("before"));

    std::fs::write(
        directory.join("caption.js"),
        "export const caption = () => \"after\";\n",
    )
    .expect("rewrite caption");

    let reloaded = render(&mut context);
    assert!(
        reloaded.contains("after"),
        "the imported module was served from the cache:\n{reloaded}"
    );

    std::fs::remove_dir_all(&directory).ok();
}

/// An embedded runtime reloads on a save, with no host doing anything but
/// asking for it once.
///
/// The binary has `--watch` because the person running it is the person
/// editing. A host that embeds the runtime has no flag to offer, so a debug
/// build simply *is* the development build — and this is the test that says so,
/// since the behaviour is otherwise invisible until someone saves a file.
#[gpui::test]
fn an_embedded_runtime_reloads_when_a_source_changes(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory = std::env::temp_dir().join(format!("gpui-shell-watch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a temporary application");
    let source = |caption: &str| {
        format!(
            "import {{ View, v_flex, text }} from \"gpui\";\n\
             export default class Panel extends View {{\n\
               render() {{ return v_flex().child(text(\"{caption}\")); }}\n\
             }}\n"
        )
    };
    std::fs::write(directory.join("main.js"), source("before")).expect("writing main.js");

    let view_type = runtime.load_app(&directory, "main.js").expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let view = context.update(|window, cx| {
        let object = runtime
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        let view = cx.new(|_| ScriptView::new(runtime.clone(), object));
        gpui_shell::watch::reload_in_debug(
            &runtime,
            &view,
            directory.clone(),
            "main.js",
            window,
            cx,
        );
        view
    });

    let description = |context: &mut VisualTestContext| {
        context.update(|_, cx| {
            view.read(cx)
                .snapshot()
                .map(gpui_shell::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };

    draw(&mut context, &view);
    assert!(description(&mut context).contains("before"));

    // The watcher compares modification stamps, so the file has to look older
    // than the write that follows it — a test that runs inside one filesystem
    // tick would otherwise see nothing change.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(directory.join("main.js"), source("after")).expect("rewriting main.js");

    // Two polls, with real time in between. The poll interval is on the
    // executor's clock, which `advance_clock` moves; the debounce is measured
    // against the wall, because it is absorbing a burst of saves from an editor
    // rather than counting frames. So the first poll notices the change and the
    // second one — after the tree has been still for the debounce window —
    // reports it.
    let settle = |context: &mut VisualTestContext| {
        context
            .executor()
            .advance_clock(gpui_shell::watch::POLL_INTERVAL * 2);
        context.run_until_parked();
    };
    settle(&mut context);
    std::thread::sleep(std::time::Duration::from_millis(250));
    settle(&mut context);

    draw(&mut context, &view);

    assert!(
        description(&mut context).contains("after"),
        "a saved change should have reached the view without anyone asking: {}",
        description(&mut context)
    );

    let _ = std::fs::remove_dir_all(&directory);
}

fn draw(context: &mut VisualTestContext, view: &gpui::Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| gpui::IntoElement::into_any_element(view),
    );
}

/// A granted `process.exit` reaches the host, with the code the script asked
/// for.
///
/// The request used to be written into a cell no production code read: the
/// script got a success and the window stayed open. So the test is not "the
/// flag was set" but "the host was told", which is the only version of this
/// that can go wrong quietly.
#[gpui::test]
fn a_granted_exit_reaches_the_host(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));
    gpui_shell::set_capabilities(
        gpui_shell::Capabilities::new().with_read_roots([std::env::temp_dir()]),
    );

    let asked: std::rc::Rc<std::cell::Cell<Option<i32>>> = Default::default();
    let recorded = asked.clone();
    gpui_shell::on_exit_request(move |code, _, _| recorded.set(Some(code)));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, v_flex, text } from "gpui";

export default class Quitter extends View {
  init() {
    process.exit(7);
  }

  render() {
    return v_flex().child(text("still here"));
  }
}
"#;
    let view_type = runtime.load_source("quitter.js", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    assert_eq!(
        asked.get(),
        Some(7),
        "the host was never told the script asked to exit"
    );

    gpui_shell::clear_exit_handler();
}

/// A watcher does not keep its view alive, and stops when the view goes.
///
/// The loop polls every quarter second for the life of the window. Holding the
/// view strongly would mean a panel removed from a dock is never dropped — the
/// runtime it points at is never dropped either — and the poller goes on stating
/// a directory for a panel nobody can see. Mount and unmount a few and they
/// accumulate.
#[gpui::test]
fn a_watcher_releases_its_view(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory = std::env::temp_dir().join(format!("gpui-shell-release-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a temporary application");
    std::fs::write(
        directory.join("main.js"),
        "import { View, v_flex } from \"gpui\";\n\
         export default class Panel extends View { render() { return v_flex(); } }\n",
    )
    .expect("writing main.js");

    let view_type = runtime.load_app(&directory, "main.js").expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let weak = context.update(|window, cx| {
        let object = runtime
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        let view = cx.new(|_| ScriptView::new(runtime.clone(), object));
        gpui_shell::watch::reload_in_debug(
            &runtime,
            &view,
            directory.clone(),
            "main.js",
            window,
            cx,
        );
        view.downgrade()
    });

    // Nothing else is holding it: the panel it stood for has been removed.
    context
        .executor()
        .advance_clock(gpui_shell::watch::POLL_INTERVAL * 2);
    context.run_until_parked();

    assert!(
        weak.upgrade().is_none(),
        "the watcher is still holding the view it was watching for"
    );

    let _ = std::fs::remove_dir_all(&directory);
}

/// A second runtime on one thread is refused, because the policy it would run
/// under is not its own.
///
/// The capability grant, the store, the exit handler and the native module
/// registry are thread state. Two runtimes would share the last installer's
/// permissions with nothing saying so — the second would simply run under the
/// first one's grant. Enforcing it beats documenting it.
#[gpui::test]
fn a_second_runtime_on_one_thread_is_refused(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let first = ShellRuntime::new().expect("the first runtime");
    let error = ShellRuntime::new()
        .err()
        .expect("a second runtime must be refused")
        .to_string();
    assert!(
        error.contains("already running") && error.contains("permissions"),
        "the refusal has to say what would go wrong, got: {error}"
    );

    // Dropping the first releases the thread, so a host that tears one down and
    // starts another is not blocked.
    drop(first);
    let _second = ShellRuntime::new().expect("a runtime after the first was dropped");
}
