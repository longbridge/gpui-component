//! TypeScript declarations for the script API (design doc §14.4).
//!
//! An application written against this runtime is JavaScript, so the only
//! checking it gets before it runs is whatever an editor can infer. A `.d.ts`
//! turns that into a real contract: completion for a surface no one memorizes,
//! and `// @ts-check` catching a mistyped style name, a color token that does
//! not exist, or `.p("auto")` — which the runtime rejects — at the call site.
//! It is also the form in which the API can be handed to a model, which is an
//! explicit audience here.
//!
//! # Why these declarations can be trusted
//!
//! They are *generated from the tables the runtime dispatches through*, not
//! transcribed from documentation:
//!
//! * The style methods come from [`style::known_names`] — the same list the
//!   JS prelude loops over when it builds the element prototype. A method GPUI
//!   adds upstream appears here without anyone writing it down, and a name that
//!   type-checks is a name the dispatcher will accept.
//! * A parametric method's argument type is *probed*: [`argument_of`] asks
//!   [`style::apply_param`] which literals it accepts, so the difference
//!   between `Length`, `DefiniteLength`, `AbsoluteLength`, a color and a bare
//!   number is decided by the code that enforces it rather than by a second
//!   hand-written table that could disagree with the first.
//! * The color union comes from [`theme::color_token_names`], so a mistyped
//!   token is a type error, and the phase union comes from [`ScopePhase`]
//!   itself.
//!
//! # What they deliberately do not cover
//!
//! * **Capabilities.** Every `fs`, `store`, `clipboard` and `process` call
//!   type-checks; whether it is *granted* is a manifest question answered at
//!   run time (§19.2). Types cannot express a grant.
//! * **Element lifetime.** An element is consumed when it is used and belongs
//!   to one render pass; so does the `cx` handed to `render`. TypeScript has no
//!   affine types, so reusing an element still type-checks and still throws.
//! * **Which methods suit which component.** Every element shares one
//!   prototype, so `.checked(true)` is declared on all of them and is simply
//!   inert on a `div`. Narrowing that would mean inventing a type hierarchy the
//!   runtime does not have.
//! * **Retained entities** ([`crate::entities`]) and anything else not exported
//!   by the `gpui` module today.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use gpui::StyleRefinement;

use crate::scope::ScopePhase;
use crate::style;
use crate::theme::color_token_names;
use crate::value::Bridged;

/// The file [`write_to`] writes. Fixed, because an editor finds the
/// declarations by having them in the project, not by being told where.
pub const FILE_NAME: &str = "gpui.d.ts";

/// Emits the TypeScript declarations for the script API.
///
/// The output is deterministic — no timestamps, no reflection order — so
/// regenerating it after a runtime upgrade produces a reviewable diff rather
/// than a reshuffled file.
pub fn declarations() -> String {
    let (nullary, parametric) = style_methods();

    let mut out = String::with_capacity(160 * 1024);
    out.push_str(PREAMBLE);
    out.push_str("declare module \"gpui\" {\n");
    out.push_str(VALUE_TYPES);
    out.push_str(&color_types());
    out.push_str(&view_types());
    out.push_str("  /**\n");
    out.push_str("   * A description of one element, built by chaining.\n");
    out.push_str("   *\n");
    out.push_str("   * Every method returns the same element, so a chain is one\n");
    out.push_str("   * expression. An element is consumed when it is used as a child and\n");
    out.push_str("   * belongs to the render pass that built it; storing one and using it\n");
    out.push_str("   * again throws, which no type can prevent.\n");
    out.push_str("   */\n");
    out.push_str("  export interface Element {\n");
    out.push_str(ELEMENT_METHODS);
    out.push_str(&parametric_styles(&parametric));
    out.push_str(&nullary_styles(&nullary));
    out.push_str("  }\n");
    out.push_str(CONSTRUCTORS);
    out.push_str(CAPABILITIES);
    out.push_str(SCHEDULING);
    out.push_str("}\n");
    out
}

