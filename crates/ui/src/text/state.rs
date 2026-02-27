use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::Poll,
    time::Duration,
};
use futures::Stream as _;

use gpui::{
    App, AppContext as _, Bounds, ClipboardItem, Context, FocusHandle, IntoElement, KeyBinding,
    ListState, ParentElement as _, Pixels, Point, Render, SharedString, Styled as _, Task, Window,
    prelude::FluentBuilder as _, px,
};

use crate::{
    ActiveTheme, ElementExt,
    async_util::{Sender, Receiver, unbounded, stream::StreamExt as _},
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
    tx: Sender<UpdateOptions>,
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

        let (tx, rx) = unbounded::<UpdateOptions>();
        let (tx_result, rx_result) = unbounded::<Result<(), SharedString>>();
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
        let update_options = UpdateOptions {
            append,
            content: self.parsed_content.clone(),
            pending_text: text.to_string(),
            highlight_theme: cx.theme().highlight_theme.clone(),
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
        // Store content coordinates (not affected by scrolling)
        let scroll_offset = if self.scrollable {
            self.list_state.scroll_px_offset_for_scrollbar()
        } else {
            Point::default()
        };
        let pos = pos - self.bounds.origin - scroll_offset;
        self.selection_positions = (Some(pos), Some(pos));
        self.is_selecting = true;
    }

    pub(super) fn update_selection(&mut self, pos: Point<Pixels>) {
        let scroll_offset = if self.scrollable {
            self.list_state.scroll_px_offset_for_scrollbar()
        } else {
            Point::default()
        };
        let pos = pos - self.bounds.origin - scroll_offset;
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

    /// Return the selection start/end in window coordinates.
    pub(crate) fn selection_points(&self) -> Option<(Point<Pixels>, Point<Pixels>)> {
        let scroll_offset = if self.scrollable {
            self.list_state.scroll_px_offset_for_scrollbar()
        } else {
            Point::default()
        };

        selection_points(
            self.selection_positions.0,
            self.selection_positions.1,
            self.bounds,
            scroll_offset,
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
        let (document, mut node_cx) = {
            let content = self.parsed_content.lock().unwrap();
            (content.document.clone(), content.node_cx.clone())
        };

        node_cx.code_block_actions = self.code_block_actions.clone();
        node_cx.style = self.text_view_style.clone();

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
    rx: Pin<Box<Receiver<UpdateOptions>>>,
    tx_result: Sender<Result<(), SharedString>>,
}

impl UpdateFuture {
    fn new(
        format: TextViewFormat,
        rx: Receiver<UpdateOptions>,
        tx_result: Sender<Result<(), SharedString>>,
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
            },
            rx: Box::pin(rx),
            tx_result,
        }
    }
}

impl Future for UpdateFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.rx.as_mut().poll_next(cx) {
                Poll::Ready(Some(options)) => {
                    if options.append {
                        self.pending_text.push_str(options.pending_text.as_str());
                    } else {
                        self.pending_text = options.pending_text.clone();
                    }
                    self.options = options;

                    // Process immediately without debounce
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
                Poll::Ready(None) => return Poll::Ready(()),
                Poll::Pending => return Poll::Pending,
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
}

fn parse_content(format: TextViewFormat, options: &UpdateOptions) -> Result<(), SharedString> {
    let mut node_cx = NodeContext {
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

fn selection_points(
    start: Option<Point<Pixels>>,
    end: Option<Point<Pixels>>,
    bounds: Bounds<Pixels>,
    scroll_offset: Point<Pixels>,
) -> Option<(Point<Pixels>, Point<Pixels>)> {
    if let (Some(start), Some(end)) = (start, end) {
        // Convert content coordinates to window coordinates
        let start = start + scroll_offset + bounds.origin;
        let end = end + scroll_offset + bounds.origin;
        return Some((start, end));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::point;

    #[test]
    fn test_text_view_state_selection_points() {
        assert_eq!(
            selection_points(None, None, Default::default(), Point::default()),
            None
        );
        assert_eq!(
            selection_points(
                None,
                Some(point(px(10.), px(20.))),
                Default::default(),
                Point::default()
            ),
            None
        );
        assert_eq!(
            selection_points(
                Some(point(px(10.), px(20.))),
                None,
                Default::default(),
                Point::default()
            ),
            None
        );

        // 10,10 start
        //   |------|
        //   |      |
        //   |------|
        //         50,50
        assert_eq!(
            selection_points(
                Some(point(px(10.), px(10.))),
                Some(point(px(50.), px(50.))),
                Default::default(),
                Point::default()
            ),
            Some((point(px(10.), px(10.)), point(px(50.), px(50.))))
        );

        // 10,10
        //   |------|
        //   |      |
        //   |------|
        //         50,50 start
        assert_eq!(
            selection_points(
                Some(point(px(50.), px(50.))),
                Some(point(px(10.), px(10.))),
                Default::default(),
                Point::default()
            ),
            Some((point(px(50.), px(50.)), point(px(10.), px(10.))))
        );

        //        50,10 start
        //   |------|
        //   |      |
        //   |------|
        // 10,50
        assert_eq!(
            selection_points(
                Some(point(px(50.), px(10.))),
                Some(point(px(10.), px(50.))),
                Default::default(),
                Point::default()
            ),
            Some((point(px(50.), px(10.)), point(px(10.), px(50.))))
        );

        //        50,10
        //   |------|
        //   |      |
        //   |------|
        // 10,50 start
        assert_eq!(
            selection_points(
                Some(point(px(10.), px(50.))),
                Some(point(px(50.), px(10.))),
                Default::default(),
                Point::default()
            ),
            Some((point(px(10.), px(50.)), point(px(50.), px(10.))))
        );
    }
}
