# Rendering GPUI Elements Above Native WebViews: Platform Composition Research

> Research date: 2026-07-30
> Baseline: `gpui-wry` uses `lb-wry 0.53.3`; GPUI is based on Zed commit
> `huacnlee/zed:gpui-webview-overlay` at `fbe67f26a3`. This document records the
> native composition research, the implemented macOS layered-scene spike, and
> the remaining Windows and Linux architecture.

## Conclusion

Adding a `z-index` to an existing wry WebView cannot solve this problem across
platforms. GPUI elements are composed into one GPU surface, while wry creates a
platform-native child view or window by default. Paint order inside GPUI cannot
cross the native window hierarchy.

The recommended end state is a GPUI **native surface slot**:

1. After prepaint, GPUI submits the native surface bounds, clip, visibility,
   and stacking information every frame.
2. The platform backend inserts the WebView into that platform's compositable
   visual or view tree.
3. GPUI continues to draw its overlays, but the platform backend guarantees
   that their GPU visual is above the WebView visual. Input is routed to the
   overlay or WebView according to GPUI hit testing.
4. Validate the design on macOS first. Windows must use WebView2 composition
   hosting. Linux cannot promise an implementation with the same cost.

GPUI already has a better natural split point than an arbitrary `z-index`.
`Window::draw_roots` paints the root, calls `paint_deferred_draws`, and finally
paints the window prompt, active drag, or core tooltip
([GPUI `Window::draw_roots`](https://github.com/zed-industries/zed/blob/66d95fb1945b7a7be671427f11ebfb42c339bdb4/crates/gpui/src/window.rs#L2839-L2923)).
The preferred model is therefore:

```text
base scene paint
    |
native portal (WebView visual)
    |
deferred overlay scene
    |
prompt / active drag / core tooltip scene
```

This follows the existing GPUI paint pipeline more closely than hard-coding
component names as overlays.

| Platform | Default wry representation | Reordering siblings directly | Recommendation |
| --- | --- | --- | --- |
| macOS | `WKWebView` as an `NSView` subview | **Promising; validate first** | Put WKWebView and the GPUI Metal views in an explicit sibling view/layer hierarchy; validate transparent overlays, input, and resize |
| Windows | WebView2 controller hosted in a separate `WS_CHILD` HWND | **Unreliable; not a final design** | Fork or extend wry to use WebView2 Composition Controller and attach the WebView visual to GPUI's DirectComposition tree |
| Linux | X11 child window or GTK `WebKitWebView` widget | X11 child window: **no**; a unified GTK tree may work | Wayland requires the window and layout to join the GTK widget tree; otherwise only a limited snapshot/offscreen fallback is possible |

## Why the Current Implementation Always Covers GPUI

`WebViewElement::paint` does not draw the WebView; it only registers a hitbox.
wry's native object displays the page, while `prepaint` only calls
`set_bounds`. Later GPUI paint ordering for Popover, Dialog, or Notification
can only reorder pixels inside the same GPUI scene. It cannot reorder an
external `NSView`, child `HWND`, or GTK widget.

wry defines `build_as_child` in native platform terms: a child window on
Windows, an `NSView` under the content view on macOS, and an X11-only child
window on Linux. Wayland requires a GTK container. These are explicit wry API
semantics, not a GPUI-specific effect
([official wry `WebViewBuilder::build_as_child` API](https://docs.rs/wry/0.53.3/wry/struct.WebViewBuilder.html#method.build_as_child),
[official wry child WebView example](https://github.com/tauri-apps/wry/tree/v0.53.3#child-webviews)).

The pinned wry Windows source creates `WRY_WEBVIEW` with
`WS_CHILD | WS_CLIPCHILDREN`, calls `SetWindowPos(..., HWND_TOP, ...)`, and
then creates an ordinary `ICoreWebView2Controller`. It does not use a
composition controller
([wry 0.53.3 child HWND creation](https://github.com/tauri-apps/wry/blob/v0.53.3/src/webview2/mod.rs#L310-L376),
[wry 0.53.3 ordinary WebView2 controller creation](https://github.com/tauri-apps/wry/blob/v0.53.3/src/webview2/mod.rs#L545-L594)).
This directly explains why the WebView remains in front of the GPUI scene on
Windows.

## What `Application::run_embedded` Solves

Zed PR [#60574](https://github.com/zed-industries/zed/pull/60574) added
`Application::run_embedded` and `ApplicationHandle`. They invert ownership of
the event loop and GPUI application: when an external host owns the run loop
and `Platform::run` returns immediately after its launch callback,
`ApplicationHandle` keeps the application alive and lets the host re-enter
GPUI through `update()` and `to_async()`. The PR did not add an embeddable
`NSView`, `HWND`, `CAMetalLayer`, GPU texture, or scene-stacking API.

Its first consumer, `embedded_gpui`, lets guest UI participate like ordinary
GPUI elements because the guest outputs a retained display list that the host
replays as **the host's own GPUI primitives**. It does not insert a guest
native window or surface into the GPUI scene
([`embedded_gpui` README](https://github.com/zed-industries/embedded_gpui#readme)).
WKWebView, WebView2, and WebKitGTK do not expose a browser display list that an
application can replay every frame, so this mechanism cannot be applied
directly to a wry WebView.

It can still be reused indirectly:

- If the macOS spike uses an external native shell that owns the run loop and
  embeds a GPUI overlay renderer in a selected `NSView` or `CAMetalLayer`,
  `run_embedded` can manage application lifetime.
- If GPUI later gains a truly embedded `PlatformWindow`, it can be the entry
  point through which the host drives GPUI.
- It does not create the `base GPUI < WebView < overlay GPUI` layers or handle
  scene separation, alpha, hit testing, focus, IME, or WebView input
  forwarding.

This problem still requires a native surface slot and overlay plane.
`run_embedded` is a reusable startup and lifetime building block, not the
composition solution.

## macOS: One NSView / CALayer Tree

### Primary-source facts

- AppKit's `addSubview(_:positioned:relativeTo:)` inserts a view immediately
  above or below a sibling. A `nil` relative view places it above or below all
  siblings
  ([Apple `NSView.addSubview` ordering API](https://developer.apple.com/documentation/appkit/nsview/addsubview%28_%3Apositioned%3Arelativeto%3A%29)).
- Apple's view hierarchy guide says that a view which must draw in front of
  another should be a subview or descendant of the rear view, and that the
  positioned API controls ordering
  ([Apple, Working with the View Hierarchy](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/CocoaViewsGuide/WorkingWithAViewHierarchy/WorkingWithAViewHierarchy.html)).
- A layer-backed view caches drawing in its `CALayer`; enabling `wantsLayer`
  on a parent makes the subtree layer-backed. Apple also warns against adding
  ordinary subviews to a layer-hosting view, which must be managed entirely
  through Core Animation
  ([Apple `NSView.wantsLayer`](https://developer.apple.com/documentation/appkit/nsview/wantslayer)).
- wry 0.53.3 inserts `WKWebView` into its parent with `addSubview` and exposes
  no relative sibling-order policy
  ([wry WKWebView insertion source](https://github.com/tauri-apps/wry/blob/v0.53.3/src/wkwebview/mod.rs#L615-L674)).
- GPUI's macOS backend creates `GPUIView`, explicitly enables `wantsLayer`,
  and adds it to `NSWindow.contentView`
  ([GPUI macOS window source](https://github.com/zed-industries/zed/blob/66d95fb1945b7a7be671427f11ebfb42c339bdb4/crates/gpui_macos/src/window.rs#L880-L984)).
  Its renderer uses `CAMetalLayer`
  ([GPUI Metal renderer source](https://github.com/zed-industries/zed/blob/66d95fb1945b7a7be671427f11ebfb42c339bdb4/crates/gpui_macos/src/metal_renderer.rs#L151-L177)).

### Viable design

The smallest prototype is not a new transparent GPUI window. It explicitly
rebuilds sibling order below the same content view:

```text
NSWindow.contentView
+-- GPUI container NSView
    +-- GPUI base CAMetalLayer / view
    +-- WKWebView NSView
    +-- GPUI overlay CAMetalLayer / view (transparent)
```

GPUI currently has one complete scene and one `CAMetalLayer`. Putting that
entire view above WKWebView also puts the GPUI background above the page.
There are two options:

1. **Two GPUI surfaces:** render base and overlay scenes separately, with the
   WebView between them.
2. **Transparent hole punching:** keep one GPUI surface above the WebView,
   clear base pixels inside WebView bounds to transparent, and retain only
   overlay pixels in that region.

The second option touches less code but must validate a non-opaque
`CAMetalLayer`, clear alpha, window background, and text antialiasing together.
The current renderer derives `CAMetalLayer.opaque` from the window's
`transparent` option, so an ordinary opaque window cannot be assumed to
support local transparency. The first option has clearer, more durable
boundaries, but requires routing the GPUI scene by stacking plane.

### macOS prototype acceptance criteria

- Popovers, dialogs, and notifications cross WebView bounds with correct
  shadows and translucency.
- The WebView outside an overlay still receives mouse, scroll, keyboard, and
  drag/drop input.
- GPUI hit testing intercepts input inside the overlay and immediately returns
  it to the WebView after dismissal.
- Retina scale, live resize, fullscreen, multiple WebViews, WebView focus, and
  IME all work.
- Native bounds and clip remain consistent with the GPUI content mask when the
  WebView enters or leaves scrolling and clipping containers.

## Windows: Join the DirectComposition Visual Tree

### Why HWND z-order is insufficient

An ordinary wry WebView is a child HWND. Moving it behind the GPUI HWND has two
structural problems:

1. All GPUI pixels remain in one surface, which cannot represent
   `GPUI base < WebView < GPUI overlay`.
2. A child HWND is a rectangular native window. HWND z-order cannot express
   GPUI clipping, rounded corners, translucency, or per-element hit testing.

`SetWindowPos(HWND_BOTTOM)`, temporarily hiding the WebView, or creating a
transparent top-level window for each popover are only workarounds. The last
also creates cross-window focus, IME, movement, DPI, taskbar, and accessibility
consistency problems.

### Primary-source facts

- Microsoft provides `CreateCoreWebView2CompositionControllerAsync` for visual
  hosting. The host must set `RootVisualTarget` and forward mouse/pointer input
  to WebView
  ([Microsoft CreateCoreWebView2CompositionControllerAsync](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2environment.createcorewebview2compositioncontrollerasync)).
- `RootVisualTarget` may be an `IDCompositionVisual` or
  `Windows::UI::Composition::ContainerVisual`. WebView attaches its visual
  tree there; the application controls position, commit, and input
  ([Microsoft `ICoreWebView2CompositionController`](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2compositioncontroller)).
- Microsoft's API overview also states that a WebView2 composition tree can
  attach to `IDCompositionVisual`, `IDCompositionTarget`, or `ContainerVisual`
  ([Microsoft WebView2 API overview: Rendering using Composition](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis#rendering-webview2-using-composition)).
- GPUI's Windows renderer already creates an `IDCompositionDevice`,
  `IDCompositionTarget`, and root `IDCompositionVisual`, then sets the GPUI
  swap chain as the visual content
  ([GPUI `DirectComposition` source](https://github.com/zed-industries/zed/blob/66d95fb1945b7a7be671427f11ebfb42c339bdb4/crates/gpui_windows/src/directx_renderer.rs#L898-L917)).
- GPUI's composition swap chain uses `DXGI_ALPHA_MODE_PREMULTIPLIED`
  ([GPUI composition swap-chain source](https://github.com/zed-industries/zed/blob/66d95fb1945b7a7be671427f11ebfb42c339bdb4/crates/gpui_windows/src/directx_renderer.rs#L1180-L1203)).
  This provides the alpha mode needed for transparent visual composition, but
  does not prove that the current scene produces correct hole or overlay
  alpha. Rendering must still be split and tested.

### Recommended visual tree

```text
IDCompositionTarget (GPUI HWND)
+-- root
    +-- GPUI base visual (swap chain)
    +-- WebView slot visual
    |   +-- WebView2 composition tree
    +-- GPUI overlay visual (transparent swap chain)
```

This requires changing wry or implementing a Windows-specific WebView2 host.
wry 0.53.3 does not expose a composition controller. Beyond pixels, the host
must implement `SendMouseInput` / `SendPointerInput`, cursor, drag/drop, focus,
IME, accessibility provider, and DPI coordinate conversion. Visual output
without input is not a complete implementation.

### Windows implementation note

#### `lb-wry 0.53.3` capability boundary

Inspection of the pinned source gives a precise result: **it exposes an
ordinary controller to callers, but does not create, store, or expose a
Composition Controller**.

- `WebViewBuilderExtWindows` offers additional browser arguments, data
  directory, browser accelerator keys, theme, HTTPS scheme, environment, and
  incognito options, but no composition-hosting option
  ([wry 0.53.3 Windows builder extension](https://github.com/tauri-apps/wry/blob/v0.53.3/src/lib.rs#L1668-L1810)).
- `WebViewExtWindows::controller()` returns `ICoreWebView2Controller`
  ([wry 0.53.3 Windows WebView extension](https://github.com/tauri-apps/wry/blob/v0.53.3/src/lib.rs#L2224-L2263)).
  An already-created ordinary controller cannot be upgraded to a composition
  controller. The environment must create one through
  `CreateCoreWebView2CompositionController[WithOptions]`.
- The internal `WebView` stores `ICoreWebView2Controller`, but its construction
  path only calls `CreateCoreWebView2Controller` or
  `CreateCoreWebView2ControllerWithOptions`
  ([wry 0.53.3 controller construction](https://github.com/tauri-apps/wry/blob/v0.53.3/src/webview2/mod.rs#L549-L643)).
- Its drag/drop helper registers `IDropTarget` around wry's child HWND. It is
  not composition-controller drag forwarding
  ([wry 0.53.3 drag/drop controller](https://github.com/tauri-apps/wry/blob/v0.53.3/src/webview2/drag_drop.rs#L39-L97)).

It is therefore impossible to obtain `webview.controller()` outside
`gpui-wry` and add a `RootVisualTarget` afterward. The smallest correct change
must occur before wry constructs the controller. Maintain a narrow wry fork
rather than copying all initialization, protocol, navigation, IPC, and
lifecycle logic into `crates/webview`.

#### Composition Controller host responsibilities

Microsoft defines the following composition-hosting contract:

- `RootVisualTarget` accepts `IDCompositionVisual`, `IDCompositionTarget`, or
  `Windows::UI::Composition::ContainerVisual`
  ([WebView2 composition overview](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis#connecting-to-the-visual-tree)).
  Since GPUI already uses DirectComposition, pass a slot
  `IDCompositionVisual` created by **the same GPUI `IDCompositionDevice`**.
  Do not create an independent target tree for the same HWND.
- After `put_RootVisualTarget` connects the WebView tree, the host must still
  set the ordinary controller's `Bounds` and commit visual-tree changes on its
  device
  ([Microsoft `ICoreWebView2CompositionController`](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2compositioncontroller)).
- Mouse, touch, and pen first reach the parent HWND. The host converts them to
  WebView client coordinates and calls `SendMouseInput` or
  `SendPointerInput`. WebView reports cursor changes through `CursorChanged`;
  the host updates `WM_SETCURSOR` or the parent HWND. It must also send mouse
  leave events or cursor state becomes incorrect
  ([Microsoft composition input contract](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis#forwarding-input),
  [Microsoft cursor contract](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2compositioncontroller)).
- External drops do not enter WebView automatically. The host registers
  `IDropTarget`, forwards `DragEnter`, `DragOver`, `DragLeave`, and `Drop` to
  `ICoreWebView2CompositionController3`, and converts points to WebView client
  coordinates
  ([Microsoft `ICoreWebView2CompositionController3`](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2compositioncontroller3)).
- WebView remains a child of the parent HWND in the accessibility tree by
  default. Correct spatial and hierarchical integration with GPUI AccessKit
  may use the `IRawElementProviderSimple` provider returned by
  `ICoreWebView2CompositionController2::get_AutomationProvider`
  ([Microsoft `ICoreWebView2CompositionController2`](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2compositioncontroller2),
  [WebView2 composition accessibility overview](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis#accessibility)).
- Focus, visibility, bounds, and keyboard behavior use the ordinary
  `ICoreWebView2Controller` that the composition controller also implements:
  `MoveFocus`, Got/LostFocus, MoveFocusRequested, AcceleratorKeyPressed, and
  related APIs. The host must call `NotifyParentWindowPositionChanged` when
  the WebView parent or ancestor HWND moves; Microsoft says this is required
  for accessibility and WebView dialogs
  ([Microsoft `ICoreWebView2Controller`](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2controller)).
- Microsoft does not require forwarding `WM_IME_*` through a
  composition-specific API, because the interface has no IME forwarding
  method. Keyboard and IME still depend on the controller's internal HWND and
  focus. **GPUI must not continue consuming or rewriting these messages while
  WebView has focus.** Runtime validation with CJK input methods is required;
  successful COM compilation alone is not proof.

The last two points mean the input boundary is not simply "when GPUI hit
testing misses, call SendMouseInput." Pointer input can be forwarded
explicitly, but keyboard and IME require real controller focus. When Tab
leaves the WebView, the host must handle `MoveFocusRequested` and restore GPUI
focus.

#### Concrete changes to GPUI Windows DirectComposition

`DirectComposition` currently contains:

```text
IDCompositionTarget
+-- comp_visual(content = GPUI swap chain)
```

Its constructor creates one target and one visual, and `set_swap_chain` makes
that visual the root
([current GPUI DirectComposition wrapper](https://github.com/zed-industries/zed/blob/66d95fb1945b7a7be671427f11ebfb42c339bdb4/crates/gpui_windows/src/directx_renderer.rs#L898-L917)).
It must become:

```text
IDCompositionTarget
+-- root_visual
    +-- base_visual(content = base GPUI swap chain)
    +-- portal_container
    |   +-- webview_slot_visual(s)
    +-- overlay_visual(content = transparent overlay GPUI swap chain)
```

Concrete change points:

1. Add `root_visual`, `base_visual`, `portal_container`, and `overlay_visual`
   to `DirectComposition`. Call `comp_target.SetRoot` only during construction
   and device recovery; `set_swap_chain` must no longer replace the root.
2. Add an overlay swap chain, texture, and RTV to `DirectXResources`. The
   existing composition swap chain already uses
   `DXGI_ALPHA_MODE_PREMULTIPLIED`, which the overlay can reuse
   ([GPUI composition swap chain](https://github.com/zed-industries/zed/blob/66d95fb1945b7a7be671427f11ebfb42c339bdb4/crates/gpui_windows/src/directx_renderer.rs#L1180-L1203)).
3. Render the base scene and
   `paint_deferred_draws + prompt/drag/tooltip` scene to separate RTVs. The
   overlay clear must be `[0, 0, 0, 0]`, not `[1, 1, 1, 1]` as used by the
   current opaque-window path
   ([current GPUI `draw` clear selection](https://github.com/zed-industries/zed/blob/66d95fb1945b7a7be671427f11ebfb42c339bdb4/crates/gpui_windows/src/directx_renderer.rs#L301-L355)).
4. In one frame, portal update sets slot visual offset, clip, visibility, and
   order, then commits them with GPUI visual changes. `RootVisualTarget`
   attaches the tree; WebView `Bounds` still controls raster size.
5. Device-loss recovery currently drops and rebuilds all DirectComposition
   state and the swap chain
   ([GPUI renderer recovery path](https://github.com/zed-industries/zed/blob/66d95fb1945b7a7be671427f11ebfb42c339bdb4/crates/gpui_windows/src/directx_renderer.rs#L232-L300)).
   A portal handle must carry a generation. After recovery, the old slot visual
   is invalid and the WebView host must rebind `RootVisualTarget`; it cannot
   retain a raw COM pointer forever.
6. `GPUI_DISABLE_DIRECT_COMPOSITION` falls back to an HWND swap chain. This
   mode has no visual slot and must report that capability as unavailable. It
   must not silently fall back to the old child WebView while claiming overlay
   support.

`crates/webview` must not call `CreateTargetForHwnd` again for the GPUI HWND.
Microsoft permits at most one composition target per layer for an HWND, and
`topmost` decides whether the visual tree is above or below child windows
([Microsoft `CreateTargetForHwnd`](https://learn.microsoft.com/en-us/windows/desktop/api/dcomp/nf-dcomp-idcompositiondevice-createtargetforhwnd)).
GPUI already owns the topmost target; extend its root tree.

#### Transparent child HWND overlay versus Composition Controller

Keeping the wry child HWND and creating a transparent GPUI child or owned HWND
above it is suitable for a short spike, not a product architecture:

| Dimension | Transparent overlay HWND | Composition Controller |
| --- | --- | --- |
| Visual order | Depends on two HWNDs' z-order; the GPUI scene still needs splitting or duplication | Natively expresses base / WebView / overlay visual order |
| Clip, corners, animation | HWND region, layered window, and GPUI clip need duplicate synchronization | Visual offset, clip, and opacity commit in one transaction |
| Input | An extra WndProc must decide hit testing and pass-through | Pointer forwarding is an official WebView2 contract; still complex, but bounded |
| Focus, IME, accessibility | Cross-HWND focus, IME, and UIA spatial relationships are fragile | The controller retains official focus and UIA APIs |
| Resize and DPI | Two native windows require tear-free synchronization | Visual tree and swap chains commit together |
| Multiple WebViews and nested overlays | HWND count and z-order state grow rapidly | A portal container manages multiple slot visuals |

Windows 8 and later permit child `WS_EX_LAYERED` windows
([Microsoft extended window styles](https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles)),
but this proves only that the API can create one, not that it satisfies GPUI
per-element hit testing, focus, and atomic composition.
`WS_EX_TRANSPARENT` is not general input pass-through; Microsoft limits its
classic use to same-thread child hierarchies, while top-level use also requires
a layered style
([Microsoft DWM best practices](https://learn.microsoft.com/en-us/windows/win32/dwm/bestpractices-ovw)).

**Decision: use Composition Controller for the final implementation.**
A transparent overlay HWND is allowed only for early visual validation or an
"open a separate popup window" fallback when DirectComposition is disabled.
It must not implement the claim that arbitrary GPUI elements can cover a
WebView in the same window.

#### Minimum compilable API and module boundaries

Keep WebView2 initialization in a narrow wry fork and visual-tree ownership in
GPUI:

```rust
// gpui: cross-platform, no COM types
pub struct NativeSurfacePortal { /* opaque id + generation */ }

impl Window {
    pub fn create_native_surface_portal(
        &mut self,
        cx: &mut App,
    ) -> Result<NativeSurfacePortal>;
}

impl NativeSurfacePortal {
    pub fn capabilities(&self) -> NativeSurfaceCapabilities;
    pub fn set_geometry(&self, bounds: Bounds<Pixels>, clip: Bounds<Pixels>);
    pub fn set_visible(&self, visible: bool);
}

// gpui_windows: Windows-only extension trait exposing the COM attachment point
pub trait NativeSurfacePortalExtWindows {
    fn parent_hwnd(&self) -> HWND;
    fn root_visual_target(&self) -> IUnknown;
    fn generation(&self) -> u64;
}

// wry fork: Windows builder extension
pub trait WebViewBuilderExtWindowsComposition {
    fn build_as_composition_child(
        self,
        parent: HWND,
        root_visual_target: IUnknown,
    ) -> Result<WebView>;
}

pub trait WebViewExtWindowsComposition {
    fn composition_controller(
        &self,
    ) -> Option<ICoreWebView2CompositionController>;
}
```

Responsibilities:

- `gpui` / `gpui_windows`: visual creation and ordering, geometry and clip,
  overlay swap chain, commit, and device-loss generation.
- wry fork: environment and controller creation, existing WebView handlers and
  lifecycle, `RootVisualTarget` binding, and composition-controller exposure.
- `crates/webview/src/platform/windows.rs`: adapt GPUI mouse, pointer, focus,
  and drag events to the composition controller; observe cursor and focus; and
  rebind when the portal generation changes.
- `WebViewElement`: submit only declarative bounds, clip, visibility, and a
  GPUI hitbox; never manipulate the DComp tree directly.

`root_visual_target()` exposes a COM object rather than
`IDCompositionDevice`, preventing wry from modifying the full GPUI tree. The
portal handle guarantees the slot visual's lifetime.

#### Phased Windows implementation checklist

1. **Compilation gate:** add a composition construction path to the wry fork
   while reusing all existing WebView initialization. Type checks prove the
   ordinary path is unchanged and the composition path returns
   `ICoreWebView2CompositionController`.
2. **Visual-only gate:** GPUI creates the `base / slot / overlay` tree. Use a
   solid-color test visual instead of WebView to prove order, clip, resize,
   DPI, and device recovery.
3. **WebView picture gate:** bind WebView `RootVisualTarget` to the slot.
   Validate multiple WebViews, move/resize/visibility, scrolling clip,
   transparent overlay, and absence of one-frame z-order flashes.
4. **Mouse/cursor gate:** cover move, down, up, double-click, wheel,
   horizontal wheel, and leave; coordinate conversion, capture, and
   CursorChanged. Do not forward when overlay hit testing succeeds.
5. **Touch/pen/drag gate:** implement `SendPointerInput` and
   `ICoreWebView2CompositionController3` drag/drop. Validate internal WebView
   dragging and drag-in from other applications.
6. **Focus/IME gate:** validate click-to-focus, Tab/Shift-Tab,
   `MoveFocusRequested`, accelerators, CJK IME composition and candidate
   windows, and WebView dialogs. Call `NotifyParentWindowPositionChanged` when
   the window moves.
7. **Accessibility gate:** attach the WebView automation provider at the
   correct GPUI/AccessKit parent and screen bounds; validate Narrator order.
8. **Recovery/capability gate:** recreate and rebind slots after GPU device
   loss. Return explicit unsupported status when DComp is disabled and use a
   separate WebView window or popup fallback.

The first three stages prove only that WebView can display with an overlay.
Interactive, accessible, recoverable product support requires stages 4-8 too.

## Linux: GTK/WebKitGTK and Native Child-Window Constraints

### Primary-source facts

- wry explicitly states that `build_as_child` supports only X11 on Linux.
  Wayland should use `WebViewBuilderExtUnix::new_gtk` with `gtk::Fixed`
  ([wry build_as_child platform notes](https://docs.rs/wry/0.53.3/wry/struct.WebViewBuilder.html#method.build_as_child)).
- `GtkFixed` positions child widgets in pixels and performs no automatic
  layout
  ([GTK 3 `GtkFixed`](https://docs.gtk.org/gtk3/class.Fixed.html)).
- `GtkOverlay` places overlay widgets above its main child, and an overlay
  child's index determines draw order when children overlap
  ([GTK 3 `GtkOverlay`](https://docs.gtk.org/gtk3/class.Overlay.html),
  [GTK 3 `reorder_overlay`](https://docs.gtk.org/gtk3/method.Overlay.reorder_overlay.html)).
- Normal GTK 3 drawing propagates from the toplevel through the widget
  hierarchy in back-to-front order. The documentation also recognizes that a
  toplevel may contain multiple native subwindows. Only content in the same
  GTK/GDK hierarchy is naturally governed by GTK overlay ordering
  ([GTK 3 Drawing Model](https://docs.gtk.org/gtk3/drawing-model.html)).
- WebKitGTK's public snapshot API asynchronously captures a visible region or
  full document and returns a static snapshot. It does not expose WebKit's
  live GPU surface to GPUI
  ([WebKitGTK `WebView.get_snapshot`](https://webkitgtk.org/reference/webkit2gtk/stable/method.WebView.get_snapshot.html)).
- GTK explicitly limits `OffscreenWindow` to snapshots of widgets outside a
  normal widget hierarchy. It is itself a toplevel and cannot be embedded in
  another toplevel
  ([GTK 3 `OffscreenWindow` index entry](https://docs.gtk.org/gtk3/index.html#classes)).

### Feasibility

On X11, treating the WebKitGTK/X11 child window and GPUI surface as two
rectangular native windows and changing stacking still cannot express
arbitrary overlays inside GPUI. wry explicitly does not support this child
window route on Wayland.

In theory, WebKitWebView can be the main child of `GtkOverlay`, with a GTK
widget capable of presenting the GPUI overlay surface above it. That requires
the GPUI Linux window backend itself to integrate with GTK/GDK surface
lifetime and coordinate renderer surfaces, event loops, input, and Wayland
subsurfaces. It is not a local `crates/webview` change.

The snapshot route can draw a WebView image as a GPUI texture and obtain
arbitrary GPUI z-order. The official API is asynchronous, however, with no
evidence that it supports real-time scrolling, video, animation, or
low-latency input. It is only a preview or frozen-state fallback, not the final
interactive WebView implementation.

## Recommended GPUI Core Abstraction

Do not expose platform details as a vague `z_index(i32)`. Introduce a limited,
testable stacking plane:

```rust
enum NativeSurfacePlane {
    BelowGpui,
    BetweenBaseAndOverlay,
    AboveGpui,
}
```

`BetweenBaseAndOverlay` is the main path. GPUI also needs to:

- Collect window-space bounds, content mask or clip, and visibility for each
  native slot every frame.
- Define the native portal insertion point between the end of
  `root_element.paint` and the start of `paint_deferred_draws`. `Deferred`
  already promises to paint after its ancestors, with higher priority closer
  to the viewer
  ([GPUI `Deferred` source](https://github.com/zed-industries/zed/blob/66d95fb1945b7a7be671427f11ebfb42c339bdb4/crates/gpui/src/elements/deferred.rs#L7-L96)).
- Continue collecting the window prompt, active drag, and core tooltip after
  the deferred overlay scene; the current pipeline paints them later.
- Avoid submitting an additional surface when the overlay plane is empty.
- Hit-test before native input forwarding, so a transparent overlay view does
  not consume all WebView input.
- Create, update, and destroy native slots on the platform UI thread in sync
  with the GPUI frame lifecycle.

The platform trait should describe capabilities instead of pretending all
platforms are identical:

```rust
struct NativeSurfaceCapabilities {
    can_embed_between_gpui_planes: bool,
    can_clip_non_rectangular: bool,
    requires_forwarded_pointer_input: bool,
}
```

Linux can then report missing capabilities instead of silently degrading to
"WebView is always on top."

### Existing overlay coverage audit

The current split directly covers:

- `Popover`, `ContextMenu`, `PopupMenu`, and fallback native menu.
- Popups for `Select`, `Combobox`, and `DatePicker`.
- Input completion, code-action, and hover popups.
- `gpui-component::Tooltip` and plot tooltip.

These implementations use `gpui::deferred`; examples include
[`Popover`](https://github.com/longbridge/gpui-component/blob/be3c8413766cafc736a0c1c80306ff0f293e04f3/crates/ui/src/popover.rs#L335-L350),
[`PopupMenu`](https://github.com/longbridge/gpui-component/blob/be3c8413766cafc736a0c1c80306ff0f293e04f3/crates/ui/src/menu/popup_menu.rs#L1330-L1345),
and
[`Tooltip`](https://github.com/longbridge/gpui-component/blob/be3c8413766cafc736a0c1c80306ff0f293e04f3/crates/ui/src/tooltip.rs#L489-L515).

Not every visually overlay-like component was originally in the deferred
plane:

- The GPUI window prompt, active drag, and core tooltip are not in
  `deferred_draws`, but `draw_roots` paints them separately afterward. The
  final phase must also target the top surface.
- `Root::render_dialog_layer`, `render_sheet_layer`, and
  `render_notification_layer` return ordinary absolute or relative elements
  and do not wrap themselves in `deferred`
  ([Root layer source](https://github.com/longbridge/gpui-component/blob/be3c8413766cafc736a0c1c80306ff0f293e04f3/crates/ui/src/root.rs#L157-L277)).
  If a caller adds them as ordinary later root children, they paint after the
  WebView element but remain in the base scene and cannot cross the native
  portal automatically.
- Dialog, Sheet, and Notification therefore need explicit migration to
  deferred or a more direct GPUI `OverlayPlane` element. Migration must test
  content masks, modal hitboxes, focus traps, animation, and nested popovers.
  Splitting the renderer without migrating these Root layers is incomplete.

The current spike wraps all three Root layers in `deferred`. This is required
to validate the architecture, but sheet animation, notification placement,
nested popups, and modal focus traps still need dedicated regression testing
before upstreaming.

`draw_roots` calculates `mouse_hit_test` only after prepainting all deferred
elements, prompts, drags, and tooltips. This is useful for native input routing:
run the existing GPUI hit test first and forward an unhandled event to the
composition WebView. Platform events still reach the GPUI window first, so
Windows must explicitly convert unconsumed events to `SendMouseInput` or
`SendPointerInput`.

## Implementation Order and Decision Gates

1. **macOS spike:** prove
   `GPUI base / WKWebView / GPUI overlay` pixels and input in one window. Hole
   punching may validate the API if stable, but still compare it with two
   surfaces.
2. **GPUI scene separation:** use
   `root paint / native portal / deferred paint / prompt-drag-tooltip paint`
   as the existing skeleton. Promote the overlay plane from a platform hack
   into a renderer/window abstraction, move Dialog, Sheet, and Notification
   into the top plane, and validate alpha, resize, and hit testing with a
   non-WebView test visual.
3. **Windows composition host:** reuse GPUI's `IDCompositionDevice` and root
   tree, create a WebView2 composition controller, and complete pointer,
   drag/drop, IME, and accessibility support.
4. **Linux feasibility spike:** test a GTK-hosted WebKitWebView with a GPUI
   overlay widget on X11 and Wayland separately. Promise support only after
   live pixels and input both work; snapshots do not pass.

Re-evaluate the unified native-embedding route if any of these occur:

- macOS cannot produce stable local transparent GPUI overlays in an ordinary
  opaque window.
- The Windows WebView2 composition visual cannot safely share the lifecycle of
  GPUI's device and target.
- Wayland requires a complete GPUI event/window-backend rewrite with
  unacceptable maintenance cost.

Even if Linux cannot reach parity, macOS and Windows can share native-slot and
overlay-plane semantics while reporting different platform capabilities.

## 2026-07-30 macOS Spike Results

The current worktree implements and has run the first phase:

- A GPUI frame records a scene-operation boundary after the root and inspector
  but before deferred and window overlays.
- `PlatformWindow::draw_layered` preserves single-surface behavior by default.
  The macOS backend replays operations on either side of the boundary into
  base and transparent-overlay Metal renderers.
- Both renderers share one Metal device and sprite atlas so atlas tile IDs
  remain valid in the second renderer.
- After WKWebView joins the view hierarchy, the macOS window creates a
  transparent sibling overlay `NSView/CAMetalLayer`, producing
  `base CAMetalLayer / WKWebView / overlay CAMetalLayer`.
- When the overlay has no drawable primitives, `hitTest:` returns `nil` and
  WebView receives input. With Popover, Dialog, or similar content, input goes
  to GPUI so outside clicks can dismiss the overlay.
- Dialog, Sheet, and Notification have moved into the deferred/top plane.
- The WebView example uses an 800x600 window and provides both Popover and
  Dialog scenarios.

Runtime validation covered:

1. A Popover renders completely above a real `WKWebView`.
2. Clicking the WebView area outside the Popover closes it without a crash.
3. Dialog, translucent backdrop, text, and buttons render above WebView.
4. The Dialog close hitbox works, and WebView continues working afterward.
5. Notification renders completely above a running WebView page.
6. PopupMenu background, labels, separators, and link items render completely
   above WebView.
7. WebView remains clickable while the overlay is empty.
8. Resize and drawable-size updates reach both renderers.

### Focus ownership is part of the portal contract

A native WebView and GPUI maintain separate focus systems. Giving native
keyboard focus to the WebView does not automatically clear GPUI's focused
element. Without an explicit handoff, a GPUI `Input` can keep drawing its
blinking caret after the user clicks the WebView, even though subsequent
typing belongs to the WebView. The result presents two apparent input targets
and makes keyboard ownership ambiguous.

When GPUI receives a mouse-down inside WebView bounds, the WebView element
treats it as a focus boundary and clears GPUI window focus. This covers the
top-plane path where the event dismisses a Popover over the WebView region.
When the top plane is empty, AppKit sends the event directly to `WKWebView`.
`gpui-wry` therefore installs a lifecycle-scoped local `NSEvent` monitor for
mouse-down events. The monitor checks the event window, native-hit-tests the
content view, and clears this GPUI window's logical focus only when the hit
view is the managed `WKWebView` or one of its descendants. The monitor token
is removed when the `WebView` is dropped.

Keeping this handoff in `gpui-wry` is deliberate. It avoids adding a
WebView-specific focus event, AppKit responder override, or native-descendant
classification to GPUI. The GPUI platform patch stays limited to scene
composition, while the component that owns the embedded native view also owns
its focus boundary. Returning from WebView to GPUI uses the normal GPUI focus
path.

This is not example-only polish. Any native-surface-slot API must define:

- which focus system owns keyboard and IME input at every point;
- how pointer, Tab, Shift-Tab, programmatic focus, and overlay dismissal
  transfer that ownership;
- that the losing system immediately removes visible focus indicators,
  including carets and focus rings;
- that only the owner receives text, composition, and accessibility focus
  events.

Regression validation must first focus the address `Input`, confirm its caret
is visible, and then click the WebView both with no overlay and while a Popover
is open. Both paths stop the GPUI caret as WebView receives focus. Runtime
validation confirmed that the address Input loses its focus indication after
a direct WebView click and that clicking the WebView region dismisses an open
Popover. The reverse transition must restore exactly one GPUI focus target.

On macOS, `Cmd+C`, `Cmd+V`, `Cmd+X`, and `Cmd+A` are AppKit key equivalents
routed through standard Edit-menu selectors and the responder chain. A bare
GPUI example without those menu items can display and focus WKWebView while
copy and paste shortcuts still do nothing. The example therefore installs an
Edit menu whose `MenuItem::os_action` entries map to Cut, Copy, Paste, and
Select All. When WKWebView owns native focus, AppKit routes those selectors to
WebKit; when GPUI owns focus, the same items dispatch the corresponding GPUI
input actions. Native-surface examples and host applications must preserve
this responder-chain integration rather than implementing clipboard shortcuts
as WebView-specific JavaScript. Runtime validation copied a sentinel from a
GPUI Input into a WebView text field and another sentinel from the WebView back
into the GPUI Input using only `Cmd+C` and `Cmd+V`.

The Notification layer creates a deferred element only when the list is
non-empty. This avoids a permanent empty top-plane node and unnecessary scene
operations. After opening a Dialog or Notification, the WebView example
explicitly invalidates itself because the example entity currently composes
these Root layers while the state lives in the `Root` entity.

The spike found and fixed three non-obvious issues:

- An independent renderer with an independent atlas fails because replayed
  tile IDs do not exist in the second atlas.
- Re-locking window state inside Objective-C `hitTest:` causes a re-entrant
  deadlock.
- Passing the overlay `NSView` itself to GPUI input processing violates
  `NSTextInputContext` assumptions and causes a null-pointer abort. Events must
  borrow the original GPUI native view's input context.

To keep a transparent overlay from permanently intercepting input,
`Scene::is_empty` checks actual drawable primitives rather than paint-operation
count. Empty `StartLayer/EndLayer` operations still increase `Scene::len()`.
Unit tests cover empty layers, drawable primitives, and replay.

### What the spike has not proved

- The top plane is currently a window-level switch and one scene split point,
  not a complete native-surface-slot API. Multiple WebViews in one window and
  interleaved ordering between them are not expressible.
- If the overlay has any drawable primitive, the full transparent NSView takes
  input. This supports modal and outside-click behavior, but the production
  API should carry top-plane hit regions and explicit `capture`,
  `dismiss-then-consume`, and `pass-through` policies.
- macOS still needs regression testing for IME, drag/drop, accessibility,
  fullscreen, scale-factor changes, multiple windows, WebView focus, and live
  resize.
- Windows CompositionController and Linux GTK/WebKitGTK paths are not
  implemented. macOS success does not prove cross-platform completion.
