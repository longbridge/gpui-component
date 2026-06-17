use std::ops::Range;
use std::rc::Rc;

use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Sizable as _,
    avatar::Avatar,
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    h_flex,
    highlighter::Language,
    input::{
        DocumentRangeSemanticTokensProvider, Input, InputEvent, InputState, Rope, RopeExt, TabSize,
    },
    resizable::{h_resizable, resizable_panel},
    status_bar::StatusBar,
    text::{
        MarkdownNode, MarkdownParseContext, MarkdownPlugin, TextViewStyle, markdown, markdown_ast,
    },
    v_flex,
};
use gpui_component_assets::Assets;
use gpui_component_story::Open;
use lsp_types::{SemanticToken, SemanticTokenType, SemanticTokens, SemanticTokensLegend};

/// Markers, each mapped to a different `HighlightTheme` token-type name so
/// `TODO`, `FIXME`, … render in distinct colors.
const MARKERS: &[(&str, &str)] = &[
    ("TODO", "keyword"),
    ("FIXME", "string"),
    ("XXX", "number"),
    ("HACK", "function"),
    ("NOTE", "type"),
];

#[derive(Clone)]
struct TickerNode {
    symbol: String,
}

#[derive(Clone)]
struct UserCardNode {
    id: String,
}

#[derive(Clone, Copy)]
struct TickerQuote {
    name: &'static str,
    price: f64,
    change: f64,
}

#[derive(Clone)]
struct TickerPlugin {
    apple_quote: TickerQuote,
    tesla_quote: TickerQuote,
}

impl TickerPlugin {
    fn new(apple_quote: TickerQuote, tesla_quote: TickerQuote) -> Self {
        Self {
            apple_quote,
            tesla_quote,
        }
    }

    fn quote(&self, symbol: &str) -> TickerQuote {
        match symbol {
            "AAPL.US" => self.apple_quote,
            "TSLA.US" => self.tesla_quote,
            _ => TickerQuote {
                name: "Unknown",
                price: 0.0,
                change: 0.0,
            },
        }
    }
}

#[derive(Clone)]
struct UserCardPlugin;

impl UserCardPlugin {
    fn new() -> Self {
        Self
    }
}

fn mdx_attr(attrs: &[markdown_ast::AttributeContent], name: &str) -> Option<String> {
    attrs.iter().find_map(|attr| match attr {
        markdown_ast::AttributeContent::Property(prop) if prop.name == name => {
            match prop.value.as_ref() {
                Some(markdown_ast::AttributeValue::Literal(value)) => Some(value.clone()),
                _ => None,
            }
        }
        _ => None,
    })
}

fn html_tag_name(value: &str) -> Option<&str> {
    value
        .trim()
        .strip_prefix('<')?
        .split([' ', '/', '>'])
        .next()
}

fn html_attr(value: &str, name: &str) -> Option<String> {
    let pattern = format!("{name}=\"");
    let start = value.find(&pattern)? + pattern.len();
    let end = value[start..].find('"')?;
    Some(value[start..start + end].to_string())
}

fn ticker_symbol(value: &str) -> Option<&str> {
    let symbol = value.strip_prefix('$')?;
    if symbol.is_empty()
        || !symbol.contains('.')
        || !symbol
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.')
    {
        return None;
    }
    Some(symbol)
}

fn parse_ticker_block(
    node: &markdown_ast::Node,
    cx: &MarkdownParseContext<'_>,
) -> Option<MarkdownNode> {
    let markdown_ast::Node::Paragraph(paragraph) = node else {
        return None;
    };
    let [markdown_ast::Node::Text(text)] = paragraph.children.as_slice() else {
        return None;
    };
    let symbol = ticker_symbol(&text.value)?;
    Some(
        MarkdownNode::new("ticker")
            .with_text(format!("${symbol}"))
            .with_markdown(cx.node_source(node).unwrap_or(text.value.as_str()))
            .with_data(TickerNode {
                symbol: symbol.to_string(),
            }),
    )
}

