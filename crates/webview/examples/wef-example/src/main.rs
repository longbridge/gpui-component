use std::time::Duration;

use futures_util::StreamExt;
use gpui::{
    App, AppContext, Application, Bounds, Context, Entity, IntoElement, ParentElement, Render,
    Styled, Timer, Window, WindowBounds, WindowOptions, div, px, size,
};
use gpui_component::{
    Root,
    input::{InputEvent, InputState, TextInput},
};
use gpui_webview::{
    WebView,
    events::TitleChangedEvent,
    wef::{self, Frame, FuncRegistry, Settings},
};
use serde::Serialize;

struct Main {
    address_state: Entity<InputState>,
    webview: Entity<WebView>,
}

impl Main {
    fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let background_executor = cx.background_executor().clone();

        let func_registry = FuncRegistry::builder()
            .with_spawner(move |fut| {
                background_executor.spawn(fut).detach();
            })
            .register("toUppercase", |value: String| value.to_uppercase())
            .register("addInt", |a: i32, b: i32| a + b)
            .register("parseInt", |value: String| value.parse::<i32>())
            .register_async("sleep", |millis: u64| async move {
                Timer::after(Duration::from_millis(millis)).await;
                "ok"
            })
            .register("emit", |frame: Frame| {
                #[derive(Debug, Serialize)]
                struct Message {
                    event: String,
                    data: String,
                }

                frame.emit(Message {
                    event: "custom".to_string(),
                    data: "ok".to_string(),
                });
            })
            .build();

        cx.new(|cx| {
            let url = "https://www.google.com";

            // create webview
            let webview = WebView::with_func_registry(url, func_registry.clone(), window, cx);

            window
                .subscribe(&webview, cx, |_, event: &TitleChangedEvent, window, _| {
                    window.set_window_title(&event.title);
                })
                .detach();

            // create address input
            let address_state = cx.new(|cx| InputState::new(window, cx).default_value(url));

            window
                .subscribe(&address_state, cx, {
                    let webview = webview.clone();
                    move |state, event: &InputEvent, _, cx| {
                        if let InputEvent::PressEnter { .. } = event {
                            let url = state.read(cx).value();
                            webview.read(cx).browser().load_url(&url);
                        }
                    }
                })
                .detach();

            Self {
                address_state,
                webview,
            }
        })
    }
}

impl Render for Main {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(TextInput::new(&self.address_state))
            .child(self.webview.clone())
            .children(Root::render_modal_layer(window, cx))
    }
}

fn run() {
    Application::new().run(|cx: &mut App| {
        if cfg!(target_os = "linux") {
            cx.spawn(async move |cx| {
                let (tx, rx) = flume::unbounded();

                cx.background_spawn(async move {
                    let mut timer = Timer::interval(Duration::from_millis(1000 / 60));
                    while timer.next().await.is_some() {
                        _ = tx.send_async(()).await;
                    }
                })
                .detach();

                while rx.recv_async().await.is_ok() {
                    wef::do_message_work();
                }
            })
            .detach();
        }

        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(500.), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let main = Main::new(window, cx);
                cx.new(|cx| Root::new(main.into(), window, cx))
            },
        )
        .unwrap();
        cx.activate(true);
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    wef::launch(Settings::new(), run);
    Ok(())
}
