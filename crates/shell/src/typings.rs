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
//! * Their documentation is GPUI's own, carried on the same reflection entries,
//!   so hovering `.items_center()` shows the sentence upstream wrote rather than
//!   one transcribed here. The seventy-odd methods bound by hand — reflection
//!   reaches no-argument methods only — carry a description written beside the
//!   name in [`style`], which is where a description that stops matching shows
//!   up in the same diff as the change.
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
//!
//! # What an application adds
//!
//! Host-registered native modules cannot be generated here, because only the
//! host knows what it registered. The declarations leave a `NativeModules`
//! interface for an application to augment, which is what turns `native("...")`
//! from `Record<string, (...args: any[]) => any>` into a checked name with
//! completing functions. `crates/story/js/quotes/market.d.ts` is the worked
//! example.

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
    out.push_str(&PREAMBLE.replace("{version}", crate::plugin_api::VERSION));
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

/// Refreshes the declarations in every directory of an application that imports
/// the `gpui` module.
///
/// One file at the root is enough for an editor that has the whole application
/// open, and not enough for anything else: a subdirectory opened on its own, a
/// tool pointed at one file, a script vendored elsewhere. Since the file is
/// generated and ignored rather than committed, a copy per directory costs
/// nothing anybody has to look at.
///
/// Failures are collected rather than raised. A directory that cannot be written
/// is a worse editing experience, not a reason to refuse to run the application.
pub fn refresh_tree(root: &Path) -> Vec<PathBuf> {
    directories_importing_gpui(root)
        .into_iter()
        .filter_map(|directory| match refresh(&directory) {
            Ok(written) => written,
            Err(error) => {
                tracing::debug!("could not write {}: {error}", directory.display());
                None
            }
        })
        .collect()
}

/// Directories holding at least one script that imports `gpui`.
///
/// Bounded the way the source watcher is bounded, and for the same reason: an
/// application directory is whatever someone pointed the runtime at, and a
/// symlink farm or a vendored tree must not turn a startup step into an
/// unbounded walk.
fn directories_importing_gpui(root: &Path) -> Vec<PathBuf> {
    const MAX_DEPTH: usize = 8;
    const MAX_FILES: usize = 4_096;
    const SKIPPED: [&str; 2] = ["node_modules", "target"];

    let mut found = Vec::new();
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    let mut seen = 0usize;

    while let Some((directory, depth)) = pending.pop() {
        let mut imports = false;

        for entry in std::fs::read_dir(&directory)
            .into_iter()
            .flatten()
            .flatten()
        {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();

            if name.starts_with('.') || SKIPPED.contains(&name.as_ref()) {
                continue;
            }

            if path.is_dir() {
                if depth < MAX_DEPTH {
                    pending.push((path, depth + 1));
                }
                continue;
            }

            seen += 1;
            if seen > MAX_FILES {
                return found;
            }

            if !imports
                && matches!(
                    path.extension().and_then(|extension| extension.to_str()),
                    Some("js" | "mjs")
                )
                && std::fs::read_to_string(&path).is_ok_and(|source| imports_gpui(&source))
            {
                imports = true;
            }
        }

        if imports {
            found.push(directory);
        }
    }

    found
}

/// Whether a script imports the built-in module.
///
/// Matching the quoted specifier rather than the bare word, so a file that only
/// mentions gpui in a comment or a string does not collect a copy it has no use
/// for.
fn imports_gpui(source: &str) -> bool {
    ["\"gpui\"", "'gpui'"]
        .iter()
        .any(|specifier| source.contains(specifier))
}

/// Rewrites the declarations beside an application when they are not current.
///
/// This is what a host should call in a development build. An application never
/// has to remember to regenerate anything, and cannot end up editing against a
/// runtime it is not running: the process that will execute the script is the
/// one that describes it.
///
/// Nothing is written when the file already matches, so an editor watching the
/// directory is not woken on every launch, and a read-only checkout is not an
/// error worth reporting. Returns the path only when it actually wrote.
pub fn refresh(directory: &Path) -> std::io::Result<Option<PathBuf>> {
    let path = directory.join(FILE_NAME);
    let current = declarations();

    if std::fs::read_to_string(&path).is_ok_and(|committed| committed == current) {
        return Ok(None);
    }

    std::fs::write(&path, current)?;
    Ok(Some(path))
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
        out.push_str(&doc_comment(style::documentation(name), 4));
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
         // table. A name here is a name the runtime dispatches, and the\n    \
         // documentation is GPUI's own.",
        names.len()
    );
    for name in names {
        out.push_str(&doc_comment(style::documentation(name), 4));
        let _ = writeln!(out, "    {name}(): Element;");
    }
    out
}

