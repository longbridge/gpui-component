use gpui::{App, IntoElement, SharedString, Window};
use markdown::mdast;

use crate::description_list::{DescriptionItem, DescriptionList};

use super::{MarkdownNode, MarkdownParseContext, MarkdownPlugin};

const NODE_NAME: &str = "frontmatter";

#[derive(Debug, Clone, PartialEq)]
struct FrontmatterEntry {
    key: SharedString,
    value: SharedString,
}

#[derive(Debug, Clone, PartialEq)]
struct Frontmatter {
    entries: Vec<FrontmatterEntry>,
}

impl Frontmatter {
    fn text(&self) -> String {
        self.entries
            .iter()
            .map(|entry| format!("{}: {}", entry.key, entry.value))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Renders top-level YAML frontmatter mappings as a description list.
///
/// Enable frontmatter parsing with [`super::MarkdownExtensions::frontmatter`]
/// before registering this plugin. Values are rendered as plain text; YAML
/// sequences and other unsupported top-level values use TextView's YAML code
/// block fallback.
#[derive(Default)]
pub struct FrontmatterPlugin;

impl FrontmatterPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl MarkdownPlugin for FrontmatterPlugin {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        NODE_NAME
    }

    fn parse(&self, node: &mdast::Node, cx: &MarkdownParseContext<'_>) -> Option<MarkdownNode> {
        let mdast::Node::Yaml(yaml) = node else {
            return None;
        };
        let frontmatter = parse_frontmatter(&yaml.value)?;
        let text = frontmatter.text();

        Some(
            MarkdownNode::new(NODE_NAME, frontmatter)
                .text(text)
                .markdown(cx.node_source(node).unwrap_or(yaml.value.as_str())),
        )
    }

    fn render(&self, node: &MarkdownNode, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let frontmatter = node.data::<Frontmatter>().expect("frontmatter node data");

        DescriptionList::horizontal()
            .label_width(gpui::rems(12.))
            .columns(1)
            .children(
                frontmatter.entries.iter().map(|entry| {
                    DescriptionItem::new(entry.key.clone()).value(entry.value.clone())
                }),
            )
    }
}

fn parse_frontmatter(value: &str) -> Option<Frontmatter> {
    #[derive(Clone, Copy)]
    enum ScalarStyle {
        Folded,
        Literal,
        Nested,
    }

    struct Entry {
        key: String,
        value: String,
        style: ScalarStyle,
    }

    fn push_continuation(entry: &mut Entry, line: &str) {
        let line = line.trim();
        match entry.style {
            ScalarStyle::Folded => {
                if line.is_empty() {
                    if !entry.value.is_empty() && !entry.value.ends_with('\n') {
                        entry.value.push('\n');
                    }
                    return;
                }
                if !entry.value.is_empty() && !entry.value.ends_with('\n') {
                    entry.value.push(' ');
                }
            }
            ScalarStyle::Literal | ScalarStyle::Nested => {
                if !entry.value.is_empty() {
                    entry.value.push('\n');
                }
            }
        }
        entry.value.push_str(line);
    }

    fn is_plain_key(key: &str) -> bool {
        !key.is_empty()
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    }

    let mut entries = Vec::new();
    let mut current: Option<Entry> = None;

    for line in value.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if current
                .as_ref()
                .is_some_and(|entry| matches!(entry.style, ScalarStyle::Literal))
            {
                push_continuation(current.as_mut()?, line);
            }
            continue;
        }

        let is_top_level = !line.starts_with([' ', '\t']);
        if trimmed.starts_with('#')
            && (is_top_level
                || !current
                    .as_ref()
                    .is_some_and(|entry| matches!(entry.style, ScalarStyle::Literal)))
        {
            continue;
        }

        if is_top_level {
            let (key, raw_value) = line.split_once(':')?;
            let key = key.trim();
            if !is_plain_key(key) {
                return None;
            }

            if let Some(entry) = current.take() {
                entries.push(entry);
            }

            let raw_value = raw_value.trim();
            let (value, style) = match raw_value {
                ">" | ">-" | ">+" => (String::new(), ScalarStyle::Folded),
                "|" | "|-" | "|+" => (String::new(), ScalarStyle::Literal),
                "" => (String::new(), ScalarStyle::Nested),
                _ => (raw_value.to_string(), ScalarStyle::Folded),
            };
            current = Some(Entry {
                key: key.to_string(),
                value,
                style,
            });
        } else {
            push_continuation(current.as_mut()?, line);
        }
    }

    if let Some(entry) = current {
        entries.push(entry);
    }
    if entries.is_empty() {
        return None;
    }

    Some(Frontmatter {
        entries: entries
            .into_iter()
            .map(|entry| FrontmatterEntry {
                key: entry.key.into(),
                value: entry.value.into(),
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_mapping_as_metadata() {
        let frontmatter = parse_frontmatter(
            "name: gpui-component-dev\ndescription: Contributing to `crates/ui`.",
        )
        .expect("frontmatter mapping");

        assert_eq!(frontmatter.entries.len(), 2);
        assert_eq!(frontmatter.entries[0].key.as_ref(), "name");
        assert_eq!(frontmatter.entries[0].value.as_ref(), "gpui-component-dev");
        assert_eq!(
            frontmatter.entries[1].value.as_ref(),
            "Contributing to `crates/ui`."
        );
    }

    #[test]
    fn parses_block_scalars() {
        let frontmatter = parse_frontmatter(
            "description: >-\n  First line\n  second line.\nnotes: |-\n  # literal content\n\n  second line",
        )
        .expect("frontmatter mapping");

        assert_eq!(
            frontmatter.entries[0].value.as_ref(),
            "First line second line."
        );
        assert_eq!(
            frontmatter.entries[1].value.as_ref(),
            "# literal content\n\nsecond line"
        );
    }

    #[test]
    fn rejects_non_mapping_yaml() {
        assert!(parse_frontmatter("- name: example").is_none());
    }
}
