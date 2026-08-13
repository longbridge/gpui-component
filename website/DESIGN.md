# Website Design

Design rules for the documentation site in `website/`. It exists so later
changes stay coherent instead of drifting back into a generic template.

The site is the first thing a visitor sees of a library that renders native
desktop UI. So it has one job before anything else: make it obvious, in a
single screen, what this project is — and then prove it is real.

## Principles

1. **Say what it is, immediately.** The headline states the product in plain
   language ("Build native desktop apps in Rust."). Positioning that only makes
   sense to someone who already knows the project — architecture layers,
   "ship fast or own everything" — belongs further down the page, not in the
   hero.
2. **Show the real thing, never a mock.** The page embeds the actual
   WebAssembly build of the gallery. No greyed-out wireframe placeholders, no
   invented product screenshots. A fake UI panel on the homepage of a UI
   library reads as an admission that there is nothing to show, and a
   convincing one gets mistaken for a product the project does not ship.
3. **Verifiable facts over adjectives.** Star count, licence, platforms, real
   type names (`DockArea`, `Rope`, `Tiles`), real numbers from the README (120
   FPS, 200K lines). A developer judges credibility from specifics.
4. **Same palette and typeface as the library.** Colours come from
   `crates/ui/src/theme/default-theme.json`; code colours come from the same
   shiki theme the docs use; the type is the platform font, as in a real app.
   The site must not invent a look the components cannot produce.
5. **Restraint carries the design.** Hierarchy comes from type scale, weight
   and hairlines — not from colour fills or decorative gradients.

## Colour

Defined in `.vitepress/theme/style.css`, mapped from the default theme so the
site and the documented components share one palette.

| Token | Light | Dark | Theme source |
| --- | --- | --- | --- |
| `--background` | `#ffffff` | `#0a0a0a` | `background` |
| `--foreground` | `#0a0a0a` | `#fafafa` | `foreground` |
| `--border` | `#e5e5e5` | `#262626` | `border` |
| `--secondary` | `#f5f5f5` | `#262626` | `secondary.background` |
| `--muted-foreground` | `#737373` | `#a3a3a3` | `muted.foreground` |
| `--sidebar` | `#fafafa` | `#0f0f0f` | `sidebar.background` |
| `--titlebar` | `#f8f8f8` | `#171717` | `title_bar.background` |
| `--brand` | `#171717` | `#fafafa` | `primary.background` |
| `--data-1…5` | `#93c5fd` → `#1e40af` | same | `chart_1…chart_5` |
| logo accent | `#3b82f6` | `#b54e05` | `chart_2` / dark `entity.name.type` |
| `--selection` | `#55a0fc` | same | `selection.background` |
| `--success` | `#22c55e` | same | `success.background` |
| `--code-*` | macos-classic-light | macos-classic-dark | `src/*.theme.json` |

Rules that follow from this:

- **The brand colour is near-black (near-white in dark mode).** It is used for
  primary buttons, focus rings and the active sidebar indicator — never as an
  "accent" to add interest, because it is the same value as body text. Section
  kickers and captions use `--muted-foreground` instead.
- **Never use `--brand` as a background behind text you did not also invert.**
  Text selection in particular uses `--selection`: black text on a near-black
  selection is unreadable.
- **Saturated colour is reserved for data**, exactly as the theme reserves
  `chart_*`. The capability diagrams and charts may use `--data-*`; marketing
  surfaces may not.
- **The logo accent is per-theme**: `#3b82f6` (`chart_2`) on light, `#b54e05`
  on dark — the latter is the dark syntax theme's `entity.name.type`, so the
  mark stays keyed to the code colours a reader sees on that background. The
  mark is split into two paths — an open `C`, and the bar-and-stem that turns it
  into a `G` — so that stroke can carry the accent while the rest stays neutral.
  The values are baked into `public/logo.svg` and `logo-dark.svg`; making them
  follow a token would mean rendering the mark inline instead of as an image.
  In-page diagrams keep using `--data-2` and are unaffected.
- **`--success` marks "running"** — the live WASM indicator and the example
  badge — where near-black would not read as a live signal.
- **Hand-written code snippets use the `--code-*` tokens**, which are lifted
  from the same macos-classic theme shiki applies in the docs. Never invent
  highlighting for a snippet.
- Links are distinguished by a rule, not a hue, since the brand colour equals
  the text colour. See `.vp-doc a`.

## Typography

The platform font, because that is what the library itself renders with: SF Pro
on macOS, Segoe UI Variable on Windows, with `Noto Sans SC` / `PingFang SC` for
Chinese. Monospace prefers `ui-monospace` / SF Mono and falls back to
`JetBrains Mono`. No webfont is downloaded for body text. Base size 15px.

- **Display** — `clamp(2.2rem, 4.3vw, 3.6rem)`, weight 660, tracking `-0.042em`.
- **Section heading** — `clamp(2rem, 3.6vw, 3rem)`, weight 660, tracking `-0.045em`.
- **Body** — 1rem, line-height 1.7; docs prose is capped at `46rem`.
- **Kicker / label** — 0.66–0.68rem mono, uppercase, wide tracking, muted.
  Small mono labels, not colour, mark structure.