fn parse_user_card_block(
    node: &markdown_ast::Node,
    cx: &MarkdownParseContext<'_>,
) -> Option<MarkdownNode> {
    match node {
        markdown_ast::Node::MdxJsxFlowElement(element)
            if element.name.as_deref() == Some("UserCard") =>
        {
            let id = mdx_attr(&element.attributes, "id")?;
            Some(
                MarkdownNode::new("user-card")
                    .with_text(id.clone())
                    .with_markdown(cx.node_source(node).unwrap_or_default())
                    .with_data(UserCardNode { id }),
            )
        }
        markdown_ast::Node::Html(raw) if html_tag_name(&raw.value) == Some("UserCard") => {
            let id = html_attr(&raw.value, "id")?;
            Some(
                MarkdownNode::new("user-card")
                    .with_text(id.clone())
                    .with_markdown(cx.node_source(node).unwrap_or(raw.value.as_str()))
                    .with_data(UserCardNode { id }),
            )
        }
        _ => None,
    }
}

fn render_ticker_quote(
    node: &MarkdownNode,
    plugin: &TickerPlugin,
    cx: &mut App,
) -> impl IntoElement {
    let ticker = node
        .data::<TickerNode>()
        .expect("ticker markdown node data");
    let symbol = ticker.symbol.as_str();
    let quote = plugin.quote(symbol);
    let up = quote.change >= 0.0;
    let trend = if up { cx.theme().green } else { cx.theme().red };

    v_flex()
        .w(px(240.))
        .gap_1p5()
        .px_3()
        .py_2()
        .rounded(cx.theme().radius)
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .child(
            h_flex()
                .items_center()
                .justify_between()
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .line_height(relative(1.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format!("${symbol}")),
                        )
                        .child(
                            div()
                                .text_xs()
                                .line_height(relative(1.))
                                .text_color(cx.theme().muted_foreground)
                                .child(quote.name),
                        ),
                )
                .child(
                    h_flex()
                        .items_center()
                        .gap_0p5()
                        .px_1()
                        .py_0p5()
                        .rounded(cx.theme().radius)
                        .bg(trend.opacity(0.12))
                        .text_xs()
                        .line_height(relative(1.))
                        .text_color(trend)
                        .child(
                            Icon::new(if up {
                                IconName::ArrowUp
                            } else {
                                IconName::ArrowDown
                            })
                            .xsmall(),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!("{:+.1}%", quote.change)),
                        ),
                ),
        )
        .child(
            h_flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_lg()
                        .line_height(relative(1.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(format!("{:.2}", quote.price)),
                )
                .child(
                    div()
                        .text_xs()
                        .line_height(relative(1.))
                        .text_color(cx.theme().muted_foreground)
                        .child("Last"),
                ),
        )
}

fn render_user_card(node: &MarkdownNode, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let user = node
        .data::<UserCardNode>()
        .expect("user-card markdown node data");
    let id = user.id.as_str();
    let (name, avatar) = match id {
        "huacnlee" => (
            "Jason Lee",
            "https://avatars.githubusercontent.com/u/5518?v=4",
        ),
        "madcodelife" => (
            "Floyd Wang",
            "https://avatars.githubusercontent.com/u/28998859?v=4",
        ),
        _ => ("Unknown", ""),
    };

    let following = window.use_keyed_state(
        SharedString::from(format!("user-card-follow-{id}")),
        cx,
        |_, _| false,
    );
    let is_following = *following.read(cx);

    h_flex()
        .w(px(300.))
        .items_center()
        .gap_3()
        .px_3()
        .py_2()
        .rounded(cx.theme().radius)
        .border_1()
        .border_color(cx.theme().border)
        .child(
            Avatar::new()
                .name(name)
                .with_size(px(24.))
                .when(!avatar.is_empty(), |this| this.src(avatar)),
        )
        .child(
            div()
                .flex_1()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .child(name),
        )
        .child(
            Button::new(SharedString::from(format!("follow-{id}")))
                .outline()
                .small()
                .label(if is_following { "Following" } else { "Follow" })
                .on_click(move |_, _, cx| {
                    following.update(cx, |v, cx| {
                        *v = !*v;
                        cx.notify();
                    });
                }),
        )
}