/// Renders a Rust doc comment as a JSDoc block at `indent` spaces.
///
/// The text comes from GPUI's reflection table rather than from anything
/// written here, so it arrives as whatever upstream wrote: usually one sentence
/// and a link to the Tailwind page the method is modelled on. A single line
/// stays on one line, because six hundred four-line blocks would bury the
/// surface they are describing.
///
/// Nothing is emitted for a method the table has no documentation for — the
/// parametric styles and the handful named by hand — because inventing a
/// sentence is how generated declarations start disagreeing with the runtime.
fn doc_comment(documentation: Option<&str>, indent: usize) -> String {
    let Some(text) = documentation else {
        return String::new();
    };

    let pad = " ".repeat(indent);
    // A doc that closed the comment early would take the rest of the file with
    // it. Upstream has none today; this costs one scan to keep it that way.
    let text = text.replace("*/", "*\u{200b}/");
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());

    let Some(first) = lines.next() else {
        return String::new();
    };
    let rest: Vec<&str> = lines.collect();

    if rest.is_empty() {
        return format!("{pad}/** {} */\n", first.trim());
    }

    let mut out = format!("{pad}/**\n{pad} * {}\n", first.trim());
    for line in rest {
        let _ = writeln!(out, "{pad} *\n{pad} * {}", line.trim());
    }
    let _ = writeln!(out, "{pad} */");
    out
}

