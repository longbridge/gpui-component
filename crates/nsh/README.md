# gpui-nsh

[Native Shell](https://nsh.dev) (NSH) in GPUI Kit apps, in both directions:

- **NSH in GPUI Kit.** `NshView` runs an NSH app (a TSX bundle, or a Rust root) inside a GPUI Kit window. It fills the
  space its parent gives it and follows GPUI Kit's theme and light or dark mode.
- **GPUI Base in NSH.** Apps use GPUI Base's controls as NSH native components (`gpui-base.<Name>`): `Button`,
  `Toggle`, `Link`, `Switch`, `Checkbox`, `Radio`, `RadioGroup`, `ToggleGroup`, `Tabs`, `Input`, `Slider`, `Select`,
  and the rest listed in `src/base/inventory.rs`.

```rust
use gpui_nsh::{AppOptions, Bundle, NshApp, NshView};

let view = cx.new(|cx| NshView::new(AppOptions::default(), window, cx));
// SAFETY: the bundle is the application's own `nsh build` output.
let bundle = unsafe { Bundle::read_trusted(path) }?;
view.read(cx).load(NshApp::Js(bundle), cx);
```

```tsx
// `crates/nsh/types/gpui-base.ts`, copied into the app or imported by a relative path.
import { Switch } from "./gpui-base";

<Switch
  checked={on()}
  onChange={(e) => on.set(e.checked)}
  rootClass="flex items_center gap_2"
  trackClass="flex w_9 h_5 p_0p5 rounded_full bg_muted"
  trackCheckedClass="justify_end bg_primary"
  thumbClass="size_4 rounded_full bg_background"
/>
```

## One implementation

GPUI Base owns each control's behavior: pointer and keyboard interaction, focus, accessibility and state. It installs
no presentation, so the NSH app supplies it with class strings for the control (`rootClass`), each of its parts
(`trackClass`, `thumbClass`, ...) and their states (`checkedClass`, `disabledClass`, ...). NSH keeps `class` for the
Tailwind classes of the box every element has, which still apply around the control. A class string
holds GPUI's and GPUI Base's style methods that take no argument (`flex items_center gap_2 rounded_full px_3`, found by
reflection as GPUI Shell finds them) and `bg_<token>`, `text_<token>` and `border_<token>` for each color of GPUI
Base's semantic tokens.

Each control is declared once, in Rust, as a `Control` of a `Catalog` (`src/base/`): its props, its events, and the
builder calls that create the GPUI Base element. The same catalog:

- registers the controls with every `NshView`;
- generates their TypeScript declarations, `types/gpui-base.ts` (a test fails when the file is stale; run
  `UPDATE_TYPES=1 cargo test -p gpui-nsh` to write it again).

The bridge (`src/catalog.rs`) is generic: it decodes props, sends events to JS, keeps a stateful control's entities
across frames, measures the element's natural size for NSH's layout, and shows a change an event reports before JS
sets the prop back. It holds no behavior of any control.

`src/base/inventory.rs` lists every GPUI Base module as registered controls or the reason it is not registered
(containers of app content, overlays, list views and infrastructure); a test keeps it in step with GPUI Base.

An application adds controls of its own the same way and registers them with `NshView::with_catalog`.

## Notes

- NSH draws with GPUI Fast, so this crate enables `gpui-kit/gpui-fast`; the application and NSH share one GPUI.
- Open the window with `inactive_frame_interval: None`, so NSH animates at full rate while the window is not focused.
- A control reports its natural size (its content on one line) to NSH's layout; once the app's style or a flex parent
  gives its box a width, its height is measured at that width.
- Text inside the NSH app is NSH's: it is black unless the app colors it (Tailwind `dark:` variants follow the host's
  mode). Text inside a control takes GPUI Base's foreground token unless its class says otherwise.

See `examples/nsh` for a runnable example.