Two constraints that are easy to get wrong:

- **Negative tracking and wide tracking are for Latin only.** CJK glyphs sit on
  a fixed em grid, so `html[lang^="zh"]` resets body tracking and reduces the
  kicker's letter-spacing. Do not apply Latin tracking globally.
- **Numerals use `tabular-nums`** in tables, keys and counters so they do not
  reflow when values change.

## Layout

- Page width `1280px`, gutter `1.5rem` (`1rem` under 640px).
- **One container for every band.** Each `<section>` under `.home` is
  full-bleed with a `border-top` hairline, and its content sits in
  `.band__inner`, which owns the width and the `--section-gap` vertical
  padding. Sections must butt directly against each other — the hairline is the
  only separator. A stray margin between two sections is a bug; verify the gap
  is `0`.
- Nav, hero, every band and the footer must resolve to the **same left edge and
  width** (x=80, w=1280 at 1440px). Check this after any layout change.
- Navigation is a **toolbar**: brand, a hairline divider, then the sections on
  the left; search, stars, language and appearance collected on the right.
  Height 3.5rem/56px. The docs navbar is the same toolbar — VitePress's own
  navbar is reordered by CSS, and the language and star controls are injected
  through the `nav-bar-content-after` slot rather than living in `nav` items, so
  the real search keeps working.
- The hero is two columns: copy, and a macOS window holding a real snippet from
  the Quick Start guide. Its vertical rhythm is 20 / 20 / 24 / 24 / 20 px.
- **The WASM gallery must not be in the first screen.** It is a full
  WebAssembly build and takes time to load; it sits in the band below the hero
  and mounts through an `IntersectionObserver`.
- Cards in a row must align internally, not just at their outer edges: absorb
  the slack after the description (`margin-bottom: auto`, or `min-height` when
  two cards must match) so chips, snippets and previews line up. Equal-length
  code snippets are part of that contract.
- Grid backgrounds are hairline blueprint grids masked to fade at the edges. No
  colour wash behind headlines.
- Check for horizontal overflow at phone widths. Grid columns must be
  `minmax(0, 1fr)` — a bare `1fr` takes its minimum from content, and a
  `nowrap` snippet will push the page wider than the viewport.

## Surfaces and the window language

Anything showing the library running is framed as a **macOS window** — the
`.mac-window` class in `style.css`. It is the closest visual analogue to what
the library actually produces, so it reads as a native application rather than
a screenshot card.

The frame is: a hairline outer stroke, an inner top highlight, layered soft
shadows, and real traffic lights (`#ff5f57`, `#febc2e`, `#28c840`). The title
is centred and independent of the lights, as macOS does.

Used by the hero snippet, the gallery band and `ComponentExample.vue` on every
component page, so the three never diverge.

**Do not put document tabs inside the window chrome.** A tab strip in the
titlebar fights the traffic lights, and a browser-style tab row below it is not
how gpui-component presents views. View switching uses the library's own
**segmented control** (`.segmented`, mapped from `tab_bar.segmented.background`
and `tab.active.background`), placed in the section heading — outside the
window, which stays pure chrome.

Radii: `--radius-control` 0.375rem for controls, `--radius-card` 0.625rem for
cards, `--radius-surface` 0.875rem for large surfaces, 0.75rem for windows.

## Motion

Entrances only, and short: `rise` at 620ms on `cubic-bezier(.16, 1, .3, 1)`,
staggered 70ms. Live indicators use a slow 2.4s pulse. Everything sits inside
`@media (prefers-reduced-motion: no-preference)`; nothing conveys meaning
through motion alone.

## Content rules

- The crate is **not published**. Installation must show the git dependency,
  never `cargo add`, and the UI must not display a version number.
- Code samples must be real API, verified against `crates/ui` and
  `crates/base`.
- Capability copy tracks the README's feature list — 120 FPS rendering, complex
  data tables, virtualized lists, the 200K-line editor, freeform docking,
  multi-theme support. Update it when the README's features change.
- Capability previews are **diagrams**, not product mocks: they share one
  padding box and one gap, and they may use `--data-*` to read as UI. A diagram
  that needs a scrollbar needs its track too, or it looks like a glitch.
- Landing-page copy lives in one bilingual `copy` object in `index.vue`. Both
  locales must be updated together, matching the site-wide rule that
  `website/docs/` and `website/zh-CN/docs/` stay in sync.
- Gallery demos should be the components that demonstrate density and
  capability. A demo that looks trivial or renders without highlighting
  undersells the library and should be swapped out.

## Files

| File | Role |
| --- | --- |
| `.vitepress/theme/style.css` | Tokens, `.mac-window`, VitePress overrides, doc typography |
| `index.vue` | Landing page: markup, bilingual copy, page-scoped styles |
| `.vitepress/theme/index.ts` | Theme entry; injects nav controls and the example window |
| `.vitepress/theme/components/ComponentExample.vue` | Windowed live example on component pages |
| `.vitepress/config.mts` | Navigation, sidebar generation, locales |
| `src/*.theme.json` | shiki syntax themes; the source of `--code-*` |