const PREAMBLE: &str = "\
// Auto-generated — add `gpui.d.ts` to your .gitignore.
//
// The built-in `gpui` module, as TypeScript declarations, for script API
// {version}. Do not edit: gpui-shell rewrites this on every run, in every
// directory that imports the module, from the runtime that is about to execute
// the script. A committed copy could only ever be the stale one.
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

  /**
   * Opens a dialog on the window's root, and answers the stack's new depth.
   *
   * Takes a **function returning an element**, not an element: an element
   * belongs to the render pass that built it, and a dialog outlives the call
   * that opened it. The function runs when the dialog draws, and again whenever
   * it redraws. Whatever it closes over is the dialog's state.
   *
   * An overlay is window-level rather than view-level, which is why it is not on
   * `Context`: `cx.notify()` re-renders one view, this changes what the user is
   * looking at. Legal from an event handler or a task, not from `render`.
   */
  export function open_dialog(content: () => Element, options?: DialogOptions): number;
  /** Closes the topmost dialog, and answers whether it found one. */
  export function close_dialog(): boolean;
  /** Closes every dialog, and answers how many it closed. */
  export function close_all_dialogs(): number;
  /** Whether any dialog is open. Legal from `render`, unlike the rest. */
  export function has_active_dialog(): boolean;

  /**
   * Opens the sheet on the right, replacing whatever was there. At most one is
   * ever open.
   */
  export function open_sheet(content: () => Element): void;
  /** The same, anchored to the side you name. */
  export function open_sheet_at(side: SheetSide, content: () => Element): void;
  /** Closes the sheet, and answers whether one was open. */
  export function close_sheet(): boolean;
  /** Whether the sheet is open. Legal from `render`, unlike the rest. */
  export function has_active_sheet(): boolean;

  /** Posts a toast, and answers its id — the generated one when none was given. */
  export function push_toast(options: ToastOptions): string;
  /** Retracts one toast by id, and answers whether it was still showing. */
  export function remove_toast(id: string): boolean;
  /** Retracts every toast, and answers how many it retracted. */
  export function clear_toasts(): number;


  /** Which edge the sheet is anchored to. */
  export type SheetSide = "left" | "right" | "top" | "bottom";

  export interface DialogOptions {
    /** Whether Escape closes it. Default `true`. */
    escape_dismissable?: boolean;
    /** Whether pressing the backdrop closes it. Default `true`. */
    backdrop_dismissable?: boolean;
  }

  export interface ToastOptions {
    /** The sentence the user reads. */
    title: string;
    /** A second line. */
    description?: string;
    /** Default `info`. */
    level?: "info" | "success" | "warning" | "error";
    /**
     * Milliseconds, or `null` to stay until dismissed. Omitting it keeps the
     * five-second default, which is why `null` and absent are not the same.
     */
    timeout?: number | null;
    /**
     * Identity, for replacing and dismissing. A repeated failure posted under
     * one id is a standing message rather than a pile.
     */
    id?: string;
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
   * `init` runs once when the view is created. `render` returns exactly one
   * element, and runs when the view is invalidated — by `cx.notify()`, a
   * reload, or a theme change — not on every frame. Never store an element on
   * the instance: it belongs to the render that built it.
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
     * What a screen reader announces. An icon-only control has no text of its
     * own and announces nothing without it.
     */
    accessibility_label(description: string): Element;
    /**
     * A stable name for this element, used as its identity.
     *
     * Without one, an element is identified by where it sits in the tree the
     * render built — which shifts the moment a conditional child appears above
     * it, taking the pressed state, the focus and anything else keyed by
     * identity with it. Name anything whose identity has to survive that.
     *
     * `Button`, `Checkbox` and `Switch` take their identity from `new(id)` and
     * ignore this.
     */
    id(name: string): Element;

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

  /**
   * A vector image from the application's own directory.
   *
   * The path resolves against the application root — the directory passed to
   * gpui-shell — not against the file that asked for it, the way a web
   * application's public directory works. It inherits the surrounding text
   * color unless it sets its own.
   */
  export function svg(path: string): Element;

  /**
   * Retained text state, created once and kept on the view.
   *
   * `InputState.new(...)` needs a live host call, so it belongs in `init` or in
   * an event handler — never in `render`.
   */
  export interface InputStateHandle {
    value(): string;
    set_value(next: string): void;
    /** `change`, `submit`, `focus` or `blur`. */
    on(event: "change" | "submit" | "focus" | "blur", handler: (event: any, cx: Context) => void): boolean;
    release(): boolean;
  }

  export interface InputStateType {
    new: (options?: { placeholder?: string; value?: string }) => InputStateHandle;
  }

  export const InputState: InputStateType;

  export interface InputType {
    new: (state: InputStateHandle) => Element;
  }

  /** The frame around retained text state. */
  export const Input: InputType;

  /** The theme the host installed. Read-only. */
  export interface Theme {
    colors: Record<ColorToken, string>;
    spacing: Record<string, number>;
    radius: Record<string, number>;
    mode: "light" | "dark";
    is_dark: boolean;
  }

  export function theme(): Theme;
  /** Switches palette. Returns whether anything changed. */
  export function set_theme(mode: "light" | "dark"): boolean;
  /**
   * States the script API version this application needs, at the first line,
   * where a mismatch is still cheap. Throws when the runtime cannot satisfy it.
   */
  export function require_api(version: string): string;
  /**
   * The native modules this host registered, declared by the application.
   *
   * Empty here, because only the host knows what it granted. An application
   * describes its own in a `.d.ts` beside its source, and from then on
   * `native("...")` is typed — the module name is checked, and its functions
   * complete:
   *
   * ```ts
   * declare module "gpui" {
   *   interface NativeModules {
   *     market: {
   *       quotes(): { symbol: string; last: string }[];
   *       watch(symbol: string): boolean;
   *     };
   *   }
   * }
   * ```
   *
   * Declaring nothing costs nothing: with no entries the untyped overload
   * below still applies, so an application that never writes one keeps working
   * exactly as before.
   */
  export interface NativeModules {}

  /**
   * A module the host registered in Rust. Throws when no such module exists,
   * naming the ones that do.
   */
  export function native<Name extends keyof NativeModules & string>(
    module: Name,
  ): NativeModules[Name];
  export function native(module: string): Record<string, (...args: any[]) => any>;
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
   * Every call returns a promise: the syscall runs off the main thread, because
   * a disk has no bound on how long it takes and blocking here would stop the
   * frame and the VM together.
   *
   * A **denial still throws at the call site** rather than rejecting, because
   * the capability check costs nothing and a rejected promise nobody awaited is
   * a denial nobody sees.
   */
  export interface FileSystem {
    /** Refuses a file over 64 MiB, naming it and the limit. */
    read_text(path: string): Promise<string>;
    write_text(path: string, contents: string): Promise<void>;
    /** Sorted by name. */
    read_dir(path: string): Promise<DirEntry[]>;
    /** Throws on a path outside the granted roots, rather than answering false. */
    exists(path: string): Promise<boolean>;
    /** Not recursive: a directory must be empty. */
    remove(path: string): Promise<void>;
    create_dir_all(path: string): Promise<void>;
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
        "accessibility_label",
        "id",
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

    /// Refreshing writes once and then leaves the file alone.
    ///
    /// The second half is what makes this safe to run on every launch: an editor
    /// watching the directory is not woken, and a checkout whose files are
    /// read-only is not an error nobody can act on.
    #[test]
    fn refresh_writes_once_and_then_says_nothing() {
        let directory =
            std::env::temp_dir().join(format!("gpui-shell-refresh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a temporary directory");

        let written = refresh(&directory).expect("the first refresh");
        assert_eq!(
            written.as_deref(),
            Some(directory.join(FILE_NAME).as_path())
        );
        assert_eq!(
            std::fs::read_to_string(directory.join(FILE_NAME)).expect("the file"),
            declarations()
        );

        assert_eq!(
            refresh(&directory).expect("the second refresh"),
            None,
            "an up-to-date file must not be rewritten"
        );

        // A stale one is replaced, which is the case this exists for.
        std::fs::write(directory.join(FILE_NAME), "// from an older runtime\n")
            .expect("overwriting");
        assert!(refresh(&directory).expect("the third refresh").is_some());

        let _ = std::fs::remove_dir_all(&directory);
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
