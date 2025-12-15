use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::Poll,
    time::Duration,
};

use gpui::{
    App, AppContext as _, Bounds, ClipboardItem, Context, FocusHandle, IntoElement, KeyBinding,
    ListState, ParentElement as _, Pixels, Point, Render, SharedString, Size, Styled as _, Task,
    Window, prelude::FluentBuilder as _, px,
};
use smol::{Timer, stream::StreamExt as _};

use crate::{
    ActiveTheme, ElementExt,
    highlighter::HighlightTheme,
    input::{self, Copy},
    text::{
        CodeBlockActionsFn, TextViewStyle,
        document::ParsedDocument,
        format,
        node::{self, NodeContext},
    },
    v_flex,
};

const UPDATE_DELAY: Duration = Duration::from_millis(50);

const CONTEXT: &'static str = "TextView";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys(vec![
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", input::Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", input::Copy, Some(CONTEXT)),
    ]);
}

/// The content format of the text view.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TextViewFormat {
    /// Markdown view
    Markdown,
    /// HTML view
    Html,
}

/// The state of a TextView.
pub struct TextViewState {
    pub(super) focus_handle: FocusHandle,
    pub(super) list_state: ListState,

    /// The bounds of the text view
    bounds: Bounds<Pixels>,

    pub(super) selectable: bool,
    pub(super) scrollable: bool,
    pub(super) text_view_style: TextViewStyle,
    pub(super) code_block_actions: Option<Arc<CodeBlockActionsFn>>,

    pub(super) is_selecting: bool,
    /// The local (in TextView) position of the selection.
    selection_positions: (Option<Point<Pixels>>, Option<Point<Pixels>>),

    pub(super) parsed_content: Arc<Mutex<ParsedContent>>,
    text: SharedString,
    parsed_error: Option<SharedString>,
    tx: smol::channel::Sender<UpdateOptions>,
    _parse_task: Task<()>,
    _receive_task: Task<()>,
}

impl TextViewState {
    /// Create a Markdown TextViewState.
    pub fn markdown(text: &str, cx: &mut Context<Self>) -> Self {
        Self::new(TextViewFormat::Markdown, text, cx)
    }

    /// Create a HTML TextViewState.
    pub fn html(text: &str, cx: &mut Context<Self>) -> Self {
        Self::new(TextViewFormat::Html, text, cx)
    }

    /// Create a new TextViewState.
    fn new(format: TextViewFormat, text: &str, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let (tx, rx) = smol::channel::unbounded::<UpdateOptions>();
        let (tx_result, rx_result) = smol::channel::unbounded::<Result<(), SharedString>>();
        let _receive_task = cx.spawn({
            async move |weak_self, cx| {
                while let Ok(parsed_result) = rx_result.recv().await {
                    _ = weak_self.update(cx, |state, cx| {
                        if let Err(err) = &parsed_result {
                            state.parsed_error = Some(err.clone());
                        }
                        state.clear_selection();
                        cx.notify();
                    });
                }
            }
        });

        let _parse_task = cx.background_spawn(UpdateFuture::new(format, rx, tx_result, cx));

        let mut this = Self {
            focus_handle,
            bounds: Bounds::default(),
            selection_positions: (None, None),
            selectable: false,
            scrollable: false,
            list_state: ListState::new(0, gpui::ListAlignment::Top, px(1000.)),
            text_view_style: TextViewStyle::default(),
            code_block_actions: None,
            is_selecting: false,
            parsed_content: Default::default(),
            parsed_error: None,
            text: text.to_string().into(),
            tx,
            _parse_task,
            _receive_task,
        };
        this.increment_update(&text, false, cx);
        this
    }

    /// Get the text content.
    pub(crate) fn source(&self) -> SharedString {
        self.parsed_content.lock().unwrap().document.source.clone()
    }

    /// Set whether the text is selectable, default false.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Set whether the text is selectable, default false.
    pub fn set_selectable(&mut self, selectable: bool, cx: &mut Context<Self>) {
        self.selectable = selectable;
        cx.notify();
    }

