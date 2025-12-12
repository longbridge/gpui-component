use gpui::*;
use gpui_component::{
    button::Button,
    h_flex,
    text::{TextView, TextViewState},
    v_flex,
};
use gpui_component_assets::Assets;

pub struct Example {
    markdown_state: Entity<TextViewState>,
    tx: smol::channel::Sender<String>,
    scroll_handle: ScrollHandle,
    _task: Task<()>,
    _update_task: Task<()>,
}

const EXAMPLE: &str = include_str!("./fixtures/test.md");

impl Example {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let markdown_state =
            cx.new(|cx| TextViewState::markdown("# Streaming Markdown Parse\n\n", cx));
        let scroll_handle = ScrollHandle::new();

        let (tx, rx) = smol::channel::unbounded::<String>();
        let _task = cx.spawn({
            let scroll_handle = scroll_handle.clone();
            let weak_state = markdown_state.downgrade();
            async move |_, cx| {
                while let Ok(chunk) = rx.recv().await {
                    _ = weak_state.update(cx, |state, cx| {
                        // Push the new chunk to the markdown state,
                        // it will reparse and re-render automatically.
                        state.push_str(&chunk, cx);
                        scroll_handle.scroll_to_bottom();
                    });
                }
            }
        });

        Self {
            markdown_state,
            scroll_handle,
            tx,
            _task,
            _update_task: Task::ready(()),
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    /// Simulate streaming by updating markdown state in chunks
    /// 50ms for a iteration, every time adding about 5 - 20 characters
    /// This is just for demonstration; in a real app, you'd stream from a source.
    fn replay(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let tx = self.tx.clone();
        let mut current = 0;
        self.markdown_state.update(cx, |state, cx| {
            state.set_text("", cx);
        });

        self._update_task = cx.background_executor().spawn(async move {
            let chars: Vec<char> = EXAMPLE.chars().collect();
            while current < chars.len() {
                let chunk_size = (5 + rand::random::<usize>() % 15).min(chars.len() - current);
                let chunk: String = chars[current..current + chunk_size].iter().collect();
                _ = tx.try_send(chunk);
                current += chunk_size;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        });
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("example")
            .size_full()
            .p_4()
            .gap_4()
            .child(
                h_flex()
                    .w_full()
                    .child(
                        Button::new("replay")
                            .outline()
                            .label("Replay")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.replay(window, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .id("contents")
                    .flex_1()
                    .w_full()
                    .track_scroll(&self.scroll_handle)
                    .overflow_y_scroll()
                    .size_full()
                    .child(TextView::new(&self.markdown_state).selectable(true)),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        cx.activate(true);

        gpui_component_story::create_new_window_with_size(
            "Stream Markdown",
            Some(size(px(600.), px(800.))),
            Example::view,
            cx,
        );
    });
}
