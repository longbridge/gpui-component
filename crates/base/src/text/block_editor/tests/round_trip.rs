//! Mark-loss cases from main's `Paragraph::to_markdown`, through import and
//! the editor serializer.

use markdown::mdast::Node;

use super::*;
use crate::text::{MarkdownNode, markdown_ext::TestInlinePlugin};

fn round_trip(source: &str) -> String {
    model(source).source()
}

#[test]
fn bold_italic_round_trips() {
    assert_eq!(round_trip("a ***x*** b"), "a ***x*** b");
}

#[test]
fn nested_italic_in_bold_round_trips() {
    assert_eq!(round_trip("a **b*c*d** e"), "a **b*c*d** e");
}

#[test]
fn underline_round_trips() {
    assert_eq!(round_trip("a <u>x</u> b"), "a <u>x</u> b");
}

#[test]
fn highlight_round_trips_as_mark() {
    assert_eq!(round_trip("a <mark>x</mark> b"), "a <mark>x</mark> b");
}

#[test]
fn highlight_color_survives() {
    let source = r##"<mark color="#ff000059">x</mark>"##;
    assert_eq!(round_trip(source), source);
}

#[test]
fn whitespace_hoisted_out_of_emphasis() {
    // Build runs `a `{bold} + `b` through the model.
    let mut m = model("");
    m.run(EditorCommand::ToggleMark(MarkKind::Bold)).unwrap();
    m.insert_text("a ", EditOrigin::Typing).unwrap();
    m.run(EditorCommand::ToggleMark(MarkKind::Bold)).unwrap();
    m.insert_text("b", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "**a** b");
    assert_eq!(round_trip(&m.source()), "**a** b");
}

#[test]
fn literal_asterisk_escaped() {
    let mut m = model("");
    m.insert_text("a * b", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "a \\* b");
    let reparsed = model(&m.source());
    assert_eq!(
        reparsed.document().leaves().next().unwrap().text(),
        Some("a * b")
    );
}

#[test]
fn custom_node_round_trips() {
    let extensions = MarkdownExtensions::default().plugin(
        TestInlinePlugin::new("math").parse_with(|node, _| match node {
            Node::InlineMath(math) => Some(MarkdownNode::new("math", ()).text(math.value.clone())),
            _ => None,
        }),
    );
    let m = BlockEditorModel::from_markdown("$x$", extensions).unwrap();
    let leaf = m.document().leaves().next().unwrap().id();
    assert_eq!(m.doc().inline(leaf).unwrap().atoms().len(), 1);
    assert_eq!(m.source(), "$x$");
}

#[test]
fn list_continuation_indented() {
    assert_eq!(round_trip("- a\n\n  b"), "- a\n\n  b");
    assert_eq!(round_trip("10. a\n\n    b"), "10. a\n\n    b");
}

#[test]
fn table_cells_keep_marks() {
    let source = "| **a** | <u>b</u> |\n| :-- | --: |\n| *c* | d |";
    assert_eq!(round_trip(source), source);
}

#[test]
fn every_block_kind_round_trips() {
    for source in [
        "# H1\n\n###### H6",
        "> quote\n>\n> - item",
        "1. one\n2. two",
        "3. three\n4. four",
        "- [ ] todo\n- [x] done",
        "```rust\nfn main() {}\n```",
        "````\n```\n````",
        "---",
        "[id]: https://example.com \"Title\"",
        "a [link](https://example.com) b",
        "![alt](https://example.com/a.png)",
        "`code` and ~~strike~~",
    ] {
        assert_eq!(round_trip(source), source, "{source}");
    }
}
