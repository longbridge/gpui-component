use gpui::*;
use gpui_component::{
    ActiveTheme as _, Root, WindowExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Copy, Cut, Input, InputEvent, InputState, Paste, SelectAll},
    menu::DropdownMenu as _,
    popover::Popover,
    v_flex,
};
use gpui_wry::WebView;

const TEST_HTML: &str = r#"<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>Local WebView input test</title>
<style>
  :root { color-scheme: dark; font: 14px/1.5 system-ui, sans-serif;
    --background: #0a0a0a; --panel: #171717; --input: #0a0a0a;
    --border: #262626; --foreground: #fafafa; --muted: #a3a3a3; --accent: #3b82f6; }
  * { box-sizing: border-box; }
  body { margin: 0; min-height: 100vh; padding: 24px; color: var(--foreground); background: var(--background); }
  main { max-width: 760px; margin: auto; padding: 24px; border: 1px solid var(--border);
    border-radius: 8px; background: var(--panel); }
  h1 { margin: 0 0 4px; font-size: 22px; font-weight: 600; }
  h2 { margin: 24px 0 8px; font-size: 13px; color: var(--muted); font-weight: 600; }
  p { margin: 0 0 18px; color: var(--muted); }
  code { color: var(--foreground); }
  label { display: block; margin: 14px 0; color: var(--muted); font-size: 13px; }
  input, textarea { display: block; width: 100%; margin-top: 6px; padding: 9px 11px; border: 1px solid #2f2f2f;
    border-radius: 5px; outline: none; color: var(--foreground); background: var(--input); font: inherit; }
  input:focus, textarea:focus { border-color: var(--accent); }
  textarea { min-height: 90px; resize: vertical; }
  button { padding: 8px 13px; border: 1px solid #fafafa; border-radius: 5px; color: #0a0a0a;
    background: #fafafa; font: inherit; cursor: pointer; }
  button:hover { background: #f5f5f5; }
  #status { margin-top: 18px; padding: 8px 10px; border: 1px solid var(--border); border-radius: 5px;
    color: var(--muted); background: var(--input); }
  #keyboard { min-height: 40px; margin: 0; padding: 10px; overflow: hidden; white-space: nowrap;
    border: 1px solid var(--border); border-radius: 5px; color: var(--foreground); background: var(--input);
    font: 12px/1.5 ui-monospace, SFMono-Regular, Consolas, monospace; }
</style>
<main>
  <h1>Local WebView input test</h1>
  <p>Click each field and type: <code>abc XYZ 123</code></p>
  <label>English input
    <input id="english" autofocus placeholder="Type English here">
  </label>
  <label>中文 / IME input
    <input id="ime" placeholder="请输入中文">
  </label>
  <label>Multiline input
    <textarea id="multiline" placeholder="Type multiple lines here"></textarea>
  </label>
  <button id="button" type="button">WebView button</button>
  <div id="status">Focus: none</div>
  <h2>Keyboard events</h2>
  <pre id="keyboard">Click the page, then press keys...</pre>
</main>
<script>
  const status = document.getElementById('status');
  const keyboard = document.getElementById('keyboard');
  const keyHistory = [];
  function record(type, e) {
    const key = e.key === ' ' ? 'Space' : (e.key || type);
    const target = e.target && e.target.id ? e.target.id : (e.target && e.target.tagName ? e.target.tagName : '-');
    const value = e.target && typeof e.target.value === 'string' ? '=' + JSON.stringify(e.target.value) : '';
    keyHistory.push(type + '(' + key + '/' + e.code + ' @' + target + value + ')');
    while (keyHistory.length > 12) keyHistory.shift();
    keyboard.textContent = keyHistory.join('  →  ');
  }
  document.addEventListener('focusin', e => status.textContent = 'Focus: ' + (e.target.id || e.target.tagName));
  document.addEventListener('focusout', e => status.textContent = 'Focus: none');
  for (const type of ['keydown', 'keyup', 'keypress', 'input', 'compositionstart', 'compositionend']) {
    document.addEventListener(type, e => record(type, e));
  }
  document.getElementById('button').onclick = () => status.textContent = 'Button clicked';
</script>
"#;

pub struct Example {
    focus_handle: FocusHandle,
    webview: Entity<WebView>,
    address_input: Entity<InputState>,
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let webview = cx.new(|cx| {
            let builder = wry::WebViewBuilder::new().with_html(TEST_HTML);
            #[cfg(any(debug_assertions, feature = "inspector"))]
            let builder = builder.with_devtools(true);

            #[cfg(not(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
                target_os = "android"
            )))]
            {
                use gtk::prelude::*;
                use wry::WebViewBuilderExtUnix;
                // borrowed from https://github.com/tauri-apps/wry/blob/dev/examples/gtk_multiwebview.rs
                // doesn't work yet
                // TODO: How to initialize this fixed?
                let fixed = gtk::Fixed::builder().build();
                fixed.show_all();
                let webview = builder.build_gtk(&fixed).unwrap();
                return WebView::new(webview, window, cx);
            }
            #[cfg(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
                target_os = "android"
            ))]
            {
                use raw_window_handle::HasWindowHandle;

                let window_handle = window.window_handle().expect("No window handle");
                WebView::build_as_child(builder, &window_handle, window, cx).unwrap()
            }
        });

        let address_input = cx.new(|cx| {
            InputState::new(window, cx).default_value("https://longbridge.github.io/gpui-component")
        });

        cx.new(|cx| {
            let this = Self {
                focus_handle: cx.focus_handle(),
                webview,
                address_input: address_input.clone(),
            };

            cx.subscribe(
                &address_input,
                |this: &mut Self, input, event: &InputEvent, cx| match event {
                    InputEvent::PressEnter { .. } => {
                        let url = input.read(cx).value().clone();
                        this.webview.update(cx, |view, _| {
                            view.load_url(&url);
                        });
                    }
                    _ => {}
                },
            )
            .detach();

            this
        })
    }

    pub fn hide(&self, _: &mut Window, cx: &mut App) {
        self.webview.update(cx, |webview, _| webview.hide())
    }

    #[allow(unused)]
    fn go_back(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.webview.update(cx, |webview, _| {
            webview.back().unwrap();
        });
    }

    fn show_dialog(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.open_dialog(cx, |dialog, _, _| {
            dialog.title("WebView overlay dialog").child(
                v_flex()
                    .gap_3()
                    .w(px(520.))
                    .child("This modal, its translucent backdrop, text, and controls are all GPUI.")
                    .child(
                        div()
                            .p_4()
                            .rounded_lg()
                            .border_1()
                            .child("The native WKWebView must remain visible behind the backdrop."),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("dialog-secondary").label("Secondary action"))
                            .child(
                                Button::new("dialog-primary")
                                    .primary()
                                    .label("Primary action"),
                            ),
                    ),
            )
        });
        cx.notify();
    }

    fn show_notification(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.push_notification(
            "This GPUI notification is rendered above the native WebView.",
            cx,
        );
        cx.notify();
    }
}

