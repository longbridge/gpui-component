// Auto-generated — add `gpui-kit.d.ts` to your .gitignore.
//
// The built-in modules, as TypeScript declarations, for gpui-shell 0.1.0.
// Do not edit: gpui-shell rewrites this on every run, in every directory that
// imports one of them, from the runtime that is about to execute the script. A
// committed copy could only ever be the stale one.
//
// Each built-in module names the public Rust layer it exposes, so an import
// says which layer a script depends on. "gpui" also carries the shell bridge:
//
//   "gpui"       GPUI's own elements, plus what this runtime adds: views,
//                the style surface, the window, storage, scheduling.
//   "gpui-base"  gpui-base's layout helpers, components and theme.
//   "gpui-fps"   gpui-fps's performance overlay.
//
// A name belongs to exactly one of them. Nothing is re-exported for
// convenience: a name reachable from two specifiers stops saying where it came
// from.
//
// The style surface here is generated from the same tables the runtime
// dispatches through, so a style method that type-checks exists at run time,
// and a length or color the compiler refuses is one the runtime would refuse
// too. Put `// @ts-check` at the top of a script to have an editor check it.
//
// What is not expressed: capability grants (a denied `fs.readFile` still
// type-checks), element and `cx` lifetimes (both belong to one call), and
// which component a method suits (all elements share one prototype).

declare module "gpui-kit" {
  /**
   * A length. A bare number is pixels; a string carries its unit.
   *
   * `"auto"` is only accepted where the Rust signature takes `Length` — the
   * padding, gap, border and radius families take the narrower types below.
   */
  export type Length = number | import("gpui-shell").LengthString | "auto";

  /** A length that must resolve to a size: pixels, rems or a percentage. */
  export type DefiniteLength = number | import("gpui-shell").LengthString;

  /** A length with no percentage and no `"auto"`: pixels or rems. */
  export type AbsoluteLength = number | `${number}px` | `${number}rem`;

  /** A layout axis, mirroring `gpui::Axis`. */
  export type Axis = "horizontal" | "vertical";

  /**
   * What a HostModule function takes and answers.
   *
   * Named after the Rust type it mirrors, `HostValue`, rather than after the
   * shape it happens to have: `Json` would sit one capital letter away from the
   * built-in `JSON` object and mean something entirely different — a value,
   * not a parser.
   */
  export type HostValue =
    | null
    | boolean
    | number
    | string
    | HostValue[]
    | { [key: string]: HostValue };

  /**
   * A color: a semantic token name, or a `#rgb`, `#rrggbb` or `#rrggbbaa`
   * literal. Prefer a token; a literal bypasses the theme, and a theme
   * switch will not reach it.
   *
   * The union is closed, so a mistyped token is a compile error. A token
   * name that reaches a call through a variable widens to `string` and
   * has to say what it is:
   *
   *     /** @type {{ bg: import("gpui-kit").Color }} *\/
   *     const palette = tone === "blocking" ? ... : ...;
   */
  export type Color = import("gpui-base").ColorToken | `#${string}`;

  /**
   * An accessibility role, mirroring `gpui::Role` in snake_case.
   *
   * `generic_container` is deliberately absent: GPUI filters that role
   * out of the accessibility tree, so an element carrying it announces
   * nothing while looking as though it announced something.
   */
  export type Role =
    | "unknown"
    | "text_run"
    | "cell"
    | "label"
    | "image"
    | "link"
    | "row"
    | "list_item"
    | "list_marker"
    | "tree_item"
    | "list_box_option"
    | "menu_item"
    | "menu_list_option"
    | "paragraph"
    | "check_box"
    | "radio_button"
    | "text_input"
    | "button"
    | "default_button"
    | "pane"
    | "row_header"
    | "column_header"
    | "row_group"
    | "list"
    | "table"
    | "layout_table_cell"
    | "layout_table_row"
    | "layout_table"
    | "switch"
    | "menu"
    | "multiline_text_input"
    | "search_input"
    | "date_input"
    | "date_time_input"
    | "week_input"
    | "month_input"
    | "time_input"
    | "email_input"
    | "number_input"
    | "password_input"
    | "phone_number_input"
    | "url_input"
    | "abbr"
    | "alert"
    | "alert_dialog"
    | "application"
    | "article"
    | "audio"
    | "banner"
    | "blockquote"
    | "canvas"
    | "caption"
    | "caret"
    | "code"
    | "color_well"
    | "combo_box"
    | "editable_combo_box"
    | "complementary"
    | "comment"
    | "content_deletion"
    | "content_insertion"
    | "content_info"
    | "definition"
    | "description_list"
    | "details"
    | "dialog"
    | "disclosure_triangle"
    | "document"
    | "embedded_object"
    | "emphasis"
    | "feed"
    | "figure_caption"
    | "figure"
    | "footer"
    | "form"
    | "grid"
    | "grid_cell"
    | "group"
    | "header"
    | "heading"
    | "iframe"
    | "iframe_presentational"
    | "ime_candidate"
    | "keyboard"
    | "legend"
    | "line_break"
    | "list_box"
    | "log"
    | "main"
    | "mark"
    | "marquee"
    | "math"
    | "menu_bar"
    | "menu_item_check_box"
    | "menu_item_radio"
    | "menu_list_popup"
    | "meter"
    | "navigation"
    | "note"
    | "plugin_object"
    | "progress_indicator"
    | "radio_group"
    | "region"
    | "root_web_area"
    | "ruby"
    | "ruby_annotation"
    | "scroll_bar"
    | "scroll_view"
    | "search"
    | "section"
    | "section_footer"
    | "section_header"
    | "slider"
    | "spin_button"
    | "splitter"
    | "status"
    | "strong"
    | "suggestion"
    | "svg_root"
    | "tab"
    | "tab_list"
    | "tab_panel"
    | "term"
    | "time"
    | "timer"
    | "title_bar"
    | "toolbar"
    | "tooltip"
    | "tree"
    | "tree_grid"
    | "video"
    | "web_view"
    | "window"
    | "pdf_actionable_highlight"
    | "pdf_root"
    | "graphics_document"
    | "graphics_object"
    | "graphics_symbol"
    | "doc_abstract"
    | "doc_acknowledgements"
    | "doc_afterword"
    | "doc_appendix"
    | "doc_back_link"
    | "doc_biblio_entry"
    | "doc_bibliography"
    | "doc_biblio_ref"
    | "doc_chapter"
    | "doc_colophon"
    | "doc_conclusion"
    | "doc_cover"
    | "doc_credit"
    | "doc_credits"
    | "doc_dedication"
    | "doc_endnote"
    | "doc_endnotes"
    | "doc_epigraph"
    | "doc_epilogue"
    | "doc_errata"
    | "doc_example"
    | "doc_footnote"
    | "doc_foreword"
    | "doc_glossary"
    | "doc_gloss_ref"
    | "doc_index"
    | "doc_introduction"
    | "doc_note_ref"
    | "doc_notice"
    | "doc_page_break"
    | "doc_page_footer"
    | "doc_page_header"
    | "doc_page_list"
    | "doc_part"
    | "doc_preface"
    | "doc_prologue"
    | "doc_pullquote"
    | "doc_qna"
    | "doc_subtitle"
    | "doc_tip"
    | "doc_toc"
    | "list_grid"
    | "terminal"
    ;

  /**
   * Which corner of an anchored surface is pinned to its trigger,
   * mirroring `gpui::Anchor` in snake_case.
   */
  export type Anchor =
    | "top_left"
    | "top_right"
    | "bottom_left"
    | "bottom_right"
    | "top_center"
    | "bottom_center"
    | "left_center"
    | "right_center"
    ;

  /** Which pointer button opens a `Popover`. */
  export type MouseButton = "left" | "right" | "middle";

  /**
   * The script-side context for one host call.
   *
   * It is valid only for the call that produced it: an `await` returns to the
   * host and the frame it names goes away, so a `cx` kept across one reports a
   * stale-context error. Work that outlives the call takes an [`AsyncContext`]
   * instead — `cx.spawn`, `cx.timer`, `init`.
   */
  export interface Context {
    /**
     * Requests a re-render. Legal from an event handler or a task; calling it
     * during `render` throws, because notifying yourself while rendering is a
     * loop.
     */
    notify(): void;
    /**
     * `App::bind_keys`. Installs key bindings and answers how many.
     *
     * The keymap is the application's, not a window's or a view's, so a chord
     * bound here is live wherever its `context` predicate matches. The whole
     * list is validated before any of it is installed: a keymap half applied
     * because one entry had a typo is a worse state than one not applied, and
     * the script cannot see which half made it.
     *
     * Illegal from `render`.
     */
    bind_keys(bindings: KeyBinding[]): number;
    /**
     * `App::stop_propagation`. Stops this event reaching the handlers above
     * this element.
     *
     * GPUI delivers an event to every handler on the path, so a row inside a
     * list with its own `on_click` fires both. Call this from the inner one to
     * keep the event there.
     */
    stop_propagation(): void;
    /**
     * `App::propagate`. Undoes a `stop_propagation()` made earlier in the same
     * dispatch, letting the event continue.
     */
    propagate(): void;
    phase(): import("gpui-shell").ScopePhase;
    /** Reads the current `gpui_base::Theme` semantic token projection. */
    theme(): import("gpui-base").Theme;

    /**
     * Hands a URL to whatever the system opens URLs with. `App::open_url`.
     *
     * `Link`'s `href` without the element, for the case where the address is
     * not known until something has already happened — the end of a device
     * authorization, say, where waiting for a second click to open the page the
     * first click just asked for is a step nobody wanted.
     *
     * It takes an absolute `http`/`https` URL with a host and refuses
     * everything else. That guard is not about the address bar: without it this
     * is a way to hand an arbitrary URI to whichever handler the desktop
     * registered for its scheme.
     */
    open_url(url: string): void;

    /** `App::read_from_clipboard`. `undefined` when it holds no text. */
    read_from_clipboard(): string | undefined;
    /** `App::write_to_clipboard`. */
    write_to_clipboard(text: string): void;

    /**
     * A focus target the script owns, created once and kept on the view.
     * `App::focus_handle` — GPUI has no `FocusHandle::new`, and neither does
     * this.
     *
     * Focus is a fact about the window that outlives any one render, so an
     * element rebuilt every frame cannot own it. Hand the handle to an element
     * with `track_focus(...)`, and it is that element the keyboard means.
     *
     * It would produce a fresh handle on every frame, so it belongs in `init`
     * or in an event handler — never in `render`.
     */
    focus_handle(): FocusHandle;

    /**
     * Creates a retained nested view and hands back the entity that owns it.
     * `AppContext::new` — the only way GPUI makes one, and so the only way here.
     *
     * The entity is a child wherever a child is taken: `.child(entity)`, or
     * returned from `render`. Updating its props runs the child's optional
     * `update(props)` and rebuilds only that child.
     *
     * Legal from `init`, an event handler or a task; creating one during
     * `render` or layout throws.
     */
    new(Class: ViewClass, props?: import("gpui-shell").Props): Entity;

    /**
     * Calls `body(cx)` and adopts the promise it returns, so a rejection is
     * reported rather than swallowed. `App::spawn`.
     *
     * The `cx` the body receives is an [`AsyncContext`] — valid across `await`,
     * the way GPUI's `AsyncApp` is — so the whole body can keep using it.
     */
    spawn(body: (cx: AsyncContext) => unknown, opts?: import("gpui-shell").TaskOptions): Task;

    /** Resolves after `ms` on GPUI's foreground executor. */
    sleep(ms?: number): Promise<void>;

    /** One-shot and repeating callbacks on the foreground executor. */
    readonly timer: Timer;
  }

  /**
   * A context that may be held across an `await`.
   *
   * The mirror of GPUI's `AsyncApp`. An ordinary [`Context`] speaks for one
   * host call and reports clearly once that call has returned — which is what
   * catches a `cx` stashed in a closure. This one names no call at all: it
   * resolves whichever is running when a member is used, and refuses only when
   * none is.
   *
   * It is what `init` receives, and what `cx.spawn` and `cx.timer` hand their
   * callbacks — the three places whose whole job is to set up or continue work
   * that outlives the call they were started from.
   */
  export interface AsyncContext extends Context {}



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
   * What an `on_key_down` or `on_key_up` handler receives.
   *
   * `keystroke` is the whole chord in the spelling a key binding is written
   * in — `"cmd-shift-s"`, `"escape"`, `"ctrl-alt-delete"` — and is what a
   * comparison is normally written against. `key` and `modifiers` are the
   * same thing taken apart, for when only one half matters.
   *
   * The platform modifier is spelled `cmd` on every platform, including Linux
   * and Windows. GPUI spells it for the platform it was built for, which is
   * right for a keymap a person reads and wrong for a string a program
   * compares: one script runs on all three, and `event.keystroke === "cmd-s"`
   * has to mean the same thing in all three. It is also the spelling
   * `cx.bind_keys` accepts everywhere, so a binding and the event it produces
   * agree by construction.
   */
  export interface KeyEvent {
    /** The key printed on the key that was pressed, e.g. `"s"` or `"escape"`. */
    key: string;
    /** The full chord, as GPUI's `Keystroke::unparse` spells it. */
    keystroke: string;
    /** The character this keystroke would type, when it types one. */
    key_char?: string;
    modifiers: Modifiers;
    /** Whether the key is being held down. Absent on `on_key_up`. */
    is_held?: boolean;
  }

  export interface Point { x: number; y: number; }
  export interface Size { width: number; height: number; }
  /** GPUI mouse coordinates. `position` is window-relative; `local_position` is element-relative. */
  export interface MouseMoveEvent {
    position: Point;
    local_position: Point;
    bounds: import("gpui-shell").ElementBounds;
    modifiers: Modifiers;
  }

  /**
   * What an `on_mouse_down`, `on_mouse_up` or `on_mouse_down_out` handler
   * receives.
   *
   * `local_position` and `bounds` are absent when the element has not been
   * painted yet, and on an `on_mouse_down_out` press they describe an element
   * the pointer is outside of — so `local_position` there is negative, or past
   * the far edge, which is exactly what says which way.
   */
  export interface MouseButtonEvent {
    button: MouseButton;
    /** How many presses in the current sequence; `2` on a double-click. */
    click_count: number;
    position: Point;
    local_position?: Point;
    bounds?: import("gpui-shell").ElementBounds;
    modifiers: Modifiers;
  }

  /** What an `on_action` handler receives. */
  export interface ActionEvent {
    /** The action's name, as the script bound and registered it. */
    action: string;
  }

  /** One entry of `cx.bind_keys`. */
  export interface KeyBinding {
    /** The chord, e.g. `"cmd-s"`, or a sequence: `"ctrl-k ctrl-s"`. */
    keystroke: string;
    /** The action this chord dispatches. */
    action: string;
    /**
     * Where it applies, as a key-context predicate matched against the
     * `key_context(...)` an element declares — `"Editor"`, `"Pane && !modal"`.
     * Omitted, the binding is global.
     */
    context?: string;
  }

  /** What an `on_scroll_wheel` handler receives. */
  export interface ScrollWheelEvent {
    /** The scroll distance in pixels, whichever unit the device reported. */
    delta: Point;
    /** The same distance in lines, when the device reported lines. */
    delta_lines?: Point;
    touch_phase: "started" | "moved" | "ended" | "cancelled";
    position: Point;
    local_position?: Point;
    bounds?: import("gpui-shell").ElementBounds;
    modifiers: Modifiers;
  }

  /**
   * The base class of every view: subclass it and default-export the subclass.
   *
   * `init` runs once when the view is created. `render` returns one element,
   * retained entity or string, and runs when the view is invalidated — by
   * `cx.notify()`, a reload, or a theme change — not on every frame. Never
   * store an element on the instance: it belongs to the render that built it.
   */
  export abstract class View {
    constructor(props?: import("gpui-shell").Props);
    /**
     * Runs once when the view is created.
     *
     * `cx` is an [`AsyncContext`], because this is where retained things are
     * made — tasks, timers, focus handles — and the context that starts a task
     * is the one its body will still be using after an `await`.
     */
    init?(props: import("gpui-shell").Props | undefined, cx: AsyncContext): void;
    /** Runs when a parent changes this retained nested view's properties. */
    update?(props: import("gpui-shell").Props | undefined): void;
    abstract render(cx: Context): Element | Entity | string;
  }

  /** A concrete script view class that can be retained as a nested view. */
  export type ViewClass = new (props?: import("gpui-shell").Props) => View;

  /**
   * Retained ownership of one nested `View` entity.
   *
   * Create it once from `init`, an event handler or a task. Updating props
   * invokes the child's optional `update(props)` and rebuilds only that child.
   * Phase, class and released-handle validation errors throw synchronously. Native
   * construction/init/update is applied before the enclosing host entry returns;
   * failures are reported by that host entry rather than being catchable around
   * this synchronous-looking call.
   *
   * A failed `update` has a bounded shell rollback:
   * - ordinary reachable properties, including callable objects, are restored only while their post-update descriptors remain legally redefinable or deletable;
   * - shell-owned entities, tasks and nested views newly created by the update are released.
   * Unsupported mutations include JavaScript private fields and internal slots;
   * newly added non-configurable properties; making an existing configurable property non-configurable;
   * and pre-existing native handles explicitly released by update.
   */
  export interface Entity {
    set_props(props?: import("gpui-shell").Props): void;
    release(): boolean;
  }


  /**
   * A description of one element, built by chaining.
   *
   * Every method returns the same element, so a chain is one
   * expression. An element is consumed when it is used as a child and
   * belongs to the render pass that built it; storing one and using it
   * again throws, which no type can prevent.
   */
  export interface Element {
    /**
     * Passes this element to `transform` and returns exactly what it returns.
     *
     * This mirrors GPUI's `FluentBuilder.map`: it is useful for keeping an
     * imperative or conditional transformation inside a fluent expression.
     */
    map<T>(transform: (element: Element) => T): T;
    /**
     * Adds one child. The child is consumed; using it again throws.
     *
     * A **string is an element**, exactly as `&str`, `String` and
     * `SharedString` implement `IntoElement` in GPUI: `.child("hello")` is how
     * text is written, and the style comes from the element holding it.
     *
     * An `Entity` from `cx.new(...)` is a child too, the way an `Entity<V>` is
     * renderable in GPUI — that is how a retained nested view is mounted. One
     * entity may appear once per parent snapshot; a second mount in the same
     * description is refused before any of it is published.
     */
    child(child: Element | Entity | string | number | boolean): Element;
    /** Adds several children, in order. */
    children(children: Iterable<Element | Entity | string | number | boolean>): Element;
    /**
     * Fills the `content` slot of a `Collapsible`, a `Popover`, a `HoverCard`
     * or a `Popup`.
     *
     * A slot is not a child: the element is consumed here and rendered by the
     * component itself — for a `Collapsible`, only while it is `open`; for the
     * two anchored surfaces, in a layer above the rest of the window. Adding it
     * as a child as well throws.
     *
     * It takes an element, not a function returning one, and that is on purpose
     * even though `window.open_dialog` takes a function. A dialog is a view of
     * its own, opened from an event and outliving the render that opened it. A
     * popover's content is part of *this* render: it is described beside its
     * trigger and rebuilt with it, which is exactly what makes `cx.notify()`
     * reach inside an open surface. A function would make it a separate view,
     * invalidated separately — pick an item in an open menu, watch a count
     * outside the menu change and the same count inside it stay put.
     *
     * A `HoverCard` wraps what it is given in an element of its own, so that
     * moving the pointer onto the card keeps it open. Styles written here land
     * on the inner element; the region the pointer has to reach is the wrapper
     * around it.
     */
    content(element: Element): Element;
    /**
     * Fills an `Avatar`'s `image` slot, which takes an `AvatarImage`.
     *
     * Consumed exactly as `content` is — a slot element is not also drawn as a
     * child. Base renders this one when it is there and the `fallback` when it
     * is not, so filling both is how a picture gets something to fall back to.
     */
    image(element: Element): Element;
    /** Fills an `Avatar`'s `fallback` slot, which takes an `AvatarFallback`. */
    fallback(element: Element): Element;
    /** Fills an `AccordionItem`'s `header` slot, which takes an `AccordionHeader`. */
    header(element: Element): Element;
    /** Fills an `AccordionItem`'s `panel` slot, which takes an `AccordionPanel`. */
    panel(element: Element): Element;
    /**
     * Fills the `trigger` slot of a `Popover` or a `HoverCard`: the element
     * that is on screen while the surface is closed, and that opens it.
     *
     * Consumed exactly as `content` is. A surface with no trigger draws
     * nothing at all. A `Popup` takes its trigger in `Popup.new(id, trigger)`
     * instead, because its trigger's bounds are what the content is anchored
     * to.
     */
    trigger(element: Element): Element;
    /**
     * Fills the editor slot of a `NumberInput`.
     *
     * Left empty, the frame draws the bare editor for the state it was built
     * from, which is what a number input almost always wants. Fill it to put
     * something else there — but not `Input.new(state)`: that is the *framed*
     * editor, and a frame inside this frame draws two borders. Adornments
     * beside the editor are ordinary `child(...)` calls on the number input.
     */
    input(element: Element): Element;
    /**
     * Supplies the look of a `NumberInput`'s decrement button.
     *
     * Not optional in practice. The step button is built by the base layer and
     * is completely unstyled — no size, no content — so a number input that
     * leaves this empty has a decrement control that cannot be seen and cannot
     * be pressed.
     *
     * It behaves unlike every other slot: the element is not rendered, it is
     * *replayed*. Its styles, its state styles, its accessibility label and its
     * children are moved onto the button the base layer built, because that
     * button is what receives the press. Give it an `h_flex()` or a `div()`. A
     * `Button.new(id)` works too, but its id is dropped — the step button is
     * already identified. A `text(...)` or an `svg(...)` on its own has no
     * children to move and loses what it draws, so wrap it.
     *
     * `disabled(...)` and `on_click(...)` written here are overwritten: the
     * number input owns whether stepping is allowed and what a press does.
     */
    decrement_button(element: Element): Element;
    /** The increment button, replayed exactly as `decrement_button` is. */
    increment_button(element: Element): Element;
    /**
     * Stacks both of a `NumberInput`'s step buttons to the right of the text,
     * rather than putting one on each side of it.
     */
    controls_right(): Element;
    /**
     * Applies `branch` only when `condition` is truthy, keeping the chain in
     * one piece. `branch` must return the element.
     */
    when(condition: unknown, branch: (el: Element) => Element): Element;

    /**
     * `handler(event, cx)` on activation. Keyboard activation is available
     * only on components whose Base primitive supports it; `Tab` is currently
     * pointer-only pending the compound keyboard behavior tracked in #2838.
     */
    on_click(handler: (event: ClickEvent, cx: Context) => void): Element;
    /** GPUI `InteractiveElement::on_mouse_move`, delivered while this element is hovered. */
    on_mouse_move(handler: (event: MouseMoveEvent, cx: Context) => void): Element;
    /** GPUI `InteractiveElement::on_hover`; reports both pointer entry and exit. */
    on_hover(handler: (hovered: boolean, cx: Context) => void): Element;
    /**
     * GPUI `InteractiveElement::on_key_down`, delivered while this element or
     * something inside it holds the keyboard.
     *
     * A key event travels the focus path, so `track_focus(handle)` is half of
     * this registration rather than a separate concern: without it the handler
     * sits on an element the keyboard never reaches and nothing arrives. The
     * event continues to the handlers above unless `cx.stop_propagation()`
     * says otherwise.
     *
     * Wired on `div`, `h_flex`, `v_flex`, `Button`, `Link`, `Checkbox`,
     * `Switch`, `Radio`, `Toggle`, `Tabs` and `Tab`. On any other component it
     * is recorded and never reaches GPUI, and the log says so — wrap it and
     * write the handler on the wrapper. The same list applies to `on_key_up`,
     * the four pointer handlers, `on_action` and `key_context`.
     *
     * Wired is not the same as reachable. A key travels the focus path, so a
     * component that accepts no focus handle — `Tab` — hears presses and never
     * hears keys, however well both are wired.
     */
    on_key_down(handler: (event: KeyEvent, cx: Context) => void): Element;
    /** GPUI `InteractiveElement::on_key_up`, on the same focus path as `on_key_down`. */
    on_key_up(handler: (event: KeyEvent, cx: Context) => void): Element;
    /**
     * GPUI `InteractiveElement::on_mouse_down`, for one button.
     *
     * Lower-level than `on_click`, and the reason to reach for it is that a
     * press is not a click: it fires before the release, it reports which
     * button, and `click_count` distinguishes a double-click. Registering it
     * for two buttons on one element is fine — the two handlers are
     * independent.
     */
    on_mouse_down(
      button: MouseButton,
      handler: (event: MouseButtonEvent, cx: Context) => void,
    ): Element;
    /** GPUI `InteractiveElement::on_mouse_up`, for one button. */
    on_mouse_up(
      button: MouseButton,
      handler: (event: MouseButtonEvent, cx: Context) => void,
    ): Element;
    /**
     * GPUI `InteractiveElement::on_mouse_down_out`: a press anywhere *outside*
     * this element, delivered during the capture phase.
     *
     * This is how a surface a script drew itself is dismissed by a press
     * elsewhere — the same listener base's own components close on. It fires
     * for any button.
     */
    on_mouse_down_out(handler: (event: MouseButtonEvent, cx: Context) => void): Element;
    /**
     * GPUI `InteractiveElement::on_scroll_wheel`: wheel and trackpad scrolling
     * over this element.
     *
     * For scrolling a region, `overflow_scroll()` is the answer and this is
     * not: it hands GPUI's own retained scroll container the job. Use this when
     * the gesture drives something else — a zoom, a value, a custom viewport.
     */
    on_scroll_wheel(handler: (event: ScrollWheelEvent, cx: Context) => void): Element;
    /**
     * `handler(event, cx)` when the named action is dispatched to this element
     * or to something inside it.
     *
     * An action is the level above a keystroke: `cx.bind_keys` says which
     * chord means `"save"`, in which context, and this says what `"save"`
     * does. A menu item or a button dispatching the same name through
     * `window.dispatch_action("save")` reaches the same handler without
     * pretending to be a keyboard.
     *
     * Registering several on one element is fine and they are independent. An
     * action none of them names carries on to an element further out.
     */
    on_action(action: string, handler: (event: ActionEvent, cx: Context) => void): Element;
    /**
     * `InteractiveElement::key_context`: the key-binding context this element
     * and its subtree sit in.
     *
     * What a binding's `context` predicate is matched against, so one chord can
     * mean one thing in a list and another in an editor. The value is a name or
     * a predicate expression, not free text; an unparsable one is reported and
     * the context is left unset.
     */
    key_context(context: string): Element;
    /**
     * An `AccordionHeader`'s announced heading level — "heading level 3" — as
     * `aria-level` means it. Defaults to 3. It announces; it sizes nothing.
     */
    aria_level(level: number): Element;
    /**
     * Whether an `AccordionPanel` stays in the tree while shut. Off by default;
     * on, its content keeps a scroll position or a half-typed field across a
     * close and reopen.
     */
    keep_mounted(value?: boolean): Element;
    /**
     * `handler(key, cx)` when a row of a virtual list is clicked, where `key`
     * is what the list's `get_key(index)` returned for that row.
     *
     * A key rather than an index, because the two stop agreeing exactly when it
     * matters: the box was captured on the frame the row was drawn, and a
     * filter or a sort can reorder the list before the click is delivered. The
     * key names the item that was pressed; the index would name whatever slid
     * into its place.
     *
     * One handler for the list rather than one per row, and that is a limit
     * rather than a shorthand: a handler registered inside the item renderer
     * throws. Handlers belong to the render pass that registered them and are
     * released with it; a row is rebuilt on every frame the list is scrolled,
     * so a per-row handler would accumulate for as long as the view stood —
     * twenty visible rows over a thousand frames is twenty thousand functions
     * nothing can reach and nothing releases.
     *
     * The key is normally enough: the script already holds the data the row was
     * built from. A row with several independently clickable parts needs a
     * handler lifetime scoped to one batch of items, which this runtime does
     * not have yet; when it does, this restriction lifts and `on_click` inside
     * an item renderer starts working, with no change to anything written
     * against `on_item_click`.
     */
    on_item_click(handler: (key: string, cx: Context) => void): Element;
    /**
     * `handler(value, cx)`, on a toggle. The script owns the new value.
     *
     * A `Radio` only ever reports `true`. It cannot deselect itself, so an
     * already checked — or disabled — radio reports nothing at all, and
     * clearing a group is the script's own business.
     */
    on_change(handler: (checked: boolean, cx: Context) => void): Element;

    /**
     * `handler(action, cx)` on a `NumberInput`, where `action` is
     * `"increment"` or `"decrement"`.
     *
     * **Replaces the built-in stepping.** Without a handler the control steps
     * itself: it adds or subtracts the state's `set_step(...)`, clamps to
     * `set_min(...)` and `set_max(...)`, and re-applies the numeric mask. All
     * of that lives in the closure this replaces, so once a handler is set none
     * of it runs — the script is the only thing that can move the value, and it
     * moves it with `state.set_value(...)`.
     *
     * Both the step buttons and the Up and Down keys report through it.
     */
    on_step(handler: (action: "increment" | "decrement", cx: Context) => void): Element;
    /**
     * `handler(open, cx)`, when something other than the script changed a
     * `Popover`'s open state: a press on the trigger, a press outside it, or
     * Escape. Storage the value and call `cx.notify()`, the way `on_change`
     * stores a checkbox's.
     *
     * A `HoverCard` accepts this too, and today never calls it: the base layer
     * only reports a change it observes between two of its own renders, which
     * its open state cannot produce. A hover card's open state is its own, so
     * nothing is lost except the notification.
     */
    on_open_change(handler: (open: boolean, cx: Context) => void): Element;
    /**
     * `handler(_, cx)` on Enter in an open `Select` or `Combobox`.
     *
     * There is no payload, because the root holds neither the options nor the
     * selection: what was confirmed is whatever the script had highlighted, and
     * the script is the only side that knows. Confirming a *closed* root opens
     * it instead, so this never runs for that case.
     */
    on_confirm(handler: (event: {}, cx: Context) => void): Element;
    /**
     * `handler(_, cx)` on Escape in an open `Select` or `Combobox`, before
     * `on_open_change(false)` — which is what lets a script commit a pending
     * value on the way out.
     */
    on_dismiss(handler: (event: {}, cx: Context) => void): Element;
    /**
     * The label a hover shows over this element, once the pointer has rested
     * on it for half a second.
     *
     * It takes a string, not an element, and that is a real limit rather than
     * a shorthand: the window's tooltip layer rebuilds its content on every
     * frame the label is up, so a function here would be the one piece of a
     * description re-entered once a frame. What is drawn is the shell's own
     * label, in the theme's surface, border, radius and spacing.
     *
     * Wired on a plain `div`, `h_flex` or `v_flex`, and on a `Button` — which
     * is the case it exists for, an icon-only control with no text of its own.
     * Anything else needs a wrapper around it to carry the hover.
     *
     * Where the label goes is base's to decide: it is placed against this
     * element and flipped and clamped to stay inside the window. There is no
     * `align` and no `offset`, because base's tooltip has neither, and the
     * side it prefers is not chooseable from a script yet.
     *
     * A tooltip is not a substitute for `accessibility_label`. A screen reader
     * announces the label; the tooltip is for the pointer.
     */
    tooltip(text: string): Element;
    /** Blocks activation and reports the disabled state. Draw it yourself. */
    disabled(value: boolean): Element;
    /** Reports the selected state of a `Button`. */
    selected(value: boolean): Element;
    /**
     * This item's one-based position and its collection's total size, so a
     * screen reader can announce "tab 2 of 5" or "option 2 of 5". Announced,
     * never drawn: a tab list or radio group that omits it looks identical and
     * says nothing about where the reader is in the set.
     */
    set_position(position: number, size: number): Element;
    /** The controlled value of a `Checkbox`, `Switch` or `Radio`. */
    checked(value: boolean): Element;
    /** The controlled state of a `Toggle`: a button that stays down. */
    pressed(value: boolean): Element;
    /**
     * The announced progress percentage of a `Progress`, clamped to `0..=100`.
     *
     * It moves nothing on screen: size the `ProgressIndicator` from the same
     * number to draw the bar.
     */
    value(percent: number): Element;
    /**
     * Withdraws a `Progress` value from the accessibility tree — "still
     * working, no idea how far". It does not animate anything; a barber-pole
     * or a sliding indicator is yours to draw, and `transition` on the
     * indicator is how it moves.
     */
    indeterminate(value: boolean): Element;
    /**
     * What a screen reader announces. An icon-only control has no text of its
     * own and announces nothing without it.
     */
    accessibility_label(description: string): Element;
    /**
     * What this element announces itself as.
     *
     * Only where the element has one to give: a plain `div`, `h_flex` or
     * `v_flex` — which is how a script builds the listbox, toolbar or dialog
     * base has no component for — and a `Button` or `Checkbox`, whose role is
     * an explicit override (a button that opens a menu, a checkbox that is a
     * menu item). Every other component announces a role of its own, and a
     * `role` there is reported and dropped rather than silently overwritten.
     */
    role(name: Role): Element;
    /**
     * The selected state of an option in a list the script built itself.
     *
     * Plain elements only. `Tab` and `Radio` announce their own selection from
     * `selected(...)` and `checked(...)`.
     */
    aria_selected(value: boolean): Element;
    /**
     * Announces this element as the focused one while an ancestor actually
     * holds the keyboard — the highlighted option of a combobox whose input
     * keeps focus. It needs a `role` to produce a node at all, and GPUI
     * ignores the claim unless a focused ancestor is present, so it is safe to
     * set unconditionally on the highlighted child.
     *
     * Plain elements only.
     */
    aria_active_descendant(): Element;
    /**
     * Tracks a `FocusHandle` the script owns, so `handle.is_focused()` answers
     * for this element and `handle.focus()` moves the keyboard onto it.
     *
     * Honoured by plain elements and by `Button`, `Checkbox`, `Radio`,
     * `Toggle`, `Popup`, `Select` and `Combobox`. `Link`, `Switch` and the rest
     * build their own focus handle and have no builder to replace it; a handle
     * given to one of them is reported and dropped. A `DatePicker` takes its
     * handle in `DatePicker.new(id, handle)` instead.
     *
     * On a `Select` or a `Combobox` this is the *trigger's* handle — what holds
     * the keyboard while the list is shut. Put the same handle on the element
     * you drew as the trigger, or nothing focusable is on screen and Escape and
     * Enter reach nothing.
     */
    track_focus(handle: FocusHandle): Element;
    /**
     * Gives a virtual list the scroll position held by a
     * `VirtualListScrollHandle`, so the script can drive it with
     * `scroll_to_item` and `scroll_to_bottom`.
     *
     * Optional. Without one the list keeps a position of its own, filed under
     * the id it was built with — which is the same place a `Scrollbar` named
     * after that id looks, so the bar works either way.
     */
    track_scroll(handle: import("gpui-base").VirtualListScrollHandle): Element;
    /**
     * Which item a virtual list measures to infer its size across the axis it
     * scrolls: a vertical list takes its width from this item, a horizontal
     * one its height. Defaults to the first.
     *
     * The name is base's own builder, kept verbatim.
     */
    with_item_to_measure_index(index: number): Element;
    /**
     * The handle a `Select` or `Combobox` moves the keyboard to when it opens,
     * and away from when Escape closes it.
     *
     * Put the same handle on the element you drew as the list, and the list can
     * then style itself from `handle.is_focused()`. It does **not** give you
     * arrow-key navigation — see `Select` for what is and is not there.
     */
    content_focus_handle(handle: FocusHandle): Element;
    /**
     * Where this element sits in the window's Tab order. A whole number;
     * setting it also makes the element a tab stop.
     *
     * Honoured by plain elements and by every bound control except `Tab`,
     * `Tabs` and the table, group and progress parts, which base leaves out of
     * keyboard focus entirely.
     */
    tab_index(index: number): Element;
    /**
     * Whether Tab can land on this element. `false` keeps its place in the
     * order without making it reachable, which is what a container that
     * forwards focus to its first child wants.
     */
    tab_stop(value: boolean): Element;
    /** Sets the absolute HTTP(S) target opened by a `Link`. */
    href(url: string): Element;
    /**
     * A stable name for this element, used as its identity.
     *
     * Without one, an element is identified by where it sits in the tree the
     * render built — which shifts the moment a conditional child appears above
     * it, taking the pressed state, the focus and anything else keyed by
     * identity with it. Name anything whose identity has to survive that.
     *
     * Any component whose factory takes an id is already identified by that id
     * and ignores this.
     */
    id(name: string): Element;
    /** Owns wheel and touch scrolling on both axes for overflowing children. */
    overflow_scroll(): Element;
    /** Owns horizontal wheel and touch scrolling for overflowing children. */
    overflow_x_scroll(): Element;
    /** Owns vertical wheel and touch scrolling for overflowing children. */
    overflow_y_scroll(): Element;
    /** Scrolls both axes and paints base-layer scrollbars. */
    overflow_scrollbar(): Element;
    /** Scrolls horizontally and paints a base-layer scrollbar. */
    overflow_x_scrollbar(): Element;
    /** Scrolls vertically and paints a base-layer scrollbar. */
    overflow_y_scrollbar(): Element;
    /**
     * A `Scrollbar`'s visibility policy. Omitted, it follows the theme, which
     * is what every bar painted by `overflow_*_scrollbar` does.
     */
    mode(value: import("gpui-base").ScrollbarMode): Element;
    /**
     * The content size a `Scrollbar` measures its thumb against, in pixels,
     * for when the script knows it and the scroll area does not — a list that
     * paints a window of rows rather than all of them.
     */
    scroll_size(width: number, height: number): Element;
    /**
     * Makes a `Scrollbar` take its viewport from its own box rather than from
     * the scroll area it drives. The way to run a bar down the rows of a table
     * without it reaching up over the fixed header.
     */
    viewport_from_layout(): Element;
    /**
     * How far a `resizable_panel()` may be dragged, in pixels.
     *
     * Two arguments rather than a range, which JavaScript cannot write. The
     * minimum is required — a panel always has one, and base's own is 100 —
     * while the maximum is optional and defaults to unbounded. Omit the call
     * entirely to keep both of base's defaults.
     */
    size_range(min: number, max?: number): Element;
    /**
     * `handler(sizes, cx)` on an `h_resizable()` or `v_resizable()`, once a drag
     * of one of its handles has ended. `sizes` is the pixel size of every panel,
     * in the order they were added.
     *
     * Nothing has to be done with it. The sizes live in the window, keyed by the
     * group's id, so dragging works and survives repaints whether or not this is
     * wired: it is for persisting a layout or showing a width, not for making
     * the group resize.
     */
    on_resize(handler: (sizes: number[], cx: Context) => void): Element;
    /**
     * The orientation a `RadioGroup` or `ToggleGroup` announces.
     *
     * Semantic only: it does **not** lay the group out. A group is a plain
     * block until the script says `.flex().flex_row()` or `.flex_col()`, so set
     * both — the axis for what a screen reader says, the layout for what is
     * drawn. Omitted, each container keeps its own default: `RadioGroup` is
     * vertical, `ToggleGroup` horizontal.
     */
    axis(value: Axis): Element;
    /**
     * A `Table`'s total number of rows, including rows outside the range the
     * script rendered, so a screen reader can announce "row 5 of 200". A table
     * that draws every row it has does not need it.
     */
    row_count(count: number): Element;
    /** A `Table`'s total number of columns, including unrendered ones. */
    column_count(count: number): Element;
    /**
     * Whether a `Collapsible` renders the element in its `content` slot — its
     * ordinary children are rendered either way — or whether a `Popover`,
     * `Select`, `Combobox` or `DatePicker` is showing.
     *
     * Setting it at all makes a `Popover` controlled: the script holds the open
     * state, is told about every change through `on_open_change`, and decides
     * what to do about it. Leaving it off leaves the popover to open and close
     * itself from `default_open`. The three combobox roots have no uncontrolled
     * mode at all: they start shut and stay shut until the script says
     * otherwise.
     *
     * A `Popup` has no open state to set. It shows whatever is in its `content`
     * slot, so `.when(open, el => el.content(...))` is how one is opened.
     */
    open(value: boolean): Element;
    /**
     * Whether a `Popover` starts open. Read once, when the surface is first
     * described; a controlled popover ignores it from then on.
     */
    default_open(value: boolean): Element;
    /**
     * Whether pressing outside an open `Popover` closes it. Default `true`.
     */
    overlay_closable(value: boolean): Element;
    /**
     * Which corner of a `Popover` or `HoverCard` is pinned to its trigger, or
     * where an `fps_monitor()` is pinned inside its relative parent. Omitted,
     * each keeps its own default: `Popover` is `top_left`, `HoverCard` is
     * `top_center`, and `fps_monitor()` is `top_right`.
     *
     * The surface is clamped into the window either way, so an anchor near an
     * edge is a preference rather than a promise.
     */
    anchor(value: Anchor): Element;
    /** Which pointer button opens a `Popover`. Default `left`. */
    mouse_button(value: MouseButton): Element;
    /**
     * How long, in milliseconds, the pointer must rest on a `HoverCard`'s
     * trigger before the card appears. Default 600.
     */
    open_delay(ms: number): Element;
    /**
     * How long, in milliseconds, a `HoverCard` waits after the pointer leaves
     * both the trigger and the card before closing. Default 300; it is what
     * lets the pointer cross the gap between the two.
     */
    close_delay(ms: number): Element;
    /** Animates later target changes entirely in native GPUI code. */
    transition(property: import("gpui-shell").MotionProperty, policy: number | import("gpui-shell").TransitionPolicy): Element;
    /** Springs later target changes entirely in native GPUI code. */
    spring(property: import("gpui-shell").MotionProperty, policy?: import("gpui-shell").SpringPolicy): Element;

