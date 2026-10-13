//! `EditorDocument` to canonical Markdown. Blocks are written here; inline
//! content goes through main's `MarkdownSource` with the editor's options
//! (`<mark>`, whitespace hoisting, escaping). Table cells use the same inline
//! path rather than `Paragraph::to_markdown`, which is lossy on main.

use super::{
    document::{AtomData, BlockContent, EditorDocument},
    id::BlockId,
    inline::{InlineAtom, InlineText},
    kind::BlockKindData,
};
use crate::text::node::{ColumnumnAlign, MarkdownSource, MarkdownSourceOptions};

pub(crate) fn editor_source_options() -> MarkdownSourceOptions {
    MarkdownSourceOptions {
        highlight_as_html: true,
        hoist_whitespace: true,
        escape_literals: true,
    }
}

pub(crate) fn serialize(doc: &EditorDocument) -> String {
    block(doc, doc.root)
}

fn join_blocks(doc: &EditorDocument, ids: &[BlockId]) -> String {
    render_children(doc, ids)
        .into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Serialize siblings, dropping the ones with no Markdown form (empty
/// paragraphs). Note: an empty paragraph between two lists of the same type
/// is therefore lost, and main's parser reads the two lists back as one
/// (documented lossiness; a `*` vs `-` marker switch does not keep them
/// apart in this parser).
fn render_children(doc: &EditorDocument, ids: &[BlockId]) -> Vec<(BlockId, String)> {
    ids.iter()
        .map(|id| (*id, block(doc, *id)))
        .filter(|(_, text)| !text.is_empty())
        .collect()
}

fn block(doc: &EditorDocument, id: BlockId) -> String {
    let Some(entry) = doc.get(id) else {
        return String::new();
    };
    let children = doc.children(id);
    match (&entry.kind, &entry.content) {
        (BlockKindData::Root, _) => join_blocks(doc, children),
        (BlockKindData::Paragraph, BlockContent::Inline(inline)) => inline_markdown(inline),
        (BlockKindData::Heading { level }, BlockContent::Inline(inline)) => {
            let text = inline_markdown(inline);
            format!(
                "{} {}",
                "#".repeat(*level as usize),
                text.replace('\n', " ")
            )
        }
        (BlockKindData::Blockquote, _) => join_blocks(doc, children)
            .lines()
            .map(|line| {
                if line.is_empty() {
                    ">".to_string()
                } else {
                    format!("> {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        (BlockKindData::List { ordered, start }, _) => {
            let loose = children.iter().any(|item| {
                matches!(
                    doc.kind(*item),
                    Some(BlockKindData::ListItem { spread: true, .. })
                )
            });
            // An item with no content has no stable Markdown form (`-` under
            // a paragraph reads as a setext underline); skip it.
            children
                .iter()
                .filter(|item| !list_item(doc, **item, "- ").trim().ends_with('-'))
                .enumerate()
                .map(|(ix, item)| {
                    let marker = if *ordered {
                        format!("{}. ", start.unwrap_or(1) as usize + ix)
                    } else {
                        "- ".to_string()
                    };
                    list_item(doc, *item, &marker)
                })
                .collect::<Vec<_>>()
                .join(if loose { "\n\n" } else { "\n" })
        }
        (BlockKindData::ListItem { .. }, _) => list_item(doc, id, "- "),
        (BlockKindData::CodeBlock { lang }, BlockContent::Plain(code)) => {
            let longest = code.split(|c| c != '`').map(str::len).max().unwrap_or(0);
            let fence = "`".repeat(longest.max(2) + 1);
            format!(
                "{fence}{}\n{code}\n{fence}",
                lang.as_deref().unwrap_or_default()
            )
        }
        (BlockKindData::Table { aligns }, _) => {
            let mut lines = Vec::new();
            for (row_ix, row) in children.iter().enumerate() {
                let cells: Vec<String> = doc
                    .children(*row)
                    .iter()
                    .map(|cell| match doc.inline(*cell) {
                        Some(inline) => inline_markdown(inline)
                            .replace('\n', " ")
                            .trim()
                            .to_string(),
                        None => String::new(),
                    })
                    .collect();
                let width = cells.len();
                lines.push(format!("| {} |", cells.join(" | ")));
                if row_ix == 0 {
                    let delimiters: Vec<&str> = (0..width)
                        .map(|ix| match aligns.get(ix).copied().unwrap_or_default() {
                            ColumnumnAlign::Left => ":--",
                            ColumnumnAlign::Center => ":-:",
                            ColumnumnAlign::Right => "--:",
                        })
                        .collect();
                    lines.push(format!("| {} |", delimiters.join(" | ")));
                }
            }
            lines.join("\n")
        }
        (BlockKindData::HorizontalRule, _) => "---".to_string(),
        // Documented lossiness: a Markdown hard break is an inline concern on
        // main; only the HTML `<br>` block form survives.
        (BlockKindData::Break { html }, _) => {
            if *html {
                "<br>".to_string()
            } else {
                String::new()
            }
        }
        (
            BlockKindData::Definition {
                identifier,
                url,
                title,
            },
            _,
        ) => match title {
            Some(title) => format!("[{identifier}]: {url} \"{title}\""),
            None => format!("[{identifier}]: {url}"),
        },
        (BlockKindData::Custom, BlockContent::Atom(AtomData::Custom(node))) => node.to_markdown(),
        // Documented lossiness: `Unknown` has no source to write back.
        _ => String::new(),
    }
}

/// A list item with continuation lines indented by the marker width.
fn list_item(doc: &EditorDocument, item: BlockId, marker: &str) -> String {
    let (checked, spread) = match doc.kind(item) {
        Some(BlockKindData::ListItem { checked, spread }) => (*checked, *spread),
        _ => (None, false),
    };
    let children = doc.children(item);
    let mut body = String::new();
    // Empty paragraphs have no Markdown form; skip them.
    let children = render_children(doc, children);
    for (ix, (child, text)) in children.iter().enumerate() {
        if ix > 0 {
            let prev_is_list = matches!(
                doc.kind(children[ix - 1].0),
                Some(BlockKindData::List { .. })
            );
            let is_list = matches!(doc.kind(*child), Some(BlockKindData::List { .. }));
            // Two paragraphs in a row need a blank line, or they merge.
            body.push_str(if spread || (!is_list && !prev_is_list) || prev_is_list {
                "\n\n"
            } else {
                "\n"
            });
        }
        body.push_str(text);
    }
    let checkbox = match checked {
        Some(true) => "[x] ",
        Some(false) => "[ ] ",
        None => "",
    };
    let indent = " ".repeat(marker.len());
    let mut out = String::new();
    for (ix, line) in format!("{checkbox}{body}").split('\n').enumerate() {
        if ix == 0 {
            out.push_str(marker);
            out.push_str(line);
        } else {
            out.push('\n');
            if !line.is_empty() {
                out.push_str(&indent);
                out.push_str(line);
            }
        }
    }
    out.trim_end().to_string()
}

// Inline content goes through `MarkdownSource`, then line starts that
// would read as block syntax are escaped.
thread_local! {
    // Set while serializing for the view (see [`block_for_render`]).
    static FOR_RENDER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Markdown for one block, as the view renders it: empty text leaves get a
/// zero-width placeholder (so they still parse to a leaf with a layout) and
/// a paragraph's edge spaces are kept as `&#32;` (so the caret can sit after
/// a typed trailing space). Not round-trip output; never saved.
pub(crate) fn block_for_render(doc: &EditorDocument, id: BlockId) -> String {
    FOR_RENDER.with(|flag| flag.set(true));
    let out = block(doc, id);
    FOR_RENDER.with(|flag| flag.set(false));
    out
}

pub(crate) fn inline_markdown(inline: &InlineText) -> String {
    let out = inline_markdown_canonical(inline);
    if !FOR_RENDER.with(|flag| flag.get()) {
        return out;
    }
    let text = inline.text();
    if out.is_empty() {
        return "\u{200b}".to_string();
    }
    let lead = text.len() - text.trim_start_matches(' ').len();
    let trail = text.len() - text.trim_end_matches(' ').len();
    format!("{}{out}{}", "&#32;".repeat(lead), "&#32;".repeat(trail))
}

fn inline_markdown_canonical(inline: &InlineText) -> String {
    let mut source = MarkdownSource::with_options(editor_source_options());
    for (range, marks, atom) in inline.pieces() {
        match atom {
            Some(InlineAtom::Custom(node)) => source.push_marked(&node.to_markdown(), marks),
            Some(InlineAtom::Image(image)) => {
                let alt = image.alt.as_deref().unwrap_or_default();
                let markdown = match &image.title {
                    Some(title) => format!("![{alt}]({} \"{title}\")", image.url),
                    None => format!("![{alt}]({})", image.url),
                };
                source.push_marked(&markdown, marks)
            }
            None => source.push_literal(&inline.text()[range], marks),
        }
    }
    // Markdown drops a paragraph's edge whitespace; leaving it in would shift
    // the content column of a list item.
    source
        .finish()
        .trim()
        .split('\n')
        .map(escape_line_start)
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape_line_start(line: &str) -> String {
    let trimmed = line.trim_start();
    let lead = &line[..line.len() - trimmed.len()];
    let first = trimmed.chars().next();
    if matches!(first, Some('#' | '-' | '+' | '=')) {
        return format!("{lead}\\{trimmed}");
    }
    let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && matches!(trimmed[digits..].chars().next(), Some('.' | ')')) {
        return format!("{lead}{}\\{}", &trimmed[..digits], &trimmed[digits..]);
    }
    line.to_string()
}