impl Focusable for Example {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let webview = self.webview.clone();

        v_flex()
            .p_2()
            .gap_3()
            .size_full()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Input::new(&self.address_input))
                    .child(
                        Popover::new("webview-overlay-proof")
                            .trigger(Button::new("overlay-trigger").label("Overlay proof"))
                            .w(px(320.))
                            .child("This GPUI popover is rendered above the native WebView."),
                    )
                    .child(
                        Button::new("dialog-trigger")
                            .label("Open dialog")
                            .on_click(cx.listener(Self::show_dialog)),
                    )
                    .child(
                        Button::new("notification-trigger")
                            .label("Notify")
                            .on_click(cx.listener(Self::show_notification)),
                    )
                    .child(
                        Button::new("popup-menu-trigger")
                            .label("Popup menu")
                            .dropdown_menu(|menu, _, _| {
                                menu.label("GPUI above WebView").separator().link(
                                    "Open GPUI Component",
                                    "https://longbridge.github.io/gpui-component/",
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .border_1()
                    .h(gpui::px(400.))
                    .border_color(cx.theme().border)
                    .child(webview.clone()),
            )
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

fn main() {
    gpui_platform::application().run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);
        cx.set_menus([Menu::new("Edit").items([
            MenuItem::os_action("Cut", Cut, OsAction::Cut),
            MenuItem::os_action("Copy", Copy, OsAction::Copy),
            MenuItem::os_action("Paste", Paste, OsAction::Paste),
            MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
        ])]);
        let window_bounds = WindowBounds::centered(size(px(800.), px(600.)), cx);

        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(window_bounds),
                    ..Default::default()
                },
                |window, cx| {
                    let view = Example::new(window, cx);
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}