    /// Set whether the text is selectable, default false.
    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }

    /// Set whether the text is selectable, default false.
    pub fn set_scrollable(&mut self, scrollable: bool, cx: &mut Context<Self>) {
        self.scrollable = scrollable;
        cx.notify();
    }

    /// Set the text content.
    pub fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if self.text.as_str() == text {
            return;
        }

        self.text = text.to_string().into();
        self.parsed_error = None;
        self.increment_update(text, false, cx);
    }

    /// Append partial text content to the existing text.
    pub fn push_str(&mut self, new_text: &str, cx: &mut Context<Self>) {
        if new_text.is_empty() {
            return;
        }
        self.increment_update(new_text, true, cx);
    }

    /// Return the selected text.
    pub fn selected_text(&self) -> String {
        self.parsed_content.lock().unwrap().document.selected_text()
    }

    fn increment_update(&mut self, text: &str, append: bool, cx: &mut Context<Self>) {
        let code_block_actions = self.code_block_actions.clone();
        let update_options = UpdateOptions {
            append,
            content: self.parsed_content.clone(),
            pending_text: text.to_string(),
            highlight_theme: cx.theme().highlight_theme.clone(),
            code_block_actions: code_block_actions.clone(),
        };

        // Parse at first time by blocking.
        _ = self.tx.try_send(update_options);
    }

    /// Save bounds and unselect if bounds changed.
    pub(super) fn update_bounds(&mut self, bounds: Bounds<Pixels>) {
        if self.bounds.size != bounds.size {
            self.clear_selection();
        }
        self.bounds = bounds;
    }

    pub(super) fn clear_selection(&mut self) {
        self.selection_positions = (None, None);
        self.is_selecting = false;
    }

    pub(super) fn start_selection(&mut self, pos: Point<Pixels>) {
        let pos = pos - self.bounds.origin;
        self.selection_positions = (Some(pos), Some(pos));
        self.is_selecting = true;
    }

    pub(super) fn update_selection(&mut self, pos: Point<Pixels>) {
        let pos = pos - self.bounds.origin;
        if let (Some(start), Some(_)) = self.selection_positions {
            self.selection_positions = (Some(start), Some(pos))
        }
    }

    pub(super) fn end_selection(&mut self) {
        self.is_selecting = false;
    }

    pub(crate) fn has_selection(&self) -> bool {
        if let (Some(start), Some(end)) = self.selection_positions {
            start != end
        } else {
            false
        }
    }

    /// Return the bounds of the selection in window coordinates.
    pub(crate) fn selection_bounds(&self) -> Bounds<Pixels> {
        selection_bounds(
            self.selection_positions.0,
            self.selection_positions.1,
            self.bounds,
        )
    }

    pub(super) fn on_action_copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        let selected_text = self.selected_text().trim().to_string();
        if selected_text.is_empty() {
            return;
        }

        cx.write_to_clipboard(ClipboardItem::new_string(selected_text));
    }

    pub(crate) fn is_selectable(&self) -> bool {
        self.selectable
    }
}

impl Render for TextViewState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.entity();
        let (document, node_cx) = {
            let content = self.parsed_content.lock().unwrap();
            (content.document.clone(), content.node_cx.clone())
        };

        v_flex()
            .size_full()
            .map(|this| match &mut self.parsed_error {
                None => this.child(document.render_root(
                    if self.scrollable {
                        Some(self.list_state.clone())
                    } else {
                        None
                    },
                    &node_cx,
                    window,
                    cx,
                )),
                Some(err) => this.child(
                    v_flex()
                        .gap_1()
                        .child("Failed to parse content")
                        .child(err.to_string()),
                ),
            })
            .on_prepaint(move |bounds, _, cx| {
                state.update(cx, |state, _| {
                    state.update_bounds(bounds);
                })
            })
    }
}

#[derive(PartialEq, Default)]
pub(crate) struct ParsedContent {
    pub(crate) document: ParsedDocument,
    pub(crate) node_cx: node::NodeContext,
}

struct UpdateFuture {
    format: TextViewFormat,
    options: UpdateOptions,
    pending_text: String,
    timer: Timer,
    rx: Pin<Box<smol::channel::Receiver<UpdateOptions>>>,
    tx_result: smol::channel::Sender<Result<(), SharedString>>,
    delay: Duration,
}

impl UpdateFuture {
    fn new(
        format: TextViewFormat,
        rx: smol::channel::Receiver<UpdateOptions>,
        tx_result: smol::channel::Sender<Result<(), SharedString>>,
        cx: &App,
    ) -> Self {
        Self {
            format,
            pending_text: String::new(),
            options: UpdateOptions {
                append: false,
                pending_text: String::new(),
                content: Default::default(),
                highlight_theme: cx.theme().highlight_theme.clone(),
                code_block_actions: None,
            },
            timer: Timer::never(),
            rx: Box::pin(rx),
            tx_result,
            delay: UPDATE_DELAY,
        }
    }
}

