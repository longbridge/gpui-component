use std::{pin::Pin, sync::Arc, task::Poll, time::Duration};

use gpui::{
    App, AppContext as _, Bounds, ClipboardItem, Context, FocusHandle, KeyBinding, ListState,
    ParentElement as _, Pixels, Point, Render, SharedString, Size, Styled as _, Task, Window,
    canvas, prelude::FluentBuilder as _, px,
};
use smol::{Timer, stream::StreamExt as _};

use crate::{
    ActiveTheme,
    highlighter::HighlightTheme,
    input::{self, Copy},
    text::{
        CodeBlockActionsFn, TextViewStyle,
        node::{self, NodeContext},
    },
    v_flex,
};

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
    format: TextViewFormat,
    pub(super) text: String,
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

    pub(super) parsed_result: Option<Result<ParsedContent, SharedString>>,
    tx: smol::channel::Sender<UpdateOptions>,
    _need_reparse: bool,
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
        let (tx_result, rx_result) =
            smol::channel::unbounded::<Result<ParsedContent, SharedString>>();
        let _receive_task = cx.spawn({
            async move |weak_self, cx| {
                while let Ok(parsed_result) = rx_result.recv().await {
                    _ = weak_self.update(cx, |state, cx| {
                        state.parsed_result = Some(parsed_result);
                        state.clear_selection();
                        cx.notify();
                    });
                }
            }
        });

        let _parse_task = cx.background_spawn(UpdateFuture::new(format, rx, tx_result, cx));

        Self {
            format,
            text: text.to_string(),
            focus_handle,
            bounds: Bounds::default(),
            selection_positions: (None, None),
            selectable: false,
            scrollable: false,
            list_state: ListState::new(0, gpui::ListAlignment::Top, px(1000.)),
            text_view_style: TextViewStyle::default(),
            code_block_actions: None,
            is_selecting: false,
            parsed_result: None,
            tx,
            _need_reparse: true,
            _parse_task,
            _receive_task,
        }
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
        if text == self.text {
            return;
        }

        self.text = text.to_string();
        self._need_reparse = true;
        cx.notify();
    }

    /// Append partial text content to the existing text.
    pub fn push_str(&mut self, text: &str, cx: &mut Context<Self>) {
        if text.is_empty() {
            return;
        }

        self.text.push_str(&text);
        self._need_reparse = true;
        cx.notify();
    }

    /// Return the selected text.
    pub fn selected_text(&self) -> String {
        if let Some(text) = self
            .parsed_result
            .as_ref()
            .and_then(|res| res.as_ref().ok())
            .map(|parsed| parsed.root_node.selected_text())
        {
            return text;
        }

        return String::new();
    }

    pub(super) fn parse_if_needed(&mut self, cx: &mut Context<Self>) {
        if !self._need_reparse {
            return;
        }
        let code_block_actions = self.code_block_actions.clone();

        self._need_reparse = false;
        let update_options = UpdateOptions {
            text: self.text.clone().into(),
            text_view_style: self.text_view_style.clone(),
            highlight_theme: cx.theme().highlight_theme.clone(),
            code_block_actions: code_block_actions.clone(),
        };
        // Parse at first time by blocking.
        if self.parsed_result.is_none() {
            self.parsed_result = Some(parse_content(self.format, &update_options));
        }

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
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.parse_if_needed(cx);

        let state = cx.entity();
        v_flex()
            .size_full()
            .map(|this| match &mut self.parsed_result {
                Some(Ok(content)) => this.child(content.root_node.render_root(
                    Some(self.list_state.clone()),
                    &content.node_cx,
                    window,
                    cx,
                )),
                Some(Err(err)) => this.child(
                    v_flex()
                        .gap_1()
                        .child("Failed to parse content")
                        .child(err.to_string()),
                ),
                None => this,
            })
            .child(canvas(
                move |bounds, _, cx| {
                    state.update(cx, |state, _| {
                        state.update_bounds(bounds);
                    })
                },
                |_, _, _, _| {},
            ))
    }
}

#[derive(PartialEq)]
pub(crate) struct ParsedContent {
    pub(crate) root_node: node::Node,
    pub(crate) node_cx: node::NodeContext,
}

struct UpdateFuture {
    format: TextViewFormat,
    options: UpdateOptions,
    timer: Timer,
    rx: Pin<Box<smol::channel::Receiver<UpdateOptions>>>,
    tx_result: smol::channel::Sender<Result<ParsedContent, SharedString>>,
    delay: Duration,
}

impl UpdateFuture {
    #[allow(clippy::too_many_arguments)]
    fn new(
        format: TextViewFormat,
        rx: smol::channel::Receiver<UpdateOptions>,
        tx_result: smol::channel::Sender<Result<ParsedContent, SharedString>>,
        cx: &App,
    ) -> Self {
        Self {
            format,
            options: UpdateOptions {
                text: SharedString::default(),
                text_view_style: TextViewStyle::default(),
                highlight_theme: cx.theme().highlight_theme.clone(),
                code_block_actions: None,
            },
            timer: Timer::never(),
            rx: Box::pin(rx),
            tx_result,
            delay: Duration::from_millis(200),
        }
    }
}

impl Future for UpdateFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.rx.poll_next(cx) {
                Poll::Ready(Some(options)) => {
                    self.options = options;
                    let delay = self.delay;
                    self.timer.set_after(delay);
                    continue;
                }
                Poll::Ready(None) => return Poll::Ready(()),
                Poll::Pending => {}
            }

            match self.timer.poll_next(cx) {
                Poll::Ready(Some(_)) => {
                    let res = parse_content(self.format, &self.options);
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
    text: SharedString,
    text_view_style: TextViewStyle,
    highlight_theme: Arc<HighlightTheme>,
    code_block_actions: Option<Arc<CodeBlockActionsFn>>,
}

fn parse_content(
    format: TextViewFormat,
    options: &UpdateOptions,
) -> Result<ParsedContent, SharedString> {
    let mut node_cx = NodeContext {
        style: options.text_view_style.clone(),
        code_block_actions: options.code_block_actions.clone(),
        ..NodeContext::default()
    };

    let res = match format {
        TextViewFormat::Markdown => super::format::markdown::parse(
            options.text.as_str(),
            &options.text_view_style,
            &mut node_cx,
            &options.highlight_theme,
        ),
        TextViewFormat::Html => super::format::html::parse(options.text.as_str(), &mut node_cx),
    };
    res.map(move |root_node| ParsedContent { root_node, node_cx })
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