impl MarkdownPlugin for TickerPlugin {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "ticker"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        parse_ticker_block(node, cx)
    }

    fn render(&self, node: &MarkdownNode, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        render_ticker_quote(node, self, cx)
    }
}

impl MarkdownPlugin for UserCardPlugin {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "user-card"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        parse_user_card_block(node, cx)
    }

    fn render(&self, node: &MarkdownNode, window: &mut Window, cx: &mut App) -> impl IntoElement {
        render_user_card(node, window, cx)
    }
}

/// Example [`DocumentRangeSemanticTokensProvider`]: tags `TODO` / `FIXME` /
/// `XXX` / `HACK` / `NOTE` markers anywhere in the document, each with its
/// own semantic token type so they render in distinct theme colors.
///
/// Installed on `input_state.lsp.semantic_tokens_provider`, exactly like the
/// other LSP providers (`document_color_provider`, `hover_provider`, …). The
/// editor fetches it (debounced) on document change, caches the result, and
/// composes it into the render pipeline on top of the tree-sitter syntax
/// highlighting. This example scans synchronously and returns a ready task;
/// a real language server would return tokens from an async request, and a
/// heavy local parser (syntect, …) would offload to a background task.
struct MarkerHighlighter;

impl DocumentRangeSemanticTokensProvider for MarkerHighlighter {
    fn legend(&self) -> SemanticTokensLegend {
        SemanticTokensLegend {
            token_types: MARKERS
                .iter()
                .map(|(_, name)| SemanticTokenType::from(name.to_string()))
                .collect(),
            token_modifiers: vec![],
        }
    }

    fn semantic_tokens(
        &self,
        text: &Rope,
        range: Range<usize>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<SemanticTokens>> {
        // Scan the requested range and collect absolute
        // (line, character, length, token_type) hits. `token_type` indexes
        // the legend, so each marker gets its own color.
        let slice = text.slice(range.clone()).to_string();
        let mut hits: Vec<(u32, u32, u32, u32)> = Vec::new();
        for (token_type, (marker, _)) in MARKERS.iter().enumerate() {
            let mut from = 0;
            while let Some(rel) = slice[from..].find(marker) {
                let abs = range.start + from + rel;
                let pos = text.offset_to_position(abs);
                hits.push((
                    pos.line,
                    pos.character,
                    marker.chars().count() as u32,
                    token_type as u32,
                ));
                from += rel + marker.len();
            }
        }
        hits.sort_unstable();

        // Delta-encode into LSP semantic tokens — the exact format a real
        // language server returns from `textDocument/semanticTokens/range`.
        let mut data = Vec::with_capacity(hits.len());
        let (mut prev_line, mut prev_char) = (0u32, 0u32);
        for (line, character, length, token_type) in hits {
            let delta_line = line - prev_line;
            let delta_start = if delta_line == 0 {
                character - prev_char
            } else {
                character
            };
            data.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type,
                token_modifiers_bitset: 0,
            });
            prev_line = line;
            prev_char = character;
        }

        Task::ready(Ok(SemanticTokens {
            result_id: None,
            data,
        }))
    }
}

pub struct Example {
    input_state: Entity<InputState>,
    /// When `true`, tables wrap cell content to fit the width; when `false`
    /// (the default), tables keep cells on one line and scroll horizontally.
    table_wrap: bool,
    _subscriptions: Vec<Subscription>,
}