    /**
     * Which thumb of a range slider a `SliderThumb` is: the one at the start
     * of the range, or the one at its end. Default `false`, the end — which
     * is the only thumb a single-value slider has.
     */
    start(value: boolean): Element;
    /**
     * How the filled part of a `SliderIndicator` looks. `declare` receives a
     * detached element that collects the styles, exactly as `hover` does; its
     * return value is ignored.
     *
     * Only how it looks. Where it is comes from the state on every frame,
     * because a fill the script positioned would be frozen at the value the
     * render that positioned it saw — the user drags, the value changes, the
     * screen reader announces the new one, and the bar stays put. An indicator
     * with no `range_style` has no fill at all, which is a slider drawn as a
     * groove and a knob.
     */
    range_style(declare: (el: Element) => Element | void): Element;
    /**
     * How every cell of an `OtpInput` looks. `declare` receives a detached
     * element that collects the styles, exactly as `hover` does; its return
     * value is ignored.
     *
     * Give it a size. The cells are drawn by the shell rather than described
     * by the script, so an `OtpInput` without this one is a row of boxes with
     * no size, no border and no background — nothing on screen at all.
     */
    cell_style(declare: (el: Element) => Element | void): Element;
    /**
     * Layered on top of `cell_style` for the one cell the next digit lands in,
     * while the code holds the keyboard and is not disabled. A refinement
     * rather than a replacement, the way `hover` is: declare only what differs.
     */
    cell_active_style(declare: (el: Element) => Element | void): Element;
    /**
     * The blinking mark drawn in that cell while it is still empty. Give it a
     * width, a height and a background; with no `caret_style` there is no
     * caret, and the only sign of where typing goes is `cell_active_style`.
     *
     * Not `cursor_style`: everywhere else in this API `cursor` is the pointer.
     */
    caret_style(declare: (el: Element) => Element | void): Element;
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
    /**
     * Displays the tab at `index` in `group` when this element is clicked.
     *
     * One of the twelve **dock commands**, which are how an element a dock's
     * chrome drew says what it does. A chrome handler runs once per container
     * per frame for as long as the dock is on screen, so it may not register an
     * event handler — one created there would pile up for as long as the dock
     * stood. A command carries no script value: it names a container in the
     * area and what to ask it, and base does the work.
     *
     * Every command takes the object its handler was given — the group, the
     * dock, the tile — as its first argument. They belong on a `div`, an
     * `h_flex` or a `v_flex`; a `Button` builds its own interior and has
     * nowhere to put one.
     */
    select_tab(group: import("gpui-base").DockGroup, index: number): Element;
    /** Closes `panel` when this element is clicked, if its group allows it. */
    close_panel(group: import("gpui-base").DockGroup, panel: number): Element;
    /** Zooms the group in, or back out. */
    toggle_zoom(group: import("gpui-base").DockGroup): Element;
    /**
     * Makes this element the drag source for the tab at `index`, carrying
     * base's own panel payload — so dropping it on another group, or on the
     * area itself, moves the panel there.
     */
    drag_tab(group: import("gpui-base").DockGroup, index: number): Element;
    /**
     * Accepts a dragged panel here. `index` is the slot it lands in; leave it
     * out to append, which is what a drop past the last tab means.
     */
    drop_tab(group: import("gpui-base").DockGroup, index?: number): Element;
    /** Opens or closes the dock when this element is clicked. */
    toggle_dock(dock: import("gpui-base").DockRegion): Element;
    /**
     * Drags the dock's edge. Base clamps every size it is given against the
     * area and the opposite dock, so nothing here has to.
     */
    resize_dock(dock: import("gpui-base").DockRegion): Element;
    /** Drags the tile around its canvas, raising it first. */
    move_tile(tile: import("gpui-base").DockTile): Element;
    /** Drags one edge or corner of the tile. */
    resize_tile(
      tile: import("gpui-base").DockTile,
      side: import("gpui-base").TileResizeSide,
    ): Element;
    /** Brings the tile above the others when this element is pressed. */
    raise_tile(tile: import("gpui-base").DockTile): Element;
    /** Zooms the tile to fill its dock, or back out. */
    toggle_tile_zoom(tile: import("gpui-base").DockTile): Element;
    /** Closes the tile. */
    close_tile(tile: import("gpui-base").DockTile): Element;

    // Style methods that take an argument. Which length type a method
    // accepts follows its Rust signature, so `.p("auto")` and
    // `.rounded("50%")` are type errors here for the same reason they
    // throw at run time.
    /** Sets the background colour. */
    bg(value: Color): Element;
    /** Sets the border width on all four sides. Draws nothing without a colour. */
    border(value: AbsoluteLength): Element;
    /** Sets the border width on the bottom. */
    border_b(value: AbsoluteLength): Element;
    /** Sets the border colour. Draws nothing without a width. */
    border_color(value: Color): Element;
    /** Sets the border width on the left. */
    border_l(value: AbsoluteLength): Element;
    /** Sets the border width on the right. */
    border_r(value: AbsoluteLength): Element;
    /** Sets the border width on the top. */
    border_t(value: AbsoluteLength): Element;
    /** Sets the border width on the left and right. */
    border_x(value: AbsoluteLength): Element;
    /** Sets the border width on the top and bottom. */
    border_y(value: AbsoluteLength): Element;
    /** Sets the bottom offset of a positioned element. */
    bottom(value: Length): Element;
    /** Sets the size this child starts from before growing or shrinking. */
    flex_basis(value: Length): Element;
    /** Sets how much of the free space this child takes. */
    flex_grow(value: number): Element;
    /** Sets how readily this child gives space back. */
    flex_shrink(value: number): Element;
    /** Sets the font family. */
    font_family(value: string): Element;
    /** Sets the font weight to a number between 100 and 900. */
    font_weight(value: number): Element;
    /** Sets the gap between children on both axes. */
    gap(value: DefiniteLength): Element;
    /** Sets the gap between children along the main axis. */
    gap_x(value: DefiniteLength): Element;
    /** Sets the gap between children along the cross axis. */
    gap_y(value: DefiniteLength): Element;
    /** Sets the height. */
    h(value: Length): Element;
    /** Sets all four offsets of a positioned element. */
    inset(value: Length): Element;
    /** Sets the left offset of a positioned element. */
    left(value: Length): Element;
    /** Sets the line height. A bare number is a multiplier (`1.45`), not pixels; a string is a length. */
    line_height(value: DefiniteLength): Element;
    /** Sets the margin on all four sides. */
    m(value: Length): Element;
    /** Sets the maximum height. */
    max_h(value: Length): Element;
    /** Sets the maximum width and height together. */
    max_size(value: Length): Element;
    /** Sets the maximum width. */
    max_w(value: Length): Element;
    /** Sets the margin on the bottom. */
    mb(value: Length): Element;
    /** Sets the minimum height. */
    min_h(value: Length): Element;
    /** Sets the minimum width and height together. */
    min_size(value: Length): Element;
    /** Sets the minimum width. */
    min_w(value: Length): Element;
    /** Sets the margin on the left. */
    ml(value: Length): Element;
    /** Sets the margin on the right. */
    mr(value: Length): Element;
    /** Sets the margin on the top. */
    mt(value: Length): Element;
    /** Sets the margin on the left and right. */
    mx(value: Length): Element;
    /** Sets the margin on the top and bottom. */
    my(value: Length): Element;
    /** Sets the opacity of the element and everything in it, from 0 to 1. */
    opacity(value: number): Element;
    /** Sets the padding on all four sides. */
    p(value: DefiniteLength): Element;
    /** Sets the padding on the bottom. */
    pb(value: DefiniteLength): Element;
    /** Sets the padding on the left. */
    pl(value: DefiniteLength): Element;
    /** Sets the padding on the right. */
    pr(value: DefiniteLength): Element;
    /** Sets the padding on the top. */
    pt(value: DefiniteLength): Element;
    /** Sets the padding on the left and right. */
    px(value: DefiniteLength): Element;
    /** Sets the padding on the top and bottom. */
    py(value: DefiniteLength): Element;
    /** Sets the right offset of a positioned element. */
    right(value: Length): Element;
    /** Sets the corner radius on all four corners. */
    rounded(value: AbsoluteLength): Element;
    /** Sets the corner radius on the two bottom corners. */
    rounded_b(value: AbsoluteLength): Element;
    /** Sets the corner radius on the bottom-left corner. */
    rounded_bl(value: AbsoluteLength): Element;
    /** Sets the corner radius on the bottom-right corner. */
    rounded_br(value: AbsoluteLength): Element;
    /** Sets the corner radius on the two left corners. */
    rounded_l(value: AbsoluteLength): Element;
    /** Sets the corner radius on the two right corners. */
    rounded_r(value: AbsoluteLength): Element;
    /** Sets the corner radius on the two top corners. */
    rounded_t(value: AbsoluteLength): Element;
    /** Sets the corner radius on the top-left corner. */
    rounded_tl(value: AbsoluteLength): Element;
    /** Sets the corner radius on the top-right corner. */
    rounded_tr(value: AbsoluteLength): Element;
    /** Sets the width and the height together. */
    size(value: Length): Element;
    /** Sets the background painted behind the text itself. */
    text_bg(value: Color): Element;
    /** Sets the text colour, which children inherit. */
    text_color(value: Color): Element;
    /** Sets the font size. */
    text_size(value: AbsoluteLength): Element;
    /** Sets the top offset of a positioned element. */
    top(value: Length): Element;
    /** Sets the width. */
    w(value: Length): Element;

