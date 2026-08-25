//! The filesystem surface, which is asynchronous.
//!
//! These need a real `App`, because that is where the executors are: a capability
//! check runs on the calling thread and the syscall behind it does not. The
//! denial cases live next to the resolver in `capability.rs` and `host.rs`,
//! where they need no window at all — a refusal never reaches the disk.

use std::ops::Deref;

use crate::{Capabilities, ShellRuntime};
use gpui::{TestAppContext, VisualTestContext};

/// A view that does its filesystem work in a task and records the outcome, so
/// the assertion can be made on what the script saw rather than on what the host
/// did.
const PROBE: &str = r#"
import { View, v_flex, text, fs, spawn } from "gpui";

export default class Probe extends View {
  init() {
    this.state = "pending";

    spawn(async (cx) => {
      try {
        await fs.write_text("notes.txt", "hello");
        const back = await fs.read_text("notes.txt");
        const names = (await fs.read_dir(".")).map((entry) => entry.name);
        const there = await fs.exists("notes.txt");

        await fs.mkdir("nested/deeper", { recursive: true });
        const nested = await fs.exists("nested/deeper");

        await fs.remove_file("notes.txt");
        const gone = !(await fs.exists("notes.txt"));

        this.state = `${back}|${names.join(",")}|${there}|${nested}|${gone}`;
      } catch (error) {
        this.state = `failed: ${error.message}`;
      }
      cx.notify();
    });
  }

  render() {
    return v_flex().child(text(this.state));
  }
}
"#;

#[gpui::test]
fn every_fs_call_settles_through_a_promise(cx: &mut TestAppContext) {
    let directory = std::env::temp_dir().join(format!("gpui-shell-fs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a granted root");

    cx.update(|cx| crate::init(cx));
    crate::set_capabilities(
        Capabilities::new()
            .read_roots([directory.clone()])
            .write_roots([directory.clone()]),
    );

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source("probe.js", PROBE).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    // Nothing has happened yet: the calls returned promises and the work is on
    // a background thread. That is the property this whole change is about.
    let before = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });
    assert!(
        before.contains("pending"),
        "the first render should have found the work still in flight, got: {before}"
    );

    context.run_until_parked();

    let after = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });
    assert!(
        after.contains("hello|notes.txt|true|true|true"),
        "the round trip did not settle as expected: {after}"
    );

    let _ = std::fs::remove_dir_all(&directory);
}

/// A file over the ceiling is refused by name rather than by an out-of-memory
/// somewhere inside the VM.
#[gpui::test]
fn an_oversized_read_is_refused_by_name(cx: &mut TestAppContext) {
    let directory = std::env::temp_dir().join(format!("gpui-shell-fs-big-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a granted root");

    // Sparse where the filesystem allows it: the test is about the check, not
    // about moving sixty-five megabytes.
    let big = directory.join("big.bin");
    let file = std::fs::File::create(&big).expect("a large file");
    file.set_len(65 * 1024 * 1024).expect("a large length");
    drop(file);

    cx.update(|cx| crate::init(cx));
    crate::set_capabilities(Capabilities::new().read_roots([directory.clone()]));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = PROBE.replace(
        r#"await fs.write_text("notes.txt", "hello");"#,
        r#"await fs.read_text("big.bin");"#,
    );
    let view_type = runtime.load_source("big.js", &source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    context.run_until_parked();

    let rendered = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });
    assert!(
        rendered.contains("big.bin") && rendered.contains("limit"),
        "an oversized read should name the file and the limit: {rendered}"
    );

    let _ = std::fs::remove_dir_all(&directory);
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

/// The store answers from memory and reaches the disk on its own.
///
/// `set` stays synchronous — a setting a script can read during `render` without
/// awaiting is the whole point of the cache — and the write it makes necessary
/// happens on a background thread. `flush` is for a script that has to know the
/// write landed.
const STORE_PROBE: &str = r#"
import { View, v_flex, text, store, spawn } from "gpui";

export default class Probe extends View {
  init() {
    // Synchronous, against the cache. A burst of these is one file, not four.
    store.set("window", { title: "Notes", size: [640, 480] });
    store.set("theme", "dark");
    store.set("scratch", 1);
    store.remove("scratch");

    // Readable immediately, with nothing awaited.
    this.state = `${store.get("window").title}|${store.keys().join(",")}`;

    spawn(async (cx) => {
      await store.flush();
      this.state += "|flushed";
      cx.notify();
    });
  }

  render() {
    return v_flex().child(text(this.state));
  }
}
"#;

#[gpui::test]
fn the_store_answers_from_memory_and_persists_off_thread(cx: &mut TestAppContext) {
    let directory = std::env::temp_dir().join(format!("gpui-shell-store-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a directory for the store");
    let file = directory.join("store.json");

    cx.update(|cx| crate::init(cx));
    crate::set_capabilities(Capabilities::new().store(true));
    crate::set_store_path(file.clone());

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source("store.js", STORE_PROBE).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    // The cache answered during `init`, before anything reached the disk.
    let immediate = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });
    assert!(
        immediate.contains("Notes|window,theme"),
        "the store should answer from memory without awaiting: {immediate}"
    );

    context.run_until_parked();

    let settled = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });
    assert!(
        settled.contains("flushed"),
        "flush never resolved: {settled}"
    );

    // It reached the disk, atomically, and the removed key did not.
    let written = std::fs::read_to_string(&file).expect("the store file exists");
    assert!(written.contains("\"title\": \"Notes\""), "{written}");
    assert!(!written.contains("scratch"), "{written}");
    assert!(
        !directory.join("store.json.tmp").exists(),
        "the temporary file should have been renamed away"
    );

    let _ = std::fs::remove_dir_all(&directory);
}