/// Writes the declarations next to an application, so an editor picks them up.
///
/// Creates `directory` when it is missing: the usual caller is a `types`
/// subcommand pointed at a directory an application has not been written into
/// yet, and failing on that would be a worse answer than making it.
pub fn write_to(directory: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(directory)?;
    let path = directory.join(FILE_NAME);
    std::fs::write(&path, declarations())?;
    Ok(path)
}

/// Splits the style surface into the two halves that need different treatment:
/// the no-argument methods, which are all alike, and the parametric ones, which
/// each need an argument type.
///
/// Both come out of one sorted, deduplicated list, so the two halves cannot
/// overlap and cannot together miss a name the runtime accepts.
fn style_methods() -> (Vec<&'static str>, Vec<&'static str>) {
    style::known_names()
        .into_iter()
        .partition(|name| style::param_style_name(name).is_none())
}

/// What a parametric style method accepts.
///
/// Named after the GPUI types they mirror, because the whole point of the
/// distinction is that the Rust signature is what rejects `.p("auto")`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Argument {
    Length,
    DefiniteLength,
    AbsoluteLength,
    Color,
    Number,
    /// Nothing the probe recognizes. Emitted as `never` rather than `any` so a
    /// style method added to the runtime without a matching literal here fails
    /// loudly at the first call site instead of silently accepting anything.
    Unrecognized,
}

impl Argument {
    fn ts_type(self) -> &'static str {
        match self {
            Argument::Length => "Length",
            Argument::DefiniteLength => "DefiniteLength",
            Argument::AbsoluteLength => "AbsoluteLength",
            Argument::Color => "Color",
            Argument::Number => "number",
            Argument::Unrecognized => "never",
        }
    }
}

/// Asks the runtime what `name` accepts, by handing it one literal of each
/// shape and seeing which are refused.
///
/// The order matters and follows the containment of the grammars: a color is
/// the only argument that takes `#rrggbb`, `Length` the only one that takes
/// `"auto"`, `DefiniteLength` the only remaining one that takes a percentage,
/// and `AbsoluteLength` the only remaining one that takes any string at all.
/// `line_height` classifies as a definite length, which is the right *type*;
/// its bare number means a multiplier rather than pixels, and that is a
/// documentation matter rather than a typing one.
fn argument_of(name: &str) -> Argument {
    let accepts = |value: Bridged| style::apply_param(name, &[value], StyleRefinement::default());

    if accepts(Bridged::Str("#ff0000".into())).is_ok() {
        Argument::Color
    } else if accepts(Bridged::Str("auto".into())).is_ok() {
        Argument::Length
    } else if accepts(Bridged::Str("50%".into())).is_ok() {
        Argument::DefiniteLength
    } else if accepts(Bridged::Str("12px".into())).is_ok() {
        Argument::AbsoluteLength
    } else if accepts(Bridged::Number(1.)).is_ok() {
        Argument::Number
    } else {
        Argument::Unrecognized
    }
}

fn color_types() -> String {
    let mut out = String::new();
    out.push_str("  /** Every semantic color token the installed palette defines. */\n");
    out.push_str("  export type ColorToken =\n");
    for name in color_token_names() {
        let _ = writeln!(out, "    | \"{name}\"");
    }
    out.push_str("    ;\n\n");
    out.push_str("  /**\n");
    out.push_str("   * A color: a semantic token name, or a `#rgb`, `#rrggbb` or `#rrggbbaa`\n");
    out.push_str("   * literal. Prefer a token; a literal bypasses the theme, and a theme\n");
    out.push_str("   * switch will not reach it.\n");
    out.push_str("   *\n");
    out.push_str("   * The union is closed, so a mistyped token is a compile error. A token\n");
    out.push_str("   * name that reaches a call through a variable widens to `string` and\n");
    out.push_str("   * has to say what it is:\n");
    out.push_str("   *\n");
    out.push_str("   *     /** @type {{ bg: import(\"gpui\").Color }} *\\/\n");
    out.push_str("   *     const palette = tone === \"blocking\" ? ... : ...;\n");
    out.push_str("   */\n");
    out.push_str("  export type Color = ColorToken | `#${string}`;\n\n");
    out
}