    // The 3148 no-argument style methods, generated from GPUI's reflection
    // table. A name here is a name the runtime dispatches, and the
    // documentation is GPUI's own.
    /**
     * Sets the position of the element to `absolute`.
     *
     * [Docs](https://tailwindcss.com/docs/position)
     */
    absolute(): Element;
    /**
     * Sets the aspect ratio of the element to 1/1 – equal width and height.
     *
     * [Docs](https://tailwindcss.com/docs/aspect-ratio)
     */
    aspect_square(): Element;
    /**
     * Sets the display type of the element to `block`.
     *
     * [Docs](https://tailwindcss.com/docs/display)
     */
    block(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 0px
     */
    border_0(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 1px
     */
    border_1(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 10px
     */
    border_10(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 11px
     */
    border_11(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 12px
     */
    border_12(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 16px
     */
    border_16(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 2px
     */
    border_2(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 20px
     */
    border_20(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 24px
     */
    border_24(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 3px
     */
    border_3(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 32px
     */
    border_32(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 4px
     */
    border_4(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 5px
     */
    border_5(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 6px
     */
    border_6(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 7px
     */
    border_7(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 8px
     */
    border_8(): Element;
    /**
     * Sets the border width of the element. [Docs](https://tailwindcss.com/docs/border-width)
     *
     * 9px
     */
    border_9(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 0px
     */
    border_b_0(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 1px
     */
    border_b_1(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 10px
     */
    border_b_10(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 11px
     */
    border_b_11(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 12px
     */
    border_b_12(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 16px
     */
    border_b_16(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 2px
     */
    border_b_2(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 20px
     */
    border_b_20(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 24px
     */
    border_b_24(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 3px
     */
    border_b_3(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 32px
     */
    border_b_32(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 4px
     */
    border_b_4(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 5px
     */
    border_b_5(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 6px
     */
    border_b_6(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 7px
     */
    border_b_7(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 8px
     */
    border_b_8(): Element;
    /**
     * Sets the border width of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 9px
     */
    border_b_9(): Element;
    /** Sets the border style of the element. */
    border_dashed(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 0px
     */
    border_l_0(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 1px
     */
    border_l_1(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 10px
     */
    border_l_10(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 11px
     */
    border_l_11(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 12px
     */
    border_l_12(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 16px
     */
    border_l_16(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 2px
     */
    border_l_2(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 20px
     */
    border_l_20(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 24px
     */
    border_l_24(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 3px
     */
    border_l_3(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 32px
     */
    border_l_32(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 4px
     */
    border_l_4(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 5px
     */
    border_l_5(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 6px
     */
    border_l_6(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 7px
     */
    border_l_7(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 8px
     */
    border_l_8(): Element;
    /**
     * Sets the border width of the left side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 9px
     */
    border_l_9(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 0px
     */
    border_r_0(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 1px
     */
    border_r_1(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 10px
     */
    border_r_10(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 11px
     */
    border_r_11(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 12px
     */
    border_r_12(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 16px
     */
    border_r_16(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 2px
     */
    border_r_2(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 20px
     */
    border_r_20(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 24px
     */
    border_r_24(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 3px
     */
    border_r_3(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 32px
     */
    border_r_32(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 4px
     */
    border_r_4(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 5px
     */
    border_r_5(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 6px
     */
    border_r_6(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 7px
     */
    border_r_7(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 8px
     */
    border_r_8(): Element;
    /**
     * Sets the border width of the right side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 9px
     */
    border_r_9(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 0px
     */
    border_t_0(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 1px
     */
    border_t_1(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 10px
     */
    border_t_10(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 11px
     */
    border_t_11(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 12px
     */
    border_t_12(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 16px
     */
    border_t_16(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 2px
     */
    border_t_2(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 20px
     */
    border_t_20(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 24px
     */
    border_t_24(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 3px
     */
    border_t_3(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 32px
     */
    border_t_32(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 4px
     */
    border_t_4(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 5px
     */
    border_t_5(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 6px
     */
    border_t_6(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 7px
     */
    border_t_7(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 8px
     */
    border_t_8(): Element;
    /**
     * Sets the border width of the top side of the element. [Docs](https://tailwindcss.com/docs/border-width#individual-sides)
     *
     * 9px
     */
    border_t_9(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 0px
     */
    border_x_0(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 1px
     */
    border_x_1(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 10px
     */
    border_x_10(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 11px
     */
    border_x_11(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 12px
     */
    border_x_12(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 16px
     */
    border_x_16(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 2px
     */
    border_x_2(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 20px
     */
    border_x_20(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 24px
     */
    border_x_24(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 3px
     */
    border_x_3(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 32px
     */
    border_x_32(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 4px
     */
    border_x_4(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 5px
     */
    border_x_5(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 6px
     */
    border_x_6(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 7px
     */
    border_x_7(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 8px
     */
    border_x_8(): Element;
    /**
     * Sets the border width of the vertical sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 9px
     */
    border_x_9(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 0px
     */
    border_y_0(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 1px
     */
    border_y_1(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 10px
     */
    border_y_10(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 11px
     */
    border_y_11(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 12px
     */
    border_y_12(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 16px
     */
    border_y_16(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 2px
     */
    border_y_2(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 20px
     */
    border_y_20(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 24px
     */
    border_y_24(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 3px
     */
    border_y_3(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 32px
     */
    border_y_32(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 4px
     */
    border_y_4(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 5px
     */
    border_y_5(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 6px
     */
    border_y_6(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 7px
     */
    border_y_7(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 8px
     */
    border_y_8(): Element;
    /**
     * Sets the border width of the horizontal sides of the element. [Docs](https://tailwindcss.com/docs/border-width#horizontal-and-vertical-sides)
     *
     * 9px
     */
    border_y_9(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 0px
     */
    bottom_0(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 2px (0.125rem)
     */
    bottom_0p5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 4px (0.25rem)
     */
    bottom_1(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40px (2.5rem)
     */
    bottom_10(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 44px (2.75rem)
     */
    bottom_11(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 448px (28rem)
     */
    bottom_112(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 48px (3rem)
     */
    bottom_12(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 512px (32rem)
     */
    bottom_128(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 64px (4rem)
     */
    bottom_16(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8% (1/12)
     */
    bottom_1_12(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (1/2)
     */
    bottom_1_2(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 33% (1/3)
     */
    bottom_1_3(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 25% (1/4)
     */
    bottom_1_4(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20% (1/5)
     */
    bottom_1_5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16% (1/6)
     */
    bottom_1_6(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 6px (0.375rem)
     */
    bottom_1p5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8px (0.5rem)
     */
    bottom_2(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80px (5rem)
     */
    bottom_20(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 96px (6rem)
     */
    bottom_24(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 66% (2/3)
     */
    bottom_2_3(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (2/4)
     */
    bottom_2_4(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40% (2/5)
     */
    bottom_2_5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 10px (0.625rem)
     */
    bottom_2p5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 12px (0.75rem)
     */
    bottom_3(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 128px (8rem)
     */
    bottom_32(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 75% (3/4)
     */
    bottom_3_4(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 60% (3/5)
     */
    bottom_3_5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 14px (0.875rem)
     */
    bottom_3p5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16px (1rem)
     */
    bottom_4(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 160px (10rem)
     */
    bottom_40(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 192px (12rem)
     */
    bottom_48(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (4/5)
     */
    bottom_4_5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20px (1.25rem)
     */
    bottom_5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 224px (14rem)
     */
    bottom_56(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (5/6)
     */
    bottom_5_6(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 24px (1.5rem)
     */
    bottom_6(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 256px (16rem)
     */
    bottom_64(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 28px (1.75rem)
     */
    bottom_7(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 288px (18rem)
     */
    bottom_72(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 32px (2rem)
     */
    bottom_8(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 320px (20rem)
     */
    bottom_80(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 36px (2.25rem)
     */
    bottom_9(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 384px (24rem)
     */
    bottom_96(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * Auto
     */
    bottom_auto(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 100%
     */
    bottom_full(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 0px
     */
    bottom_neg_0(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 2px (0.125rem)
     */
    bottom_neg_0p5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 4px (0.25rem)
     */
    bottom_neg_1(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40px (2.5rem)
     */
    bottom_neg_10(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 44px (2.75rem)
     */
    bottom_neg_11(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 448px (28rem)
     */
    bottom_neg_112(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 48px (3rem)
     */
    bottom_neg_12(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 512px (32rem)
     */
    bottom_neg_128(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 64px (4rem)
     */
    bottom_neg_16(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8% (1/12)
     */
    bottom_neg_1_12(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (1/2)
     */
    bottom_neg_1_2(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 33% (1/3)
     */
    bottom_neg_1_3(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 25% (1/4)
     */
    bottom_neg_1_4(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20% (1/5)
     */
    bottom_neg_1_5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16% (1/6)
     */
    bottom_neg_1_6(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 6px (0.375rem)
     */
    bottom_neg_1p5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8px (0.5rem)
     */
    bottom_neg_2(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80px (5rem)
     */
    bottom_neg_20(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 96px (6rem)
     */
    bottom_neg_24(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 66% (2/3)
     */
    bottom_neg_2_3(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (2/4)
     */
    bottom_neg_2_4(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40% (2/5)
     */
    bottom_neg_2_5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 10px (0.625rem)
     */
    bottom_neg_2p5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 12px (0.75rem)
     */
    bottom_neg_3(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 128px (8rem)
     */
    bottom_neg_32(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 75% (3/4)
     */
    bottom_neg_3_4(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 60% (3/5)
     */
    bottom_neg_3_5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 14px (0.875rem)
     */
    bottom_neg_3p5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16px (1rem)
     */
    bottom_neg_4(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 160px (10rem)
     */
    bottom_neg_40(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 192px (12rem)
     */
    bottom_neg_48(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (4/5)
     */
    bottom_neg_4_5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20px (1.25rem)
     */
    bottom_neg_5(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 224px (14rem)
     */
    bottom_neg_56(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (5/6)
     */
    bottom_neg_5_6(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 24px (1.5rem)
     */
    bottom_neg_6(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 256px (16rem)
     */
    bottom_neg_64(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 28px (1.75rem)
     */
    bottom_neg_7(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 288px (18rem)
     */
    bottom_neg_72(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 32px (2rem)
     */
    bottom_neg_8(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 320px (20rem)
     */
    bottom_neg_80(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 36px (2.25rem)
     */
    bottom_neg_9(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 384px (24rem)
     */
    bottom_neg_96(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 100%
     */
    bottom_neg_full(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 1px
     */
    bottom_neg_px(): Element;
    /**
     * Sets the bottom value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 1px
     */
    bottom_px(): Element;
    /** Sets the column end of this element to auto. */
    col_end_auto(): Element;
    /** Sets the row span of this element. */
    col_span_full(): Element;
    /** Sets the column start of this element to auto. */
    col_start_auto(): Element;
    /**
     * Sets the element to pack content items along the container's cross axis
     *
     * such that there is an equal amount of space on each side of each item.
     *
     * [Docs](https://tailwindcss.com/docs/align-content#space-around)
     */
    content_around(): Element;
    /**
     * Sets the element to pack content items along the container's cross axis
     *
     * such that there is an equal amount of space between each item.
     *
     * [Docs](https://tailwindcss.com/docs/align-content#space-between)
     */
    content_between(): Element;
    /**
     * Sets the element to pack content items in the center of the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-content#center)
     */
    content_center(): Element;
    /**
     * Sets the element to pack content items against the end of the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-content#end)
     */
    content_end(): Element;
    /**
     * Sets the element to pack content items along the container's cross axis
     *
     * such that there is an equal amount of space between each item.
     *
     * [Docs](https://tailwindcss.com/docs/align-content#space-evenly)
     */
    content_evenly(): Element;
    /**
     * Sets the element to pack content items in their default position as if no align-content value was set.
     *
     * [Docs](https://tailwindcss.com/docs/align-content#normal)
     */
    content_normal(): Element;
    /**
     * Sets the element to pack content items against the start of the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-content#start)
     */
    content_start(): Element;
    /**
     * Sets the element to allow content items to fill the available space along the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-content#stretch)
     */
    content_stretch(): Element;
    /**
     * Sets cursor style when hovering over an element to `alias`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_alias(): Element;
    /**
     * Sets cursor style when hovering over an element to `col-resize`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_col_resize(): Element;
    /**
     * Sets cursor style when hovering over an element to `context-menu`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_context_menu(): Element;
    /**
     * Sets cursor style when hovering over an element to `copy`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_copy(): Element;
    /**
     * Sets cursor style when hovering over an element to `crosshair`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_crosshair(): Element;
    /**
     * Sets the cursor style when hovering an element to `default`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_default(): Element;
    /**
     * Sets cursor style when hovering over an element to `e-resize`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_e_resize(): Element;
    /**
     * Sets cursor style when hovering over an element to `ew-resize`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_ew_resize(): Element;
    /**
     * Sets cursor style when hovering over an element to `grab`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_grab(): Element;
    /**
     * Sets cursor style when hovering over an element to `grabbing`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_grabbing(): Element;
    /**
     * Sets cursor style when hovering over an element to `move`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_move(): Element;
    /**
     * Sets cursor style when hovering over an element to `n-resize`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_n_resize(): Element;
    /**
     * Sets cursor style when hovering over an element to `nesw-resize`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_nesw_resize(): Element;
    /**
     * Sets cursor style when hovering over an element to `no-drop`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_no_drop(): Element;
    /**
     * Sets cursor style when hovering over an element to `not-allowed`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_not_allowed(): Element;
    /**
     * Sets cursor style when hovering over an element to `ns-resize`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_ns_resize(): Element;
    /**
     * Sets cursor style when hovering over an element to `nwse-resize`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_nwse_resize(): Element;
    /**
     * Sets the cursor style when hovering an element to `pointer`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_pointer(): Element;
    /**
     * Sets cursor style when hovering over an element to `row-resize`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_row_resize(): Element;
    /**
     * Sets cursor style when hovering over an element to `s-resize`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_s_resize(): Element;
    /**
     * Sets cursor style when hovering over an element to `text`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_text(): Element;
    /**
     * Sets cursor style when hovering over an element to `vertical-text`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_vertical_text(): Element;
    /**
     * Sets cursor style when hovering over an element to `w-resize`.
     *
     * [Docs](https://tailwindcss.com/docs/cursor)
     */
    cursor_w_resize(): Element;
    /** Draws a debug border around this element. */
    debug(): Element;
    /** Draws a debug border on all conforming elements below this element. */
    debug_below(): Element;
    debug_blue(): Element;
    debug_green(): Element;
    debug_pink(): Element;
    debug_red(): Element;
    debug_yellow(): Element;
    /**
     * Sets the display type of the element to `flex`.
     *
     * [Docs](https://tailwindcss.com/docs/display)
     */
    flex(): Element;
    /**
     * Sets the element to allow a flex item to grow and shrink as needed, ignoring its initial size.
     *
     * [Docs](https://tailwindcss.com/docs/flex#flex-1)
     */
    flex_1(): Element;
    /**
     * Sets the element to allow a flex item to grow and shrink, taking into account its initial size.
     *
     * [Docs](https://tailwindcss.com/docs/flex#auto)
     */
    flex_auto(): Element;
    /**
     * Sets the flex direction of the element to `column`.
     *
     * [Docs](https://tailwindcss.com/docs/flex-direction#column)
     */
    flex_col(): Element;
    /**
     * Sets the flex direction of the element to `column-reverse`.
     *
     * [Docs](https://tailwindcss.com/docs/flex-direction#column-reverse)
     */
    flex_col_reverse(): Element;
    /**
     * Disables flex item growth (flex-grow: 0).
     *
     * [Docs](https://tailwindcss.com/docs/flex-grow#dont-grow)
     */
    flex_grow_0(): Element;
    /**
     * Enables flex item growth (flex-grow: 1).
     *
     * [Docs](https://tailwindcss.com/docs/flex-grow#grow-1)
     */
    flex_grow_1(): Element;
    /**
     * Sets the element to allow a flex item to shrink but not grow, taking into account its initial size.
     *
     * [Docs](https://tailwindcss.com/docs/flex#initial)
     */
    flex_initial(): Element;
    /**
     * Sets the element to prevent a flex item from growing or shrinking.
     *
     * [Docs](https://tailwindcss.com/docs/flex#none)
     */
    flex_none(): Element;
    /**
     * Sets the element to prevent flex items from wrapping, causing inflexible items to overflow the container if necessary.
     *
     * [Docs](https://tailwindcss.com/docs/flex-wrap#dont-wrap)
     */
    flex_nowrap(): Element;
    /**
     * Sets the flex direction of the element to `row`.
     *
     * [Docs](https://tailwindcss.com/docs/flex-direction#row)
     */
    flex_row(): Element;
    /**
     * Sets the flex direction of the element to `row-reverse`.
     *
     * [Docs](https://tailwindcss.com/docs/flex-direction#row-reverse)
     */
    flex_row_reverse(): Element;
    /**
     * Disables flex item shrinking (flex-shrink: 0).
     *
     * [Docs](https://tailwindcss.com/docs/flex-shrink#dont-shrink)
     */
    flex_shrink_0(): Element;
    /**
     * Enables flex item shrinking (flex-shrink: 1).
     *
     * [Docs](https://tailwindcss.com/docs/flex-shrink#shrink-1)
     */
    flex_shrink_1(): Element;
    /**
     * Sets the element to allow flex items to wrap.
     *
     * [Docs](https://tailwindcss.com/docs/flex-wrap#wrap-normally)
     */
    flex_wrap(): Element;
    /**
     * Sets the element wrap flex items in the reverse direction.
     *
     * [Docs](https://tailwindcss.com/docs/flex-wrap#wrap-reversed)
     */
    flex_wrap_reverse(): Element;
    /** Sets the font weight to black (900). */
    font_black(): Element;
    /** Sets the font weight to bold (700). */
    font_bold(): Element;
    /** Sets the font weight to extra bold (800). */
    font_extrabold(): Element;
    /** Sets the font weight to extra light (200). */
    font_extralight(): Element;
    /** Sets the font weight to light (300). */
    font_light(): Element;
    /** Sets the font weight to medium (500). */
    font_medium(): Element;
    /** Sets the font weight to normal (400). */
    font_normal(): Element;
    /** Sets the font weight to semibold (600). */
    font_semibold(): Element;
    /** Sets the font weight to thin (100). */
    font_thin(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 0px
     */
    gap_0(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 2px (0.125rem)
     */
    gap_0p5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 4px (0.25rem)
     */
    gap_1(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 40px (2.5rem)
     */
    gap_10(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 44px (2.75rem)
     */
    gap_11(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 448px (28rem)
     */
    gap_112(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 48px (3rem)
     */
    gap_12(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 512px (32rem)
     */
    gap_128(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 64px (4rem)
     */
    gap_16(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 8% (1/12)
     */
    gap_1_12(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 50% (1/2)
     */
    gap_1_2(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 33% (1/3)
     */
    gap_1_3(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 25% (1/4)
     */
    gap_1_4(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 20% (1/5)
     */
    gap_1_5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 16% (1/6)
     */
    gap_1_6(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 6px (0.375rem)
     */
    gap_1p5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 8px (0.5rem)
     */
    gap_2(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 80px (5rem)
     */
    gap_20(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 96px (6rem)
     */
    gap_24(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 66% (2/3)
     */
    gap_2_3(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 50% (2/4)
     */
    gap_2_4(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 40% (2/5)
     */
    gap_2_5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 10px (0.625rem)
     */
    gap_2p5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 12px (0.75rem)
     */
    gap_3(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 128px (8rem)
     */
    gap_32(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 75% (3/4)
     */
    gap_3_4(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 60% (3/5)
     */
    gap_3_5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 14px (0.875rem)
     */
    gap_3p5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 16px (1rem)
     */
    gap_4(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 160px (10rem)
     */
    gap_40(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 192px (12rem)
     */
    gap_48(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 80% (4/5)
     */
    gap_4_5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 20px (1.25rem)
     */
    gap_5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 224px (14rem)
     */
    gap_56(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 80% (5/6)
     */
    gap_5_6(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 24px (1.5rem)
     */
    gap_6(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 256px (16rem)
     */
    gap_64(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 28px (1.75rem)
     */
    gap_7(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 288px (18rem)
     */
    gap_72(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 32px (2rem)
     */
    gap_8(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 320px (20rem)
     */
    gap_80(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 36px (2.25rem)
     */
    gap_9(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 384px (24rem)
     */
    gap_96(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 100%
     */
    gap_full(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 0px
     */
    gap_neg_0(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 2px (0.125rem)
     */
    gap_neg_0p5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 4px (0.25rem)
     */
    gap_neg_1(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 40px (2.5rem)
     */
    gap_neg_10(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 44px (2.75rem)
     */
    gap_neg_11(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 448px (28rem)
     */
    gap_neg_112(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 48px (3rem)
     */
    gap_neg_12(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 512px (32rem)
     */
    gap_neg_128(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 64px (4rem)
     */
    gap_neg_16(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 8% (1/12)
     */
    gap_neg_1_12(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 50% (1/2)
     */
    gap_neg_1_2(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 33% (1/3)
     */
    gap_neg_1_3(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 25% (1/4)
     */
    gap_neg_1_4(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 20% (1/5)
     */
    gap_neg_1_5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 16% (1/6)
     */
    gap_neg_1_6(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 6px (0.375rem)
     */
    gap_neg_1p5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 8px (0.5rem)
     */
    gap_neg_2(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 80px (5rem)
     */
    gap_neg_20(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 96px (6rem)
     */
    gap_neg_24(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 66% (2/3)
     */
    gap_neg_2_3(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 50% (2/4)
     */
    gap_neg_2_4(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 40% (2/5)
     */
    gap_neg_2_5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 10px (0.625rem)
     */
    gap_neg_2p5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 12px (0.75rem)
     */
    gap_neg_3(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 128px (8rem)
     */
    gap_neg_32(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 75% (3/4)
     */
    gap_neg_3_4(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 60% (3/5)
     */
    gap_neg_3_5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 14px (0.875rem)
     */
    gap_neg_3p5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 16px (1rem)
     */
    gap_neg_4(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 160px (10rem)
     */
    gap_neg_40(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 192px (12rem)
     */
    gap_neg_48(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 80% (4/5)
     */
    gap_neg_4_5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 20px (1.25rem)
     */
    gap_neg_5(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 224px (14rem)
     */
    gap_neg_56(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 80% (5/6)
     */
    gap_neg_5_6(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 24px (1.5rem)
     */
    gap_neg_6(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 256px (16rem)
     */
    gap_neg_64(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 28px (1.75rem)
     */
    gap_neg_7(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 288px (18rem)
     */
    gap_neg_72(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 32px (2rem)
     */
    gap_neg_8(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 320px (20rem)
     */
    gap_neg_80(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 36px (2.25rem)
     */
    gap_neg_9(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 384px (24rem)
     */
    gap_neg_96(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 100%
     */
    gap_neg_full(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 1px
     */
    gap_neg_px(): Element;
    /**
     * Sets the gap between rows and columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap)
     *
     * 1px
     */
    gap_px(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 0px
     */
    gap_x_0(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 2px (0.125rem)
     */
    gap_x_0p5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 4px (0.25rem)
     */
    gap_x_1(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 40px (2.5rem)
     */
    gap_x_10(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 44px (2.75rem)
     */
    gap_x_11(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 448px (28rem)
     */
    gap_x_112(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 48px (3rem)
     */
    gap_x_12(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 512px (32rem)
     */
    gap_x_128(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 64px (4rem)
     */
    gap_x_16(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 8% (1/12)
     */
    gap_x_1_12(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 50% (1/2)
     */
    gap_x_1_2(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 33% (1/3)
     */
    gap_x_1_3(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 25% (1/4)
     */
    gap_x_1_4(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 20% (1/5)
     */
    gap_x_1_5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 16% (1/6)
     */
    gap_x_1_6(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 6px (0.375rem)
     */
    gap_x_1p5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 8px (0.5rem)
     */
    gap_x_2(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 80px (5rem)
     */
    gap_x_20(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 96px (6rem)
     */
    gap_x_24(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 66% (2/3)
     */
    gap_x_2_3(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 50% (2/4)
     */
    gap_x_2_4(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 40% (2/5)
     */
    gap_x_2_5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 10px (0.625rem)
     */
    gap_x_2p5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 12px (0.75rem)
     */
    gap_x_3(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 128px (8rem)
     */
    gap_x_32(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 75% (3/4)
     */
    gap_x_3_4(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 60% (3/5)
     */
    gap_x_3_5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 14px (0.875rem)
     */
    gap_x_3p5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 16px (1rem)
     */
    gap_x_4(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 160px (10rem)
     */
    gap_x_40(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 192px (12rem)
     */
    gap_x_48(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 80% (4/5)
     */
    gap_x_4_5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 20px (1.25rem)
     */
    gap_x_5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 224px (14rem)
     */
    gap_x_56(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 80% (5/6)
     */
    gap_x_5_6(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 24px (1.5rem)
     */
    gap_x_6(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 256px (16rem)
     */
    gap_x_64(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 28px (1.75rem)
     */
    gap_x_7(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 288px (18rem)
     */
    gap_x_72(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 32px (2rem)
     */
    gap_x_8(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 320px (20rem)
     */
    gap_x_80(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 36px (2.25rem)
     */
    gap_x_9(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 384px (24rem)
     */
    gap_x_96(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 100%
     */
    gap_x_full(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 0px
     */
    gap_x_neg_0(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 2px (0.125rem)
     */
    gap_x_neg_0p5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 4px (0.25rem)
     */
    gap_x_neg_1(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 40px (2.5rem)
     */
    gap_x_neg_10(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 44px (2.75rem)
     */
    gap_x_neg_11(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 448px (28rem)
     */
    gap_x_neg_112(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 48px (3rem)
     */
    gap_x_neg_12(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 512px (32rem)
     */
    gap_x_neg_128(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 64px (4rem)
     */
    gap_x_neg_16(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 8% (1/12)
     */
    gap_x_neg_1_12(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 50% (1/2)
     */
    gap_x_neg_1_2(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 33% (1/3)
     */
    gap_x_neg_1_3(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 25% (1/4)
     */
    gap_x_neg_1_4(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 20% (1/5)
     */
    gap_x_neg_1_5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 16% (1/6)
     */
    gap_x_neg_1_6(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 6px (0.375rem)
     */
    gap_x_neg_1p5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 8px (0.5rem)
     */
    gap_x_neg_2(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 80px (5rem)
     */
    gap_x_neg_20(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 96px (6rem)
     */
    gap_x_neg_24(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 66% (2/3)
     */
    gap_x_neg_2_3(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 50% (2/4)
     */
    gap_x_neg_2_4(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 40% (2/5)
     */
    gap_x_neg_2_5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 10px (0.625rem)
     */
    gap_x_neg_2p5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 12px (0.75rem)
     */
    gap_x_neg_3(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 128px (8rem)
     */
    gap_x_neg_32(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 75% (3/4)
     */
    gap_x_neg_3_4(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 60% (3/5)
     */
    gap_x_neg_3_5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 14px (0.875rem)
     */
    gap_x_neg_3p5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 16px (1rem)
     */
    gap_x_neg_4(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 160px (10rem)
     */
    gap_x_neg_40(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 192px (12rem)
     */
    gap_x_neg_48(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 80% (4/5)
     */
    gap_x_neg_4_5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 20px (1.25rem)
     */
    gap_x_neg_5(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 224px (14rem)
     */
    gap_x_neg_56(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 80% (5/6)
     */
    gap_x_neg_5_6(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 24px (1.5rem)
     */
    gap_x_neg_6(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 256px (16rem)
     */
    gap_x_neg_64(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 28px (1.75rem)
     */
    gap_x_neg_7(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 288px (18rem)
     */
    gap_x_neg_72(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 32px (2rem)
     */
    gap_x_neg_8(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 320px (20rem)
     */
    gap_x_neg_80(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 36px (2.25rem)
     */
    gap_x_neg_9(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 384px (24rem)
     */
    gap_x_neg_96(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 100%
     */
    gap_x_neg_full(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 1px
     */
    gap_x_neg_px(): Element;
    /**
     * Sets the gap between columns in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 1px
     */
    gap_x_px(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 0px
     */
    gap_y_0(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 2px (0.125rem)
     */
    gap_y_0p5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 4px (0.25rem)
     */
    gap_y_1(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 40px (2.5rem)
     */
    gap_y_10(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 44px (2.75rem)
     */
    gap_y_11(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 448px (28rem)
     */
    gap_y_112(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 48px (3rem)
     */
    gap_y_12(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 512px (32rem)
     */
    gap_y_128(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 64px (4rem)
     */
    gap_y_16(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 8% (1/12)
     */
    gap_y_1_12(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 50% (1/2)
     */
    gap_y_1_2(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 33% (1/3)
     */
    gap_y_1_3(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 25% (1/4)
     */
    gap_y_1_4(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 20% (1/5)
     */
    gap_y_1_5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 16% (1/6)
     */
    gap_y_1_6(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 6px (0.375rem)
     */
    gap_y_1p5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 8px (0.5rem)
     */
    gap_y_2(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 80px (5rem)
     */
    gap_y_20(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 96px (6rem)
     */
    gap_y_24(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 66% (2/3)
     */
    gap_y_2_3(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 50% (2/4)
     */
    gap_y_2_4(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 40% (2/5)
     */
    gap_y_2_5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 10px (0.625rem)
     */
    gap_y_2p5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 12px (0.75rem)
     */
    gap_y_3(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 128px (8rem)
     */
    gap_y_32(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 75% (3/4)
     */
    gap_y_3_4(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 60% (3/5)
     */
    gap_y_3_5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 14px (0.875rem)
     */
    gap_y_3p5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 16px (1rem)
     */
    gap_y_4(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 160px (10rem)
     */
    gap_y_40(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 192px (12rem)
     */
    gap_y_48(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 80% (4/5)
     */
    gap_y_4_5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 20px (1.25rem)
     */
    gap_y_5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 224px (14rem)
     */
    gap_y_56(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 80% (5/6)
     */
    gap_y_5_6(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 24px (1.5rem)
     */
    gap_y_6(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 256px (16rem)
     */
    gap_y_64(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 28px (1.75rem)
     */
    gap_y_7(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 288px (18rem)
     */
    gap_y_72(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 32px (2rem)
     */
    gap_y_8(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 320px (20rem)
     */
    gap_y_80(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 36px (2.25rem)
     */
    gap_y_9(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 384px (24rem)
     */
    gap_y_96(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 100%
     */
    gap_y_full(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 0px
     */
    gap_y_neg_0(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 2px (0.125rem)
     */
    gap_y_neg_0p5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 4px (0.25rem)
     */
    gap_y_neg_1(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 40px (2.5rem)
     */
    gap_y_neg_10(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 44px (2.75rem)
     */
    gap_y_neg_11(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 448px (28rem)
     */
    gap_y_neg_112(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 48px (3rem)
     */
    gap_y_neg_12(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 512px (32rem)
     */
    gap_y_neg_128(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 64px (4rem)
     */
    gap_y_neg_16(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 8% (1/12)
     */
    gap_y_neg_1_12(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 50% (1/2)
     */
    gap_y_neg_1_2(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 33% (1/3)
     */
    gap_y_neg_1_3(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 25% (1/4)
     */
    gap_y_neg_1_4(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 20% (1/5)
     */
    gap_y_neg_1_5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 16% (1/6)
     */
    gap_y_neg_1_6(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 6px (0.375rem)
     */
    gap_y_neg_1p5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 8px (0.5rem)
     */
    gap_y_neg_2(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 80px (5rem)
     */
    gap_y_neg_20(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 96px (6rem)
     */
    gap_y_neg_24(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 66% (2/3)
     */
    gap_y_neg_2_3(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 50% (2/4)
     */
    gap_y_neg_2_4(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 40% (2/5)
     */
    gap_y_neg_2_5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 10px (0.625rem)
     */
    gap_y_neg_2p5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 12px (0.75rem)
     */
    gap_y_neg_3(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 128px (8rem)
     */
    gap_y_neg_32(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 75% (3/4)
     */
    gap_y_neg_3_4(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 60% (3/5)
     */
    gap_y_neg_3_5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 14px (0.875rem)
     */
    gap_y_neg_3p5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 16px (1rem)
     */
    gap_y_neg_4(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 160px (10rem)
     */
    gap_y_neg_40(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 192px (12rem)
     */
    gap_y_neg_48(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 80% (4/5)
     */
    gap_y_neg_4_5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 20px (1.25rem)
     */
    gap_y_neg_5(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 224px (14rem)
     */
    gap_y_neg_56(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 80% (5/6)
     */
    gap_y_neg_5_6(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 24px (1.5rem)
     */
    gap_y_neg_6(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 256px (16rem)
     */
    gap_y_neg_64(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 28px (1.75rem)
     */
    gap_y_neg_7(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 288px (18rem)
     */
    gap_y_neg_72(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 32px (2rem)
     */
    gap_y_neg_8(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 320px (20rem)
     */
    gap_y_neg_80(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 36px (2.25rem)
     */
    gap_y_neg_9(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 384px (24rem)
     */
    gap_y_neg_96(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 100%
     */
    gap_y_neg_full(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 1px
     */
    gap_y_neg_px(): Element;
    /**
     * Sets the gap between rows in flex layouts. [Docs](https://tailwindcss.com/docs/gap#changing-row-and-column-gaps-independently)
     *
     * 1px
     */
    gap_y_px(): Element;
    /**
     * Sets the display type of the element to `grid`.
     *
     * [Docs](https://tailwindcss.com/docs/display)
     */
    grid(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 0px
     */
    h_0(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 2px (0.125rem)
     */
    h_0p5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 4px (0.25rem)
     */
    h_1(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 40px (2.5rem)
     */
    h_10(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 44px (2.75rem)
     */
    h_11(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 448px (28rem)
     */
    h_112(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 48px (3rem)
     */
    h_12(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 512px (32rem)
     */
    h_128(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 64px (4rem)
     */
    h_16(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 8% (1/12)
     */
    h_1_12(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 50% (1/2)
     */
    h_1_2(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 33% (1/3)
     */
    h_1_3(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 25% (1/4)
     */
    h_1_4(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 20% (1/5)
     */
    h_1_5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 16% (1/6)
     */
    h_1_6(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 6px (0.375rem)
     */
    h_1p5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 8px (0.5rem)
     */
    h_2(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 80px (5rem)
     */
    h_20(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 96px (6rem)
     */
    h_24(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 66% (2/3)
     */
    h_2_3(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 50% (2/4)
     */
    h_2_4(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 40% (2/5)
     */
    h_2_5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 10px (0.625rem)
     */
    h_2p5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 12px (0.75rem)
     */
    h_3(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 128px (8rem)
     */
    h_32(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 75% (3/4)
     */
    h_3_4(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 60% (3/5)
     */
    h_3_5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 14px (0.875rem)
     */
    h_3p5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 16px (1rem)
     */
    h_4(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 160px (10rem)
     */
    h_40(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 192px (12rem)
     */
    h_48(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 80% (4/5)
     */
    h_4_5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 20px (1.25rem)
     */
    h_5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 224px (14rem)
     */
    h_56(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 80% (5/6)
     */
    h_5_6(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 24px (1.5rem)
     */
    h_6(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 256px (16rem)
     */
    h_64(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 28px (1.75rem)
     */
    h_7(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 288px (18rem)
     */
    h_72(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 32px (2rem)
     */
    h_8(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 320px (20rem)
     */
    h_80(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 36px (2.25rem)
     */
    h_9(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 384px (24rem)
     */
    h_96(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * Auto
     */
    h_auto(): Element;
    h_flex(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 100%
     */
    h_full(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 0px
     */
    h_neg_0(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 2px (0.125rem)
     */
    h_neg_0p5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 4px (0.25rem)
     */
    h_neg_1(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 40px (2.5rem)
     */
    h_neg_10(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 44px (2.75rem)
     */
    h_neg_11(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 448px (28rem)
     */
    h_neg_112(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 48px (3rem)
     */
    h_neg_12(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 512px (32rem)
     */
    h_neg_128(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 64px (4rem)
     */
    h_neg_16(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 8% (1/12)
     */
    h_neg_1_12(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 50% (1/2)
     */
    h_neg_1_2(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 33% (1/3)
     */
    h_neg_1_3(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 25% (1/4)
     */
    h_neg_1_4(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 20% (1/5)
     */
    h_neg_1_5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 16% (1/6)
     */
    h_neg_1_6(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 6px (0.375rem)
     */
    h_neg_1p5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 8px (0.5rem)
     */
    h_neg_2(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 80px (5rem)
     */
    h_neg_20(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 96px (6rem)
     */
    h_neg_24(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 66% (2/3)
     */
    h_neg_2_3(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 50% (2/4)
     */
    h_neg_2_4(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 40% (2/5)
     */
    h_neg_2_5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 10px (0.625rem)
     */
    h_neg_2p5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 12px (0.75rem)
     */
    h_neg_3(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 128px (8rem)
     */
    h_neg_32(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 75% (3/4)
     */
    h_neg_3_4(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 60% (3/5)
     */
    h_neg_3_5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 14px (0.875rem)
     */
    h_neg_3p5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 16px (1rem)
     */
    h_neg_4(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 160px (10rem)
     */
    h_neg_40(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 192px (12rem)
     */
    h_neg_48(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 80% (4/5)
     */
    h_neg_4_5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 20px (1.25rem)
     */
    h_neg_5(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 224px (14rem)
     */
    h_neg_56(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 80% (5/6)
     */
    h_neg_5_6(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 24px (1.5rem)
     */
    h_neg_6(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 256px (16rem)
     */
    h_neg_64(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 28px (1.75rem)
     */
    h_neg_7(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 288px (18rem)
     */
    h_neg_72(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 32px (2rem)
     */
    h_neg_8(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 320px (20rem)
     */
    h_neg_80(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 36px (2.25rem)
     */
    h_neg_9(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 384px (24rem)
     */
    h_neg_96(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 100%
     */
    h_neg_full(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 1px
     */
    h_neg_px(): Element;
    /**
     * Sets the height of the element. [Docs](https://tailwindcss.com/docs/height)
     *
     * 1px
     */
    h_px(): Element;
    /**
     * Sets the display type of the element to `none`.
     *
     * [Docs](https://tailwindcss.com/docs/display)
     */
    hidden(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 0px
     */
    inset_0(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 2px (0.125rem)
     */
    inset_0p5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 4px (0.25rem)
     */
    inset_1(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40px (2.5rem)
     */
    inset_10(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 44px (2.75rem)
     */
    inset_11(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 448px (28rem)
     */
    inset_112(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 48px (3rem)
     */
    inset_12(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 512px (32rem)
     */
    inset_128(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 64px (4rem)
     */
    inset_16(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8% (1/12)
     */
    inset_1_12(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (1/2)
     */
    inset_1_2(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 33% (1/3)
     */
    inset_1_3(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 25% (1/4)
     */
    inset_1_4(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20% (1/5)
     */
    inset_1_5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16% (1/6)
     */
    inset_1_6(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 6px (0.375rem)
     */
    inset_1p5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8px (0.5rem)
     */
    inset_2(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80px (5rem)
     */
    inset_20(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 96px (6rem)
     */
    inset_24(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 66% (2/3)
     */
    inset_2_3(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (2/4)
     */
    inset_2_4(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40% (2/5)
     */
    inset_2_5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 10px (0.625rem)
     */
    inset_2p5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 12px (0.75rem)
     */
    inset_3(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 128px (8rem)
     */
    inset_32(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 75% (3/4)
     */
    inset_3_4(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 60% (3/5)
     */
    inset_3_5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 14px (0.875rem)
     */
    inset_3p5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16px (1rem)
     */
    inset_4(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 160px (10rem)
     */
    inset_40(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 192px (12rem)
     */
    inset_48(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (4/5)
     */
    inset_4_5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20px (1.25rem)
     */
    inset_5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 224px (14rem)
     */
    inset_56(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (5/6)
     */
    inset_5_6(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 24px (1.5rem)
     */
    inset_6(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 256px (16rem)
     */
    inset_64(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 28px (1.75rem)
     */
    inset_7(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 288px (18rem)
     */
    inset_72(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 32px (2rem)
     */
    inset_8(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 320px (20rem)
     */
    inset_80(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 36px (2.25rem)
     */
    inset_9(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 384px (24rem)
     */
    inset_96(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * Auto
     */
    inset_auto(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 100%
     */
    inset_full(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 0px
     */
    inset_neg_0(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 2px (0.125rem)
     */
    inset_neg_0p5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 4px (0.25rem)
     */
    inset_neg_1(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40px (2.5rem)
     */
    inset_neg_10(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 44px (2.75rem)
     */
    inset_neg_11(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 448px (28rem)
     */
    inset_neg_112(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 48px (3rem)
     */
    inset_neg_12(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 512px (32rem)
     */
    inset_neg_128(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 64px (4rem)
     */
    inset_neg_16(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8% (1/12)
     */
    inset_neg_1_12(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (1/2)
     */
    inset_neg_1_2(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 33% (1/3)
     */
    inset_neg_1_3(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 25% (1/4)
     */
    inset_neg_1_4(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20% (1/5)
     */
    inset_neg_1_5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16% (1/6)
     */
    inset_neg_1_6(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 6px (0.375rem)
     */
    inset_neg_1p5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8px (0.5rem)
     */
    inset_neg_2(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80px (5rem)
     */
    inset_neg_20(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 96px (6rem)
     */
    inset_neg_24(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 66% (2/3)
     */
    inset_neg_2_3(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (2/4)
     */
    inset_neg_2_4(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40% (2/5)
     */
    inset_neg_2_5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 10px (0.625rem)
     */
    inset_neg_2p5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 12px (0.75rem)
     */
    inset_neg_3(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 128px (8rem)
     */
    inset_neg_32(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 75% (3/4)
     */
    inset_neg_3_4(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 60% (3/5)
     */
    inset_neg_3_5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 14px (0.875rem)
     */
    inset_neg_3p5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16px (1rem)
     */
    inset_neg_4(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 160px (10rem)
     */
    inset_neg_40(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 192px (12rem)
     */
    inset_neg_48(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (4/5)
     */
    inset_neg_4_5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20px (1.25rem)
     */
    inset_neg_5(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 224px (14rem)
     */
    inset_neg_56(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (5/6)
     */
    inset_neg_5_6(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 24px (1.5rem)
     */
    inset_neg_6(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 256px (16rem)
     */
    inset_neg_64(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 28px (1.75rem)
     */
    inset_neg_7(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 288px (18rem)
     */
    inset_neg_72(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 32px (2rem)
     */
    inset_neg_8(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 320px (20rem)
     */
    inset_neg_80(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 36px (2.25rem)
     */
    inset_neg_9(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 384px (24rem)
     */
    inset_neg_96(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 100%
     */
    inset_neg_full(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 1px
     */
    inset_neg_px(): Element;
    /**
     * Sets the top, right, bottom, and left values of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 1px
     */
    inset_px(): Element;
    /**
     * Sets the visibility of the element to `hidden`.
     *
     * [Docs](https://tailwindcss.com/docs/visibility)
     */
    invisible(): Element;
    /**
     * Sets the font style of the element to italic.
     *
     * [Docs](https://tailwindcss.com/docs/font-style#italicizing-text)
     */
    italic(): Element;
    /**
     * Sets the element to align flex items along the baseline of the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-items#baseline)
     */
    items_baseline(): Element;
    /**
     * Sets the element to align flex items along the center of the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-items#center)
     */
    items_center(): Element;
    /**
     * Sets the element to align flex items to the end of the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-items#end)
     */
    items_end(): Element;
    /**
     * Sets the element to align flex items to the start of the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-items#start)
     */
    items_start(): Element;
    /**
     * Sets the element to stretch flex items to fill the available space along the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-items#stretch)
     */
    items_stretch(): Element;
    /**
     * Sets the element to justify items along the container's main axis such
     *
     * that there is an equal amount of space on each side of each item.
     *
     * [Docs](https://tailwindcss.com/docs/justify-content#space-around)
     */
    justify_around(): Element;
    /**
     * Sets the element to justify flex items along the container's main axis
     *
     * such that there is an equal amount of space between each item.
     *
     * [Docs](https://tailwindcss.com/docs/justify-content#space-between)
     */
    justify_between(): Element;
    /**
     * Sets the element to justify flex items along the center of the container's main axis.
     *
     * [Docs](https://tailwindcss.com/docs/justify-content#center)
     */
    justify_center(): Element;
    /**
     * Sets the element to justify flex items against the end of the container's main axis.
     *
     * [Docs](https://tailwindcss.com/docs/justify-content#end)
     */
    justify_end(): Element;
    /**
     * Sets the element to justify items along the container's main axis such
     *
     * that there is an equal amount of space around each item, but also
     *
     * accounting for the doubling of space you would normally see between
     *
     * each item when using justify-around.
     *
     * [Docs](https://tailwindcss.com/docs/justify-content#space-evenly)
     */
    justify_evenly(): Element;
    /**
     * Sets the element to justify flex items against the start of the container's main axis.
     *
     * [Docs](https://tailwindcss.com/docs/justify-content#start)
     */
    justify_start(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 0px
     */
    left_0(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 2px (0.125rem)
     */
    left_0p5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 4px (0.25rem)
     */
    left_1(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40px (2.5rem)
     */
    left_10(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 44px (2.75rem)
     */
    left_11(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 448px (28rem)
     */
    left_112(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 48px (3rem)
     */
    left_12(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 512px (32rem)
     */
    left_128(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 64px (4rem)
     */
    left_16(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8% (1/12)
     */
    left_1_12(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (1/2)
     */
    left_1_2(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 33% (1/3)
     */
    left_1_3(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 25% (1/4)
     */
    left_1_4(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20% (1/5)
     */
    left_1_5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16% (1/6)
     */
    left_1_6(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 6px (0.375rem)
     */
    left_1p5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8px (0.5rem)
     */
    left_2(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80px (5rem)
     */
    left_20(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 96px (6rem)
     */
    left_24(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 66% (2/3)
     */
    left_2_3(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (2/4)
     */
    left_2_4(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40% (2/5)
     */
    left_2_5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 10px (0.625rem)
     */
    left_2p5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 12px (0.75rem)
     */
    left_3(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 128px (8rem)
     */
    left_32(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 75% (3/4)
     */
    left_3_4(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 60% (3/5)
     */
    left_3_5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 14px (0.875rem)
     */
    left_3p5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16px (1rem)
     */
    left_4(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 160px (10rem)
     */
    left_40(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 192px (12rem)
     */
    left_48(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (4/5)
     */
    left_4_5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20px (1.25rem)
     */
    left_5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 224px (14rem)
     */
    left_56(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (5/6)
     */
    left_5_6(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 24px (1.5rem)
     */
    left_6(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 256px (16rem)
     */
    left_64(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 28px (1.75rem)
     */
    left_7(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 288px (18rem)
     */
    left_72(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 32px (2rem)
     */
    left_8(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 320px (20rem)
     */
    left_80(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 36px (2.25rem)
     */
    left_9(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 384px (24rem)
     */
    left_96(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * Auto
     */
    left_auto(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 100%
     */
    left_full(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 0px
     */
    left_neg_0(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 2px (0.125rem)
     */
    left_neg_0p5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 4px (0.25rem)
     */
    left_neg_1(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40px (2.5rem)
     */
    left_neg_10(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 44px (2.75rem)
     */
    left_neg_11(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 448px (28rem)
     */
    left_neg_112(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 48px (3rem)
     */
    left_neg_12(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 512px (32rem)
     */
    left_neg_128(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 64px (4rem)
     */
    left_neg_16(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8% (1/12)
     */
    left_neg_1_12(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (1/2)
     */
    left_neg_1_2(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 33% (1/3)
     */
    left_neg_1_3(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 25% (1/4)
     */
    left_neg_1_4(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20% (1/5)
     */
    left_neg_1_5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16% (1/6)
     */
    left_neg_1_6(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 6px (0.375rem)
     */
    left_neg_1p5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8px (0.5rem)
     */
    left_neg_2(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80px (5rem)
     */
    left_neg_20(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 96px (6rem)
     */
    left_neg_24(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 66% (2/3)
     */
    left_neg_2_3(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (2/4)
     */
    left_neg_2_4(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40% (2/5)
     */
    left_neg_2_5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 10px (0.625rem)
     */
    left_neg_2p5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 12px (0.75rem)
     */
    left_neg_3(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 128px (8rem)
     */
    left_neg_32(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 75% (3/4)
     */
    left_neg_3_4(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 60% (3/5)
     */
    left_neg_3_5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 14px (0.875rem)
     */
    left_neg_3p5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16px (1rem)
     */
    left_neg_4(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 160px (10rem)
     */
    left_neg_40(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 192px (12rem)
     */
    left_neg_48(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (4/5)
     */
    left_neg_4_5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20px (1.25rem)
     */
    left_neg_5(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 224px (14rem)
     */
    left_neg_56(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (5/6)
     */
    left_neg_5_6(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 24px (1.5rem)
     */
    left_neg_6(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 256px (16rem)
     */
    left_neg_64(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 28px (1.75rem)
     */
    left_neg_7(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 288px (18rem)
     */
    left_neg_72(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 32px (2rem)
     */
    left_neg_8(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 320px (20rem)
     */
    left_neg_80(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 36px (2.25rem)
     */
    left_neg_9(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 384px (24rem)
     */
    left_neg_96(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 100%
     */
    left_neg_full(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 1px
     */
    left_neg_px(): Element;
    /**
     * Sets the left value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 1px
     */
    left_px(): Element;
    /**
     * Sets the decoration of the text to have a line through it.
     *
     * [Docs](https://tailwindcss.com/docs/text-decoration-line#adding-a-line-through-text)
     */
    line_through(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 0px
     */
    m_0(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 2px (0.125rem)
     */
    m_0p5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 4px (0.25rem)
     */
    m_1(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 40px (2.5rem)
     */
    m_10(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 44px (2.75rem)
     */
    m_11(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 448px (28rem)
     */
    m_112(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 48px (3rem)
     */
    m_12(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 512px (32rem)
     */
    m_128(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 64px (4rem)
     */
    m_16(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 8% (1/12)
     */
    m_1_12(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 50% (1/2)
     */
    m_1_2(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 33% (1/3)
     */
    m_1_3(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 25% (1/4)
     */
    m_1_4(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 20% (1/5)
     */
    m_1_5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 16% (1/6)
     */
    m_1_6(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 6px (0.375rem)
     */
    m_1p5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 8px (0.5rem)
     */
    m_2(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 80px (5rem)
     */
    m_20(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 96px (6rem)
     */
    m_24(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 66% (2/3)
     */
    m_2_3(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 50% (2/4)
     */
    m_2_4(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 40% (2/5)
     */
    m_2_5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 10px (0.625rem)
     */
    m_2p5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 12px (0.75rem)
     */
    m_3(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 128px (8rem)
     */
    m_32(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 75% (3/4)
     */
    m_3_4(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 60% (3/5)
     */
    m_3_5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 14px (0.875rem)
     */
    m_3p5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 16px (1rem)
     */
    m_4(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 160px (10rem)
     */
    m_40(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 192px (12rem)
     */
    m_48(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 80% (4/5)
     */
    m_4_5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 20px (1.25rem)
     */
    m_5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 224px (14rem)
     */
    m_56(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 80% (5/6)
     */
    m_5_6(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 24px (1.5rem)
     */
    m_6(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 256px (16rem)
     */
    m_64(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 28px (1.75rem)
     */
    m_7(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 288px (18rem)
     */
    m_72(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 32px (2rem)
     */
    m_8(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 320px (20rem)
     */
    m_80(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 36px (2.25rem)
     */
    m_9(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 384px (24rem)
     */
    m_96(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * Auto
     */
    m_auto(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 100%
     */
    m_full(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 0px
     */
    m_neg_0(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 2px (0.125rem)
     */
    m_neg_0p5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 4px (0.25rem)
     */
    m_neg_1(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 40px (2.5rem)
     */
    m_neg_10(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 44px (2.75rem)
     */
    m_neg_11(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 448px (28rem)
     */
    m_neg_112(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 48px (3rem)
     */
    m_neg_12(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 512px (32rem)
     */
    m_neg_128(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 64px (4rem)
     */
    m_neg_16(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 8% (1/12)
     */
    m_neg_1_12(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 50% (1/2)
     */
    m_neg_1_2(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 33% (1/3)
     */
    m_neg_1_3(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 25% (1/4)
     */
    m_neg_1_4(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 20% (1/5)
     */
    m_neg_1_5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 16% (1/6)
     */
    m_neg_1_6(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 6px (0.375rem)
     */
    m_neg_1p5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 8px (0.5rem)
     */
    m_neg_2(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 80px (5rem)
     */
    m_neg_20(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 96px (6rem)
     */
    m_neg_24(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 66% (2/3)
     */
    m_neg_2_3(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 50% (2/4)
     */
    m_neg_2_4(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 40% (2/5)
     */
    m_neg_2_5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 10px (0.625rem)
     */
    m_neg_2p5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 12px (0.75rem)
     */
    m_neg_3(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 128px (8rem)
     */
    m_neg_32(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 75% (3/4)
     */
    m_neg_3_4(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 60% (3/5)
     */
    m_neg_3_5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 14px (0.875rem)
     */
    m_neg_3p5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 16px (1rem)
     */
    m_neg_4(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 160px (10rem)
     */
    m_neg_40(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 192px (12rem)
     */
    m_neg_48(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 80% (4/5)
     */
    m_neg_4_5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 20px (1.25rem)
     */
    m_neg_5(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 224px (14rem)
     */
    m_neg_56(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 80% (5/6)
     */
    m_neg_5_6(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 24px (1.5rem)
     */
    m_neg_6(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 256px (16rem)
     */
    m_neg_64(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 28px (1.75rem)
     */
    m_neg_7(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 288px (18rem)
     */
    m_neg_72(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 32px (2rem)
     */
    m_neg_8(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 320px (20rem)
     */
    m_neg_80(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 36px (2.25rem)
     */
    m_neg_9(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 384px (24rem)
     */
    m_neg_96(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 100%
     */
    m_neg_full(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 1px
     */
    m_neg_px(): Element;
    /**
     * Sets the margin of the element. [Docs](https://tailwindcss.com/docs/margin)
     *
     * 1px
     */
    m_px(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 0px
     */
    max_h_0(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 2px (0.125rem)
     */
    max_h_0p5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 4px (0.25rem)
     */
    max_h_1(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 40px (2.5rem)
     */
    max_h_10(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 44px (2.75rem)
     */
    max_h_11(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 448px (28rem)
     */
    max_h_112(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 48px (3rem)
     */
    max_h_12(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 512px (32rem)
     */
    max_h_128(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 64px (4rem)
     */
    max_h_16(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 8% (1/12)
     */
    max_h_1_12(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 50% (1/2)
     */
    max_h_1_2(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 33% (1/3)
     */
    max_h_1_3(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 25% (1/4)
     */
    max_h_1_4(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 20% (1/5)
     */
    max_h_1_5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 16% (1/6)
     */
    max_h_1_6(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 6px (0.375rem)
     */
    max_h_1p5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 8px (0.5rem)
     */
    max_h_2(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 80px (5rem)
     */
    max_h_20(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 96px (6rem)
     */
    max_h_24(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 66% (2/3)
     */
    max_h_2_3(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 50% (2/4)
     */
    max_h_2_4(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 40% (2/5)
     */
    max_h_2_5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 10px (0.625rem)
     */
    max_h_2p5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 12px (0.75rem)
     */
    max_h_3(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 128px (8rem)
     */
    max_h_32(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 75% (3/4)
     */
    max_h_3_4(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 60% (3/5)
     */
    max_h_3_5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 14px (0.875rem)
     */
    max_h_3p5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 16px (1rem)
     */
    max_h_4(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 160px (10rem)
     */
    max_h_40(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 192px (12rem)
     */
    max_h_48(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 80% (4/5)
     */
    max_h_4_5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 20px (1.25rem)
     */
    max_h_5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 224px (14rem)
     */
    max_h_56(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 80% (5/6)
     */
    max_h_5_6(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 24px (1.5rem)
     */
    max_h_6(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 256px (16rem)
     */
    max_h_64(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 28px (1.75rem)
     */
    max_h_7(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 288px (18rem)
     */
    max_h_72(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 32px (2rem)
     */
    max_h_8(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 320px (20rem)
     */
    max_h_80(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 36px (2.25rem)
     */
    max_h_9(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 384px (24rem)
     */
    max_h_96(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * Auto
     */
    max_h_auto(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 100%
     */
    max_h_full(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 0px
     */
    max_h_neg_0(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 2px (0.125rem)
     */
    max_h_neg_0p5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 4px (0.25rem)
     */
    max_h_neg_1(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 40px (2.5rem)
     */
    max_h_neg_10(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 44px (2.75rem)
     */
    max_h_neg_11(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 448px (28rem)
     */
    max_h_neg_112(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 48px (3rem)
     */
    max_h_neg_12(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 512px (32rem)
     */
    max_h_neg_128(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 64px (4rem)
     */
    max_h_neg_16(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 8% (1/12)
     */
    max_h_neg_1_12(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 50% (1/2)
     */
    max_h_neg_1_2(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 33% (1/3)
     */
    max_h_neg_1_3(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 25% (1/4)
     */
    max_h_neg_1_4(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 20% (1/5)
     */
    max_h_neg_1_5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 16% (1/6)
     */
    max_h_neg_1_6(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 6px (0.375rem)
     */
    max_h_neg_1p5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 8px (0.5rem)
     */
    max_h_neg_2(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 80px (5rem)
     */
    max_h_neg_20(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 96px (6rem)
     */
    max_h_neg_24(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 66% (2/3)
     */
    max_h_neg_2_3(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 50% (2/4)
     */
    max_h_neg_2_4(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 40% (2/5)
     */
    max_h_neg_2_5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 10px (0.625rem)
     */
    max_h_neg_2p5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 12px (0.75rem)
     */
    max_h_neg_3(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 128px (8rem)
     */
    max_h_neg_32(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 75% (3/4)
     */
    max_h_neg_3_4(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 60% (3/5)
     */
    max_h_neg_3_5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 14px (0.875rem)
     */
    max_h_neg_3p5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 16px (1rem)
     */
    max_h_neg_4(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 160px (10rem)
     */
    max_h_neg_40(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 192px (12rem)
     */
    max_h_neg_48(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 80% (4/5)
     */
    max_h_neg_4_5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 20px (1.25rem)
     */
    max_h_neg_5(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 224px (14rem)
     */
    max_h_neg_56(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 80% (5/6)
     */
    max_h_neg_5_6(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 24px (1.5rem)
     */
    max_h_neg_6(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 256px (16rem)
     */
    max_h_neg_64(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 28px (1.75rem)
     */
    max_h_neg_7(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 288px (18rem)
     */
    max_h_neg_72(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 32px (2rem)
     */
    max_h_neg_8(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 320px (20rem)
     */
    max_h_neg_80(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 36px (2.25rem)
     */
    max_h_neg_9(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 384px (24rem)
     */
    max_h_neg_96(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 100%
     */
    max_h_neg_full(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 1px
     */
    max_h_neg_px(): Element;
    /**
     * Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
     *
     * 1px
     */
    max_h_px(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 0px
     */
    max_size_0(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 2px (0.125rem)
     */
    max_size_0p5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 4px (0.25rem)
     */
    max_size_1(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 40px (2.5rem)
     */
    max_size_10(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 44px (2.75rem)
     */
    max_size_11(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 448px (28rem)
     */
    max_size_112(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 48px (3rem)
     */
    max_size_12(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 512px (32rem)
     */
    max_size_128(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 64px (4rem)
     */
    max_size_16(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 8% (1/12)
     */
    max_size_1_12(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 50% (1/2)
     */
    max_size_1_2(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 33% (1/3)
     */
    max_size_1_3(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 25% (1/4)
     */
    max_size_1_4(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 20% (1/5)
     */
    max_size_1_5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 16% (1/6)
     */
    max_size_1_6(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 6px (0.375rem)
     */
    max_size_1p5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 8px (0.5rem)
     */
    max_size_2(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 80px (5rem)
     */
    max_size_20(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 96px (6rem)
     */
    max_size_24(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 66% (2/3)
     */
    max_size_2_3(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 50% (2/4)
     */
    max_size_2_4(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 40% (2/5)
     */
    max_size_2_5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 10px (0.625rem)
     */
    max_size_2p5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 12px (0.75rem)
     */
    max_size_3(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 128px (8rem)
     */
    max_size_32(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 75% (3/4)
     */
    max_size_3_4(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 60% (3/5)
     */
    max_size_3_5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 14px (0.875rem)
     */
    max_size_3p5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 16px (1rem)
     */
    max_size_4(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 160px (10rem)
     */
    max_size_40(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 192px (12rem)
     */
    max_size_48(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 80% (4/5)
     */
    max_size_4_5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 20px (1.25rem)
     */
    max_size_5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 224px (14rem)
     */
    max_size_56(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 80% (5/6)
     */
    max_size_5_6(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 24px (1.5rem)
     */
    max_size_6(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 256px (16rem)
     */
    max_size_64(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 28px (1.75rem)
     */
    max_size_7(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 288px (18rem)
     */
    max_size_72(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 32px (2rem)
     */
    max_size_8(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 320px (20rem)
     */
    max_size_80(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 36px (2.25rem)
     */
    max_size_9(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 384px (24rem)
     */
    max_size_96(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * Auto
     */
    max_size_auto(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 100%
     */
    max_size_full(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 0px
     */
    max_size_neg_0(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 2px (0.125rem)
     */
    max_size_neg_0p5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 4px (0.25rem)
     */
    max_size_neg_1(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 40px (2.5rem)
     */
    max_size_neg_10(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 44px (2.75rem)
     */
    max_size_neg_11(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 448px (28rem)
     */
    max_size_neg_112(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 48px (3rem)
     */
    max_size_neg_12(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 512px (32rem)
     */
    max_size_neg_128(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 64px (4rem)
     */
    max_size_neg_16(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 8% (1/12)
     */
    max_size_neg_1_12(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 50% (1/2)
     */
    max_size_neg_1_2(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 33% (1/3)
     */
    max_size_neg_1_3(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 25% (1/4)
     */
    max_size_neg_1_4(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 20% (1/5)
     */
    max_size_neg_1_5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 16% (1/6)
     */
    max_size_neg_1_6(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 6px (0.375rem)
     */
    max_size_neg_1p5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 8px (0.5rem)
     */
    max_size_neg_2(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 80px (5rem)
     */
    max_size_neg_20(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 96px (6rem)
     */
    max_size_neg_24(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 66% (2/3)
     */
    max_size_neg_2_3(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 50% (2/4)
     */
    max_size_neg_2_4(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 40% (2/5)
     */
    max_size_neg_2_5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 10px (0.625rem)
     */
    max_size_neg_2p5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 12px (0.75rem)
     */
    max_size_neg_3(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 128px (8rem)
     */
    max_size_neg_32(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 75% (3/4)
     */
    max_size_neg_3_4(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 60% (3/5)
     */
    max_size_neg_3_5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 14px (0.875rem)
     */
    max_size_neg_3p5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 16px (1rem)
     */
    max_size_neg_4(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 160px (10rem)
     */
    max_size_neg_40(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 192px (12rem)
     */
    max_size_neg_48(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 80% (4/5)
     */
    max_size_neg_4_5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 20px (1.25rem)
     */
    max_size_neg_5(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 224px (14rem)
     */
    max_size_neg_56(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 80% (5/6)
     */
    max_size_neg_5_6(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 24px (1.5rem)
     */
    max_size_neg_6(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 256px (16rem)
     */
    max_size_neg_64(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 28px (1.75rem)
     */
    max_size_neg_7(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 288px (18rem)
     */
    max_size_neg_72(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 32px (2rem)
     */
    max_size_neg_8(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 320px (20rem)
     */
    max_size_neg_80(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 36px (2.25rem)
     */
    max_size_neg_9(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 384px (24rem)
     */
    max_size_neg_96(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 100%
     */
    max_size_neg_full(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 1px
     */
    max_size_neg_px(): Element;
    /**
     * Sets the maximum width and height of the element.
     *
     * 1px
     */
    max_size_px(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 0px
     */
    max_w_0(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 2px (0.125rem)
     */
    max_w_0p5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 4px (0.25rem)
     */
    max_w_1(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 40px (2.5rem)
     */
    max_w_10(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 44px (2.75rem)
     */
    max_w_11(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 448px (28rem)
     */
    max_w_112(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 48px (3rem)
     */
    max_w_12(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 512px (32rem)
     */
    max_w_128(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 64px (4rem)
     */
    max_w_16(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 8% (1/12)
     */
    max_w_1_12(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 50% (1/2)
     */
    max_w_1_2(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 33% (1/3)
     */
    max_w_1_3(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 25% (1/4)
     */
    max_w_1_4(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 20% (1/5)
     */
    max_w_1_5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 16% (1/6)
     */
    max_w_1_6(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 6px (0.375rem)
     */
    max_w_1p5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 8px (0.5rem)
     */
    max_w_2(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 80px (5rem)
     */
    max_w_20(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 96px (6rem)
     */
    max_w_24(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 66% (2/3)
     */
    max_w_2_3(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 50% (2/4)
     */
    max_w_2_4(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 40% (2/5)
     */
    max_w_2_5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 10px (0.625rem)
     */
    max_w_2p5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 12px (0.75rem)
     */
    max_w_3(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 128px (8rem)
     */
    max_w_32(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 75% (3/4)
     */
    max_w_3_4(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 60% (3/5)
     */
    max_w_3_5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 14px (0.875rem)
     */
    max_w_3p5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 16px (1rem)
     */
    max_w_4(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 160px (10rem)
     */
    max_w_40(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 192px (12rem)
     */
    max_w_48(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 80% (4/5)
     */
    max_w_4_5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 20px (1.25rem)
     */
    max_w_5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 224px (14rem)
     */
    max_w_56(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 80% (5/6)
     */
    max_w_5_6(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 24px (1.5rem)
     */
    max_w_6(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 256px (16rem)
     */
    max_w_64(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 28px (1.75rem)
     */
    max_w_7(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 288px (18rem)
     */
    max_w_72(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 32px (2rem)
     */
    max_w_8(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 320px (20rem)
     */
    max_w_80(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 36px (2.25rem)
     */
    max_w_9(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 384px (24rem)
     */
    max_w_96(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * Auto
     */
    max_w_auto(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 100%
     */
    max_w_full(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 0px
     */
    max_w_neg_0(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 2px (0.125rem)
     */
    max_w_neg_0p5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 4px (0.25rem)
     */
    max_w_neg_1(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 40px (2.5rem)
     */
    max_w_neg_10(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 44px (2.75rem)
     */
    max_w_neg_11(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 448px (28rem)
     */
    max_w_neg_112(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 48px (3rem)
     */
    max_w_neg_12(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 512px (32rem)
     */
    max_w_neg_128(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 64px (4rem)
     */
    max_w_neg_16(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 8% (1/12)
     */
    max_w_neg_1_12(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 50% (1/2)
     */
    max_w_neg_1_2(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 33% (1/3)
     */
    max_w_neg_1_3(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 25% (1/4)
     */
    max_w_neg_1_4(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 20% (1/5)
     */
    max_w_neg_1_5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 16% (1/6)
     */
    max_w_neg_1_6(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 6px (0.375rem)
     */
    max_w_neg_1p5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 8px (0.5rem)
     */
    max_w_neg_2(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 80px (5rem)
     */
    max_w_neg_20(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 96px (6rem)
     */
    max_w_neg_24(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 66% (2/3)
     */
    max_w_neg_2_3(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 50% (2/4)
     */
    max_w_neg_2_4(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 40% (2/5)
     */
    max_w_neg_2_5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 10px (0.625rem)
     */
    max_w_neg_2p5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 12px (0.75rem)
     */
    max_w_neg_3(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 128px (8rem)
     */
    max_w_neg_32(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 75% (3/4)
     */
    max_w_neg_3_4(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 60% (3/5)
     */
    max_w_neg_3_5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 14px (0.875rem)
     */
    max_w_neg_3p5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 16px (1rem)
     */
    max_w_neg_4(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 160px (10rem)
     */
    max_w_neg_40(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 192px (12rem)
     */
    max_w_neg_48(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 80% (4/5)
     */
    max_w_neg_4_5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 20px (1.25rem)
     */
    max_w_neg_5(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 224px (14rem)
     */
    max_w_neg_56(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 80% (5/6)
     */
    max_w_neg_5_6(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 24px (1.5rem)
     */
    max_w_neg_6(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 256px (16rem)
     */
    max_w_neg_64(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 28px (1.75rem)
     */
    max_w_neg_7(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 288px (18rem)
     */
    max_w_neg_72(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 32px (2rem)
     */
    max_w_neg_8(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 320px (20rem)
     */
    max_w_neg_80(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 36px (2.25rem)
     */
    max_w_neg_9(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 384px (24rem)
     */
    max_w_neg_96(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 100%
     */
    max_w_neg_full(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 1px
     */
    max_w_neg_px(): Element;
    /**
     * Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
     *
     * 1px
     */
    max_w_px(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 0px
     */
    mb_0(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    mb_0p5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    mb_1(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    mb_10(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    mb_11(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 448px (28rem)
     */
    mb_112(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 48px (3rem)
     */
    mb_12(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 512px (32rem)
     */
    mb_128(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 64px (4rem)
     */
    mb_16(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8% (1/12)
     */
    mb_1_12(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (1/2)
     */
    mb_1_2(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 33% (1/3)
     */
    mb_1_3(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 25% (1/4)
     */
    mb_1_4(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20% (1/5)
     */
    mb_1_5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16% (1/6)
     */
    mb_1_6(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    mb_1p5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    mb_2(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80px (5rem)
     */
    mb_20(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 96px (6rem)
     */
    mb_24(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 66% (2/3)
     */
    mb_2_3(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (2/4)
     */
    mb_2_4(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40% (2/5)
     */
    mb_2_5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    mb_2p5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    mb_3(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 128px (8rem)
     */
    mb_32(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 75% (3/4)
     */
    mb_3_4(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 60% (3/5)
     */
    mb_3_5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    mb_3p5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16px (1rem)
     */
    mb_4(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 160px (10rem)
     */
    mb_40(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 192px (12rem)
     */
    mb_48(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (4/5)
     */
    mb_4_5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    mb_5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 224px (14rem)
     */
    mb_56(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (5/6)
     */
    mb_5_6(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    mb_6(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 256px (16rem)
     */
    mb_64(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    mb_7(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 288px (18rem)
     */
    mb_72(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 32px (2rem)
     */
    mb_8(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 320px (20rem)
     */
    mb_80(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    mb_9(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 384px (24rem)
     */
    mb_96(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * Auto
     */
    mb_auto(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 100%
     */
    mb_full(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 0px
     */
    mb_neg_0(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    mb_neg_0p5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    mb_neg_1(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    mb_neg_10(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    mb_neg_11(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 448px (28rem)
     */
    mb_neg_112(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 48px (3rem)
     */
    mb_neg_12(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 512px (32rem)
     */
    mb_neg_128(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 64px (4rem)
     */
    mb_neg_16(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8% (1/12)
     */
    mb_neg_1_12(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (1/2)
     */
    mb_neg_1_2(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 33% (1/3)
     */
    mb_neg_1_3(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 25% (1/4)
     */
    mb_neg_1_4(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20% (1/5)
     */
    mb_neg_1_5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16% (1/6)
     */
    mb_neg_1_6(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    mb_neg_1p5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    mb_neg_2(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80px (5rem)
     */
    mb_neg_20(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 96px (6rem)
     */
    mb_neg_24(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 66% (2/3)
     */
    mb_neg_2_3(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (2/4)
     */
    mb_neg_2_4(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40% (2/5)
     */
    mb_neg_2_5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    mb_neg_2p5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    mb_neg_3(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 128px (8rem)
     */
    mb_neg_32(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 75% (3/4)
     */
    mb_neg_3_4(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 60% (3/5)
     */
    mb_neg_3_5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    mb_neg_3p5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16px (1rem)
     */
    mb_neg_4(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 160px (10rem)
     */
    mb_neg_40(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 192px (12rem)
     */
    mb_neg_48(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (4/5)
     */
    mb_neg_4_5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    mb_neg_5(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 224px (14rem)
     */
    mb_neg_56(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (5/6)
     */
    mb_neg_5_6(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    mb_neg_6(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 256px (16rem)
     */
    mb_neg_64(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    mb_neg_7(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 288px (18rem)
     */
    mb_neg_72(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 32px (2rem)
     */
    mb_neg_8(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 320px (20rem)
     */
    mb_neg_80(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    mb_neg_9(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 384px (24rem)
     */
    mb_neg_96(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 100%
     */
    mb_neg_full(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 1px
     */
    mb_neg_px(): Element;
    /**
     * Sets the bottom margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 1px
     */
    mb_px(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 0px
     */
    min_h_0(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 2px (0.125rem)
     */
    min_h_0p5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 4px (0.25rem)
     */
    min_h_1(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 40px (2.5rem)
     */
    min_h_10(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 44px (2.75rem)
     */
    min_h_11(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 448px (28rem)
     */
    min_h_112(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 48px (3rem)
     */
    min_h_12(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 512px (32rem)
     */
    min_h_128(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 64px (4rem)
     */
    min_h_16(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 8% (1/12)
     */
    min_h_1_12(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 50% (1/2)
     */
    min_h_1_2(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 33% (1/3)
     */
    min_h_1_3(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 25% (1/4)
     */
    min_h_1_4(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 20% (1/5)
     */
    min_h_1_5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 16% (1/6)
     */
    min_h_1_6(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 6px (0.375rem)
     */
    min_h_1p5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 8px (0.5rem)
     */
    min_h_2(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 80px (5rem)
     */
    min_h_20(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 96px (6rem)
     */
    min_h_24(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 66% (2/3)
     */
    min_h_2_3(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 50% (2/4)
     */
    min_h_2_4(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 40% (2/5)
     */
    min_h_2_5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 10px (0.625rem)
     */
    min_h_2p5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 12px (0.75rem)
     */
    min_h_3(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 128px (8rem)
     */
    min_h_32(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 75% (3/4)
     */
    min_h_3_4(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 60% (3/5)
     */
    min_h_3_5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 14px (0.875rem)
     */
    min_h_3p5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 16px (1rem)
     */
    min_h_4(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 160px (10rem)
     */
    min_h_40(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 192px (12rem)
     */
    min_h_48(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 80% (4/5)
     */
    min_h_4_5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 20px (1.25rem)
     */
    min_h_5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 224px (14rem)
     */
    min_h_56(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 80% (5/6)
     */
    min_h_5_6(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 24px (1.5rem)
     */
    min_h_6(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 256px (16rem)
     */
    min_h_64(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 28px (1.75rem)
     */
    min_h_7(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 288px (18rem)
     */
    min_h_72(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 32px (2rem)
     */
    min_h_8(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 320px (20rem)
     */
    min_h_80(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 36px (2.25rem)
     */
    min_h_9(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 384px (24rem)
     */
    min_h_96(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * Auto
     */
    min_h_auto(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 100%
     */
    min_h_full(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 0px
     */
    min_h_neg_0(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 2px (0.125rem)
     */
    min_h_neg_0p5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 4px (0.25rem)
     */
    min_h_neg_1(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 40px (2.5rem)
     */
    min_h_neg_10(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 44px (2.75rem)
     */
    min_h_neg_11(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 448px (28rem)
     */
    min_h_neg_112(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 48px (3rem)
     */
    min_h_neg_12(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 512px (32rem)
     */
    min_h_neg_128(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 64px (4rem)
     */
    min_h_neg_16(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 8% (1/12)
     */
    min_h_neg_1_12(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 50% (1/2)
     */
    min_h_neg_1_2(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 33% (1/3)
     */
    min_h_neg_1_3(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 25% (1/4)
     */
    min_h_neg_1_4(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 20% (1/5)
     */
    min_h_neg_1_5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 16% (1/6)
     */
    min_h_neg_1_6(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 6px (0.375rem)
     */
    min_h_neg_1p5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 8px (0.5rem)
     */
    min_h_neg_2(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 80px (5rem)
     */
    min_h_neg_20(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 96px (6rem)
     */
    min_h_neg_24(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 66% (2/3)
     */
    min_h_neg_2_3(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 50% (2/4)
     */
    min_h_neg_2_4(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 40% (2/5)
     */
    min_h_neg_2_5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 10px (0.625rem)
     */
    min_h_neg_2p5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 12px (0.75rem)
     */
    min_h_neg_3(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 128px (8rem)
     */
    min_h_neg_32(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 75% (3/4)
     */
    min_h_neg_3_4(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 60% (3/5)
     */
    min_h_neg_3_5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 14px (0.875rem)
     */
    min_h_neg_3p5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 16px (1rem)
     */
    min_h_neg_4(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 160px (10rem)
     */
    min_h_neg_40(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 192px (12rem)
     */
    min_h_neg_48(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 80% (4/5)
     */
    min_h_neg_4_5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 20px (1.25rem)
     */
    min_h_neg_5(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 224px (14rem)
     */
    min_h_neg_56(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 80% (5/6)
     */
    min_h_neg_5_6(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 24px (1.5rem)
     */
    min_h_neg_6(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 256px (16rem)
     */
    min_h_neg_64(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 28px (1.75rem)
     */
    min_h_neg_7(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 288px (18rem)
     */
    min_h_neg_72(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 32px (2rem)
     */
    min_h_neg_8(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 320px (20rem)
     */
    min_h_neg_80(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 36px (2.25rem)
     */
    min_h_neg_9(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 384px (24rem)
     */
    min_h_neg_96(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 100%
     */
    min_h_neg_full(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 1px
     */
    min_h_neg_px(): Element;
    /**
     * Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
     *
     * 1px
     */
    min_h_px(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 0px
     */
    min_size_0(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 2px (0.125rem)
     */
    min_size_0p5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 4px (0.25rem)
     */
    min_size_1(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 40px (2.5rem)
     */
    min_size_10(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 44px (2.75rem)
     */
    min_size_11(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 448px (28rem)
     */
    min_size_112(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 48px (3rem)
     */
    min_size_12(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 512px (32rem)
     */
    min_size_128(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 64px (4rem)
     */
    min_size_16(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 8% (1/12)
     */
    min_size_1_12(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 50% (1/2)
     */
    min_size_1_2(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 33% (1/3)
     */
    min_size_1_3(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 25% (1/4)
     */
    min_size_1_4(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 20% (1/5)
     */
    min_size_1_5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 16% (1/6)
     */
    min_size_1_6(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 6px (0.375rem)
     */
    min_size_1p5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 8px (0.5rem)
     */
    min_size_2(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 80px (5rem)
     */
    min_size_20(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 96px (6rem)
     */
    min_size_24(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 66% (2/3)
     */
    min_size_2_3(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 50% (2/4)
     */
    min_size_2_4(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 40% (2/5)
     */
    min_size_2_5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 10px (0.625rem)
     */
    min_size_2p5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 12px (0.75rem)
     */
    min_size_3(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 128px (8rem)
     */
    min_size_32(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 75% (3/4)
     */
    min_size_3_4(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 60% (3/5)
     */
    min_size_3_5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 14px (0.875rem)
     */
    min_size_3p5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 16px (1rem)
     */
    min_size_4(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 160px (10rem)
     */
    min_size_40(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 192px (12rem)
     */
    min_size_48(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 80% (4/5)
     */
    min_size_4_5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 20px (1.25rem)
     */
    min_size_5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 224px (14rem)
     */
    min_size_56(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 80% (5/6)
     */
    min_size_5_6(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 24px (1.5rem)
     */
    min_size_6(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 256px (16rem)
     */
    min_size_64(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 28px (1.75rem)
     */
    min_size_7(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 288px (18rem)
     */
    min_size_72(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 32px (2rem)
     */
    min_size_8(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 320px (20rem)
     */
    min_size_80(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 36px (2.25rem)
     */
    min_size_9(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 384px (24rem)
     */
    min_size_96(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * Auto
     */
    min_size_auto(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 100%
     */
    min_size_full(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 0px
     */
    min_size_neg_0(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 2px (0.125rem)
     */
    min_size_neg_0p5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 4px (0.25rem)
     */
    min_size_neg_1(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 40px (2.5rem)
     */
    min_size_neg_10(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 44px (2.75rem)
     */
    min_size_neg_11(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 448px (28rem)
     */
    min_size_neg_112(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 48px (3rem)
     */
    min_size_neg_12(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 512px (32rem)
     */
    min_size_neg_128(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 64px (4rem)
     */
    min_size_neg_16(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 8% (1/12)
     */
    min_size_neg_1_12(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 50% (1/2)
     */
    min_size_neg_1_2(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 33% (1/3)
     */
    min_size_neg_1_3(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 25% (1/4)
     */
    min_size_neg_1_4(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 20% (1/5)
     */
    min_size_neg_1_5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 16% (1/6)
     */
    min_size_neg_1_6(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 6px (0.375rem)
     */
    min_size_neg_1p5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 8px (0.5rem)
     */
    min_size_neg_2(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 80px (5rem)
     */
    min_size_neg_20(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 96px (6rem)
     */
    min_size_neg_24(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 66% (2/3)
     */
    min_size_neg_2_3(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 50% (2/4)
     */
    min_size_neg_2_4(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 40% (2/5)
     */
    min_size_neg_2_5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 10px (0.625rem)
     */
    min_size_neg_2p5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 12px (0.75rem)
     */
    min_size_neg_3(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 128px (8rem)
     */
    min_size_neg_32(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 75% (3/4)
     */
    min_size_neg_3_4(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 60% (3/5)
     */
    min_size_neg_3_5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 14px (0.875rem)
     */
    min_size_neg_3p5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 16px (1rem)
     */
    min_size_neg_4(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 160px (10rem)
     */
    min_size_neg_40(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 192px (12rem)
     */
    min_size_neg_48(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 80% (4/5)
     */
    min_size_neg_4_5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 20px (1.25rem)
     */
    min_size_neg_5(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 224px (14rem)
     */
    min_size_neg_56(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 80% (5/6)
     */
    min_size_neg_5_6(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 24px (1.5rem)
     */
    min_size_neg_6(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 256px (16rem)
     */
    min_size_neg_64(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 28px (1.75rem)
     */
    min_size_neg_7(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 288px (18rem)
     */
    min_size_neg_72(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 32px (2rem)
     */
    min_size_neg_8(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 320px (20rem)
     */
    min_size_neg_80(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 36px (2.25rem)
     */
    min_size_neg_9(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 384px (24rem)
     */
    min_size_neg_96(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 100%
     */
    min_size_neg_full(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 1px
     */
    min_size_neg_px(): Element;
    /**
     * Sets the minimum width and height of the element.
     *
     * 1px
     */
    min_size_px(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 0px
     */
    min_w_0(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 2px (0.125rem)
     */
    min_w_0p5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 4px (0.25rem)
     */
    min_w_1(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 40px (2.5rem)
     */
    min_w_10(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 44px (2.75rem)
     */
    min_w_11(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 448px (28rem)
     */
    min_w_112(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 48px (3rem)
     */
    min_w_12(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 512px (32rem)
     */
    min_w_128(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 64px (4rem)
     */
    min_w_16(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 8% (1/12)
     */
    min_w_1_12(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 50% (1/2)
     */
    min_w_1_2(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 33% (1/3)
     */
    min_w_1_3(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 25% (1/4)
     */
    min_w_1_4(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 20% (1/5)
     */
    min_w_1_5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 16% (1/6)
     */
    min_w_1_6(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 6px (0.375rem)
     */
    min_w_1p5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 8px (0.5rem)
     */
    min_w_2(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 80px (5rem)
     */
    min_w_20(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 96px (6rem)
     */
    min_w_24(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 66% (2/3)
     */
    min_w_2_3(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 50% (2/4)
     */
    min_w_2_4(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 40% (2/5)
     */
    min_w_2_5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 10px (0.625rem)
     */
    min_w_2p5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 12px (0.75rem)
     */
    min_w_3(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 128px (8rem)
     */
    min_w_32(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 75% (3/4)
     */
    min_w_3_4(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 60% (3/5)
     */
    min_w_3_5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 14px (0.875rem)
     */
    min_w_3p5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 16px (1rem)
     */
    min_w_4(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 160px (10rem)
     */
    min_w_40(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 192px (12rem)
     */
    min_w_48(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 80% (4/5)
     */
    min_w_4_5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 20px (1.25rem)
     */
    min_w_5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 224px (14rem)
     */
    min_w_56(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 80% (5/6)
     */
    min_w_5_6(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 24px (1.5rem)
     */
    min_w_6(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 256px (16rem)
     */
    min_w_64(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 28px (1.75rem)
     */
    min_w_7(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 288px (18rem)
     */
    min_w_72(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 32px (2rem)
     */
    min_w_8(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 320px (20rem)
     */
    min_w_80(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 36px (2.25rem)
     */
    min_w_9(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 384px (24rem)
     */
    min_w_96(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * Auto
     */
    min_w_auto(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 100%
     */
    min_w_full(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 0px
     */
    min_w_neg_0(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 2px (0.125rem)
     */
    min_w_neg_0p5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 4px (0.25rem)
     */
    min_w_neg_1(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 40px (2.5rem)
     */
    min_w_neg_10(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 44px (2.75rem)
     */
    min_w_neg_11(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 448px (28rem)
     */
    min_w_neg_112(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 48px (3rem)
     */
    min_w_neg_12(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 512px (32rem)
     */
    min_w_neg_128(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 64px (4rem)
     */
    min_w_neg_16(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 8% (1/12)
     */
    min_w_neg_1_12(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 50% (1/2)
     */
    min_w_neg_1_2(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 33% (1/3)
     */
    min_w_neg_1_3(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 25% (1/4)
     */
    min_w_neg_1_4(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 20% (1/5)
     */
    min_w_neg_1_5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 16% (1/6)
     */
    min_w_neg_1_6(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 6px (0.375rem)
     */
    min_w_neg_1p5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 8px (0.5rem)
     */
    min_w_neg_2(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 80px (5rem)
     */
    min_w_neg_20(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 96px (6rem)
     */
    min_w_neg_24(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 66% (2/3)
     */
    min_w_neg_2_3(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 50% (2/4)
     */
    min_w_neg_2_4(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 40% (2/5)
     */
    min_w_neg_2_5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 10px (0.625rem)
     */
    min_w_neg_2p5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 12px (0.75rem)
     */
    min_w_neg_3(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 128px (8rem)
     */
    min_w_neg_32(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 75% (3/4)
     */
    min_w_neg_3_4(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 60% (3/5)
     */
    min_w_neg_3_5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 14px (0.875rem)
     */
    min_w_neg_3p5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 16px (1rem)
     */
    min_w_neg_4(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 160px (10rem)
     */
    min_w_neg_40(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 192px (12rem)
     */
    min_w_neg_48(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 80% (4/5)
     */
    min_w_neg_4_5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 20px (1.25rem)
     */
    min_w_neg_5(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 224px (14rem)
     */
    min_w_neg_56(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 80% (5/6)
     */
    min_w_neg_5_6(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 24px (1.5rem)
     */
    min_w_neg_6(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 256px (16rem)
     */
    min_w_neg_64(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 28px (1.75rem)
     */
    min_w_neg_7(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 288px (18rem)
     */
    min_w_neg_72(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 32px (2rem)
     */
    min_w_neg_8(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 320px (20rem)
     */
    min_w_neg_80(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 36px (2.25rem)
     */
    min_w_neg_9(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 384px (24rem)
     */
    min_w_neg_96(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 100%
     */
    min_w_neg_full(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 1px
     */
    min_w_neg_px(): Element;
    /**
     * Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
     *
     * 1px
     */
    min_w_px(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 0px
     */
    ml_0(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    ml_0p5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    ml_1(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    ml_10(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    ml_11(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 448px (28rem)
     */
    ml_112(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 48px (3rem)
     */
    ml_12(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 512px (32rem)
     */
    ml_128(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 64px (4rem)
     */
    ml_16(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8% (1/12)
     */
    ml_1_12(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (1/2)
     */
    ml_1_2(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 33% (1/3)
     */
    ml_1_3(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 25% (1/4)
     */
    ml_1_4(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20% (1/5)
     */
    ml_1_5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16% (1/6)
     */
    ml_1_6(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    ml_1p5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    ml_2(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80px (5rem)
     */
    ml_20(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 96px (6rem)
     */
    ml_24(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 66% (2/3)
     */
    ml_2_3(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (2/4)
     */
    ml_2_4(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40% (2/5)
     */
    ml_2_5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    ml_2p5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    ml_3(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 128px (8rem)
     */
    ml_32(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 75% (3/4)
     */
    ml_3_4(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 60% (3/5)
     */
    ml_3_5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    ml_3p5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16px (1rem)
     */
    ml_4(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 160px (10rem)
     */
    ml_40(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 192px (12rem)
     */
    ml_48(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (4/5)
     */
    ml_4_5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    ml_5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 224px (14rem)
     */
    ml_56(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (5/6)
     */
    ml_5_6(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    ml_6(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 256px (16rem)
     */
    ml_64(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    ml_7(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 288px (18rem)
     */
    ml_72(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 32px (2rem)
     */
    ml_8(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 320px (20rem)
     */
    ml_80(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    ml_9(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 384px (24rem)
     */
    ml_96(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * Auto
     */
    ml_auto(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 100%
     */
    ml_full(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 0px
     */
    ml_neg_0(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    ml_neg_0p5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    ml_neg_1(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    ml_neg_10(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    ml_neg_11(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 448px (28rem)
     */
    ml_neg_112(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 48px (3rem)
     */
    ml_neg_12(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 512px (32rem)
     */
    ml_neg_128(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 64px (4rem)
     */
    ml_neg_16(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8% (1/12)
     */
    ml_neg_1_12(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (1/2)
     */
    ml_neg_1_2(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 33% (1/3)
     */
    ml_neg_1_3(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 25% (1/4)
     */
    ml_neg_1_4(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20% (1/5)
     */
    ml_neg_1_5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16% (1/6)
     */
    ml_neg_1_6(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    ml_neg_1p5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    ml_neg_2(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80px (5rem)
     */
    ml_neg_20(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 96px (6rem)
     */
    ml_neg_24(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 66% (2/3)
     */
    ml_neg_2_3(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (2/4)
     */
    ml_neg_2_4(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40% (2/5)
     */
    ml_neg_2_5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    ml_neg_2p5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    ml_neg_3(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 128px (8rem)
     */
    ml_neg_32(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 75% (3/4)
     */
    ml_neg_3_4(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 60% (3/5)
     */
    ml_neg_3_5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    ml_neg_3p5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16px (1rem)
     */
    ml_neg_4(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 160px (10rem)
     */
    ml_neg_40(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 192px (12rem)
     */
    ml_neg_48(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (4/5)
     */
    ml_neg_4_5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    ml_neg_5(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 224px (14rem)
     */
    ml_neg_56(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (5/6)
     */
    ml_neg_5_6(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    ml_neg_6(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 256px (16rem)
     */
    ml_neg_64(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    ml_neg_7(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 288px (18rem)
     */
    ml_neg_72(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 32px (2rem)
     */
    ml_neg_8(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 320px (20rem)
     */
    ml_neg_80(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    ml_neg_9(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 384px (24rem)
     */
    ml_neg_96(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 100%
     */
    ml_neg_full(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 1px
     */
    ml_neg_px(): Element;
    /**
     * Sets the left margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 1px
     */
    ml_px(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 0px
     */
    mr_0(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    mr_0p5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    mr_1(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    mr_10(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    mr_11(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 448px (28rem)
     */
    mr_112(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 48px (3rem)
     */
    mr_12(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 512px (32rem)
     */
    mr_128(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 64px (4rem)
     */
    mr_16(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8% (1/12)
     */
    mr_1_12(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (1/2)
     */
    mr_1_2(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 33% (1/3)
     */
    mr_1_3(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 25% (1/4)
     */
    mr_1_4(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20% (1/5)
     */
    mr_1_5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16% (1/6)
     */
    mr_1_6(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    mr_1p5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    mr_2(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80px (5rem)
     */
    mr_20(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 96px (6rem)
     */
    mr_24(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 66% (2/3)
     */
    mr_2_3(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (2/4)
     */
    mr_2_4(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40% (2/5)
     */
    mr_2_5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    mr_2p5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    mr_3(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 128px (8rem)
     */
    mr_32(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 75% (3/4)
     */
    mr_3_4(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 60% (3/5)
     */
    mr_3_5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    mr_3p5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16px (1rem)
     */
    mr_4(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 160px (10rem)
     */
    mr_40(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 192px (12rem)
     */
    mr_48(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (4/5)
     */
    mr_4_5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    mr_5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 224px (14rem)
     */
    mr_56(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (5/6)
     */
    mr_5_6(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    mr_6(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 256px (16rem)
     */
    mr_64(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    mr_7(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 288px (18rem)
     */
    mr_72(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 32px (2rem)
     */
    mr_8(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 320px (20rem)
     */
    mr_80(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    mr_9(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 384px (24rem)
     */
    mr_96(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * Auto
     */
    mr_auto(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 100%
     */
    mr_full(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 0px
     */
    mr_neg_0(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    mr_neg_0p5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    mr_neg_1(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    mr_neg_10(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    mr_neg_11(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 448px (28rem)
     */
    mr_neg_112(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 48px (3rem)
     */
    mr_neg_12(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 512px (32rem)
     */
    mr_neg_128(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 64px (4rem)
     */
    mr_neg_16(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8% (1/12)
     */
    mr_neg_1_12(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (1/2)
     */
    mr_neg_1_2(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 33% (1/3)
     */
    mr_neg_1_3(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 25% (1/4)
     */
    mr_neg_1_4(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20% (1/5)
     */
    mr_neg_1_5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16% (1/6)
     */
    mr_neg_1_6(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    mr_neg_1p5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    mr_neg_2(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80px (5rem)
     */
    mr_neg_20(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 96px (6rem)
     */
    mr_neg_24(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 66% (2/3)
     */
    mr_neg_2_3(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (2/4)
     */
    mr_neg_2_4(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40% (2/5)
     */
    mr_neg_2_5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    mr_neg_2p5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    mr_neg_3(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 128px (8rem)
     */
    mr_neg_32(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 75% (3/4)
     */
    mr_neg_3_4(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 60% (3/5)
     */
    mr_neg_3_5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    mr_neg_3p5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16px (1rem)
     */
    mr_neg_4(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 160px (10rem)
     */
    mr_neg_40(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 192px (12rem)
     */
    mr_neg_48(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (4/5)
     */
    mr_neg_4_5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    mr_neg_5(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 224px (14rem)
     */
    mr_neg_56(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (5/6)
     */
    mr_neg_5_6(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    mr_neg_6(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 256px (16rem)
     */
    mr_neg_64(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    mr_neg_7(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 288px (18rem)
     */
    mr_neg_72(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 32px (2rem)
     */
    mr_neg_8(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 320px (20rem)
     */
    mr_neg_80(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    mr_neg_9(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 384px (24rem)
     */
    mr_neg_96(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 100%
     */
    mr_neg_full(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 1px
     */
    mr_neg_px(): Element;
    /**
     * Sets the right margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 1px
     */
    mr_px(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 0px
     */
    mt_0(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    mt_0p5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    mt_1(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    mt_10(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    mt_11(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 448px (28rem)
     */
    mt_112(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 48px (3rem)
     */
    mt_12(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 512px (32rem)
     */
    mt_128(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 64px (4rem)
     */
    mt_16(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8% (1/12)
     */
    mt_1_12(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (1/2)
     */
    mt_1_2(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 33% (1/3)
     */
    mt_1_3(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 25% (1/4)
     */
    mt_1_4(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20% (1/5)
     */
    mt_1_5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16% (1/6)
     */
    mt_1_6(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    mt_1p5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    mt_2(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80px (5rem)
     */
    mt_20(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 96px (6rem)
     */
    mt_24(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 66% (2/3)
     */
    mt_2_3(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (2/4)
     */
    mt_2_4(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40% (2/5)
     */
    mt_2_5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    mt_2p5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    mt_3(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 128px (8rem)
     */
    mt_32(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 75% (3/4)
     */
    mt_3_4(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 60% (3/5)
     */
    mt_3_5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    mt_3p5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16px (1rem)
     */
    mt_4(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 160px (10rem)
     */
    mt_40(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 192px (12rem)
     */
    mt_48(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (4/5)
     */
    mt_4_5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    mt_5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 224px (14rem)
     */
    mt_56(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (5/6)
     */
    mt_5_6(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    mt_6(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 256px (16rem)
     */
    mt_64(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    mt_7(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 288px (18rem)
     */
    mt_72(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 32px (2rem)
     */
    mt_8(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 320px (20rem)
     */
    mt_80(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    mt_9(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 384px (24rem)
     */
    mt_96(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * Auto
     */
    mt_auto(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 100%
     */
    mt_full(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 0px
     */
    mt_neg_0(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    mt_neg_0p5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    mt_neg_1(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    mt_neg_10(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    mt_neg_11(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 448px (28rem)
     */
    mt_neg_112(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 48px (3rem)
     */
    mt_neg_12(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 512px (32rem)
     */
    mt_neg_128(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 64px (4rem)
     */
    mt_neg_16(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8% (1/12)
     */
    mt_neg_1_12(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (1/2)
     */
    mt_neg_1_2(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 33% (1/3)
     */
    mt_neg_1_3(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 25% (1/4)
     */
    mt_neg_1_4(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20% (1/5)
     */
    mt_neg_1_5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16% (1/6)
     */
    mt_neg_1_6(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    mt_neg_1p5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    mt_neg_2(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80px (5rem)
     */
    mt_neg_20(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 96px (6rem)
     */
    mt_neg_24(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 66% (2/3)
     */
    mt_neg_2_3(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 50% (2/4)
     */
    mt_neg_2_4(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 40% (2/5)
     */
    mt_neg_2_5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    mt_neg_2p5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    mt_neg_3(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 128px (8rem)
     */
    mt_neg_32(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 75% (3/4)
     */
    mt_neg_3_4(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 60% (3/5)
     */
    mt_neg_3_5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    mt_neg_3p5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 16px (1rem)
     */
    mt_neg_4(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 160px (10rem)
     */
    mt_neg_40(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 192px (12rem)
     */
    mt_neg_48(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (4/5)
     */
    mt_neg_4_5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    mt_neg_5(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 224px (14rem)
     */
    mt_neg_56(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 80% (5/6)
     */
    mt_neg_5_6(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    mt_neg_6(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 256px (16rem)
     */
    mt_neg_64(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    mt_neg_7(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 288px (18rem)
     */
    mt_neg_72(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 32px (2rem)
     */
    mt_neg_8(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 320px (20rem)
     */
    mt_neg_80(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    mt_neg_9(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 384px (24rem)
     */
    mt_neg_96(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 100%
     */
    mt_neg_full(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 1px
     */
    mt_neg_px(): Element;
    /**
     * Sets the top margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-margin-to-a-single-side)
     *
     * 1px
     */
    mt_px(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 0px
     */
    mx_0(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 2px (0.125rem)
     */
    mx_0p5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 4px (0.25rem)
     */
    mx_1(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 40px (2.5rem)
     */
    mx_10(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 44px (2.75rem)
     */
    mx_11(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 448px (28rem)
     */
    mx_112(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 48px (3rem)
     */
    mx_12(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 512px (32rem)
     */
    mx_128(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 64px (4rem)
     */
    mx_16(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 8% (1/12)
     */
    mx_1_12(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 50% (1/2)
     */
    mx_1_2(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 33% (1/3)
     */
    mx_1_3(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 25% (1/4)
     */
    mx_1_4(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 20% (1/5)
     */
    mx_1_5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 16% (1/6)
     */
    mx_1_6(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 6px (0.375rem)
     */
    mx_1p5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 8px (0.5rem)
     */
    mx_2(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 80px (5rem)
     */
    mx_20(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 96px (6rem)
     */
    mx_24(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 66% (2/3)
     */
    mx_2_3(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 50% (2/4)
     */
    mx_2_4(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 40% (2/5)
     */
    mx_2_5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 10px (0.625rem)
     */
    mx_2p5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 12px (0.75rem)
     */
    mx_3(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 128px (8rem)
     */
    mx_32(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 75% (3/4)
     */
    mx_3_4(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 60% (3/5)
     */
    mx_3_5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 14px (0.875rem)
     */
    mx_3p5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 16px (1rem)
     */
    mx_4(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 160px (10rem)
     */
    mx_40(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 192px (12rem)
     */
    mx_48(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 80% (4/5)
     */
    mx_4_5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 20px (1.25rem)
     */
    mx_5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 224px (14rem)
     */
    mx_56(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 80% (5/6)
     */
    mx_5_6(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 24px (1.5rem)
     */
    mx_6(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 256px (16rem)
     */
    mx_64(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 28px (1.75rem)
     */
    mx_7(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 288px (18rem)
     */
    mx_72(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 32px (2rem)
     */
    mx_8(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 320px (20rem)
     */
    mx_80(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 36px (2.25rem)
     */
    mx_9(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 384px (24rem)
     */
    mx_96(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * Auto
     */
    mx_auto(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 100%
     */
    mx_full(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 0px
     */
    mx_neg_0(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 2px (0.125rem)
     */
    mx_neg_0p5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 4px (0.25rem)
     */
    mx_neg_1(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 40px (2.5rem)
     */
    mx_neg_10(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 44px (2.75rem)
     */
    mx_neg_11(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 448px (28rem)
     */
    mx_neg_112(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 48px (3rem)
     */
    mx_neg_12(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 512px (32rem)
     */
    mx_neg_128(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 64px (4rem)
     */
    mx_neg_16(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 8% (1/12)
     */
    mx_neg_1_12(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 50% (1/2)
     */
    mx_neg_1_2(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 33% (1/3)
     */
    mx_neg_1_3(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 25% (1/4)
     */
    mx_neg_1_4(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 20% (1/5)
     */
    mx_neg_1_5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 16% (1/6)
     */
    mx_neg_1_6(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 6px (0.375rem)
     */
    mx_neg_1p5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 8px (0.5rem)
     */
    mx_neg_2(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 80px (5rem)
     */
    mx_neg_20(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 96px (6rem)
     */
    mx_neg_24(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 66% (2/3)
     */
    mx_neg_2_3(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 50% (2/4)
     */
    mx_neg_2_4(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 40% (2/5)
     */
    mx_neg_2_5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 10px (0.625rem)
     */
    mx_neg_2p5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 12px (0.75rem)
     */
    mx_neg_3(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 128px (8rem)
     */
    mx_neg_32(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 75% (3/4)
     */
    mx_neg_3_4(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 60% (3/5)
     */
    mx_neg_3_5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 14px (0.875rem)
     */
    mx_neg_3p5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 16px (1rem)
     */
    mx_neg_4(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 160px (10rem)
     */
    mx_neg_40(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 192px (12rem)
     */
    mx_neg_48(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 80% (4/5)
     */
    mx_neg_4_5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 20px (1.25rem)
     */
    mx_neg_5(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 224px (14rem)
     */
    mx_neg_56(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 80% (5/6)
     */
    mx_neg_5_6(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 24px (1.5rem)
     */
    mx_neg_6(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 256px (16rem)
     */
    mx_neg_64(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 28px (1.75rem)
     */
    mx_neg_7(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 288px (18rem)
     */
    mx_neg_72(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 32px (2rem)
     */
    mx_neg_8(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 320px (20rem)
     */
    mx_neg_80(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 36px (2.25rem)
     */
    mx_neg_9(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 384px (24rem)
     */
    mx_neg_96(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 100%
     */
    mx_neg_full(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 1px
     */
    mx_neg_px(): Element;
    /**
     * Sets the horizontal margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-horizontal-margin)
     *
     * 1px
     */
    mx_px(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 0px
     */
    my_0(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 2px (0.125rem)
     */
    my_0p5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 4px (0.25rem)
     */
    my_1(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 40px (2.5rem)
     */
    my_10(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 44px (2.75rem)
     */
    my_11(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 448px (28rem)
     */
    my_112(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 48px (3rem)
     */
    my_12(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 512px (32rem)
     */
    my_128(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 64px (4rem)
     */
    my_16(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 8% (1/12)
     */
    my_1_12(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 50% (1/2)
     */
    my_1_2(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 33% (1/3)
     */
    my_1_3(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 25% (1/4)
     */
    my_1_4(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 20% (1/5)
     */
    my_1_5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 16% (1/6)
     */
    my_1_6(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 6px (0.375rem)
     */
    my_1p5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 8px (0.5rem)
     */
    my_2(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 80px (5rem)
     */
    my_20(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 96px (6rem)
     */
    my_24(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 66% (2/3)
     */
    my_2_3(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 50% (2/4)
     */
    my_2_4(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 40% (2/5)
     */
    my_2_5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 10px (0.625rem)
     */
    my_2p5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 12px (0.75rem)
     */
    my_3(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 128px (8rem)
     */
    my_32(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 75% (3/4)
     */
    my_3_4(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 60% (3/5)
     */
    my_3_5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 14px (0.875rem)
     */
    my_3p5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 16px (1rem)
     */
    my_4(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 160px (10rem)
     */
    my_40(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 192px (12rem)
     */
    my_48(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 80% (4/5)
     */
    my_4_5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 20px (1.25rem)
     */
    my_5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 224px (14rem)
     */
    my_56(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 80% (5/6)
     */
    my_5_6(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 24px (1.5rem)
     */
    my_6(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 256px (16rem)
     */
    my_64(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 28px (1.75rem)
     */
    my_7(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 288px (18rem)
     */
    my_72(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 32px (2rem)
     */
    my_8(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 320px (20rem)
     */
    my_80(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 36px (2.25rem)
     */
    my_9(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 384px (24rem)
     */
    my_96(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * Auto
     */
    my_auto(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 100%
     */
    my_full(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 0px
     */
    my_neg_0(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 2px (0.125rem)
     */
    my_neg_0p5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 4px (0.25rem)
     */
    my_neg_1(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 40px (2.5rem)
     */
    my_neg_10(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 44px (2.75rem)
     */
    my_neg_11(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 448px (28rem)
     */
    my_neg_112(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 48px (3rem)
     */
    my_neg_12(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 512px (32rem)
     */
    my_neg_128(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 64px (4rem)
     */
    my_neg_16(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 8% (1/12)
     */
    my_neg_1_12(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 50% (1/2)
     */
    my_neg_1_2(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 33% (1/3)
     */
    my_neg_1_3(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 25% (1/4)
     */
    my_neg_1_4(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 20% (1/5)
     */
    my_neg_1_5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 16% (1/6)
     */
    my_neg_1_6(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 6px (0.375rem)
     */
    my_neg_1p5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 8px (0.5rem)
     */
    my_neg_2(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 80px (5rem)
     */
    my_neg_20(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 96px (6rem)
     */
    my_neg_24(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 66% (2/3)
     */
    my_neg_2_3(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 50% (2/4)
     */
    my_neg_2_4(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 40% (2/5)
     */
    my_neg_2_5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 10px (0.625rem)
     */
    my_neg_2p5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 12px (0.75rem)
     */
    my_neg_3(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 128px (8rem)
     */
    my_neg_32(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 75% (3/4)
     */
    my_neg_3_4(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 60% (3/5)
     */
    my_neg_3_5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 14px (0.875rem)
     */
    my_neg_3p5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 16px (1rem)
     */
    my_neg_4(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 160px (10rem)
     */
    my_neg_40(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 192px (12rem)
     */
    my_neg_48(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 80% (4/5)
     */
    my_neg_4_5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 20px (1.25rem)
     */
    my_neg_5(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 224px (14rem)
     */
    my_neg_56(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 80% (5/6)
     */
    my_neg_5_6(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 24px (1.5rem)
     */
    my_neg_6(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 256px (16rem)
     */
    my_neg_64(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 28px (1.75rem)
     */
    my_neg_7(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 288px (18rem)
     */
    my_neg_72(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 32px (2rem)
     */
    my_neg_8(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 320px (20rem)
     */
    my_neg_80(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 36px (2.25rem)
     */
    my_neg_9(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 384px (24rem)
     */
    my_neg_96(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 100%
     */
    my_neg_full(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 1px
     */
    my_neg_px(): Element;
    /**
     * Sets the vertical margin of the element. [Docs](https://tailwindcss.com/docs/margin#add-vertical-margin)
     *
     * 1px
     */
    my_px(): Element;
    /**
     * Sets the font style of the element to normal (not italic).
     *
     * [Docs](https://tailwindcss.com/docs/font-style#displaying-text-normally)
     */
    not_italic(): Element;
    /**
     * Sets the behavior of content that overflows the container to be hidden.
     *
     * [Docs](https://tailwindcss.com/docs/overflow#hiding-content-that-overflows)
     */
    overflow_hidden(): Element;
    /**
     * Sets the behavior of content that overflows the container on the X axis to be hidden.
     *
     * [Docs](https://tailwindcss.com/docs/overflow#hiding-content-that-overflows)
     */
    overflow_x_hidden(): Element;
    /**
     * Sets the behavior of content that overflows the container on the Y axis to be hidden.
     *
     * [Docs](https://tailwindcss.com/docs/overflow#hiding-content-that-overflows)
     */
    overflow_y_hidden(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 0px
     */
    p_0(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 2px (0.125rem)
     */
    p_0p5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 4px (0.25rem)
     */
    p_1(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 40px (2.5rem)
     */
    p_10(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 44px (2.75rem)
     */
    p_11(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 448px (28rem)
     */
    p_112(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 48px (3rem)
     */
    p_12(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 512px (32rem)
     */
    p_128(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 64px (4rem)
     */
    p_16(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 8% (1/12)
     */
    p_1_12(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 50% (1/2)
     */
    p_1_2(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 33% (1/3)
     */
    p_1_3(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 25% (1/4)
     */
    p_1_4(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 20% (1/5)
     */
    p_1_5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 16% (1/6)
     */
    p_1_6(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 6px (0.375rem)
     */
    p_1p5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 8px (0.5rem)
     */
    p_2(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 80px (5rem)
     */
    p_20(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 96px (6rem)
     */
    p_24(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 66% (2/3)
     */
    p_2_3(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 50% (2/4)
     */
    p_2_4(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 40% (2/5)
     */
    p_2_5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 10px (0.625rem)
     */
    p_2p5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 12px (0.75rem)
     */
    p_3(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 128px (8rem)
     */
    p_32(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 75% (3/4)
     */
    p_3_4(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 60% (3/5)
     */
    p_3_5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 14px (0.875rem)
     */
    p_3p5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 16px (1rem)
     */
    p_4(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 160px (10rem)
     */
    p_40(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 192px (12rem)
     */
    p_48(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 80% (4/5)
     */
    p_4_5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 20px (1.25rem)
     */
    p_5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 224px (14rem)
     */
    p_56(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 80% (5/6)
     */
    p_5_6(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 24px (1.5rem)
     */
    p_6(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 256px (16rem)
     */
    p_64(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 28px (1.75rem)
     */
    p_7(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 288px (18rem)
     */
    p_72(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 32px (2rem)
     */
    p_8(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 320px (20rem)
     */
    p_80(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 36px (2.25rem)
     */
    p_9(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 384px (24rem)
     */
    p_96(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 100%
     */
    p_full(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 0px
     */
    p_neg_0(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 2px (0.125rem)
     */
    p_neg_0p5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 4px (0.25rem)
     */
    p_neg_1(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 40px (2.5rem)
     */
    p_neg_10(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 44px (2.75rem)
     */
    p_neg_11(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 448px (28rem)
     */
    p_neg_112(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 48px (3rem)
     */
    p_neg_12(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 512px (32rem)
     */
    p_neg_128(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 64px (4rem)
     */
    p_neg_16(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 8% (1/12)
     */
    p_neg_1_12(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 50% (1/2)
     */
    p_neg_1_2(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 33% (1/3)
     */
    p_neg_1_3(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 25% (1/4)
     */
    p_neg_1_4(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 20% (1/5)
     */
    p_neg_1_5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 16% (1/6)
     */
    p_neg_1_6(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 6px (0.375rem)
     */
    p_neg_1p5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 8px (0.5rem)
     */
    p_neg_2(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 80px (5rem)
     */
    p_neg_20(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 96px (6rem)
     */
    p_neg_24(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 66% (2/3)
     */
    p_neg_2_3(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 50% (2/4)
     */
    p_neg_2_4(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 40% (2/5)
     */
    p_neg_2_5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 10px (0.625rem)
     */
    p_neg_2p5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 12px (0.75rem)
     */
    p_neg_3(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 128px (8rem)
     */
    p_neg_32(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 75% (3/4)
     */
    p_neg_3_4(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 60% (3/5)
     */
    p_neg_3_5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 14px (0.875rem)
     */
    p_neg_3p5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 16px (1rem)
     */
    p_neg_4(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 160px (10rem)
     */
    p_neg_40(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 192px (12rem)
     */
    p_neg_48(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 80% (4/5)
     */
    p_neg_4_5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 20px (1.25rem)
     */
    p_neg_5(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 224px (14rem)
     */
    p_neg_56(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 80% (5/6)
     */
    p_neg_5_6(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 24px (1.5rem)
     */
    p_neg_6(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 256px (16rem)
     */
    p_neg_64(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 28px (1.75rem)
     */
    p_neg_7(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 288px (18rem)
     */
    p_neg_72(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 32px (2rem)
     */
    p_neg_8(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 320px (20rem)
     */
    p_neg_80(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 36px (2.25rem)
     */
    p_neg_9(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 384px (24rem)
     */
    p_neg_96(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 100%
     */
    p_neg_full(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 1px
     */
    p_neg_px(): Element;
    /**
     * Sets the padding of the element. [Docs](https://tailwindcss.com/docs/padding)
     *
     * 1px
     */
    p_px(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 0px
     */
    pb_0(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    pb_0p5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    pb_1(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    pb_10(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    pb_11(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 448px (28rem)
     */
    pb_112(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 48px (3rem)
     */
    pb_12(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 512px (32rem)
     */
    pb_128(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 64px (4rem)
     */
    pb_16(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8% (1/12)
     */
    pb_1_12(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (1/2)
     */
    pb_1_2(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 33% (1/3)
     */
    pb_1_3(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 25% (1/4)
     */
    pb_1_4(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20% (1/5)
     */
    pb_1_5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16% (1/6)
     */
    pb_1_6(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    pb_1p5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    pb_2(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80px (5rem)
     */
    pb_20(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 96px (6rem)
     */
    pb_24(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 66% (2/3)
     */
    pb_2_3(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (2/4)
     */
    pb_2_4(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40% (2/5)
     */
    pb_2_5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    pb_2p5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    pb_3(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 128px (8rem)
     */
    pb_32(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 75% (3/4)
     */
    pb_3_4(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 60% (3/5)
     */
    pb_3_5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    pb_3p5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16px (1rem)
     */
    pb_4(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 160px (10rem)
     */
    pb_40(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 192px (12rem)
     */
    pb_48(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (4/5)
     */
    pb_4_5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    pb_5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 224px (14rem)
     */
    pb_56(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (5/6)
     */
    pb_5_6(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    pb_6(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 256px (16rem)
     */
    pb_64(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    pb_7(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 288px (18rem)
     */
    pb_72(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 32px (2rem)
     */
    pb_8(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 320px (20rem)
     */
    pb_80(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    pb_9(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 384px (24rem)
     */
    pb_96(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 100%
     */
    pb_full(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 0px
     */
    pb_neg_0(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    pb_neg_0p5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    pb_neg_1(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    pb_neg_10(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    pb_neg_11(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 448px (28rem)
     */
    pb_neg_112(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 48px (3rem)
     */
    pb_neg_12(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 512px (32rem)
     */
    pb_neg_128(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 64px (4rem)
     */
    pb_neg_16(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8% (1/12)
     */
    pb_neg_1_12(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (1/2)
     */
    pb_neg_1_2(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 33% (1/3)
     */
    pb_neg_1_3(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 25% (1/4)
     */
    pb_neg_1_4(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20% (1/5)
     */
    pb_neg_1_5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16% (1/6)
     */
    pb_neg_1_6(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    pb_neg_1p5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    pb_neg_2(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80px (5rem)
     */
    pb_neg_20(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 96px (6rem)
     */
    pb_neg_24(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 66% (2/3)
     */
    pb_neg_2_3(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (2/4)
     */
    pb_neg_2_4(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40% (2/5)
     */
    pb_neg_2_5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    pb_neg_2p5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    pb_neg_3(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 128px (8rem)
     */
    pb_neg_32(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 75% (3/4)
     */
    pb_neg_3_4(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 60% (3/5)
     */
    pb_neg_3_5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    pb_neg_3p5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16px (1rem)
     */
    pb_neg_4(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 160px (10rem)
     */
    pb_neg_40(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 192px (12rem)
     */
    pb_neg_48(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (4/5)
     */
    pb_neg_4_5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    pb_neg_5(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 224px (14rem)
     */
    pb_neg_56(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (5/6)
     */
    pb_neg_5_6(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    pb_neg_6(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 256px (16rem)
     */
    pb_neg_64(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    pb_neg_7(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 288px (18rem)
     */
    pb_neg_72(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 32px (2rem)
     */
    pb_neg_8(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 320px (20rem)
     */
    pb_neg_80(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    pb_neg_9(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 384px (24rem)
     */
    pb_neg_96(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 100%
     */
    pb_neg_full(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 1px
     */
    pb_neg_px(): Element;
    /**
     * Sets the bottom padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 1px
     */
    pb_px(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 0px
     */
    pl_0(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    pl_0p5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    pl_1(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    pl_10(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    pl_11(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 448px (28rem)
     */
    pl_112(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 48px (3rem)
     */
    pl_12(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 512px (32rem)
     */
    pl_128(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 64px (4rem)
     */
    pl_16(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8% (1/12)
     */
    pl_1_12(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (1/2)
     */
    pl_1_2(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 33% (1/3)
     */
    pl_1_3(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 25% (1/4)
     */
    pl_1_4(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20% (1/5)
     */
    pl_1_5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16% (1/6)
     */
    pl_1_6(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    pl_1p5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    pl_2(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80px (5rem)
     */
    pl_20(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 96px (6rem)
     */
    pl_24(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 66% (2/3)
     */
    pl_2_3(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (2/4)
     */
    pl_2_4(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40% (2/5)
     */
    pl_2_5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    pl_2p5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    pl_3(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 128px (8rem)
     */
    pl_32(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 75% (3/4)
     */
    pl_3_4(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 60% (3/5)
     */
    pl_3_5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    pl_3p5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16px (1rem)
     */
    pl_4(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 160px (10rem)
     */
    pl_40(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 192px (12rem)
     */
    pl_48(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (4/5)
     */
    pl_4_5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    pl_5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 224px (14rem)
     */
    pl_56(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (5/6)
     */
    pl_5_6(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    pl_6(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 256px (16rem)
     */
    pl_64(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    pl_7(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 288px (18rem)
     */
    pl_72(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 32px (2rem)
     */
    pl_8(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 320px (20rem)
     */
    pl_80(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    pl_9(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 384px (24rem)
     */
    pl_96(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 100%
     */
    pl_full(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 0px
     */
    pl_neg_0(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    pl_neg_0p5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    pl_neg_1(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    pl_neg_10(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    pl_neg_11(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 448px (28rem)
     */
    pl_neg_112(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 48px (3rem)
     */
    pl_neg_12(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 512px (32rem)
     */
    pl_neg_128(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 64px (4rem)
     */
    pl_neg_16(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8% (1/12)
     */
    pl_neg_1_12(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (1/2)
     */
    pl_neg_1_2(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 33% (1/3)
     */
    pl_neg_1_3(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 25% (1/4)
     */
    pl_neg_1_4(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20% (1/5)
     */
    pl_neg_1_5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16% (1/6)
     */
    pl_neg_1_6(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    pl_neg_1p5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    pl_neg_2(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80px (5rem)
     */
    pl_neg_20(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 96px (6rem)
     */
    pl_neg_24(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 66% (2/3)
     */
    pl_neg_2_3(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (2/4)
     */
    pl_neg_2_4(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40% (2/5)
     */
    pl_neg_2_5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    pl_neg_2p5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    pl_neg_3(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 128px (8rem)
     */
    pl_neg_32(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 75% (3/4)
     */
    pl_neg_3_4(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 60% (3/5)
     */
    pl_neg_3_5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    pl_neg_3p5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16px (1rem)
     */
    pl_neg_4(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 160px (10rem)
     */
    pl_neg_40(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 192px (12rem)
     */
    pl_neg_48(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (4/5)
     */
    pl_neg_4_5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    pl_neg_5(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 224px (14rem)
     */
    pl_neg_56(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (5/6)
     */
    pl_neg_5_6(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    pl_neg_6(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 256px (16rem)
     */
    pl_neg_64(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    pl_neg_7(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 288px (18rem)
     */
    pl_neg_72(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 32px (2rem)
     */
    pl_neg_8(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 320px (20rem)
     */
    pl_neg_80(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    pl_neg_9(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 384px (24rem)
     */
    pl_neg_96(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 100%
     */
    pl_neg_full(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 1px
     */
    pl_neg_px(): Element;
    /**
     * Sets the left padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 1px
     */
    pl_px(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 0px
     */
    pr_0(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    pr_0p5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    pr_1(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    pr_10(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    pr_11(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 448px (28rem)
     */
    pr_112(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 48px (3rem)
     */
    pr_12(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 512px (32rem)
     */
    pr_128(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 64px (4rem)
     */
    pr_16(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8% (1/12)
     */
    pr_1_12(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (1/2)
     */
    pr_1_2(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 33% (1/3)
     */
    pr_1_3(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 25% (1/4)
     */
    pr_1_4(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20% (1/5)
     */
    pr_1_5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16% (1/6)
     */
    pr_1_6(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    pr_1p5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    pr_2(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80px (5rem)
     */
    pr_20(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 96px (6rem)
     */
    pr_24(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 66% (2/3)
     */
    pr_2_3(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (2/4)
     */
    pr_2_4(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40% (2/5)
     */
    pr_2_5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    pr_2p5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    pr_3(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 128px (8rem)
     */
    pr_32(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 75% (3/4)
     */
    pr_3_4(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 60% (3/5)
     */
    pr_3_5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    pr_3p5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16px (1rem)
     */
    pr_4(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 160px (10rem)
     */
    pr_40(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 192px (12rem)
     */
    pr_48(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (4/5)
     */
    pr_4_5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    pr_5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 224px (14rem)
     */
    pr_56(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (5/6)
     */
    pr_5_6(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    pr_6(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 256px (16rem)
     */
    pr_64(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    pr_7(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 288px (18rem)
     */
    pr_72(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 32px (2rem)
     */
    pr_8(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 320px (20rem)
     */
    pr_80(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    pr_9(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 384px (24rem)
     */
    pr_96(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 100%
     */
    pr_full(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 0px
     */
    pr_neg_0(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    pr_neg_0p5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    pr_neg_1(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    pr_neg_10(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    pr_neg_11(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 448px (28rem)
     */
    pr_neg_112(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 48px (3rem)
     */
    pr_neg_12(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 512px (32rem)
     */
    pr_neg_128(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 64px (4rem)
     */
    pr_neg_16(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8% (1/12)
     */
    pr_neg_1_12(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (1/2)
     */
    pr_neg_1_2(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 33% (1/3)
     */
    pr_neg_1_3(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 25% (1/4)
     */
    pr_neg_1_4(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20% (1/5)
     */
    pr_neg_1_5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16% (1/6)
     */
    pr_neg_1_6(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    pr_neg_1p5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    pr_neg_2(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80px (5rem)
     */
    pr_neg_20(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 96px (6rem)
     */
    pr_neg_24(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 66% (2/3)
     */
    pr_neg_2_3(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (2/4)
     */
    pr_neg_2_4(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40% (2/5)
     */
    pr_neg_2_5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    pr_neg_2p5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    pr_neg_3(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 128px (8rem)
     */
    pr_neg_32(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 75% (3/4)
     */
    pr_neg_3_4(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 60% (3/5)
     */
    pr_neg_3_5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    pr_neg_3p5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16px (1rem)
     */
    pr_neg_4(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 160px (10rem)
     */
    pr_neg_40(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 192px (12rem)
     */
    pr_neg_48(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (4/5)
     */
    pr_neg_4_5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    pr_neg_5(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 224px (14rem)
     */
    pr_neg_56(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (5/6)
     */
    pr_neg_5_6(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    pr_neg_6(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 256px (16rem)
     */
    pr_neg_64(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    pr_neg_7(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 288px (18rem)
     */
    pr_neg_72(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 32px (2rem)
     */
    pr_neg_8(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 320px (20rem)
     */
    pr_neg_80(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    pr_neg_9(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 384px (24rem)
     */
    pr_neg_96(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 100%
     */
    pr_neg_full(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 1px
     */
    pr_neg_px(): Element;
    /**
     * Sets the right padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 1px
     */
    pr_px(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 0px
     */
    pt_0(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    pt_0p5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    pt_1(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    pt_10(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    pt_11(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 448px (28rem)
     */
    pt_112(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 48px (3rem)
     */
    pt_12(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 512px (32rem)
     */
    pt_128(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 64px (4rem)
     */
    pt_16(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8% (1/12)
     */
    pt_1_12(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (1/2)
     */
    pt_1_2(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 33% (1/3)
     */
    pt_1_3(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 25% (1/4)
     */
    pt_1_4(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20% (1/5)
     */
    pt_1_5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16% (1/6)
     */
    pt_1_6(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    pt_1p5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    pt_2(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80px (5rem)
     */
    pt_20(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 96px (6rem)
     */
    pt_24(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 66% (2/3)
     */
    pt_2_3(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (2/4)
     */
    pt_2_4(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40% (2/5)
     */
    pt_2_5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    pt_2p5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    pt_3(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 128px (8rem)
     */
    pt_32(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 75% (3/4)
     */
    pt_3_4(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 60% (3/5)
     */
    pt_3_5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    pt_3p5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16px (1rem)
     */
    pt_4(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 160px (10rem)
     */
    pt_40(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 192px (12rem)
     */
    pt_48(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (4/5)
     */
    pt_4_5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    pt_5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 224px (14rem)
     */
    pt_56(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (5/6)
     */
    pt_5_6(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    pt_6(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 256px (16rem)
     */
    pt_64(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    pt_7(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 288px (18rem)
     */
    pt_72(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 32px (2rem)
     */
    pt_8(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 320px (20rem)
     */
    pt_80(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    pt_9(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 384px (24rem)
     */
    pt_96(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 100%
     */
    pt_full(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 0px
     */
    pt_neg_0(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 2px (0.125rem)
     */
    pt_neg_0p5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 4px (0.25rem)
     */
    pt_neg_1(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40px (2.5rem)
     */
    pt_neg_10(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 44px (2.75rem)
     */
    pt_neg_11(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 448px (28rem)
     */
    pt_neg_112(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 48px (3rem)
     */
    pt_neg_12(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 512px (32rem)
     */
    pt_neg_128(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 64px (4rem)
     */
    pt_neg_16(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8% (1/12)
     */
    pt_neg_1_12(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (1/2)
     */
    pt_neg_1_2(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 33% (1/3)
     */
    pt_neg_1_3(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 25% (1/4)
     */
    pt_neg_1_4(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20% (1/5)
     */
    pt_neg_1_5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16% (1/6)
     */
    pt_neg_1_6(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 6px (0.375rem)
     */
    pt_neg_1p5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 8px (0.5rem)
     */
    pt_neg_2(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80px (5rem)
     */
    pt_neg_20(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 96px (6rem)
     */
    pt_neg_24(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 66% (2/3)
     */
    pt_neg_2_3(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 50% (2/4)
     */
    pt_neg_2_4(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 40% (2/5)
     */
    pt_neg_2_5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 10px (0.625rem)
     */
    pt_neg_2p5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 12px (0.75rem)
     */
    pt_neg_3(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 128px (8rem)
     */
    pt_neg_32(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 75% (3/4)
     */
    pt_neg_3_4(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 60% (3/5)
     */
    pt_neg_3_5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 14px (0.875rem)
     */
    pt_neg_3p5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 16px (1rem)
     */
    pt_neg_4(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 160px (10rem)
     */
    pt_neg_40(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 192px (12rem)
     */
    pt_neg_48(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (4/5)
     */
    pt_neg_4_5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 20px (1.25rem)
     */
    pt_neg_5(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 224px (14rem)
     */
    pt_neg_56(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 80% (5/6)
     */
    pt_neg_5_6(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 24px (1.5rem)
     */
    pt_neg_6(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 256px (16rem)
     */
    pt_neg_64(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 28px (1.75rem)
     */
    pt_neg_7(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 288px (18rem)
     */
    pt_neg_72(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 32px (2rem)
     */
    pt_neg_8(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 320px (20rem)
     */
    pt_neg_80(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 36px (2.25rem)
     */
    pt_neg_9(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 384px (24rem)
     */
    pt_neg_96(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 100%
     */
    pt_neg_full(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 1px
     */
    pt_neg_px(): Element;
    /**
     * Sets the top padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-padding-to-a-single-side)
     *
     * 1px
     */
    pt_px(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 0px
     */
    px_0(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 2px (0.125rem)
     */
    px_0p5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 4px (0.25rem)
     */
    px_1(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 40px (2.5rem)
     */
    px_10(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 44px (2.75rem)
     */
    px_11(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 448px (28rem)
     */
    px_112(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 48px (3rem)
     */
    px_12(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 512px (32rem)
     */
    px_128(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 64px (4rem)
     */
    px_16(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 8% (1/12)
     */
    px_1_12(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 50% (1/2)
     */
    px_1_2(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 33% (1/3)
     */
    px_1_3(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 25% (1/4)
     */
    px_1_4(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 20% (1/5)
     */
    px_1_5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 16% (1/6)
     */
    px_1_6(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 6px (0.375rem)
     */
    px_1p5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 8px (0.5rem)
     */
    px_2(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 80px (5rem)
     */
    px_20(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 96px (6rem)
     */
    px_24(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 66% (2/3)
     */
    px_2_3(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 50% (2/4)
     */
    px_2_4(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 40% (2/5)
     */
    px_2_5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 10px (0.625rem)
     */
    px_2p5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 12px (0.75rem)
     */
    px_3(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 128px (8rem)
     */
    px_32(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 75% (3/4)
     */
    px_3_4(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 60% (3/5)
     */
    px_3_5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 14px (0.875rem)
     */
    px_3p5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 16px (1rem)
     */
    px_4(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 160px (10rem)
     */
    px_40(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 192px (12rem)
     */
    px_48(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 80% (4/5)
     */
    px_4_5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 20px (1.25rem)
     */
    px_5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 224px (14rem)
     */
    px_56(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 80% (5/6)
     */
    px_5_6(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 24px (1.5rem)
     */
    px_6(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 256px (16rem)
     */
    px_64(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 28px (1.75rem)
     */
    px_7(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 288px (18rem)
     */
    px_72(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 32px (2rem)
     */
    px_8(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 320px (20rem)
     */
    px_80(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 36px (2.25rem)
     */
    px_9(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 384px (24rem)
     */
    px_96(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 100%
     */
    px_full(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 0px
     */
    px_neg_0(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 2px (0.125rem)
     */
    px_neg_0p5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 4px (0.25rem)
     */
    px_neg_1(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 40px (2.5rem)
     */
    px_neg_10(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 44px (2.75rem)
     */
    px_neg_11(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 448px (28rem)
     */
    px_neg_112(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 48px (3rem)
     */
    px_neg_12(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 512px (32rem)
     */
    px_neg_128(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 64px (4rem)
     */
    px_neg_16(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 8% (1/12)
     */
    px_neg_1_12(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 50% (1/2)
     */
    px_neg_1_2(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 33% (1/3)
     */
    px_neg_1_3(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 25% (1/4)
     */
    px_neg_1_4(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 20% (1/5)
     */
    px_neg_1_5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 16% (1/6)
     */
    px_neg_1_6(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 6px (0.375rem)
     */
    px_neg_1p5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 8px (0.5rem)
     */
    px_neg_2(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 80px (5rem)
     */
    px_neg_20(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 96px (6rem)
     */
    px_neg_24(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 66% (2/3)
     */
    px_neg_2_3(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 50% (2/4)
     */
    px_neg_2_4(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 40% (2/5)
     */
    px_neg_2_5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 10px (0.625rem)
     */
    px_neg_2p5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 12px (0.75rem)
     */
    px_neg_3(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 128px (8rem)
     */
    px_neg_32(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 75% (3/4)
     */
    px_neg_3_4(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 60% (3/5)
     */
    px_neg_3_5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 14px (0.875rem)
     */
    px_neg_3p5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 16px (1rem)
     */
    px_neg_4(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 160px (10rem)
     */
    px_neg_40(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 192px (12rem)
     */
    px_neg_48(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 80% (4/5)
     */
    px_neg_4_5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 20px (1.25rem)
     */
    px_neg_5(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 224px (14rem)
     */
    px_neg_56(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 80% (5/6)
     */
    px_neg_5_6(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 24px (1.5rem)
     */
    px_neg_6(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 256px (16rem)
     */
    px_neg_64(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 28px (1.75rem)
     */
    px_neg_7(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 288px (18rem)
     */
    px_neg_72(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 32px (2rem)
     */
    px_neg_8(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 320px (20rem)
     */
    px_neg_80(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 36px (2.25rem)
     */
    px_neg_9(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 384px (24rem)
     */
    px_neg_96(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 100%
     */
    px_neg_full(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 1px
     */
    px_neg_px(): Element;
    /**
     * Sets the horizontal padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-horizontal-padding)
     *
     * 1px
     */
    px_px(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 0px
     */
    py_0(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 2px (0.125rem)
     */
    py_0p5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 4px (0.25rem)
     */
    py_1(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 40px (2.5rem)
     */
    py_10(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 44px (2.75rem)
     */
    py_11(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 448px (28rem)
     */
    py_112(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 48px (3rem)
     */
    py_12(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 512px (32rem)
     */
    py_128(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 64px (4rem)
     */
    py_16(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 8% (1/12)
     */
    py_1_12(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 50% (1/2)
     */
    py_1_2(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 33% (1/3)
     */
    py_1_3(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 25% (1/4)
     */
    py_1_4(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 20% (1/5)
     */
    py_1_5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 16% (1/6)
     */
    py_1_6(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 6px (0.375rem)
     */
    py_1p5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 8px (0.5rem)
     */
    py_2(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 80px (5rem)
     */
    py_20(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 96px (6rem)
     */
    py_24(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 66% (2/3)
     */
    py_2_3(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 50% (2/4)
     */
    py_2_4(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 40% (2/5)
     */
    py_2_5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 10px (0.625rem)
     */
    py_2p5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 12px (0.75rem)
     */
    py_3(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 128px (8rem)
     */
    py_32(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 75% (3/4)
     */
    py_3_4(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 60% (3/5)
     */
    py_3_5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 14px (0.875rem)
     */
    py_3p5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 16px (1rem)
     */
    py_4(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 160px (10rem)
     */
    py_40(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 192px (12rem)
     */
    py_48(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 80% (4/5)
     */
    py_4_5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 20px (1.25rem)
     */
    py_5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 224px (14rem)
     */
    py_56(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 80% (5/6)
     */
    py_5_6(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 24px (1.5rem)
     */
    py_6(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 256px (16rem)
     */
    py_64(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 28px (1.75rem)
     */
    py_7(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 288px (18rem)
     */
    py_72(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 32px (2rem)
     */
    py_8(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 320px (20rem)
     */
    py_80(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 36px (2.25rem)
     */
    py_9(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 384px (24rem)
     */
    py_96(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 100%
     */
    py_full(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 0px
     */
    py_neg_0(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 2px (0.125rem)
     */
    py_neg_0p5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 4px (0.25rem)
     */
    py_neg_1(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 40px (2.5rem)
     */
    py_neg_10(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 44px (2.75rem)
     */
    py_neg_11(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 448px (28rem)
     */
    py_neg_112(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 48px (3rem)
     */
    py_neg_12(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 512px (32rem)
     */
    py_neg_128(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 64px (4rem)
     */
    py_neg_16(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 8% (1/12)
     */
    py_neg_1_12(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 50% (1/2)
     */
    py_neg_1_2(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 33% (1/3)
     */
    py_neg_1_3(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 25% (1/4)
     */
    py_neg_1_4(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 20% (1/5)
     */
    py_neg_1_5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 16% (1/6)
     */
    py_neg_1_6(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 6px (0.375rem)
     */
    py_neg_1p5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 8px (0.5rem)
     */
    py_neg_2(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 80px (5rem)
     */
    py_neg_20(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 96px (6rem)
     */
    py_neg_24(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 66% (2/3)
     */
    py_neg_2_3(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 50% (2/4)
     */
    py_neg_2_4(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 40% (2/5)
     */
    py_neg_2_5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 10px (0.625rem)
     */
    py_neg_2p5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 12px (0.75rem)
     */
    py_neg_3(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 128px (8rem)
     */
    py_neg_32(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 75% (3/4)
     */
    py_neg_3_4(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 60% (3/5)
     */
    py_neg_3_5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 14px (0.875rem)
     */
    py_neg_3p5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 16px (1rem)
     */
    py_neg_4(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 160px (10rem)
     */
    py_neg_40(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 192px (12rem)
     */
    py_neg_48(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 80% (4/5)
     */
    py_neg_4_5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 20px (1.25rem)
     */
    py_neg_5(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 224px (14rem)
     */
    py_neg_56(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 80% (5/6)
     */
    py_neg_5_6(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 24px (1.5rem)
     */
    py_neg_6(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 256px (16rem)
     */
    py_neg_64(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 28px (1.75rem)
     */
    py_neg_7(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 288px (18rem)
     */
    py_neg_72(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 32px (2rem)
     */
    py_neg_8(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 320px (20rem)
     */
    py_neg_80(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 36px (2.25rem)
     */
    py_neg_9(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 384px (24rem)
     */
    py_neg_96(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 100%
     */
    py_neg_full(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 1px
     */
    py_neg_px(): Element;
    /**
     * Sets the vertical padding of the element. [Docs](https://tailwindcss.com/docs/padding#add-vertical-padding)
     *
     * 1px
     */
    py_px(): Element;
    /**
     * Sets the position of the element to `relative`.
     *
     * [Docs](https://tailwindcss.com/docs/position)
     */
    relative(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 0px
     */
    right_0(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 2px (0.125rem)
     */
    right_0p5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 4px (0.25rem)
     */
    right_1(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40px (2.5rem)
     */
    right_10(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 44px (2.75rem)
     */
    right_11(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 448px (28rem)
     */
    right_112(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 48px (3rem)
     */
    right_12(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 512px (32rem)
     */
    right_128(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 64px (4rem)
     */
    right_16(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8% (1/12)
     */
    right_1_12(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (1/2)
     */
    right_1_2(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 33% (1/3)
     */
    right_1_3(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 25% (1/4)
     */
    right_1_4(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20% (1/5)
     */
    right_1_5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16% (1/6)
     */
    right_1_6(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 6px (0.375rem)
     */
    right_1p5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8px (0.5rem)
     */
    right_2(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80px (5rem)
     */
    right_20(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 96px (6rem)
     */
    right_24(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 66% (2/3)
     */
    right_2_3(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (2/4)
     */
    right_2_4(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40% (2/5)
     */
    right_2_5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 10px (0.625rem)
     */
    right_2p5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 12px (0.75rem)
     */
    right_3(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 128px (8rem)
     */
    right_32(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 75% (3/4)
     */
    right_3_4(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 60% (3/5)
     */
    right_3_5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 14px (0.875rem)
     */
    right_3p5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16px (1rem)
     */
    right_4(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 160px (10rem)
     */
    right_40(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 192px (12rem)
     */
    right_48(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (4/5)
     */
    right_4_5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20px (1.25rem)
     */
    right_5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 224px (14rem)
     */
    right_56(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (5/6)
     */
    right_5_6(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 24px (1.5rem)
     */
    right_6(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 256px (16rem)
     */
    right_64(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 28px (1.75rem)
     */
    right_7(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 288px (18rem)
     */
    right_72(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 32px (2rem)
     */
    right_8(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 320px (20rem)
     */
    right_80(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 36px (2.25rem)
     */
    right_9(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 384px (24rem)
     */
    right_96(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * Auto
     */
    right_auto(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 100%
     */
    right_full(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 0px
     */
    right_neg_0(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 2px (0.125rem)
     */
    right_neg_0p5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 4px (0.25rem)
     */
    right_neg_1(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40px (2.5rem)
     */
    right_neg_10(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 44px (2.75rem)
     */
    right_neg_11(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 448px (28rem)
     */
    right_neg_112(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 48px (3rem)
     */
    right_neg_12(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 512px (32rem)
     */
    right_neg_128(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 64px (4rem)
     */
    right_neg_16(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8% (1/12)
     */
    right_neg_1_12(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (1/2)
     */
    right_neg_1_2(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 33% (1/3)
     */
    right_neg_1_3(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 25% (1/4)
     */
    right_neg_1_4(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20% (1/5)
     */
    right_neg_1_5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16% (1/6)
     */
    right_neg_1_6(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 6px (0.375rem)
     */
    right_neg_1p5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8px (0.5rem)
     */
    right_neg_2(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80px (5rem)
     */
    right_neg_20(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 96px (6rem)
     */
    right_neg_24(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 66% (2/3)
     */
    right_neg_2_3(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (2/4)
     */
    right_neg_2_4(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40% (2/5)
     */
    right_neg_2_5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 10px (0.625rem)
     */
    right_neg_2p5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 12px (0.75rem)
     */
    right_neg_3(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 128px (8rem)
     */
    right_neg_32(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 75% (3/4)
     */
    right_neg_3_4(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 60% (3/5)
     */
    right_neg_3_5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 14px (0.875rem)
     */
    right_neg_3p5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16px (1rem)
     */
    right_neg_4(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 160px (10rem)
     */
    right_neg_40(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 192px (12rem)
     */
    right_neg_48(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (4/5)
     */
    right_neg_4_5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20px (1.25rem)
     */
    right_neg_5(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 224px (14rem)
     */
    right_neg_56(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (5/6)
     */
    right_neg_5_6(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 24px (1.5rem)
     */
    right_neg_6(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 256px (16rem)
     */
    right_neg_64(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 28px (1.75rem)
     */
    right_neg_7(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 288px (18rem)
     */
    right_neg_72(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 32px (2rem)
     */
    right_neg_8(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 320px (20rem)
     */
    right_neg_80(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 36px (2.25rem)
     */
    right_neg_9(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 384px (24rem)
     */
    right_neg_96(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 100%
     */
    right_neg_full(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 1px
     */
    right_neg_px(): Element;
    /**
     * Sets the right value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 1px
     */
    right_px(): Element;
    /**
     * Sets the border radius of the element. [Docs](https://tailwindcss.com/docs/border-radius)
     *
     * 16px (1rem)
     */
    rounded_2xl(): Element;
    /**
     * Sets the border radius of the element. [Docs](https://tailwindcss.com/docs/border-radius)
     *
     * 24px (1.5rem)
     */
    rounded_3xl(): Element;
    /**
     * Sets the border radius of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 16px (1rem)
     */
    rounded_b_2xl(): Element;
    /**
     * Sets the border radius of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 24px (1.5rem)
     */
    rounded_b_3xl(): Element;
    /**
     * Sets the border radius of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 9999px
     */
    rounded_b_full(): Element;
    /**
     * Sets the border radius of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 8px (0.5rem)
     */
    rounded_b_lg(): Element;
    /**
     * Sets the border radius of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 6px (0.375rem)
     */
    rounded_b_md(): Element;
    /**
     * Sets the border radius of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 0px
     */
    rounded_b_none(): Element;
    /**
     * Sets the border radius of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 4px (0.25rem)
     */
    rounded_b_sm(): Element;
    /**
     * Sets the border radius of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 12px (0.75rem)
     */
    rounded_b_xl(): Element;
    /**
     * Sets the border radius of the bottom side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 2px (0.125rem)
     */
    rounded_b_xs(): Element;
    /**
     * Sets the border radius of the bottom left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 16px (1rem)
     */
    rounded_bl_2xl(): Element;
    /**
     * Sets the border radius of the bottom left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 24px (1.5rem)
     */
    rounded_bl_3xl(): Element;
    /**
     * Sets the border radius of the bottom left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 9999px
     */
    rounded_bl_full(): Element;
    /**
     * Sets the border radius of the bottom left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 8px (0.5rem)
     */
    rounded_bl_lg(): Element;
    /**
     * Sets the border radius of the bottom left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 6px (0.375rem)
     */
    rounded_bl_md(): Element;
    /**
     * Sets the border radius of the bottom left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 0px
     */
    rounded_bl_none(): Element;
    /**
     * Sets the border radius of the bottom left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 4px (0.25rem)
     */
    rounded_bl_sm(): Element;
    /**
     * Sets the border radius of the bottom left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 12px (0.75rem)
     */
    rounded_bl_xl(): Element;
    /**
     * Sets the border radius of the bottom left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 2px (0.125rem)
     */
    rounded_bl_xs(): Element;
    /**
     * Sets the border radius of the bottom right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 16px (1rem)
     */
    rounded_br_2xl(): Element;
    /**
     * Sets the border radius of the bottom right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 24px (1.5rem)
     */
    rounded_br_3xl(): Element;
    /**
     * Sets the border radius of the bottom right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 9999px
     */
    rounded_br_full(): Element;
    /**
     * Sets the border radius of the bottom right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 8px (0.5rem)
     */
    rounded_br_lg(): Element;
    /**
     * Sets the border radius of the bottom right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 6px (0.375rem)
     */
    rounded_br_md(): Element;
    /**
     * Sets the border radius of the bottom right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 0px
     */
    rounded_br_none(): Element;
    /**
     * Sets the border radius of the bottom right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 4px (0.25rem)
     */
    rounded_br_sm(): Element;
    /**
     * Sets the border radius of the bottom right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 12px (0.75rem)
     */
    rounded_br_xl(): Element;
    /**
     * Sets the border radius of the bottom right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 2px (0.125rem)
     */
    rounded_br_xs(): Element;
    /**
     * Sets the border radius of the element. [Docs](https://tailwindcss.com/docs/border-radius)
     *
     * 9999px
     */
    rounded_full(): Element;
    /**
     * Sets the border radius of the left side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 16px (1rem)
     */
    rounded_l_2xl(): Element;
    /**
     * Sets the border radius of the left side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 24px (1.5rem)
     */
    rounded_l_3xl(): Element;
    /**
     * Sets the border radius of the left side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 9999px
     */
    rounded_l_full(): Element;
    /**
     * Sets the border radius of the left side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 8px (0.5rem)
     */
    rounded_l_lg(): Element;
    /**
     * Sets the border radius of the left side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 6px (0.375rem)
     */
    rounded_l_md(): Element;
    /**
     * Sets the border radius of the left side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 0px
     */
    rounded_l_none(): Element;
    /**
     * Sets the border radius of the left side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 4px (0.25rem)
     */
    rounded_l_sm(): Element;
    /**
     * Sets the border radius of the left side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 12px (0.75rem)
     */
    rounded_l_xl(): Element;
    /**
     * Sets the border radius of the left side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 2px (0.125rem)
     */
    rounded_l_xs(): Element;
    /**
     * Sets the border radius of the element. [Docs](https://tailwindcss.com/docs/border-radius)
     *
     * 8px (0.5rem)
     */
    rounded_lg(): Element;
    /**
     * Sets the border radius of the element. [Docs](https://tailwindcss.com/docs/border-radius)
     *
     * 6px (0.375rem)
     */
    rounded_md(): Element;
    /**
     * Sets the border radius of the element. [Docs](https://tailwindcss.com/docs/border-radius)
     *
     * 0px
     */
    rounded_none(): Element;
    /**
     * Sets the border radius of the right side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 16px (1rem)
     */
    rounded_r_2xl(): Element;
    /**
     * Sets the border radius of the right side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 24px (1.5rem)
     */
    rounded_r_3xl(): Element;
    /**
     * Sets the border radius of the right side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 9999px
     */
    rounded_r_full(): Element;
    /**
     * Sets the border radius of the right side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 8px (0.5rem)
     */
    rounded_r_lg(): Element;
    /**
     * Sets the border radius of the right side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 6px (0.375rem)
     */
    rounded_r_md(): Element;
    /**
     * Sets the border radius of the right side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 0px
     */
    rounded_r_none(): Element;
    /**
     * Sets the border radius of the right side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 4px (0.25rem)
     */
    rounded_r_sm(): Element;
    /**
     * Sets the border radius of the right side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 12px (0.75rem)
     */
    rounded_r_xl(): Element;
    /**
     * Sets the border radius of the right side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 2px (0.125rem)
     */
    rounded_r_xs(): Element;
    /**
     * Sets the border radius of the element. [Docs](https://tailwindcss.com/docs/border-radius)
     *
     * 4px (0.25rem)
     */
    rounded_sm(): Element;
    /**
     * Sets the border radius of the top side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 16px (1rem)
     */
    rounded_t_2xl(): Element;
    /**
     * Sets the border radius of the top side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 24px (1.5rem)
     */
    rounded_t_3xl(): Element;
    /**
     * Sets the border radius of the top side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 9999px
     */
    rounded_t_full(): Element;
    /**
     * Sets the border radius of the top side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 8px (0.5rem)
     */
    rounded_t_lg(): Element;
    /**
     * Sets the border radius of the top side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 6px (0.375rem)
     */
    rounded_t_md(): Element;
    /**
     * Sets the border radius of the top side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 0px
     */
    rounded_t_none(): Element;
    /**
     * Sets the border radius of the top side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 4px (0.25rem)
     */
    rounded_t_sm(): Element;
    /**
     * Sets the border radius of the top side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 12px (0.75rem)
     */
    rounded_t_xl(): Element;
    /**
     * Sets the border radius of the top side of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-sides-separately)
     *
     * 2px (0.125rem)
     */
    rounded_t_xs(): Element;
    /**
     * Sets the border radius of the top left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 16px (1rem)
     */
    rounded_tl_2xl(): Element;
    /**
     * Sets the border radius of the top left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 24px (1.5rem)
     */
    rounded_tl_3xl(): Element;
    /**
     * Sets the border radius of the top left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 9999px
     */
    rounded_tl_full(): Element;
    /**
     * Sets the border radius of the top left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 8px (0.5rem)
     */
    rounded_tl_lg(): Element;
    /**
     * Sets the border radius of the top left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 6px (0.375rem)
     */
    rounded_tl_md(): Element;
    /**
     * Sets the border radius of the top left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 0px
     */
    rounded_tl_none(): Element;
    /**
     * Sets the border radius of the top left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 4px (0.25rem)
     */
    rounded_tl_sm(): Element;
    /**
     * Sets the border radius of the top left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 12px (0.75rem)
     */
    rounded_tl_xl(): Element;
    /**
     * Sets the border radius of the top left corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 2px (0.125rem)
     */
    rounded_tl_xs(): Element;
    /**
     * Sets the border radius of the top right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 16px (1rem)
     */
    rounded_tr_2xl(): Element;
    /**
     * Sets the border radius of the top right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 24px (1.5rem)
     */
    rounded_tr_3xl(): Element;
    /**
     * Sets the border radius of the top right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 9999px
     */
    rounded_tr_full(): Element;
    /**
     * Sets the border radius of the top right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 8px (0.5rem)
     */
    rounded_tr_lg(): Element;
    /**
     * Sets the border radius of the top right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 6px (0.375rem)
     */
    rounded_tr_md(): Element;
    /**
     * Sets the border radius of the top right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 0px
     */
    rounded_tr_none(): Element;
    /**
     * Sets the border radius of the top right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 4px (0.25rem)
     */
    rounded_tr_sm(): Element;
    /**
     * Sets the border radius of the top right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 12px (0.75rem)
     */
    rounded_tr_xl(): Element;
    /**
     * Sets the border radius of the top right corner of the element. [Docs](https://tailwindcss.com/docs/border-radius#rounding-corners-separately)
     *
     * 2px (0.125rem)
     */
    rounded_tr_xs(): Element;
    /**
     * Sets the border radius of the element. [Docs](https://tailwindcss.com/docs/border-radius)
     *
     * 12px (0.75rem)
     */
    rounded_xl(): Element;
    /**
     * Sets the border radius of the element. [Docs](https://tailwindcss.com/docs/border-radius)
     *
     * 2px (0.125rem)
     */
    rounded_xs(): Element;
    /** Sets the row end of this element to "auto" */
    row_end_auto(): Element;
    /** Sets the row span of this element. */
    row_span_full(): Element;
    /** Sets the row start of this element to "auto" */
    row_start_auto(): Element;
    /**
     * Sets this element to align along the baseline of the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-self#baseline)
     */
    self_baseline(): Element;
    /**
     * Sets this element to align along the center of the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-self#center)
     */
    self_center(): Element;
    /**
     * Sets this element to align against the end of the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-self#end)
     */
    self_end(): Element;
    /**
     * Sets this element to align against the end of the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-self#end)
     */
    self_flex_end(): Element;
    /**
     * Sets this element to align against the start of the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-self#start)
     */
    self_flex_start(): Element;
    /**
     * Sets how this specific element is aligned along the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-self#start)
     */
    self_start(): Element;
    /**
     * Sets this element to stretch to fill the available space along the container's cross axis.
     *
     * [Docs](https://tailwindcss.com/docs/align-self#stretch)
     */
    self_stretch(): Element;
    /**
     * Sets the box shadow of the element.
     *
     * [Docs](https://tailwindcss.com/docs/box-shadow)
     */
    shadow_2xl(): Element;
    /**
     * Sets the box shadow of the element.
     *
     * [Docs](https://tailwindcss.com/docs/box-shadow)
     */
    shadow_2xs(): Element;
    /**
     * Sets the box shadow of the element.
     *
     * [Docs](https://tailwindcss.com/docs/box-shadow)
     */
    shadow_lg(): Element;
    /**
     * Sets the box shadow of the element.
     *
     * [Docs](https://tailwindcss.com/docs/box-shadow)
     */
    shadow_md(): Element;
    /**
     * Clears the box shadow of the element.
     *
     * [Docs](https://tailwindcss.com/docs/box-shadow)
     */
    shadow_none(): Element;
    /**
     * Sets the box shadow of the element.
     *
     * [Docs](https://tailwindcss.com/docs/box-shadow)
     */
    shadow_sm(): Element;
    /**
     * Sets the box shadow of the element.
     *
     * [Docs](https://tailwindcss.com/docs/box-shadow)
     */
    shadow_xl(): Element;
    /**
     * Sets the box shadow of the element.
     *
     * [Docs](https://tailwindcss.com/docs/box-shadow)
     */
    shadow_xs(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 0px
     */
    size_0(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 2px (0.125rem)
     */
    size_0p5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 4px (0.25rem)
     */
    size_1(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 40px (2.5rem)
     */
    size_10(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 44px (2.75rem)
     */
    size_11(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 448px (28rem)
     */
    size_112(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 48px (3rem)
     */
    size_12(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 512px (32rem)
     */
    size_128(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 64px (4rem)
     */
    size_16(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 8% (1/12)
     */
    size_1_12(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 50% (1/2)
     */
    size_1_2(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 33% (1/3)
     */
    size_1_3(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 25% (1/4)
     */
    size_1_4(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 20% (1/5)
     */
    size_1_5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 16% (1/6)
     */
    size_1_6(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 6px (0.375rem)
     */
    size_1p5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 8px (0.5rem)
     */
    size_2(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 80px (5rem)
     */
    size_20(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 96px (6rem)
     */
    size_24(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 66% (2/3)
     */
    size_2_3(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 50% (2/4)
     */
    size_2_4(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 40% (2/5)
     */
    size_2_5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 10px (0.625rem)
     */
    size_2p5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 12px (0.75rem)
     */
    size_3(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 128px (8rem)
     */
    size_32(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 75% (3/4)
     */
    size_3_4(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 60% (3/5)
     */
    size_3_5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 14px (0.875rem)
     */
    size_3p5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 16px (1rem)
     */
    size_4(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 160px (10rem)
     */
    size_40(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 192px (12rem)
     */
    size_48(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 80% (4/5)
     */
    size_4_5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 20px (1.25rem)
     */
    size_5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 224px (14rem)
     */
    size_56(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 80% (5/6)
     */
    size_5_6(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 24px (1.5rem)
     */
    size_6(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 256px (16rem)
     */
    size_64(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 28px (1.75rem)
     */
    size_7(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 288px (18rem)
     */
    size_72(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 32px (2rem)
     */
    size_8(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 320px (20rem)
     */
    size_80(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 36px (2.25rem)
     */
    size_9(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 384px (24rem)
     */
    size_96(): Element;
    /**
     * Sets the width and height of the element.
     *
     * Auto
     */
    size_auto(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 100%
     */
    size_full(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 0px
     */
    size_neg_0(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 2px (0.125rem)
     */
    size_neg_0p5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 4px (0.25rem)
     */
    size_neg_1(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 40px (2.5rem)
     */
    size_neg_10(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 44px (2.75rem)
     */
    size_neg_11(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 448px (28rem)
     */
    size_neg_112(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 48px (3rem)
     */
    size_neg_12(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 512px (32rem)
     */
    size_neg_128(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 64px (4rem)
     */
    size_neg_16(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 8% (1/12)
     */
    size_neg_1_12(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 50% (1/2)
     */
    size_neg_1_2(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 33% (1/3)
     */
    size_neg_1_3(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 25% (1/4)
     */
    size_neg_1_4(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 20% (1/5)
     */
    size_neg_1_5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 16% (1/6)
     */
    size_neg_1_6(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 6px (0.375rem)
     */
    size_neg_1p5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 8px (0.5rem)
     */
    size_neg_2(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 80px (5rem)
     */
    size_neg_20(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 96px (6rem)
     */
    size_neg_24(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 66% (2/3)
     */
    size_neg_2_3(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 50% (2/4)
     */
    size_neg_2_4(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 40% (2/5)
     */
    size_neg_2_5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 10px (0.625rem)
     */
    size_neg_2p5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 12px (0.75rem)
     */
    size_neg_3(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 128px (8rem)
     */
    size_neg_32(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 75% (3/4)
     */
    size_neg_3_4(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 60% (3/5)
     */
    size_neg_3_5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 14px (0.875rem)
     */
    size_neg_3p5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 16px (1rem)
     */
    size_neg_4(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 160px (10rem)
     */
    size_neg_40(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 192px (12rem)
     */
    size_neg_48(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 80% (4/5)
     */
    size_neg_4_5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 20px (1.25rem)
     */
    size_neg_5(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 224px (14rem)
     */
    size_neg_56(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 80% (5/6)
     */
    size_neg_5_6(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 24px (1.5rem)
     */
    size_neg_6(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 256px (16rem)
     */
    size_neg_64(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 28px (1.75rem)
     */
    size_neg_7(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 288px (18rem)
     */
    size_neg_72(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 32px (2rem)
     */
    size_neg_8(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 320px (20rem)
     */
    size_neg_80(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 36px (2.25rem)
     */
    size_neg_9(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 384px (24rem)
     */
    size_neg_96(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 100%
     */
    size_neg_full(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 1px
     */
    size_neg_px(): Element;
    /**
     * Sets the width and height of the element.
     *
     * 1px
     */
    size_px(): Element;
    /**
     * Sets the text size to 'extra extra large'.
     *
     * [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
     */
    text_2xl(): Element;
    /**
     * Sets the text size to 'extra extra extra large'.
     *
     * [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
     */
    text_3xl(): Element;
    /**
     * Sets the text size to 'base'.
     *
     * [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
     */
    text_base(): Element;
    /** Sets the text alignment to center */
    text_center(): Element;
    /**
     * Sets the text decoration to be 0px thick.
     *
     * [Docs](https://tailwindcss.com/docs/text-decoration-thickness)
     */
    text_decoration_0(): Element;
    /**
     * Sets the text decoration to be 1px thick.
     *
     * [Docs](https://tailwindcss.com/docs/text-decoration-thickness)
     */
    text_decoration_1(): Element;
    /**
     * Sets the text decoration to be 2px thick.
     *
     * [Docs](https://tailwindcss.com/docs/text-decoration-thickness)
     */
    text_decoration_2(): Element;
    /**
     * Sets the text decoration to be 4px thick.
     *
     * [Docs](https://tailwindcss.com/docs/text-decoration-thickness)
     */
    text_decoration_4(): Element;
    /**
     * Sets the text decoration to be 8px thick.
     *
     * [Docs](https://tailwindcss.com/docs/text-decoration-thickness)
     */
    text_decoration_8(): Element;
    /**
     * Removes the text decoration on this element.
     *
     * This value cascades to its child elements.
     */
    text_decoration_none(): Element;
    /**
     * Sets the text decoration style to a solid line.
     *
     * [Docs](https://tailwindcss.com/docs/text-decoration-style)
     */
    text_decoration_solid(): Element;
    /**
     * Sets the text decoration style to a wavy line.
     *
     * [Docs](https://tailwindcss.com/docs/text-decoration-style)
     */
    text_decoration_wavy(): Element;
    /**
     * Sets the truncate overflowing text with an ellipsis (…) at the end if needed.
     *
     * [Docs](https://tailwindcss.com/docs/text-overflow#ellipsis)
     */
    text_ellipsis(): Element;
    /**
     * Sets the truncate overflowing text with an ellipsis (…) in the middle if needed.
     *
     * Preserves the beginning and end of the text. Useful for filenames.
     *
     * Note: This doesn't exist in Tailwind CSS.
     */
    text_ellipsis_middle(): Element;
    /**
     * Sets the truncate overflowing text with an ellipsis (…) at the start if needed.
     *
     * Typically more adequate for file paths where the end is more important than the beginning.
     *
     * Note: This doesn't exist in Tailwind CSS.
     */
    text_ellipsis_start(): Element;
    /** Sets the text alignment to left */
    text_left(): Element;
    /**
     * Sets the text size to 'large'.
     *
     * [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
     */
    text_lg(): Element;
    /** Sets the text alignment to right */
    text_right(): Element;
    /**
     * Sets the text size to 'small'.
     *
     * [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
     */
    text_sm(): Element;
    /**
     * Sets the text size to 'extra large'.
     *
     * [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
     */
    text_xl(): Element;
    /**
     * Sets the text size to 'extra small'.
     *
     * [Docs](https://tailwindcss.com/docs/font-size#setting-the-font-size)
     */
    text_xs(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 0px
     */
    top_0(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 2px (0.125rem)
     */
    top_0p5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 4px (0.25rem)
     */
    top_1(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40px (2.5rem)
     */
    top_10(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 44px (2.75rem)
     */
    top_11(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 448px (28rem)
     */
    top_112(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 48px (3rem)
     */
    top_12(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 512px (32rem)
     */
    top_128(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 64px (4rem)
     */
    top_16(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8% (1/12)
     */
    top_1_12(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (1/2)
     */
    top_1_2(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 33% (1/3)
     */
    top_1_3(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 25% (1/4)
     */
    top_1_4(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20% (1/5)
     */
    top_1_5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16% (1/6)
     */
    top_1_6(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 6px (0.375rem)
     */
    top_1p5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8px (0.5rem)
     */
    top_2(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80px (5rem)
     */
    top_20(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 96px (6rem)
     */
    top_24(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 66% (2/3)
     */
    top_2_3(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (2/4)
     */
    top_2_4(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40% (2/5)
     */
    top_2_5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 10px (0.625rem)
     */
    top_2p5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 12px (0.75rem)
     */
    top_3(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 128px (8rem)
     */
    top_32(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 75% (3/4)
     */
    top_3_4(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 60% (3/5)
     */
    top_3_5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 14px (0.875rem)
     */
    top_3p5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16px (1rem)
     */
    top_4(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 160px (10rem)
     */
    top_40(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 192px (12rem)
     */
    top_48(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (4/5)
     */
    top_4_5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20px (1.25rem)
     */
    top_5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 224px (14rem)
     */
    top_56(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (5/6)
     */
    top_5_6(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 24px (1.5rem)
     */
    top_6(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 256px (16rem)
     */
    top_64(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 28px (1.75rem)
     */
    top_7(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 288px (18rem)
     */
    top_72(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 32px (2rem)
     */
    top_8(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 320px (20rem)
     */
    top_80(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 36px (2.25rem)
     */
    top_9(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 384px (24rem)
     */
    top_96(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * Auto
     */
    top_auto(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 100%
     */
    top_full(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 0px
     */
    top_neg_0(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 2px (0.125rem)
     */
    top_neg_0p5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 4px (0.25rem)
     */
    top_neg_1(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40px (2.5rem)
     */
    top_neg_10(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 44px (2.75rem)
     */
    top_neg_11(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 448px (28rem)
     */
    top_neg_112(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 48px (3rem)
     */
    top_neg_12(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 512px (32rem)
     */
    top_neg_128(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 64px (4rem)
     */
    top_neg_16(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8% (1/12)
     */
    top_neg_1_12(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (1/2)
     */
    top_neg_1_2(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 33% (1/3)
     */
    top_neg_1_3(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 25% (1/4)
     */
    top_neg_1_4(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20% (1/5)
     */
    top_neg_1_5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16% (1/6)
     */
    top_neg_1_6(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 6px (0.375rem)
     */
    top_neg_1p5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 8px (0.5rem)
     */
    top_neg_2(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80px (5rem)
     */
    top_neg_20(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 96px (6rem)
     */
    top_neg_24(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 66% (2/3)
     */
    top_neg_2_3(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 50% (2/4)
     */
    top_neg_2_4(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 40% (2/5)
     */
    top_neg_2_5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 10px (0.625rem)
     */
    top_neg_2p5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 12px (0.75rem)
     */
    top_neg_3(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 128px (8rem)
     */
    top_neg_32(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 75% (3/4)
     */
    top_neg_3_4(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 60% (3/5)
     */
    top_neg_3_5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 14px (0.875rem)
     */
    top_neg_3p5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 16px (1rem)
     */
    top_neg_4(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 160px (10rem)
     */
    top_neg_40(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 192px (12rem)
     */
    top_neg_48(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (4/5)
     */
    top_neg_4_5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 20px (1.25rem)
     */
    top_neg_5(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 224px (14rem)
     */
    top_neg_56(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 80% (5/6)
     */
    top_neg_5_6(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 24px (1.5rem)
     */
    top_neg_6(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 256px (16rem)
     */
    top_neg_64(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 28px (1.75rem)
     */
    top_neg_7(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 288px (18rem)
     */
    top_neg_72(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 32px (2rem)
     */
    top_neg_8(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 320px (20rem)
     */
    top_neg_80(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 36px (2.25rem)
     */
    top_neg_9(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 384px (24rem)
     */
    top_neg_96(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 100%
     */
    top_neg_full(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 1px
     */
    top_neg_px(): Element;
    /**
     * Sets the top value of a positioned element. [Docs](https://tailwindcss.com/docs/top-right-bottom-left)
     *
     * 1px
     */
    top_px(): Element;
    /**
     * Sets the truncate to prevent text from wrapping and truncate overflowing text with an ellipsis (…) if needed.
     *
     * [Docs](https://tailwindcss.com/docs/text-overflow#truncate)
     */
    truncate(): Element;
    /**
     * Sets the text decoration to underline.
     *
     * [Docs](https://tailwindcss.com/docs/text-decoration-line#underling-text)
     */
    underline(): Element;
    v_flex(): Element;
    /**
     * Sets the visibility of the element to `visible`.
     *
     * [Docs](https://tailwindcss.com/docs/visibility)
     */
    visible(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 0px
     */
    w_0(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 2px (0.125rem)
     */
    w_0p5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 4px (0.25rem)
     */
    w_1(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 40px (2.5rem)
     */
    w_10(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 44px (2.75rem)
     */
    w_11(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 448px (28rem)
     */
    w_112(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 48px (3rem)
     */
    w_12(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 512px (32rem)
     */
    w_128(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 64px (4rem)
     */
    w_16(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 8% (1/12)
     */
    w_1_12(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 50% (1/2)
     */
    w_1_2(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 33% (1/3)
     */
    w_1_3(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 25% (1/4)
     */
    w_1_4(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 20% (1/5)
     */
    w_1_5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 16% (1/6)
     */
    w_1_6(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 6px (0.375rem)
     */
    w_1p5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 8px (0.5rem)
     */
    w_2(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 80px (5rem)
     */
    w_20(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 96px (6rem)
     */
    w_24(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 66% (2/3)
     */
    w_2_3(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 50% (2/4)
     */
    w_2_4(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 40% (2/5)
     */
    w_2_5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 10px (0.625rem)
     */
    w_2p5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 12px (0.75rem)
     */
    w_3(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 128px (8rem)
     */
    w_32(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 75% (3/4)
     */
    w_3_4(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 60% (3/5)
     */
    w_3_5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 14px (0.875rem)
     */
    w_3p5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 16px (1rem)
     */
    w_4(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 160px (10rem)
     */
    w_40(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 192px (12rem)
     */
    w_48(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 80% (4/5)
     */
    w_4_5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 20px (1.25rem)
     */
    w_5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 224px (14rem)
     */
    w_56(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 80% (5/6)
     */
    w_5_6(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 24px (1.5rem)
     */
    w_6(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 256px (16rem)
     */
    w_64(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 28px (1.75rem)
     */
    w_7(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 288px (18rem)
     */
    w_72(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 32px (2rem)
     */
    w_8(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 320px (20rem)
     */
    w_80(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 36px (2.25rem)
     */
    w_9(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 384px (24rem)
     */
    w_96(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * Auto
     */
    w_auto(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 100%
     */
    w_full(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 0px
     */
    w_neg_0(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 2px (0.125rem)
     */
    w_neg_0p5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 4px (0.25rem)
     */
    w_neg_1(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 40px (2.5rem)
     */
    w_neg_10(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 44px (2.75rem)
     */
    w_neg_11(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 448px (28rem)
     */
    w_neg_112(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 48px (3rem)
     */
    w_neg_12(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 512px (32rem)
     */
    w_neg_128(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 64px (4rem)
     */
    w_neg_16(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 8% (1/12)
     */
    w_neg_1_12(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 50% (1/2)
     */
    w_neg_1_2(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 33% (1/3)
     */
    w_neg_1_3(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 25% (1/4)
     */
    w_neg_1_4(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 20% (1/5)
     */
    w_neg_1_5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 16% (1/6)
     */
    w_neg_1_6(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 6px (0.375rem)
     */
    w_neg_1p5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 8px (0.5rem)
     */
    w_neg_2(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 80px (5rem)
     */
    w_neg_20(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 96px (6rem)
     */
    w_neg_24(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 66% (2/3)
     */
    w_neg_2_3(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 50% (2/4)
     */
    w_neg_2_4(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 40% (2/5)
     */
    w_neg_2_5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 10px (0.625rem)
     */
    w_neg_2p5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 12px (0.75rem)
     */
    w_neg_3(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 128px (8rem)
     */
    w_neg_32(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 75% (3/4)
     */
    w_neg_3_4(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 60% (3/5)
     */
    w_neg_3_5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 14px (0.875rem)
     */
    w_neg_3p5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 16px (1rem)
     */
    w_neg_4(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 160px (10rem)
     */
    w_neg_40(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 192px (12rem)
     */
    w_neg_48(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 80% (4/5)
     */
    w_neg_4_5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 20px (1.25rem)
     */
    w_neg_5(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 224px (14rem)
     */
    w_neg_56(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 80% (5/6)
     */
    w_neg_5_6(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 24px (1.5rem)
     */
    w_neg_6(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 256px (16rem)
     */
    w_neg_64(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 28px (1.75rem)
     */
    w_neg_7(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 288px (18rem)
     */
    w_neg_72(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 32px (2rem)
     */
    w_neg_8(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 320px (20rem)
     */
    w_neg_80(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 36px (2.25rem)
     */
    w_neg_9(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 384px (24rem)
     */
    w_neg_96(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 100%
     */
    w_neg_full(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 1px
     */
    w_neg_px(): Element;
    /**
     * Sets the width of the element. [Docs](https://tailwindcss.com/docs/width)
     *
     * 1px
     */
    w_px(): Element;
    /**
     * Sets the whitespace of the element to `normal`.
     *
     * [Docs](https://tailwindcss.com/docs/whitespace#normal)
     */
    whitespace_normal(): Element;
    /**
     * Sets the whitespace of the element to `nowrap`.
     *
     * [Docs](https://tailwindcss.com/docs/whitespace#nowrap)
     */
    whitespace_nowrap(): Element;
  }

  /** An element with no layout of its own. */
  export function div(): Element;

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
   * A full-color image from the application's own directory.
   *
   * Unlike `svg`, this preserves the source image's colors instead of using it
   * as a theme-tinted icon mask. SVG, PNG, JPEG and other GPUI image formats
   * are supported by the host image loader.
   */
  export function image(path: string): Element;

  /** Immutable native GPUI geometry produced by `PathBuilder.build()`. */
  export interface Path {}
  export interface PathBuilder {
    move_to(x: import("gpui-shell").PathCoordinate, y: import("gpui-shell").PathCoordinate): PathBuilder;
    line_to(x: import("gpui-shell").PathCoordinate, y: import("gpui-shell").PathCoordinate): PathBuilder;
    curve_to(to_x: import("gpui-shell").PathCoordinate, to_y: import("gpui-shell").PathCoordinate, control_x: import("gpui-shell").PathCoordinate, control_y: import("gpui-shell").PathCoordinate): PathBuilder;
    cubic_bezier_to(to_x: import("gpui-shell").PathCoordinate, to_y: import("gpui-shell").PathCoordinate, control_a_x: import("gpui-shell").PathCoordinate, control_a_y: import("gpui-shell").PathCoordinate, control_b_x: import("gpui-shell").PathCoordinate, control_b_y: import("gpui-shell").PathCoordinate): PathBuilder;
    arc_to(radius_x: import("gpui-shell").PathCoordinate, radius_y: import("gpui-shell").PathCoordinate, rotation: number, large_arc: boolean, sweep: boolean, to_x: import("gpui-shell").PathCoordinate, to_y: import("gpui-shell").PathCoordinate): PathBuilder;
    add_polygon(points: ReadonlyArray<readonly [import("gpui-shell").PathCoordinate, import("gpui-shell").PathCoordinate]>, closed?: boolean): PathBuilder;
    close(): PathBuilder;
    dash_array(values: readonly number[]): PathBuilder;
    build(): Path;
  }
  export const PathBuilder: {
    fill(): PathBuilder;
    stroke(width: number): PathBuilder;
  };
  export interface BackgroundStop {}
  export interface Background {
    opacity(factor: number): Background;
    color_space(space: "srgb" | "oklab"): Background;
  }
  export const Background: {
    solid(color: Color): Background;
    stop(color: Color, percentage: number): BackgroundStop;
    linear_gradient(angle: number, from: Color | BackgroundStop, to: Color | BackgroundStop): Background;
    pattern_slash(color: Color, width: number, interval: number): Background;
    checkerboard(color: Color, size: number): Background;
  };

  /**
   * A focus target the script owns, created once and kept on the view.
   *
   * Focus is a fact about the window that outlives any one render, so an
   * element rebuilt every frame cannot own it. Hand the handle to an element
   * with `track_focus(...)`, and it is that element the keyboard means.
   *
   * Built with `cx.focus_handle()`, mirroring `App::focus_handle`.
   */
  export interface FocusHandle {
    /** Moves the keyboard onto the element tracking this handle. */
    focus(): void;
    /** Whether the element tracking this handle currently has the keyboard. */
    is_focused(): boolean;
    release(): boolean;
  }


  export interface Window {
    /**
     * Opens a dialog on the window's root, and answers the stack's new depth.
     *
     * Takes a **function returning an element**, not an element: an element
     * belongs to the render pass that built it, and a dialog outlives the call
     * that opened it. The function runs when the dialog draws, and again
     * whenever it redraws. Whatever it closes over is the dialog's state.
     *
     * Legal from an event handler or a task, not from `render`.
     */
    open_dialog(content: () => Element, options?: import("gpui-shell").DialogOptions): number;
    /** Closes the topmost dialog, and answers whether it found one. */
    close_dialog(): boolean;
    /** Closes every dialog, and answers how many it closed. */
    close_all_dialogs(): number;
    /** Whether any dialog is open. Legal from `render`, unlike the rest. */
    has_active_dialog(): boolean;

    /**
     * Opens the sheet on the right, replacing whatever was there. At most one is
     * ever open.
     */
    open_sheet(content: () => Element): void;
    /** The same, anchored at the `gpui-base` placement you name. */
    open_sheet_at(placement: import("gpui-base").Placement, content: () => Element): void;
    /** Closes the sheet, and answers whether one was open. */
    close_sheet(): boolean;
    /** Whether the sheet is open. Legal from `render`, unlike the rest. */
    has_active_sheet(): boolean;

    /** Posts a toast, and answers its id — the generated one when none was given. */
    push_toast(options: import("gpui-shell").ToastOptions): string;
    /** Retracts one toast by id, and answers whether it was still showing. */
    remove_toast(id: string): boolean;
    /** Retracts every toast, and answers how many it retracted. */
    clear_toasts(): number;

    /**
     * Key-value storage that survives a restart, backed by a file the host
     * placed. Needs the `storage` capability.
     */
    readonly localStorage: Storage;
    /**
     * Key-value storage for this run of the application, held in memory and
     * gone when the process exits. Needs no capability: a script that runs may
     * already hold its own memory.
     */
    readonly sessionStorage: Storage;

    /**
     * Paints immutable GPUI geometry with a reusable native `Background`.
     * `Window::paint_path`.
     *
     * The one element constructor reached through an object rather than as a
     * free function, and it is one because the thing it mirrors is a method on
     * the window rather than on the app. Legal from `render`, unlike the
     * overlays above — it builds a description like any other element.
     */
    paint_path(path: Path, background: Background | Color): Element;

    /**
     * `Window::dispatch_action`. Dispatches an action down this window's focus
     * path, reaching the same handlers a bound chord would.
     *
     * This is how a menu item or a toolbar button does what a keystroke does,
     * without either of them knowing about the other. Illegal from `render`.
     */
    dispatch_action(action: string): void;

    /**
     * `Window::rem_size`. The pixel value one `rem` currently means.
     *
     * Legal from `render`, like every measurement below it: a view that sizes
     * itself from the window has to ask during the pass that draws it.
     */
    rem_size(): number;
    /** `Window::line_height`, in pixels. */
    line_height(): number;
    /** `Window::viewport_size`: the drawable area, in pixels. */
    viewport_size(): Size;
    /** `Window::bounds`: where the window is on screen, and how big. */
    bounds(): import("gpui-shell").ElementBounds;
    /** `Window::mouse_position`, in window coordinates. */
    mouse_position(): Point;
    /**
     * `Window::appearance`, reduced to the two a script can draw for.
     *
     * GPUI reports four — each of light and dark has a vibrant variant — but
     * the difference is in how the platform paints *behind* the window, which
     * a script neither controls nor needs to branch on.
     */
    appearance(): "light" | "dark";
    /** `Window::is_window_active`: whether this window has the platform's focus. */
    is_window_active(): boolean;
    /** `Window::is_fullscreen`. */
    is_fullscreen(): boolean;
    /** `Window::is_maximized`. */
    is_maximized(): boolean;

    /**
     * `Window::set_rem_size`. Rescales everything expressed in rems.
     *
     * Illegal from `render`, as is everything below it: a frame that changes
     * the window it is drawing into is a frame arguing with itself. Call it
     * from an event handler or a task.
     */
    set_rem_size(size: number): void;
    /**
     * `Window::refresh`: redraw every view in this window, not just this one.
     *
     * The most expensive call on this object, and the one easiest to reach for.
     * Every view rebuilds -- retained children, charts, virtualized lists, the
     * lot -- so calling it where `cx.notify()` would do turns one view's update
     * into all of them, and calling it per incoming message turns a data feed
     * into a frame-rate problem. An application that pushed a quote through it
     * for each tick of a live market watchlist measured seven frames a second.
     *
     * Reach for it only when there is genuinely no view to notify:
     *
     * - `cx.notify()` repaints the view that owns the state that changed, which
     *   is almost always the right call.
     * - `handle.set_props(...)` repaints a nested view from its parent.
     * - A dock panel is the case that tempts you here, because a panel rebuilt
     *   by `DockArea.load` is not the instance the script created and
     *   `set_props` on the old handle reaches nothing. If you must refresh for
     *   that reason, coalesce: let a timer collect a burst into one call rather
     *   than making one per event.
     */
    refresh(): void;
    /** `Window::focus_next`: move the keyboard to the next tab stop. */
    focus_next(): void;
    /** `Window::focus_prev`: move it to the previous one. */
    focus_prev(): void;
    /** `Window::activate_window`: bring this window to the front. */
    activate_window(): void;
    /** `Window::minimize_window`. */
    minimize_window(): void;
    /** `Window::zoom_window`: the platform's zoom, not a scale factor. */
    zoom_window(): void;
    /** `Window::toggle_fullscreen`. */
    toggle_fullscreen(): void;
  }


  /** Key-value storage that survives a restart. Persisted on every write. */
  /**
   * The Web Storage API, as a browser implements it.
   *
   * Two instances exist and differ only in how long they last, which is the
   * same split Deno and Node arrived at when they needed persistent key-value
   * storage without a browser: `window.localStorage` is a file and survives a
   * restart, `window.sessionStorage` is memory and is gone when the process
   * exits.
   *
   * **Values are strings.** `setItem` converts whatever it is handed, exactly
   * as the browser does, so an object is stored as `"[object Object]"` unless
   * you `JSON.stringify` it — and reading it back is `JSON.parse`. That is not
   * an omission; it is the API this mirrors.
   *
   * Storage is per application. The host places the file, because an
   * application that could name its own storage location could name another
   * application's.
   */
  export interface Storage {
    /** How many keys are set. */
    readonly length: number;
    /** The key at `index` in a consistent order, or `null` past the end. */
    key(index: number): string | null;
    /** The value, or `null` when the key is unset. */
    getItem(key: string): string | null;
    /** Sets it, converting `value` to a string. */
    setItem(key: string, value: string): void;
    /** Removes it. Removing a key that is not set does nothing. */
    removeItem(key: string): void;
    /** Removes every key. */
    clear(): void;
    /**
     * Resolves once the pending write has reached disk.
     *
     * The one addition to the Web Storage surface, and only on
     * `localStorage`. A browser's `setItem` is durable by the time it returns;
     * this one hands the write to a background thread, so a script that must
     * know the bytes landed — before asking the host to exit, say — has
     * something to await. Ordinary code does not need it.
     */
    flush(): Promise<void>;
  }

  /** A running task. Cancelling one leaves its promise pending for ever. */
  export interface Task {
    cancel(): void;
    is_done(): boolean;
  }

  export interface Timer {
    /** Calls `handler(cx)` once, after `ms`. */
    after(ms: number, handler: (cx: AsyncContext) => void, opts?: import("gpui-shell").TaskOptions): Task;
    /**
     * Calls `handler(cx)` every `ms`. The interval is measured from the end of
     * one call, so a slow handler delays the next tick instead of stacking.
     */
    every(ms: number, handler: (cx: AsyncContext) => void, opts?: import("gpui-shell").TaskOptions): Task;
  }
}

declare module "gpui-base" {
  import {
    Color,
    Context,
    Element,
    FocusHandle,
    Placement,
  } from "gpui-kit";

  /** Every semantic color token the installed Base palette defines. */
  export type ColorToken =
    | "background"
    | "foreground"
    | "surface"
    | "surface_foreground"
    | "primary"
    | "primary_foreground"
    | "secondary"
    | "secondary_foreground"
    | "muted"
    | "muted_foreground"
    | "accent"
    | "accent_foreground"
    | "destructive"
    | "destructive_foreground"
    | "border"
    | "input"
    | "ring"
    ;


  /** One of the four edges used to place an element. Mirrors `gpui_base::Placement`. */
  export type Placement = "top" | "bottom" | "left" | "right";

  /** A component identified across renders by `new(id)`. */
  export interface ComponentType {
    new: (id: string | number) => Element;
  }

  /** A sub-part with no identity of its own, constructed with `new()`. */
  export interface PartType {
    new: () => Element;
  }

  /** A row. */
  export function h_flex(): Element;
  /** A column. */
  export function v_flex(): Element;

  /** Activation, focus, disabled and selected state. No styling. */
  export const Button: ComponentType;
  /** An external HTTP(S) resource opened through the system browser. */
  export const Link: ComponentType;
  /** A controlled toggle. No styling: draw the indicator yourself. */
  export const Checkbox: ComponentType;
  /** A controlled switch. No styling. */
  export const Switch: ComponentType;
  /**
   * A tab list. It holds no selection of its own — each `Tab` is told whether
   * it is selected, and reports activation through `on_click`, so the script
   * keeps the selected index in its own state.
   */
  export const Tabs: ComponentType;
  /** One tab. Controlled: `selected(...)` in, `on_click(...)` out. */
  export const Tab: ComponentType;
  /**
   * The progress root: the announcement, not the bar.
   *
   * It carries the progress role and the `0..=100` value a screen reader reads
   * out, and draws exactly what any other empty element draws — nothing. The
   * visible bar is a `ProgressTrack` you size and color, holding a
   * `ProgressIndicator` whose width you set from the same number you passed to
   * `value`. `Progress.new(...)` on its own puts nothing on screen.
   */
  export const Progress: ComponentType;
  /**
   * The groove. A plain element with your styles on it and no semantics of its
   * own: give it a width, a height and a background, and put the indicator in
   * it.
   */
  export const ProgressTrack: PartType;
  /**
   * The filled part. A plain element too — set its width from the percentage
   * you announced, and add `transition("width", ...)` if it should slide.
   */
  export const ProgressIndicator: PartType;
  /**
   * An avatar root. It renders its `image` slot, or its `fallback` slot when
   * there is no image, and never both.
   *
   * That choice is the whole of what it does. It draws no circle, no size and
   * no background, so the picture is yours: `w`, `h`, `rounded_full` and a
   * background go on the root, and the fallback is styled where it is written.
   *
   * ```js
   * Avatar.new().w(40).h(40).rounded_full().overflow_hidden()
   *   .image(AvatarImage.new("avatars/ada.png").size_full())
   *   .fallback(AvatarFallback.new().size_full().items_center().justify_center().child("AL"));
   * ```
   *
   * Ordinary children are drawn beside whichever slot won, which is where a
   * status dot or a badge goes.
   */
  export const Avatar: PartType;
  /**
   * The image slot: a picture from the application's own directory, at the
   * same kind of path `image(...)` takes.
   *
   * It is a slot type, not an element — used as an ordinary child it draws
   * nothing and says so in the log. Give it `size_full()` unless you want it at
   * its natural size.
   */
  export const AvatarImage: { new(path: string): Element };
  /**
   * The fallback slot: an ordinary box holding whatever stands in for the
   * image — initials, a shape, an `svg(...)`.
   *
   * A slot type like `AvatarImage`, and worth filling: an `Avatar` with an
   * image path that does not resolve has nothing else to show.
   */
  export const AvatarFallback: PartType;
  /**
   * A pagination root: a navigation landmark carrying the announced label, and
   * nothing on screen.
   *
   * The page buttons are yours. What base contributes that you cannot write
   * for yourself is which page numbers to show — that is `pagination_items`
   * below, a calculation rather than a component.
   *
   * ```js
   * Pagination.new("results").accessibility_label("Results").h_flex().gap_1().children(
   *   pagination_items(this.page, this.pages).map((item) =>
   *     item.ellipsis
   *       ? div().child("…")
   *       : Button.new(`page-${item.page}`)
   *           .selected(item.page === this.page)
   *           .on_click((_, cx) => { this.page = item.page; cx.notify(); })
   *           .child(String(item.page)),
   *   ),
   * );
   * ```
   */
  export const Pagination: ComponentType;
  /**
   * An accordion root: a group holding items, and nothing on screen.
   *
   * None of the five parts draws anything — no chevron, no border, no
   * animation, no layout. What they carry is what a screen reader reads: the
   * group, the heading and its level, the button and its expanded state, and
   * the region that button controls.
   *
   * The item owns `open` and passes it down to both the trigger and the panel,
   * so it is set once rather than three times in agreement with itself.
   *
   * ```js
   * Accordion.new("faq").child(
   *   AccordionItem.new()
   *     .open(this.open === "shipping")
   *     .header(
   *       AccordionHeader.new(
   *         AccordionTrigger.new("shipping-trigger")
   *           .on_change((open, cx) => { this.open = open ? "shipping" : null; cx.notify(); })
   *           .child("Shipping"),
   *       ).aria_level(3),
   *     )
   *     .panel(AccordionPanel.new().child("Two to five business days.")),
   * );
   * ```
   */
  export const Accordion: ComponentType;
  /**
   * One item. `open(...)` in, and the trigger's `on_change(...)` out.
   *
   * `disabled(true)` stops the trigger under it responding, whatever the
   * trigger itself says.
   */
  export const AccordionItem: PartType;
  /**
   * The heading that owns one item's trigger, which it takes at construction
   * for the same reason `Popup.new` takes its own: a heading whose button
   * arrived a frame later would announce nothing in between.
   *
   * `aria_level(n)` is what a screen reader reads out — "heading level 3" —
   * and defaults to 3. It announces; it does not size any text.
   */
  export const AccordionHeader: { new(trigger: Element): Element };
  /**
   * The region an item reveals. Left out of the tree entirely while shut,
   * unless `keep_mounted(true)` — which is how its content keeps a scroll
   * position or a half-typed field across a close and reopen.
   */
  export const AccordionPanel: PartType;
  /**
   * The button. It announces the item's expanded state and asks for the
   * opposite: `on_change` receives `true` when a shut item was pressed.
   *
   * `open` and `disabled` come from the item, so setting them here is
   * overwritten. Without an `on_change` nothing can open.
   */
  export const AccordionTrigger: ComponentType;
  /**
   * A calendar's month, and the date chosen in it. Retained: create it in
   * `init`, never in `render`.
   *
   * `month_days()` is why this exists — which dates fall in which week, where
   * the neighbouring months' days go, and how many weeks this month needs.
   * You draw the cells: a button per day, styled how you like.
   *
   * Base's `Calendar` element is deliberately not bound. It walks the same
   * grid calling a renderer once per cell — up to forty-two crossings into
   * JavaScript per frame, from inside GPUI's layout pass, for cells that carry
   * no behavior. Reading the grid here and drawing it yourself is the same
   * work without them.
   *
   * ```js
   * const grid = this.calendar.month_days()[0];
   * v_flex().children(grid.map((week) =>
   *   h_flex().children(week.map((day) =>
   *     Button.new(day)
   *       .selected(day === this.calendar.value())
   *       .on_click((_, cx) => { this.calendar.set_value(day); cx.notify(); })
   *       .child(String(Number(day.slice(8)))),
   *   )),
   * ));
   * ```
   *
   * Dates are `"YYYY-MM-DD"` — sortable as text, and readable by `new Date(s)`
   * when you need a weekday name or a localized month label.
   */
  export const CalendarState: { new(): CalendarStateHandle };
  /** A selected date: one day, a `[start, end]` range, or nothing. */
  export type CalendarDate = string | [string | null, string | null] | null;
  export interface CalendarStateHandle {
    /**
     * The grid, as months of weeks of days. One month unless base was asked
     * for more; each week is always seven days, and the first and last carry
     * the neighbouring months' days so the rows line up under their weekday
     * headings.
     */
    month_days(): string[][][];
    /** The year the grid is for. */
    year(): number;
    /** Its month, 1–12. */
    month(): number;
    /** Today, as the state read it when it was created. */
    today(): string;
    /** What is selected. */
    value(): CalendarDate;
    /** Selects a day, a range, or nothing. */
    set_value(next: CalendarDate): void;
    /** Moves the grid forward one month. Illegal from `render`. */
    next_month(): void;
    /** And back one. Illegal from `render`. */
    prev_month(): void;
    /**
     * `"change"` is the only event, and reports a date being selected. As
     * everywhere else, registering twice means the second handler.
     */
    on(event: "change", handler: (date: CalendarDate, cx: Context) => void): boolean;
    release(): boolean;
  }
  /**
   * Which page numbers to draw, and where the gaps fall.
   *
   * Keeps the first page, the last page and a window around the current one,
   * collapsing each broken run into an ellipsis. `visible_pages` defaults to
   * seven and is clamped to a minimum of five; a total of one page or fewer
   * answers an empty list, because a control for a single page is not one.
   *
   * An ellipsis names the pages it stands for, inclusive on both ends, so it
   * can be a "jump to" control rather than inert text.
   *
   * Legal from `render` — it reads nothing and is where the buttons are built.
   */
  export function pagination_items(
    current_page: number,
    total_pages: number,
    visible_pages?: number,
  ): PaginationEntry[];
  /** One entry of the page layout: a page, or a gap standing for a range. */
  export type PaginationEntry =
    | { page: number; ellipsis?: undefined }
    | { ellipsis: [first: number, last: number]; page?: undefined };
  /**
   * One option in a radio group. No styling: draw the dot yourself.
   *
   * Controlled: `checked(...)` in, `on_change(...)` out — but only ever `true`,
   * because a radio cannot deselect itself. The group lives in the script's own
   * state, and so does clearing it.
   */
  export const Radio: ComponentType;
  /**
   * A button that stays down. Controlled: `pressed(...)` in, `on_change(...)`
   * out, carrying the value the script would otherwise have to flip itself.
   *
   * No styling — an unstyled toggle is an invisible hit target with a button
   * role — so the pressed look is the script's, usually through
   * `.when(pressed, el => …)`.
   */
  export const Toggle: ComponentType;

  /**
   * A set of radios, announced as one group. It holds no selection — each
   * radio is told whether it is checked and reports the change back, so the
   * script keeps the chosen value in its own state.
   *
   * `axis` only changes what is announced; the group has no layout until the
   * script gives it one.
   */
  export const RadioGroup: ComponentType;
  /**
   * A set of toggles, announced as a toolbar. Like `RadioGroup` it holds no
   * state of its own, and its `axis` is announced rather than drawn.
   */
  export const ToggleGroup: ComponentType;

  /**
   * A semantic table root, composed the way HTML composes one: no data source
   * and no delegate, just the groups, rows and cells the script nests itself.
   * No styling — draw the grid, the padding and the header weight yourself.
   *
   * `row_count` and `column_count` describe the whole table, including rows the
   * script chose not to render. Give the root an `accessibility_label`; the
   * visual `TableCaption` below is not associated with it by assistive
   * technology.
   */
  export const Table: ComponentType;
  /** The header row group of a `Table`. */
  export const TableHeader: ComponentType;
  /** The body row group of a `Table`. */
  export const TableBody: ComponentType;
  /** One row. `TableRow.new(id, row_index)`, one-based. */
  export const TableRow: { new: (id: string | number, row_index: number) => Element };
  /** One column header. `TableHead.new(id, column_index)`, one-based. */
  export const TableHead: { new: (id: string | number, column_index: number) => Element };
  /** One data cell. `TableCell.new(id, column_index)`, one-based. */
  export const TableCell: { new: (id: string | number, column_index: number) => Element };
  /**
   * The visual slot a caption belongs in. It is an identified container and
   * nothing more: it carries no caption role, so assistive technology does not
   * tie it to the table. Name the `Table` root with `accessibility_label(...)`.
   */
  export const TableCaption: ComponentType;

  /**
   * A row of panes with draggable dividers between them. `v_resizable` is the
   * same thing stacked, and the axis is the constructor: there is no builder to
   * change it, because every panel inside is laid out from it.
   *
   * Children are `resizable_panel()` calls. Anything else is wrapped in a panel
   * with base's default constraints, which is convenient and lossy — a wrapped
   * element has no `size`, `size_range` or `visible` — so name the panels
   * whenever any of the three matters.
   *
   * The group has no size of its own: it fills whatever it is put in, exactly as
   * the Rust does, so give it a height (for `h_resizable`) or a width. Styles
   * written on it land on that frame.
   *
   * Panel sizes are the window's, not the script's. They are kept under the
   * group's id and survive every repaint, so a drag stays where the user put it
   * without any state on the view — and the id must therefore be a stable name,
   * not one built from a loop index.
   *
   * ```js
   * h_resizable("workspace").h(400)
   *   .child(resizable_panel().size(220).size_range(160, 320).child(sidebar))
   *   .child(resizable_panel().child(editor));
   * ```
   */
  export function h_resizable(id: string): Element;
  /** A column of panes with draggable dividers. See `h_resizable`. */
  export function v_resizable(id: string): Element;
  /**
   * One pane of an `h_resizable()` or `v_resizable()`, and only there: a panel
   * anywhere else throws when it is added, because its size and its drag handle
   * both belong to the group.
   *
   * Two method names mean something else here than they do anywhere else,
   * because base's panel has inherent builders that shadow the styles of the
   * same name — this reproduces that shadowing rather than inventing two new
   * words for it:
   *
   * - `size(pixels)` is the panel's initial size along the group's axis, not a
   *   width and a height. Use `w`/`h` for the cross axis.
   * - `visible(value)` is whether the panel is drawn at all, not the
   *   `visibility` style. A hidden panel keeps its place in the group, so its
   *   siblings' sizes are undisturbed while it is away. Default `true`.
   */
  export function resizable_panel(): Element;

  /**
   * A region whose `content` is materialized and rendered only while `open` is
   * true.
   *
   * That gating is the whole of it. Next to `div()` it adds one thing and
   * nothing else: no role, no announced expanded state, no chevron, no
   * animation and no trigger. Ordinary children are always rendered, so the
   * header goes there; the open state, the control that flips it and any
   * transition on the content are the script's own.
   */
  export const Collapsible: PartType;

  /**
   * A surface anchored to a trigger and opened by a press.
   *
   * It owns the press, the anchoring, the dismissal — outside press, Escape —
   * and the focus that moves into the surface and back out again. It draws
   * nothing: the trigger and the content are both elements you build and style,
   * given to `trigger(...)` and `content(...)`.
   *
   * Controlled the way a `Checkbox` is. Read `open(...)` in from your own
   * state, write it back from `on_open_change(...)`. Left uncontrolled, it
   * opens and closes itself from `default_open`, and the script never learns
   * where it got to.
   *
   * `track_focus(handle)` names what takes the keyboard when it opens — the
   * search field of a picker, say — instead of the surface itself.
   */
  export const Popover: ComponentType;
  /**
   * A surface anchored to a trigger and opened by resting the pointer on it.
   *
   * It owns its own open state: there is no `open` to control and no press to
   * handle, only `open_delay` and `close_delay`. Both delays are milliseconds,
   * and the closing one is what lets the pointer cross the gap between the
   * trigger and the card without dismissing it.
   */
  export const HoverCard: ComponentType;

  /**
   * The bare anchored surface underneath `Popover`, for when the open state
   * already belongs to something else.
   *
   * It measures its trigger, pins the chosen corner of the content to it,
   * paints that content in a layer above the rest of the window and keeps it
   * clear of the window edges. It owns nothing else — no press handling, no
   * dismissal, no open state. That is the point: a `Select` already owns those,
   * and a `Popover` underneath it would be a second control fighting the first
   * for the same Escape key.
   *
   * The trigger is a constructor argument, because the trigger's bounds are
   * what the content is anchored to. Open and close it by filling the `content`
   * slot or leaving it empty:
   *
   * ```js
   * Popup.new("options", trigger).anchor("bottom_left")
   *   .when(this.open, el => el.content(v_flex().children(options)))
   * ```
   *
   * A popup is a real element, unlike `Popover`: styles, state styles, `role`
   * and `track_focus` all land on it.
   */
  export const Popup: {
    new: (id: string | number, trigger: Element) => Element;
  };

  /**
   * A combobox root: the semantics and the keyboard, none of the picture.
   *
   * It holds no options and no selected value. What it owns is the combobox
   * role, the announced expanded state, the controlled `open` state, and the
   * transfer of the keyboard between the trigger and the list. Everything on
   * screen is yours — put the trigger and a `Popup` holding the list inside it
   * as ordinary children.
   *
   * Controlled the way a `Checkbox` is: `open(...)` in, `on_open_change(...)`
   * out. `track_focus(...)` names the trigger's focus handle and
   * `content_focus_handle(...)` the list's; without the first, nothing on
   * screen has the keyboard and no key reaches the root at all.
   *
   * **Arrow-key navigation of an open list is yours to write.** Base opens the
   * list on ↑ / ↓ / Enter, moves the keyboard onto the content handle and then
   * expects whatever is inside to run the highlight from its own key bindings.
   * Nothing does that for you — but the pieces are here: put `on_key_down` on
   * the content element the keyboard was moved to and move your own highlight,
   * or bind ↑ / ↓ to actions under a `key_context` of your own. Out of the box
   * the pointer works, Escape closes, Enter and ↓ open, and the highlight does
   * not move; a control shipped that way looks keyboard-operable and is not.
   *
   * **The highlighted option marks itself.** GPUI puts the active descendant on
   * the option element rather than on the container, so the root cannot mark
   * one for you: call `aria_active_descendant()` on whichever option you drew
   * as highlighted, and give it a `role`.
   *
   * ```js
   * Select.new("country")
   *   .accessibility_label("Country")
   *   .open(this.open)
   *   .track_focus(this.trigger_focus)
   *   .content_focus_handle(this.list_focus)
   *   .on_open_change((open, cx) => { this.open = open; cx.notify(); })
   *   .child(
   *     Popup.new("country-list", trigger)
   *       .when(this.open, el => el.content(list)),
   *   );
   * ```
   */
  export const Select: ComponentType;
  /**
   * The same root, keyed and announced as a combobox whose trigger is an
   * editable field — a `Select` with a text input in front of it. Base forwards
   * every builder to `Select` verbatim, so everything above applies here,
   * including what is missing; the one difference is that it has no
   * `accessibility_label` of its own, so name it through the input.
   */
  export const Combobox: ComponentType;
  /**
   * A date-picker root: the combobox role, the announced open state, and the
   * trigger's place in the Tab order. **It holds no date** — the date lives
   * wherever you keep it, and the calendar you draw inside it is your own.
   *
   * The focus handle is a constructor argument because base requires it: the
   * picker takes the keyboard through that handle, and there is no builder to
   * supply one later. `DatePicker.new(id, handle)` throws without a live one.
   *
   * **Enter and Escape do not reach it.** Base's picker handles both actions
   * but sets no key context, and every key binding base installs is scoped to
   * one — so nothing matches the keystroke and `on_open_change` never fires.
   * Open and close it from a press on the trigger you drew instead, and treat
   * `on_open_change` as wired for the day that changes. A `Select` does not
   * have this problem; if you need the keyboard today, build the picker's
   * trigger and calendar inside one.
   */
  export const DatePicker: {
    new: (id: string | number, focus_handle: FocusHandle) => Element;
  };

  /** When a `Scrollbar` shows itself. */
  export type ScrollbarMode = "scrolling" | "hover" | "always";

  /**
   * A scrollbar you place yourself, driving the scroll area that carries the
   * same id.
   *
   * `overflow_y_scrollbar()` is the easy case: a bar along the edges of the
   * element that scrolls. This is the other one — a bar beside a fixed table
   * header, a bar spanning two panes, a bar for a list that paints none of its
   * own. The two halves are matched **by name**, and nothing checks the match
   * before it runs, so both are needed:
   *
   * ```js
   * v_flex().relative().h(240)
   *   .child(v_flex().id("watchlist").size_full().overflow_y_scroll().children(rows))
   *   .child(Scrollbar.vertical("watchlist").absolute().inset_0());
   * ```
   *
   * The area must be the one that actually scrolls: `.id(name)` together with
   * `overflow_scroll` / `overflow_x_scroll` / `overflow_y_scroll`. Not
   * `overflow_y_scrollbar`, which paints a bar of its own and shares nothing.
   * A bar that finds no such area is reported in the log rather than drawn
   * inert.
   *
   * The bar has no size or position of its own — it fills the element it is
   * put in, so that element is the one you place — and its colors come from
   * the theme.
   */
  export const Scrollbar: {
    /** Both axes. */
    new: (id: string | number) => Element;
    /** The horizontal bar alone. */
    horizontal: (id: string | number) => Element;
    /** The vertical bar alone. */
    vertical: (id: string | number) => Element;
  };

  /** The visible items, as a half-open `[start, end)` interval. */
  export interface ItemRange {
    start: number;
    end: number;
  }

  /**
   * A list that describes only what is on screen.
   *
   * `render(range, cx)` is called with the visible interval and returns one
   * element per item in it — so a ten-thousand-row list costs the script what a
   * twenty-row one costs. It is the only callback in this API that is not an
   * event handler, and the only one the host calls during a frame rather than
   * between them: GPUI decides which rows exist while it is laying the list
   * out, so the call happens from inside layout, twice per frame (once to
   * measure a representative row, once to place the visible ones).
   *
   * Two consequences follow from that, and both are enforced rather than
   * documented away:
   *
   * * **No handlers inside the renderer.** `on_click` and the rest throw if
   *   called there. Use `on_item_click` on the list — see its note for why.
   * * **No state inside the renderer.** `InputState.new()`, `cx.focus_handle()`
   *   and the rest throw there as they do in `render()`, and `cx.notify()` is
   *   refused: asking for a re-render from inside layout is a loop.
   *
   * What the layout pass does *not* cost you is the `cx` you already had: the
   * renderer is a closure inside `render(cx)`, so the row helpers written
   * against that `cx` — `label(text, cx)`, `surface(cx)` — keep working
   * unchanged. The `cx` the renderer is handed reaches the same window and app,
   * and is there for a renderer written somewhere `render`'s is not in scope.
   *
   * The list paints no scrollbar of its own. Pair one with it by name, exactly
   * as with a scroll area:
   *
   * ```js
   * v_flex().relative().h(400)
   *   .child(v_virtual_list("rows", this.rows.length, 28,
   *     (index) => this.rows[index].id,
   *     (range) => this.rows.slice(range.start, range.end).map(row => text(row.name))))
   *   .child(Scrollbar.vertical("rows").absolute().inset_0());
   * ```
   *
   * @param id      Identity, and the name a `Scrollbar` pairs with.
   * @param item_count How many items the collection has, visible or not.
   * @param item_sizes One number for a uniform extent, or one per item —
   *   heights for `v_virtual_list`, widths for `h_virtual_list`. Base takes a
   *   single vector whose *length* is also the count; splitting the two is a
   *   deliberate difference, because mirroring it would put one number per row
   *   across the language boundary on every render, and a uniform hundred
   *   thousand rows is the case worth making cheap. An array must be exactly
   *   `item_count` long.
   * @param get_key An item's stable domain key, from its current index. It is
   *   the row's element identity, and it is what `on_item_click` reports — so a
   *   click queued before a filter or a sort reordered the list still names the
   *   item whose box was pressed rather than whatever slid into that index.
   *   Required.
   * @param render  Called with the visible range; returns one element per item
   *   in it.
   */
  export function v_virtual_list(
    id: string | number,
    item_count: number,
    item_sizes: number | number[],
    get_key: (index: number) => string,
    render: (range: ItemRange, cx: Context) => Element[],
  ): Element;

  /** `v_virtual_list` along the other axis; `item_sizes` are widths. */
  export function h_virtual_list(
    id: string | number,
    item_count: number,
    item_sizes: number | number[],
    get_key: (index: number) => string,
    render: (range: ItemRange, cx: Context) => Element[],
  ): Element;

  /**
   * A virtual list's scroll position, kept across frames so the script can move
   * it. Create it in `init()` and hand it to the list with `track_scroll`.
   *
   * A list without one still scrolls, and a `Scrollbar` named after the list
   * still drives it; this is only needed to scroll it from code.
   */
  export interface VirtualListScrollHandle {
    /**
     * Puts the item at `index` on screen before the next frame. `"top"` (the
     * default) brings it to the near edge, `"center"` to the middle.
     */
    scroll_to_item(index: number, strategy?: "top" | "center"): void;
    scroll_to_bottom(): void;
    /** Releases the handle. Using it afterwards throws. */
    release(): boolean;
  }

  export const VirtualListScrollHandle: {
    new: () => VirtualListScrollHandle;
  };

  /** Payload emitted by retained text state. Submit events carry key modifiers. */
  export interface InputEvent {
    readonly secondary?: boolean;
    readonly shift?: boolean;
  }

  /** The OTP event payload is currently empty; read the value from the state. */
  export interface OtpEvent {}

  /**
   * Retained text state, created once and kept on the view.
   *
   * `InputState.new(...)` needs a live host call, so it belongs in `init` or in
   * an event handler — never in `render`.
   */
  export interface InputState {
    value(): string;
    set_value(next: string): void;
    /** `change`, `submit`, `focus` or `blur`. */
    on(event: "change" | "submit" | "focus" | "blur", handler: (event: InputEvent, cx: Context) => void): boolean;
    /**
     * How much one step moves the value in a `NumberInput`. Default is 1;
     * `null` gives up stepping entirely.
     *
     * There is no numeric state type — the step, the bounds and the mask are
     * fields on this one, so a text state becomes a number state by being told
     * about them.
     */
    set_step(step: number | null): void;
    /** The lowest value stepping and blurring clamp to. `null` removes it. */
    set_min(min: number | null): void;
    /** The highest value stepping and blurring clamp to. `null` removes it. */
    set_max(max: number | null): void;
    /** Draws the text as a password. */
    set_masked(masked: boolean): void;
    /** Marks the state as working; the presentation is the application's. */
    set_loading(loading: boolean): void;
    release(): boolean;
  }

  export const InputState: {
    new: (options?: { placeholder?: string; value?: string }) => InputState;
  };

  /** The frame around retained text state. */
  export const Input: { new: (state: InputState) => Element };

  /**
   * A spinbutton over the same `InputState` an `Input` holds.
   *
   * There is no numeric state type. Give an ordinary `InputState` a
   * `set_step(...)` — and a `set_min(...)`/`set_max(...)` if the value is
   * bounded — and hand it here.
   *
   * Three slots, and all three carry weight: `input` (defaults to the bare
   * editor), `decrement_button` and `increment_button`. The base layer's step
   * buttons are unstyled, so an undecorated one is invisible and unhittable.
   *
   * Up and Down step it from the keyboard with nothing wired: the frame
   * declares its own key context, which the two bindings are registered
   * against.
   */
  export const NumberInput: { new: (state: InputState) => Element };

  /**
   * Retained multi-line text state, created once and kept on the view.
   *
   * Like `InputState.new(...)` this needs a live host call, so it belongs in
   * `init` or in an event handler — never in `render`.
   *
   * Give it a height. Being multi-line is carried by the state's mode rather
   * than by its layout, so the layout default is a single row even here: a
   * textarea that says nothing else is the height of an input. Pass `rows`,
   * call `set_auto_grow(...)`, or size the element with `.h(...)`.
   */
  export interface TextareaState {
    value(): string;
    set_value(next: string): void;
    /** `change`, `submit`, `focus` or `blur`. */
    on(event: "change" | "submit" | "focus" | "blur", handler: (event: InputEvent, cx: Context) => void): boolean;
    /** Shows this many rows. */
    set_rows(rows: number): void;
    /** Grows with the content, between the two row counts. */
    set_auto_grow(min_rows: number, max_rows: number): void;
    /** Wraps long lines instead of scrolling sideways. Default is on. */
    set_soft_wrap(wrap: boolean): void;
    release(): boolean;
  }

  export const TextareaState: {
    new: (options?: { placeholder?: string; value?: string; rows?: number }) => TextareaState;
  };

  /** The frame around retained multi-line text state. */
  export const Textarea: { new: (state: TextareaState) => Element };

  /** One thumb, or the two ends of a range. */
  export type SliderValue = number | [number, number];

  /**
   * Retained slider state, created once and kept on the view.
   *
   * Like `InputState.new(...)` this needs a live host call, so it belongs in
   * `init` or in an event handler — never in `render`.
   *
   * It is where a drag writes: the pointer moves, GPUI updates this, and the
   * next frame reads it back without the script being asked to describe
   * anything. Which is why the value is read out of the state — `value()` —
   * rather than held beside it: a copy in the view would be a copy the drag
   * never updated.
   */
  export interface SliderState {
    /** The current value: a number, or `[start, end]` for a range slider. */
    value(): SliderValue;
    set_value(next: SliderValue): void;
    min_value(): number;
    max_value(): number;
    step_value(): number;
    /**
     * `change` arrives on every pixel of a drag; `release` arrives once, when
     * the pointer is let go. Take the first for a live readout and the second
     * for anything that costs something — a request, a write, an undo entry.
     */
    on(event: "change" | "release", handler: (value: SliderValue, cx: Context) => void): boolean;
    release(): boolean;
  }

  export const SliderState: {
    /**
     * Defaults are `0..100` in steps of 1, starting at `min`.
     *
     * A `"logarithmic"` scale needs a `min` above zero — it maps through
     * `log(value / min)`, which has no answer at or below it.
     */
    new: (options?: {
      min?: number;
      max?: number;
      step?: number;
      scale?: "linear" | "logarithmic";
      value?: SliderValue;
    }) => SliderState;
  };

  /**
   * A slider, in four parts, none of which draws anything on its own.
   *
   * ```js
   * Slider.new(this.volume).child(
   *   SliderTrack.new(this.volume).flex().items_center().h(24).w_full().child(
   *     SliderIndicator.new(this.volume)
   *       .relative().w_full().h(6).rounded(3).bg("secondary")
   *       .range_style((fill) => fill.rounded(3).bg("primary"))
   *       .child(SliderThumb.new(this.volume).size(16).rounded(8).bg("primary").ml(-8)),
   *   ),
   * );
   * ```
   *
   * All four are needed and all four take the same state. The root announces
   * the value and owns the release; the track takes the press and the drag;
   * the indicator records the box every pointer position is measured against —
   * **a slider with no `SliderIndicator` cannot be moved at all**, which is
   * reported in the log rather than drawn; the thumb drags itself.
   *
   * The two boxes that depend on the value — the fill and the thumb — are
   * positioned by the shell, from the state, on every frame. That is not a
   * convenience: a drag never re-enters the script, so a position the script
   * computed would be the one the last render saw, and the slider would
   * announce a value its knob had never moved to. Give the thumb a size and a
   * look; the shell gives it a place.
   *
   * `axis("vertical")` is announced *and* used to place both, and each part is
   * told separately, as in Rust. A vertical slider grows from the bottom.
   */
  export const Slider: { new: (state: SliderState) => Element };
  /** The press and drag surface. Give it the height a pointer can hit. */
  export const SliderTrack: { new: (state: SliderState) => Element };
  /**
   * The groove, and the part that records the geometry. It must span the whole
   * travel of the slider: the box it records is what every pointer position is
   * divided by, so an indicator sized to the value would make the value its own
   * scale.
   */
  export const SliderIndicator: { new: (state: SliderState) => Element };
  /**
   * The knob. `start(true)` is the lower thumb of a range slider; the default
   * is the upper one, which is the only thumb a single-value slider has.
   *
   * Unlike the other three it keeps `id(...)`, because two thumbs share one
   * state and a `transition("left", ...)` needs to know which of them it is
   * following.
   */
  export const SliderThumb: { new: (state: SliderState) => Element };

  /**
   * Retained one-time-code state, created once and kept on the view.
   *
   * Like `InputState.new(...)` this needs a live host call, so it belongs in
   * `init` or in an event handler — never in `render`.
   *
   * The length is fixed when the state is created, because it is what the
   * state is: the base layer has no setter for it.
   */
  export interface OtpState {
    /** The digits entered so far — shorter than `len()` until the code is complete. */
    value(): string;
    /**
     * Sets the code from the script. Deliberately unfiltered, as in the base
     * layer: only keystrokes are digits-only. Anything past `len()` is stored
     * but never drawn.
     */
    set_value(next: string): void;
    /** How many cells there are. Fixed when the state was created. */
    len(): number;
    is_masked(): boolean;
    /** Hides the digits behind a bullet, without changing `value()`. */
    set_masked(masked: boolean): void;
    /** Moves the keyboard onto the code. */
    focus(): void;
    /**
     * `change` arrives after each edit; `complete` arrives when the last digit
     * lands. There is no `submit` — the base layer never emits one for a code
     * — and there is no event for the blink.
     */
    on(event: "change" | "complete" | "focus" | "blur", handler: (event: OtpEvent, cx: Context) => void): boolean;
    release(): boolean;
  }

  export const OtpState: {
    /** `length` is the number of cells: a whole number between 1 and 64. */
    new: (length: number, options?: { value?: string; masked?: boolean }) => OtpState;
  };

  /**
   * A fixed-length code, drawn cell by cell **by the shell**.
   *
   * ```js
   * OtpInput.new(this.code)
   *   .flex().gap(8)
   *   .cell_style((cell) =>
   *     cell.size(40).flex().items_center().justify_center()
   *       .border_1().border_color("border").rounded("md"))
   *   .cell_active_style((cell) => cell.border_color("ring"))
   *   .caret_style((caret) => caret.w(2).h(18).bg("foreground"))
   * ```
   *
   * Alone among the bound components, its cells are not the script's to
   * describe — only to style. A described cell would be frozen into the
   * snapshot the last render produced and nothing would ever thaw it: even
   * though edits emit `change`, the caret blinks on a native timer that raises
   * no script event at all.
   *
   * So the shell reads the state every frame and decides what each cell holds
   * — a digit, a bullet while the state is masked, the caret, or nothing —
   * and the three templates say what those look like. Lay the cells out by
   * styling the element itself: `.flex().gap(8)`.
   *
   * Children are allowed and are drawn after the cells, not instead of them.
   *
   * Grouping ("123 456") is not offered: the groups would be boxes the shell
   * invents, with no template to say what they look like.
   */
  export const OtpInput: { new: (state: OtpState) => Element };

  /** Where a region sits relative to the center of a dock area. */
  export type DockPlacement = "center" | "left" | "right" | "bottom";

  /** One panel, as `panels()` reports it. */
  export interface DockPanel {
    /** Stable for as long as the panel lives. Pass it to `remove_panel`. */
    readonly id: number;
    /** Namespaced: `shell:<application>/<name>`. */
    readonly name: string;
    readonly placement: DockPlacement;
    /** The container holding it, which is also `group.node` in the chrome. */
    readonly node: number;
    /** Its position in that container. */
    readonly index: number;
    /** Whether it is the one its container is showing. */
    readonly active: boolean;
    readonly visible: boolean;
    readonly closable: boolean;
    readonly zoomable: boolean;
  }

  /** One tab of a group, as a chrome handler is given it. */
  export interface DockTab {
    /** Its position in the group, which is what `select_tab` takes. */
    readonly index: number;
    readonly name: string;
    readonly id: number;
    readonly active: boolean;
    /**
     * Hidden panels are included, and keep their place in tab order — filter
     * on this rather than re-deriving an index into an already filtered list.
     */
    readonly visible: boolean;
    readonly closable: boolean;
    readonly zoomable: boolean;
  }

  /** A tab group, as `tab_bar` and `empty_group` are given it. */
  export interface DockGroup {
    readonly node: number;
    readonly active_index: number;
    readonly zoomed: boolean;
    readonly collapsed: boolean;
    readonly locked: boolean;
    readonly draggable: boolean;
    readonly droppable: boolean;
    readonly closable: boolean;
    readonly tabs: readonly DockTab[];
  }

  /** One dock, as the `dock` handler is given it. */
  export interface DockRegion {
    readonly placement: DockPlacement;
    /** Its extent along its own axis: width for left and right, height for bottom. */
    readonly size: number;
    readonly open: boolean;
    readonly collapsible: boolean;
  }

  /** One tile of a tiles canvas, as the two tile handlers are given it. */
  export interface DockTile {
    readonly node: number;
    readonly panel: { readonly name: string; readonly id: number; readonly visible: boolean };
    /**
     * Already resolved — base snaps, clamps and rounds before a skin sees
     * them, so nothing here has to be positioned by hand.
     */
    readonly bounds: import("gpui-shell").ElementBounds;
    readonly z_index: number;
    readonly moving: boolean;
    readonly resizing: boolean;
    readonly closable: boolean;
    readonly zoomed: boolean;
    readonly zoomable: boolean;
  }

  /** Where a dragged panel would land, as the `drop_indicator` handler is given it. */
  export interface DockDrop {
    /** `null` means the drop merges into the group's tabs rather than splitting beside it. */
    readonly placement: Placement | null;
    /** The hovered group's content box, in window coordinates. */
    readonly bounds: import("gpui-shell").ElementBounds;
    /** Where the placeholder starts, relative to `bounds`. */
    readonly from: import("gpui-shell").ElementBounds;
    /** Where it settles. */
    readonly to: import("gpui-shell").ElementBounds;
  }

  /** What `add_panel` is told about the panel it is adding. */
  export interface DockPanelOptions {
    /**
     * What the panel is filed under in a saved layout, and what
     * `DockArea.register_panel` finds it again by. Required.
     */
    name: string;
    /** Default `"center"`. */
    placement?: DockPlacement;
    /** Seeds the dock's extent when the panel is the first thing in it. */
    size?: number;
    /**
     * Places the panel on the region's tiles canvas instead of in a tab group.
     * A region with no canvas has nowhere to put a tile, so nothing happens.
     */
    bounds?: { x: number; y: number; width: number; height: number };
    /** Default `true`. */
    closable?: boolean;
    /** Default `true`. */
    zoomable?: boolean;
    /** Default `true`. */
    visible?: boolean;
  }

  /**
   * A dockable layout: splits, tab groups, docks and tiles that the user can
   * rearrange, and that survives a restart.
   *
   * Retained for a reason none of the other handles share. **The layout is what
   * the user changed** — a drag, a resize, a closed tab and a collapsed dock all
   * happen without the script rendering — so it lives here rather than in a
   * description that would put every one of them back the way the last render
   * described it.
   *
   * `DockArea.new(id)` needs a live host call, so it belongs in `init` or an
   * event handler, never in `render`.
   *
   * **Every edit takes effect once the call that made it has returned.**
   * `add_panel` is handed a view from `cx.new(Class)`, which is itself still
   * being constructed; `load` rebuilds panels, which constructs more. So
   * `panels()` and `dump()` read the layout as it was before this turn's edits,
   * and `on("layout_changed", …)` is where to read it after them.
   *
   * ```js
   * init(_props, cx) {
   *   DockArea.register_panel("inbox", Inbox);
   *   this.dock = DockArea.new("workspace");
   *   this.dock.add_panel(cx.new(Inbox), { name: "inbox", placement: "left", size: 240 });
   *   this.dock.on("layout_changed", () => localStorage.setItem("layout", JSON.stringify(this.dock.dump())));
   * }
   * render() {
   *   return dock_area(this.dock).size_full().tab_bar((group) => …);
   * }
   * ```
   */
  export interface DockArea {
    /** Docks `view` — a view from `cx.new(Class)`, not an element. */
    add_panel(view: import("gpui-kit").Entity, options: DockPanelOptions): void;
    /** Removes the panel with this id, wherever it sits. */
    remove_panel(id: number): void;
    /** Every panel in the area, in tree order. */
    panels(): DockPanel[];
    /**
     * The whole layout as plain data: the tree, the docks, and each panel's own
     * `serialize()` payload. Hand it back to `load` after a restart.
     */
    dump(): any;
    /**
     * Restores a layout `dump()` wrote, rebuilding each panel through the class
     * registered under its name.
     *
     * A panel whose name nothing registered is not dropped: it is carried
     * forward — name, payload and position — so uninstalling an application and
     * reinstalling it puts its panels back where they were.
     */
    load(state: any): void;
    has_dock(placement: DockPlacement): boolean;
    is_dock_open(placement: DockPlacement): boolean;
    toggle_dock(placement: DockPlacement): void;
    remove_dock(placement: DockPlacement): void;
    dock_size(placement: DockPlacement): number | null;
    set_dock_size(placement: DockPlacement, size: number): void;
    set_dock_collapsible(placement: DockPlacement, collapsible: boolean): void;
    /** A locked area cannot be dragged, dropped into, or resized. */
    is_locked(): boolean;
    set_locked(locked: boolean): void;
    is_zoomed(): boolean;
    /** Clears the zoom, whichever container holds it. */
    zoom_out(): void;
    /**
     * Fires on every edit — including each step of a tile drag — so save on a
     * timer rather than on every one.
     */
    on(event: "layout_changed", handler: (cx: Context) => void): boolean;
    release(): boolean;
  }

  export const DockArea: {
    new: (id: string, options?: { version?: number }) => DockArea;
    /**
     * Teaches the runtime to rebuild `name`'s panel from `Class` when a saved
     * layout mentions it, and answers with the namespaced name it registered
     * under.
     *
     * The class is an ordinary view class. Two of its methods carry state
     * across a restart, and both are optional:
     *
     * - `serialize()` returns plain data, and is read when the layout is saved.
     *   It runs without a host call, so it must not touch entities, `cx`, or
     *   anything else that needs one — return a value and nothing else.
     * - `deserialize(data)` is handed back whatever `serialize()` wrote, right
     *   after the view is built, with a real host call available.
     *
     * Registering the same name twice replaces the class, which is what a hot
     * reload does.
     */
    register_panel: (name: string, Class: import("gpui-kit").ViewClass) => string;
  };

  /**
   * Draws a dock area.
   *
   * Base draws **no chrome at all** — an area with none still docks, drags,
   * resizes and persists, painting only the panels — so every tab bar, dock
   * frame and drag bar is one of the six handlers below.
   *
   * Each handler is called from inside GPUI's layout pass, once per container
   * per frame, and is given base's own resolved state: never a drag event, a
   * mouse position or a hit test. It may not register event handlers — a
   * callback created there would pile up for as long as the dock stood — so the
   * elements it returns say what they do with a **command** instead:
   * `select_tab(group, i)`, `close_panel(group, id)`, `toggle_dock(dock)`,
   * `move_tile(tile)` and the rest. A command carries no script value, and base
   * does the work.
   */
  export function dock_area(area: DockArea): DockAreaElement;

  export interface DockAreaElement extends Element {
    /** The tab bar above a group's displayed panel. */
    tab_bar(handler: (group: DockGroup, cx: Context) => Element): DockAreaElement;
    /** What a group with no displayed panel shows. */
    empty_group(handler: (group: DockGroup, cx: Context) => Element | null): DockAreaElement;
    /** The hint showing where a dragged panel would land. */
    drop_indicator(handler: (drop: DockDrop, cx: Context) => Element | null): DockAreaElement;
    /**
     * One dock's chrome around its content: title strip, collapse affordance,
     * resize handle. Whatever this returns replaces the content, so put
     * `dock_content()` where the panels belong.
     */
    dock(handler: (dock: DockRegion, cx: Context) => Element | null): DockAreaElement;
    /**
     * The strip a tile is dragged by. Its height is fixed at base's drag-bar
     * height, which the snapping arithmetic assumes.
     */
    tile_drag_bar(handler: (tile: DockTile, cx: Context) => Element): DockAreaElement;
    /** A tile's resize affordances. */
    tile_resize_handles(handler: (tile: DockTile, cx: Context) => Element | null): DockAreaElement;
  }

  /**
   * Where a dock's own panels go inside the chrome the `dock` handler drew
   * around them. Legal only inside that handler, and only once.
   */
  export function dock_content(): Element;

  /** Which edge or corner of a tile a resize handle pulls. */
  export type TileResizeSide = "left" | "right" | "top" | "bottom" | "bottom_right";

  /** Semantic color roles, aligned with `gpui_base::ColorTokens`. */
  export type ColorTokens = { readonly [Role in ColorToken]: Color };
  /** Semantic spacing scale, aligned with `gpui_base::SpacingTokens`. */
  export interface SpacingTokens {
    readonly xxs: number; readonly xs: number; readonly sm: number;
    readonly md: number; readonly lg: number; readonly xl: number; readonly xxl: number;
  }
  /** Semantic radius scale, aligned with `gpui_base::RadiusTokens`. */
  export interface RadiusTokens {
    readonly none: number; readonly sm: number; readonly md: number;
    readonly lg: number; readonly xl: number; readonly full: number;
  }
  export interface SemanticThemeTokens {
    readonly colors: ColorTokens;
    readonly spacing: SpacingTokens;
    readonly radius: RadiusTokens;
  }

  /**
   * Replaces gpui-base's active semantic tokens for the current application.
   * Legal only from an event handler or task backed by a live host call.
   */
  export function set_theme(theme: {
    readonly appearance: "light" | "dark";
    readonly tokens: SemanticThemeTokens;
  }): void;
  /** The Base-aligned semantic tokens plus the current appearance. Read-only. */
  export interface Theme extends SemanticThemeTokens, ColorTokens {
    readonly appearance: "light" | "dark";
    readonly is_dark: boolean;
  }

}

declare module "gpui-shell" {
  /** The string forms accepted by gpui-shell's length bridge. */
  export type LengthString = `${number}px` | `${number}rem` | `${number}%`;

  /** The gpui-shell call scope reported by `cx.phase()`; unrelated to gpui::DispatchPhase. */
  export type ScopePhase =
    | "render"
    | "event"
    | "task"
    | "layout"
    | "none"
    ;

  /** A path coordinate in pixels or as a percentage of the painted bounds. */
  export type PathCoordinate = number | `${number}%`;

  /** The property bag carried across the JavaScript view bridge. */
  export type Props = Record<string, any>;

  /** Element-local event bounds assembled by the shell. */
  export interface ElementBounds extends import("gpui-kit").Point {
    width: number;
    height: number;
  }

  export interface DialogOptions {
    escape_dismissable?: boolean;
    backdrop_dismissable?: boolean;
  }

  export interface ToastOptions {
    title: string;
    description?: string;
    level?: "info" | "success" | "warning" | "error";
    timeout?: number | null;
    id?: string;
  }

  export interface TaskOptions {
    /** Defaults to the running view; `null` outlives every view. */
    owner?: import("gpui-kit").View | null;
  }

  export type MotionProperty = "opacity" | "width" | "height" | "left" | "top";
  export type MotionEasing = "linear" | "ease-in" | "ease-out" | "ease-in-out";
  export interface TransitionPolicy {
    /** Duration in milliseconds. */
    duration: number;
    /** Delay in milliseconds. */
    delay?: number;
    easing?: MotionEasing;
  }
  export interface SpringPolicy {
    /** Approximate response period in milliseconds. */
    response?: number;
    /** Damping ratio; 1 has no overshoot. */
    damping?: number;
    /** Settling tolerance in the target's units. */
    epsilon?: number;
  }

}

declare module "gpui-fps" {
  import { Element } from "gpui-kit";

  /**
   * The native `gpui-fps` performance HUD, shared once per window and pinned
   * to the top-right by default. Its parent must be `relative()`.
   */
  export function fps_monitor(): Element;
}

declare module "buffer" {
  export class Buffer extends Uint8Array {
    static from(value: string | ArrayBuffer | ArrayLike<number>, encoding?: string): Buffer;
    static alloc(size: number): Buffer;
    toString(encoding?: string): string;
  }
}
declare module "path" {
  export function join(...parts: string[]): string;
  export function resolve(...parts: string[]): string;
  export function dirname(path: string): string;
  export function basename(path: string, suffix?: string): string;
  const path: { join: typeof join; resolve: typeof resolve; dirname: typeof dirname; basename: typeof basename };
  export default path;
}
declare module "url" {
  export const URL: typeof globalThis.URL;
  export const URLSearchParams: typeof globalThis.URLSearchParams;
}
declare module "crypto" {
  export interface Hash { update(data: string | Uint8Array): Hash; digest(encoding?: string): string | import("buffer").Buffer; }
  export function createHash(algorithm: string): Hash;
  export function randomBytes(size: number): import("buffer").Buffer;
  export function randomUUID(): string;
  export const webcrypto: Crypto;
}
declare module "zlib" {
  export function deflateSync(data: string | Uint8Array): import("buffer").Buffer;
  export function inflateSync(data: Uint8Array): import("buffer").Buffer;
  export function gzipSync(data: string | Uint8Array): import("buffer").Buffer;
  export function gunzipSync(data: Uint8Array): import("buffer").Buffer;
}
interface Console {
  debug(...values: unknown[]): void;
  log(...values: unknown[]): void;
  info(...values: unknown[]): void;
  warn(...values: unknown[]): void;
  error(...values: unknown[]): void;
}
/**
 * Diagnostics. A global, as it is in every other JavaScript runtime, and the
 * only one: the shell used to export the same object a second time as
 * `gpui.log`, which bought a name and nothing else.
 *
 * Needs no capability — a script that runs may say something — and output goes
 * to `tracing` under the `gpui_shell::script` target.
 */
declare const console: Console;
declare module "console" {
  const console: Console;
  export default console;
}
declare module "process" {
  export interface CommandOutput { code: number; stdout: string; stderr: string; }
  export function run(command: string, args?: string[]): Promise<CommandOutput>;
  export function exit(code?: number): void;
  export function nextTick(callback: (...args: unknown[]) => void, ...args: unknown[]): void;
  export const platform: string;
  export const arch: string;
  const process: { run: typeof run; exit: typeof exit; nextTick: typeof nextTick; platform: string; arch: string };
  export default process;
}
declare module "os" {
  export function platform(): string;
  export function arch(): string;
  export const EOL: string;
  const os: { platform: typeof platform; arch: typeof arch; EOL: string };
  export default os;
}
declare module "fs/promises" {
  export interface Dirent { name: string; isDirectory(): boolean; }
  export interface MakeDirectoryOptions { recursive?: boolean; }
  export function readFile(path: string): Promise<Uint8Array>;
  export function readFile(path: string, encoding: "utf8" | { encoding: "utf8" }): Promise<string>;
  export function writeFile(path: string, contents: string | Uint8Array): Promise<void>;
  export function readdir(path: string): Promise<string[]>;
  export function readdir(path: string, options: { withFileTypes: true }): Promise<Dirent[]>;
  export function exists(path: string): Promise<boolean>;
  export function unlink(path: string): Promise<void>;
  export function rmdir(path: string): Promise<void>;
  export function mkdir(path: string, options?: MakeDirectoryOptions): Promise<void>;
}
declare module "net" {
  export interface Socket {
    write(data: string): Promise<void>;
    /** Reads raw bytes. Resolves to null after the peer reaches EOF. */
    read(maxBytes?: number): Promise<Uint8Array | null>;
    close(): void;
  }
  export function connect(host: string, port: number): Promise<Socket>;
  const net: { connect: typeof connect };
  export default net;
}
declare module "websocket" {
  export interface WebSocketSocket {
    /** Waits for the next text or binary message. */
    read(): Promise<string | Uint8Array>;
    /** Sends a text or binary message. */
    write(data: string | Uint8Array): Promise<void>;
    /** Sends and flushes a close frame. */
    close(): Promise<void>;
  }
  export interface WebSocketConnectOptions {
    /** Additional protocol headers. Credential and WebSocket control headers are refused. */
    headers?: Readonly<Record<string, string>>;
  }
  export interface WebSocketType {
    connect(url: string, options?: WebSocketConnectOptions): Promise<WebSocketSocket>;
  }
  /** Capability-gated client sockets; not the browser global constructor. */
  export const WebSocket: WebSocketType;
}
interface ShellFetchResponse {
  readonly status: number;
  readonly ok: boolean;
  readonly url: string;
  text(): Promise<string>;
  json(): Promise<unknown>;
}
interface ShellFetchOptions {
  /** GET by default; POST is available for OAuth-style form exchanges. */
  method?: "GET" | "POST";
  /** Client-managed framing headers such as Host and Content-Length are refused. */
  headers?: Record<string, string>;
  body?: string | Uint8Array;
}
declare function fetch(url: string, options?: ShellFetchOptions): Promise<ShellFetchResponse>;
declare const process: typeof import("process").default;

/**
 * The window the script is drawing into. A global: nothing to import, and
 * unlike `cx`, nothing hands it to you.
 *
 * Ambient: every call reads the host call that is running now, and throws
 * outside one. There is no handle to hold, so there is nothing to hold past the
 * call that would have made it stale.
 *
 * An overlay belongs to the window rather than to the view that opened it —
 * `cx.notify()` re-renders this view, `window.open_dialog()` changes what the
 * user is looking at — which is why these are here and not on `Context`.
 */
type GpuiShellWindow = import("gpui-kit").Window;
interface Window extends GpuiShellWindow {}
declare var window: Window & typeof globalThis;

/**
 * `window.localStorage`, reachable bare as it is in a browser, where `window`
 * *is* the global object. Here `window` is an ordinary object, so both
 * spellings are installed rather than one falling out of the other.
 */
declare const localStorage: import("gpui-kit").Storage;
/** `window.sessionStorage`, bare, for the same reason. */
declare const sessionStorage: import("gpui-kit").Storage;
