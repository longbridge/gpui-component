use std::ops::Range;
use std::rc::Rc;

use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    ActiveTheme as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    h_flex,
    highlighter::Language,
    input::{
        DocumentRangeSemanticTokensProvider, Input, InputEvent, InputState, Rope, RopeExt, TabSize,
    },
    resizable::{h_resizable, resizable_panel},
    text::markdown,
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
            _subscriptions,
        }
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
                                        .child(Clipboard::new("copy").value(code.clone()))
                                        .when_some(lang, |this, lang| {
                                            // Only show run terminal button for certain languages
                                            if lang.as_ref() == "rust" || lang.as_ref() == "python"
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
                                .flex_none()
                                .p_5()
                                .scrollable(true)
                                .selectable(true),
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