fn view_types() -> String {
    let mut out = String::new();
    out.push_str("  /** The phase a host call is in, as `cx.phase()` reports it. */\n");
    out.push_str("  export type Phase =\n");
    for phase in [
        ScopePhase::Render,
        ScopePhase::Event,
        ScopePhase::Task,
        ScopePhase::Layout,
    ] {
        let _ = writeln!(out, "    | \"{}\"", phase.as_str());
    }
    // Not a `ScopePhase`: `phase()` answers this outside any host call, where
    // there is no frame to report.
    out.push_str("    | \"none\"\n");
    out.push_str("    ;\n\n");
    out.push_str(CONTEXT_AND_VIEW);
    out
}

/// The style methods that take an argument, sorted, each typed by probe.
fn parametric_styles(names: &[&'static str]) -> String {
    let mut out = String::new();
    out.push_str("\n    // Style methods that take an argument. Which length type a method\n");
    out.push_str("    // accepts follows its Rust signature, so `.p(\"auto\")` and\n");
    out.push_str("    // `.rounded(\"50%\")` are type errors here for the same reason they\n");
    out.push_str("    // throw at run time.\n");
    for name in names {
        if *name == "line_height" {
            out.push_str(
                "    /** A bare number is a multiplier (`1.45`), not pixels; a string is a length. */\n",
            );
        }
        let _ = writeln!(
            out,
            "    {name}(value: {}): Element;",
            argument_of(name).ts_type()
        );
    }
    out
}

/// The no-argument style methods, straight from the reflection table.
fn nullary_styles(names: &[&'static str]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n    // The {} no-argument style methods, generated from GPUI's reflection\n    \
         // table. A name here is a name the runtime dispatches.",
        names.len()
    );
    for name in names {
        let _ = writeln!(out, "    {name}(): Element;");
    }
    out
}

const PREAMBLE: &str = "\
// The built-in `gpui` module, as TypeScript declarations.
//
// Generated by gpui-shell — do not edit. Regenerate with
// `gpui-shell types <directory>` after upgrading the runtime.
//
// The style surface here is generated from the same tables the runtime
// dispatches through, so a style method that type-checks exists at run time,
// and a length or color the compiler refuses is one the runtime would refuse
// too. Put `// @ts-check` at the top of a script to have an editor check it.
//
// What is not expressed: capability grants (a denied `fs.read_text` still
// type-checks), element and `cx` lifetimes (both belong to one call), and
// which component a method suits (all elements share one prototype).

";

const VALUE_TYPES: &str = r#"  /**
   * A length. A bare number is pixels; a string carries its unit.
   *
   * `"auto"` is only accepted where the Rust signature takes `Length` — the
   * padding, gap, border and radius families take the narrower types below.
   */
  export type Length = number | LengthString | "auto";

  /** A length that must resolve to a size: pixels, rems or a percentage. */
  export type DefiniteLength = number | LengthString;

  /** A length with no percentage and no `"auto"`: pixels or rems. */
  export type AbsoluteLength = number | `${number}px` | `${number}rem`;

  /** The string forms of a length: `"12px"`, `"1.5rem"`, `"50%"`. */
  export type LengthString = `${number}px` | `${number}rem` | `${number}%`;

  /** A JSON value — everything the store can persist, and nothing else. */
  export type Json =
    | null
    | boolean
    | number
    | string
    | Json[]
    | { [key: string]: Json };

"#;

const CONTEXT_AND_VIEW: &str = r#"  /**
   * The script-side context for one host call.
   *
   * It is valid only for the call that produced it: an `await` returns to the
   * host and the frame it names goes away, so a `cx` kept across one reports a
   * stale-context error. Ask for a fresh one with `with_cx` instead.
   */
  export interface Context {
    /**
     * Requests a re-render. Legal from an event handler or a task; calling it
     * during `render` throws, because notifying yourself while rendering is a
     * loop.
     */
    notify(): void;
    phase(): Phase;
  }

  /** The modifier keys held when a click was delivered. */
  export interface Modifiers {
    shift: boolean;
    control: boolean;
    alt: boolean;
    /** Command on macOS, Windows key elsewhere. */
    platform: boolean;
  }

  /** What an `on_click` handler receives. Keyboard activation counts as one. */
  export interface ClickEvent {
    click_count: number;
    modifiers: Modifiers;
  }

  /**
   * Properties handed to `init`.
   *
   * `any` rather than `unknown` because the values come from outside the type
   * system and every use would otherwise need a cast. The host currently
   * constructs a root view with no properties at all, so `init` should treat
   * its argument as absent.
   */
  export type Props = Record<string, any>;

  /**
   * The base class of every view: subclass it and default-export the subclass.
   *
   * `init` runs once when the view is created; `render` runs on every frame
   * and returns exactly one element. Never store an element on the instance —
   * it belongs to the pass that built it.
   */
  export abstract class View {
    constructor(props?: Props);
    init?(props?: Props): void;
    abstract render(cx: Context): Element;
  }

"#;

/// The element methods that are not styles.
///
/// Hand-written because each has a signature of its own; the names match the
/// behavior list the engine installs on the prototype, and
/// [`tests::every_element_method_is_accounted_for`] fails if the two drift.
const ELEMENT_METHODS: &str = r#"    /** Adds one child. The child is consumed; using it again throws. */
    child(child: Element): Element;
    /** Adds several children, in order. */
    children(children: Iterable<Element>): Element;
    /**
     * Applies `branch` only when `condition` is truthy, keeping the chain in
     * one piece. `branch` must return the element.
     */
    when(condition: unknown, branch: (el: Element) => Element): Element;

    /** `handler(event, cx)`, on click and on keyboard activation. */
    on_click(handler: (event: ClickEvent, cx: Context) => void): Element;
    /** `handler(checked, cx)`, on a toggle. The script owns the new value. */
    on_change(handler: (checked: boolean, cx: Context) => void): Element;
    /** Blocks activation and reports the disabled state. Draw it yourself. */
    disabled(value: boolean): Element;
    /** Reports the selected state of a `Button`. */
    selected(value: boolean): Element;
    /** The controlled value of a `Checkbox` or `Switch`. */
    checked(value: boolean): Element;

    /**
     * Styles applied while the pointer is over the element. `declare` receives
     * a detached element that collects the styles; its return value is
     * ignored, so a chain and a block body both work.
     */
    hover(declare: (el: Element) => Element | void): Element;
    /** Styles applied while the element is pressed. */
    active(declare: (el: Element) => Element | void): Element;
    /** Styles applied while the element has focus. */
    focus(declare: (el: Element) => Element | void): Element;
"#;

const CONSTRUCTORS: &str = r#"
  /** An element with no layout of its own. */
  export function div(): Element;
  /** A row. */
  export function h_flex(): Element;
  /** A column. */
  export function v_flex(): Element;
  /** A text element. The value is stringified. */
  export function text(value: string | number | boolean): Element;

  /**
   * A component type: a table with one factory, mirroring `Button::new(id)` on
   * the Rust side. The id identifies the element across renders.
   */
  export interface ComponentType {
    new: (id: string | number) => Element;
  }

  /** Activation, focus, disabled and selected state. No styling. */
  export const Button: ComponentType;
  /** A controlled toggle. No styling: draw the indicator yourself. */
  export const Checkbox: ComponentType;
  /** A controlled switch. No styling. */
  export const Switch: ComponentType;
"#;

const CAPABILITIES: &str = r#"
  /** One entry of `fs.read_dir`. */
  export interface DirEntry {
    name: string;
    is_dir: boolean;
  }

  /**
   * Filesystem access, confined to the roots the manifest grants.
   *
   * These calls are **synchronous** today: they block the thread that renders,
   * and they return a value rather than a promise. §17.1 makes them
   * asynchronous later, and that will change these signatures.
   */
  export interface FileSystem {
    read_text(path: string): string;
    write_text(path: string, contents: string): void;
    /** Sorted by name. */
    read_dir(path: string): DirEntry[];
    /** Throws on a path outside the granted roots, rather than answering false. */
    exists(path: string): boolean;
    /** Not recursive: a directory must be empty. */
    remove(path: string): void;
    create_dir_all(path: string): void;
  }

  /** Key-value storage that survives a restart. Persisted on every write. */
  export interface Store {
    /** `null` when the key is unset. */
    get(key: string): Json;
    set(key: string, value: Json): void;
    remove(key: string): void;
    keys(): string[];
    /** Writes the file now. Synchronous, like the rest of this surface. */
    flush(): void;
  }

  export interface Clipboard {
    /** `undefined` when the clipboard holds no text. */
    read_text(): string | undefined;
    write_text(text: string): void;
  }

  /** Diagnostics. Needs no capability: a script that runs may say something. */
  export interface Log {
    debug(message: unknown, ...rest: unknown[]): void;
    info(message: unknown, ...rest: unknown[]): void;
    warn(message: unknown, ...rest: unknown[]): void;
    error(message: unknown, ...rest: unknown[]): void;
  }

  export interface Process {
    /** Runs a command to completion and answers its exit code. */
    run(command: string, args?: string[]): number;
    /**
     * Asks the host to close this application. A request, not `exit(2)`: the
     * host decides, and the call returns.
     */
    exit(code?: number): void;
  }

  export const fs: FileSystem;
  export const store: Store;
  export const clipboard: Clipboard;
  export const log: Log;
  export const process: Process;
"#;

const SCHEDULING: &str = r#"
  /** A running task. Cancelling one leaves its promise pending for ever. */
  export interface Task {
    cancel(): void;
    is_done(): boolean;
  }

  export interface TaskOptions {
    /**
     * The view the task belongs to: it is cancelled when that view goes away.
     * Defaults to the view that is running. `null` outlives every view — and
     * is the only value other than the current view the runtime accepts today.
     */
    owner?: View | null;
  }

  /** Resolves after `ms` on GPUI's foreground executor. */
  export function sleep(ms?: number): Promise<void>;

  /**
   * Runs `body` with a context that belongs to the current host call.
   *
   * This is how code resumed after an `await` obtains a usable `cx`: the one
   * its function was called with names a frame that has already returned.
   */
  export function with_cx<T>(body: (cx: Context) => T): T;

  /**
   * Calls `body(cx)` and adopts the promise it returns, so a rejection is
   * reported rather than swallowed.
   *
   * `cx` is valid until the first `await`, and is absent when there is no host
   * call in progress — at module top level, for instance, where it is
   * `undefined` despite what this signature can say.
   */
  export function spawn(body: (cx: Context) => unknown, opts?: TaskOptions): Task;

  export interface Timer {
    /** Calls `handler(cx)` once, after `ms`. */
    after(ms: number, handler: (cx: Context) => void, opts?: TaskOptions): Task;
    /**
     * Calls `handler(cx)` every `ms`. The interval is measured from the end of
     * one call, so a slow handler delays the next tick instead of stacking.
     */
    every(ms: number, handler: (cx: Context) => void, opts?: TaskOptions): Task;
  }

  export const timer: Timer;
"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// The element methods that are not style methods, so a test can subtract
    /// them from the interface and compare what is left against the style
    /// table. Mirrors the names bound in the engine's `apply` and prelude.
    const NON_STYLE_METHODS: &[&str] = &[
        "child",
        "children",
        "when",
        "on_click",
        "on_change",
        "disabled",
        "selected",
        "checked",
        "hover",
        "active",
        "focus",
    ];

    /// Every method name declared in the `Element` interface, in order.
    fn element_methods(declarations: &str) -> Vec<String> {
        declarations
            .lines()
            .skip_while(|line| !line.starts_with("  export interface Element {"))
            .skip(1)
            .take_while(|line| !line.starts_with("  }"))
            .filter_map(|line| {
                let line = line.trim_start();
                let name = line.split('(').next()?;
                (!name.is_empty()
                    && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    && line.len() > name.len())
                .then(|| name.to_owned())
            })
            .collect()
    }

    #[test]
    fn a_reflected_style_is_declared_with_no_arguments() {
        let declarations = declarations();
        assert!(declarations.contains("\n    items_center(): Element;\n"));
        assert!(declarations.contains("\n    flex_col(): Element;\n"));
        // Reflection misses the macro-generated font weights; the runtime adds
        // them back, and so must the declarations.
        assert!(declarations.contains("\n    font_semibold(): Element;\n"));
    }

    #[test]
    fn a_parametric_style_is_declared_with_the_type_the_runtime_enforces() {
        let declarations = declarations();
        for expected in [
            "    bg(value: Color): Element;",
            "    border_color(value: Color): Element;",
            "    w(value: Length): Element;",
            "    p(value: DefiniteLength): Element;",
            "    gap(value: DefiniteLength): Element;",
            "    rounded(value: AbsoluteLength): Element;",
            "    text_size(value: AbsoluteLength): Element;",
            "    opacity(value: number): Element;",
            "    flex_grow(value: number): Element;",
        ] {
            assert!(declarations.contains(expected), "missing: {expected}");
        }
    }

    #[test]
    fn every_parametric_style_is_classified() {
        let (_, parametric) = style_methods();
        assert!(!parametric.is_empty());
        for name in parametric {
            assert_ne!(
                argument_of(name),
                Argument::Unrecognized,
                "`{name}` takes an argument the probe does not recognize; give it a \
                 literal in `argument_of` before the declarations claim it accepts nothing"
            );
        }
    }

    #[test]
    fn every_color_token_is_in_the_color_union() {
        let declarations = declarations();
        for name in color_token_names() {
            assert!(
                declarations.contains(&format!("    | \"{name}\"\n")),
                "`{name}` is missing from ColorToken"
            );
        }
        assert!(declarations.contains("export type Color = ColorToken | `#${string}`;"));
    }

    #[test]
    fn no_internal_name_leaks_into_the_surface() {
        let declarations = declarations();
        for internal in ["__id", "__apply", "__state", "__gpui", "__styleNames"] {
            assert!(
                !declarations.contains(internal),
                "`{internal}` is engine plumbing and must not be declared"
            );
        }
        assert!(!declarations.contains("__"));
    }

    #[test]
    fn the_output_is_structurally_balanced() {
        let declarations = declarations();
        let opened = declarations.matches('{').count();
        let closed = declarations.matches('}').count();
        assert_eq!(opened, closed, "unbalanced braces");

        for method in element_methods(&declarations) {
            assert!(!method.is_empty(), "a method line has no name");
        }
        assert!(declarations.ends_with("}\n"));
        assert!(declarations.contains("declare module \"gpui\" {"));
    }

    #[test]
    fn every_element_method_is_accounted_for() {
        let declared = element_methods(&declarations());
        let styles: Vec<&String> = declared
            .iter()
            .filter(|name| !NON_STYLE_METHODS.contains(&name.as_str()))
            .collect();

        assert_eq!(
            styles.len(),
            style::known_names().len(),
            "the declared style methods and the runtime's style table have diverged"
        );
        assert_eq!(
            declared.len(),
            styles.len() + NON_STYLE_METHODS.len(),
            "an element method is declared that this test does not know about"
        );

        let mut sorted = styles.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            styles.len(),
            "a style method is declared twice"
        );
    }

    #[test]
    fn no_style_method_collides_with_an_element_method() {
        // A collision would emit the same member twice and make the whole file
        // invalid TypeScript, so it has to fail here rather than in an editor.
        for name in style::known_names() {
            assert!(
                !NON_STYLE_METHODS.contains(&name),
                "`{name}` is both a style method and an element method"
            );
        }
    }

    #[test]
    fn every_style_name_is_a_valid_identifier() {
        for name in style::known_names() {
            let mut chars = name.chars();
            assert!(
                chars
                    .next()
                    .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
                    && chars.all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "`{name}` cannot be written as a TypeScript member name"
            );
        }
    }

    #[test]
    fn write_to_creates_the_file_beside_an_application() {
        let directory =
            std::env::temp_dir().join(format!("gpui-shell-typings-{}", std::process::id()));
        let path = write_to(&directory).expect("declarations are writable");

        assert_eq!(path.file_name().unwrap(), FILE_NAME);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), declarations());

        let _ = std::fs::remove_dir_all(&directory);
    }
}