impl Future for UpdateFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.rx.poll_next(cx) {
                Poll::Ready(Some(options)) => {
                    let delay = self.delay;
                    if options.append {
                        self.pending_text.push_str(options.pending_text.as_str());
                    } else {
                        self.pending_text = options.pending_text.clone();
                    }
                    self.options = options;
                    self.timer.set_after(delay);
                    continue;
                }
                Poll::Ready(None) => return Poll::Ready(()),
                Poll::Pending => {}
            }

            match self.timer.poll_next(cx) {
                Poll::Ready(Some(_)) => {
                    let pending_text = std::mem::take(&mut self.pending_text);

                    let res = parse_content(
                        self.format,
                        &UpdateOptions {
                            pending_text,
                            ..self.options.clone()
                        },
                    );
                    _ = self.tx_result.try_send(res);
                    continue;
                }
                Poll::Ready(None) | Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[derive(Clone)]
struct UpdateOptions {
    content: Arc<Mutex<ParsedContent>>,
    pending_text: String,
    append: bool,
    highlight_theme: Arc<HighlightTheme>,
    code_block_actions: Option<Arc<CodeBlockActionsFn>>,
}

fn parse_content(format: TextViewFormat, options: &UpdateOptions) -> Result<(), SharedString> {
    let mut node_cx = NodeContext {
        code_block_actions: options.code_block_actions.clone(),
        ..NodeContext::default()
    };

    let mut content = options.content.lock().unwrap();
    let mut source = String::new();
    if options.append
        && let Some(last_block) = content.document.blocks.pop()
        && let Some(span) = last_block.span()
    {
        node_cx.offset = span.start;
        let last_source = &content.document.source[span.start..];
        source.push_str(last_source);
        source.push_str(&options.pending_text);
    } else {
        source = options.pending_text.to_string();
    }

    let new_content = match format {
        TextViewFormat::Markdown => {
            format::markdown::parse(&source, &mut node_cx, &options.highlight_theme)
        }
        TextViewFormat::Html => format::html::parse(&source, &mut node_cx),
    }?;

    if options.append {
        content.document.source =
            format!("{}{}", content.document.source, options.pending_text).into();
        content.document.blocks.extend(new_content.blocks);
    } else {
        content.document = new_content;
    }

    Ok(())
}

fn selection_bounds(
    start: Option<Point<Pixels>>,
    end: Option<Point<Pixels>>,
    bounds: Bounds<Pixels>,
) -> Bounds<Pixels> {
    if let (Some(start), Some(end)) = (start, end) {
        let start = start + bounds.origin;
        let end = end + bounds.origin;

        let origin = Point {
            x: start.x.min(end.x),
            y: start.y.min(end.y),
        };
        let size = Size {
            width: (start.x - end.x).abs(),
            height: (start.y - end.y).abs(),
        };

        return Bounds { origin, size };
    }

    Bounds::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Bounds, point, px, size};

    #[test]
    fn test_text_view_state_selection_bounds() {
        assert_eq!(
            selection_bounds(None, None, Default::default()),
            Bounds::default()
        );
        assert_eq!(
            selection_bounds(None, Some(point(px(10.), px(20.))), Default::default()),
            Bounds::default()
        );
        assert_eq!(
            selection_bounds(Some(point(px(10.), px(20.))), None, Default::default()),
            Bounds::default()
        );

        // 10,10 start
        //   |------|
        //   |      |
        //   |------|
        //         50,50
        assert_eq!(
            selection_bounds(
                Some(point(px(10.), px(10.))),
                Some(point(px(50.), px(50.))),
                Default::default()
            ),
            Bounds {
                origin: point(px(10.), px(10.)),
                size: size(px(40.), px(40.))
            }
        );
        // 10,10
        //   |------|
        //   |      |
        //   |------|
        //         50,50 start
        assert_eq!(
            selection_bounds(
                Some(point(px(50.), px(50.))),
                Some(point(px(10.), px(10.))),
                Default::default()
            ),
            Bounds {
                origin: point(px(10.), px(10.)),
                size: size(px(40.), px(40.))
            }
        );
        //        50,10 start
        //   |------|
        //   |      |
        //   |------|
        // 10,50
        assert_eq!(
            selection_bounds(
                Some(point(px(50.), px(10.))),
                Some(point(px(10.), px(50.))),
                Default::default()
            ),
            Bounds {
                origin: point(px(10.), px(10.)),
                size: size(px(40.), px(40.))
            }
        );
        //        50,10
        //   |------|
        //   |      |
        //   |------|
        // 10,50 start
        assert_eq!(
            selection_bounds(
                Some(point(px(10.), px(50.))),
                Some(point(px(50.), px(10.))),
                Default::default()
            ),
            Bounds {
                origin: point(px(10.), px(10.)),
                size: size(px(40.), px(40.))
            }
        );
    }
}