const EXAMPLE: &str = include_str!("./fixtures/test.md");

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            let mut input_state = InputState::new(window, cx)
                .code_editor(Language::Markdown)
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 2,
                    ..Default::default()
                })
                .searchable(true)
                .placeholder("Enter your Markdown here...")
                .default_value(EXAMPLE);

            // Install the example range semantic tokens provider, alongside
            // the other LSP providers. It highlights TODO/FIXME/… markers.
            input_state.lsp.semantic_tokens_provider = Some(Rc::new(MarkerHighlighter));

            input_state
        });

        // Focus the input on startup so that actions (e.g. Open) can bubble
        // up through this view's element tree and reach their handlers.
        let focus_handle = input_state.focus_handle(cx);
        window.defer(cx, move |window, cx| {
            focus_handle.focus(window, cx);
        });

        let _subscriptions = vec![cx.subscribe(&input_state, |_, _, _: &InputEvent, _| {})];

        Self {
            input_state,
            // Default to horizontal scrolling for tables.
            table_wrap: false,
            _subscriptions,
        }
    }

    /// Build the markdown style: tables scroll horizontally unless `table_wrap`
    /// is on, in which case the default wrapping layout is used.
    fn text_view_style(&self) -> TextViewStyle {
        if self.table_wrap {
            return TextViewStyle::default();
        }
        let mut table = StyleRefinement::default();
        table.overflow.x = Some(Overflow::Scroll);
        TextViewStyle::default().table(table)
    }

    fn on_action_open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        let path = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: false,
            prompt: Some("Select a Markdown file".into()),
        });

        let input_state = self.input_state.clone();
        cx.spawn_in(window, async move |_, window| {
            let path = path.await.ok()?.ok()??.iter().next()?.clone();

            let content = std::fs::read_to_string(&path).ok()?;

            window
                .update(|window, cx| {
                    _ = input_state.update(cx, |this, cx| {
                        this.set_value(content, window, cx);
                    });
                })
                .ok();

            Some(())
        })
        .detach();
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("editor")
            .size_full()
            .on_action(cx.listener(Self::on_action_open))
            .child(
                v_flex()
                    .size_full()
                    .child(
                        div().flex_1().overflow_hidden().child(
                            h_resizable("container")
                                .child(
                                    resizable_panel().child(
                                        div()
                                            .id("source")
                                            .size_full()
                                            .font_family(cx.theme().mono_font_family.clone())
                                            .text_size(cx.theme().mono_font_size)
                                            .child(
                                                Input::new(&self.input_state)
                                                    .h_full()
                                                    .p_0()
                                                    .border_0()
                                                    .focus_bordered(false),
                                            ),
                                    ),
                                )
                                .child(
                                    resizable_panel().child(
                                        markdown(self.input_state.read(cx).value().clone())
                                            .code_block_actions(|code_block, _window, _cx| {
                                                let code = code_block.code();
                                                let lang = code_block.lang();

                                                h_flex()
                                                    .gap_1()
                                                    .child(
                                                        Clipboard::new("copy").value(code.clone()),
                                                    )
                                                    .when_some(lang, |this, lang| {
                                                        // Only show run terminal button for certain languages
                                                        if lang.as_ref() == "rust"
                                                            || lang.as_ref() == "python"
                                                        {
                                                            this.child(
                                                                Button::new("run-terminal")
                                                                    .icon(IconName::SquareTerminal)
                                                                    .ghost()
                                                                    .xsmall()
                                                                    .on_click(move |_, _, _cx| {
                                                                        println!(
                                                                            "Running {} code: {}",
                                                                            lang, code
                                                                        );
                                                                    }),
                                                            )
                                                        } else {
                                                            this
                                                        }
                                                    })
                                            })
                                            .plugin(TickerPlugin::new(
                                                TickerQuote {
                                                    name: "Apple Inc.",
                                                    price: 300.21,
                                                    change: 5.2,
                                                },
                                                TickerQuote {
                                                    name: "Tesla, Inc.",
                                                    price: 412.05,
                                                    change: -2.13,
                                                },
                                            ))
                                            .plugin(UserCardPlugin::new())
                                            // Tables scroll horizontally by default; the
                                            // status bar toggle switches to wrapping.
                                            .style(self.text_view_style())
                                            .flex_none()
                                            .p_5()
                                            .scrollable(true)
                                            .selectable(true),
                                    ),
                                ),
                        ),
                    )
                    .child(
                        StatusBar::new().right(
                            Button::new("table-wrap")
                                .ghost()
                                .xsmall()
                                .label(if self.table_wrap {
                                    "Table: Wrap"
                                } else {
                                    "Table: Scroll"
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.table_wrap = !this.table_wrap;
                                    cx.notify();
                                })),
                        ),
                    ),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        cx.activate(true);

        gpui_component_story::create_new_window("Markdown Editor", Example::view, cx);
    });
}
