use crate::input::EditorMode;
use std::ops::Range;

use gpui::{App, Context, HighlightStyle, WeakEntity};
use ropey::Rope;
use sum_tree::Bias;

use super::{InputBaseState, RopeExt as _};

/// A presentation style applied to a UTF-8 byte range in an input.
///
/// This is the GPUI [`HighlightStyle`] counterpart of Monaco's
/// [`IModelDeltaDecoration`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.IModelDeltaDecoration.html).
#[derive(Clone, Debug, PartialEq)]
pub struct TextDecoration {
    pub range: Range<usize>,
    pub style: HighlightStyle,
}

impl TextDecoration {
    /// Create a text decoration from a UTF-8 byte range and a GPUI style.
    pub fn new(range: Range<usize>, style: HighlightStyle) -> Self {
        Self { range, style }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct TextDecorationCollectionId(usize);

/// An independently managed collection of [`TextDecoration`]s.
///
/// This is the GPUI Component counterpart of Monaco's
/// [`IEditorDecorationsCollection`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.IEditorDecorationsCollection.html).
#[derive(Clone, Debug)]
pub struct TextDecorationCollection {
    state: WeakEntity<InputBaseState<EditorMode>>,
    id: TextDecorationCollectionId,
}

impl TextDecorationCollection {
    /// Replace all decorations in this collection.
    ///
    /// This corresponds to Monaco's
    /// [`IEditorDecorationsCollection.set`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.IEditorDecorationsCollection.html#set).
    pub fn set(&self, decorations: Vec<TextDecoration>, cx: &mut App) {
        let _ = self.state.update(cx, |state, cx| {
            let decorations = normalize(&state.text, decorations);
            if state.extras.decorations.set(self.id, decorations) {
                cx.notify();
            }
        });
    }

    /// Add decorations to this collection.
    ///
    /// This corresponds to Monaco's
    /// [`IEditorDecorationsCollection.append`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.IEditorDecorationsCollection.html#append).
    pub fn append(&self, decorations: Vec<TextDecoration>, cx: &mut App) {
        let _ = self.state.update(cx, |state, cx| {
            let decorations = normalize(&state.text, decorations);
            if state.extras.decorations.append(self.id, decorations) {
                cx.notify();
            }
        });
    }

    /// Remove all decorations from this collection.
    ///
    /// This corresponds to Monaco's
    /// [`IEditorDecorationsCollection.clear`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.IEditorDecorationsCollection.html#clear).
    pub fn clear(&self, cx: &mut App) {
        self.set(Vec::new(), cx);
    }

    /// Return the UTF-8 byte ranges in this collection.
    ///
    /// This corresponds to Monaco's
    /// [`IEditorDecorationsCollection.getRanges`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.IEditorDecorationsCollection.html#getRanges).
    pub fn get_ranges(&self, cx: &App) -> Vec<Range<usize>> {
        self.state
            .read_with(cx, |state, _| {
                state
                    .extras
                    .decorations
                    .get(self.id)
                    .unwrap_or_default()
                    .iter()
                    .map(|decoration| decoration.range.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Default)]
pub(crate) struct DecorationCollections {
    entries: Vec<(TextDecorationCollectionId, Vec<TextDecoration>)>,
}

impl DecorationCollections {
    fn create(&mut self, decorations: Vec<TextDecoration>) -> TextDecorationCollectionId {
        let id = TextDecorationCollectionId(self.entries.len());
        self.entries.push((id, decorations));
        id
    }

    fn set(&mut self, id: TextDecorationCollectionId, decorations: Vec<TextDecoration>) -> bool {
        let Some((_, current)) = self
            .entries
            .iter_mut()
            .find(|(entry_id, _)| *entry_id == id)
        else {
            return false;
        };
        *current = decorations;
        true
    }

    fn append(&mut self, id: TextDecorationCollectionId, decorations: Vec<TextDecoration>) -> bool {
        let Some((_, current)) = self
            .entries
            .iter_mut()
            .find(|(entry_id, _)| *entry_id == id)
        else {
            return false;
        };
        current.extend(decorations);
        true
    }

    fn get(&self, id: TextDecorationCollectionId) -> Option<&[TextDecoration]> {
        self.entries
            .iter()
            .find(|(entry_id, _)| *entry_id == id)
            .map(|(_, decorations)| decorations.as_slice())
    }

    pub(super) fn adjust_for_edit(&mut self, edited_range: &Range<usize>, inserted_len: usize) {
        for (_, decorations) in &mut self.entries {
            decorations.retain_mut(|decoration| {
                decoration.range =
                    adjust_range_for_edit(&decoration.range, edited_range, inserted_len);
                !decoration.range.is_empty()
            });
        }
    }

    pub(super) fn clear(&mut self) {
        for (_, decorations) in &mut self.entries {
            decorations.clear();
        }
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &[TextDecoration]> {
        self.entries
            .iter()
            .map(|(_, decorations)| decorations.as_slice())
    }
}

fn adjust_range_for_edit(
    range: &Range<usize>,
    edited_range: &Range<usize>,
    inserted_len: usize,
) -> Range<usize> {
    let removed_len = edited_range.end.saturating_sub(edited_range.start);
    let shift = |offset: usize| {
        if inserted_len >= removed_len {
            offset.saturating_add(inserted_len - removed_len)
        } else {
            offset.saturating_sub(removed_len - inserted_len)
        }
    };

    if edited_range.is_empty() {
        let start = if range.start < edited_range.start {
            range.start
        } else {
            shift(range.start)
        };
        let end = if range.end <= edited_range.start {
            range.end
        } else {
            shift(range.end)
        };
        return start..end;
    }

    let inserted_end = edited_range.start + inserted_len;
    let start = if range.start <= edited_range.start {
        range.start
    } else if range.start >= edited_range.end {
        shift(range.start)
    } else {
        edited_range.start
    };
    let end = if range.end <= edited_range.start {
        range.end
    } else if range.end >= edited_range.end {
        shift(range.end)
    } else {
        inserted_end
    };
    start..end
}

fn normalize(text: &Rope, decorations: Vec<TextDecoration>) -> Vec<TextDecoration> {
    decorations
        .into_iter()
        .filter_map(|decoration| {
            let range = text.clip_offset(decoration.range.start, Bias::Left)
                ..text.clip_offset(decoration.range.end, Bias::Right);
            (!range.is_empty()).then_some(TextDecoration {
                range,
                style: decoration.style,
            })
        })
        .collect()
}

impl InputBaseState<EditorMode> {
    /// Create an independently managed collection of text decorations.
    ///
    /// This follows Monaco's
    /// [`createDecorationsCollection`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.ICodeEditor.html#createDecorationsCollection)
    /// ownership model. Ranges use UTF-8 byte offsets into [`Self::value`].
    ///
    /// Decoration ranges follow text edits and do not need to be set again
    /// after each change. Insertions at a range boundary do not expand the
    /// range, matching Monaco's
    /// [`NeverGrowsWhenTypingAtEdges`](https://microsoft.github.io/monaco-editor/typedoc/enums/editor_editor_api.editor.TrackedRangeStickiness.html#NeverGrowsWhenTypingAtEdges)
    /// behavior. Decorations are not rendered while the input is masked.
    /// Collections live until their [`InputBaseState`] is dropped.
    ///
    /// Collections are layered in insertion order; the first collection wins
    /// when overlapping decorations set the same [`HighlightStyle`] property.
    /// Callers should avoid conflicting overlaps within one collection.
    pub fn create_decorations_collection(
        &mut self,
        decorations: Vec<TextDecoration>,
        cx: &mut Context<Self>,
    ) -> TextDecorationCollection {
        let decorations = normalize(&self.text, decorations);
        let id = self.extras.decorations.create(decorations);
        cx.notify();
        TextDecorationCollection {
            state: cx.entity().downgrade(),
            id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collections_are_independent_and_ranges_are_clipped() {
        let text = Rope::from("héllo");
        let first_style = HighlightStyle {
            font_weight: Some(gpui::FontWeight::BOLD),
            ..Default::default()
        };
        let second_style = HighlightStyle {
            background_color: Some(gpui::red()),
            ..Default::default()
        };
        let mut collections = DecorationCollections::default();

        let first = collections.create(normalize(
            &text,
            vec![TextDecoration::new(2..4, first_style)],
        ));
        let second = collections.create(normalize(
            &text,
            vec![TextDecoration::new(5..100, second_style)],
        ));

        assert_ne!(first, second);
        assert_eq!(
            collections.get(first),
            Some(&[TextDecoration::new(1..4, first_style)][..])
        );
        assert_eq!(
            collections.get(second),
            Some(&[TextDecoration::new(5..6, second_style)][..])
        );

        assert!(collections.append(first, vec![TextDecoration::new(4..5, second_style)]));
        assert_eq!(
            collections.get(first),
            Some(
                &[
                    TextDecoration::new(1..4, first_style),
                    TextDecoration::new(4..5, second_style),
                ][..]
            )
        );

        assert!(collections.set(first, Vec::new()));
        assert_eq!(collections.get(first), Some(&[][..]));
        assert_eq!(
            collections.get(second),
            Some(&[TextDecoration::new(5..6, second_style)][..])
        );
    }

    #[test]
    fn decoration_ranges_follow_text_edits() {
        let style = HighlightStyle::default();
        let mut collections = DecorationCollections::default();
        let collection = collections.create(vec![TextDecoration::new(2..6, style)]);

        collections.adjust_for_edit(&(0..0), 2);
        assert_eq!(
            collections.get(collection),
            Some(&[TextDecoration::new(4..8, style)][..])
        );

        collections.adjust_for_edit(&(6..6), 2);
        assert_eq!(
            collections.get(collection),
            Some(&[TextDecoration::new(4..10, style)][..])
        );

        collections.adjust_for_edit(&(4..10), 3);
        assert_eq!(
            collections.get(collection),
            Some(&[TextDecoration::new(4..7, style)][..])
        );

        assert_eq!(adjust_range_for_edit(&(2..6), &(2..2), 2), 4..8);
        assert_eq!(adjust_range_for_edit(&(2..6), &(6..6), 2), 2..6);
        assert_eq!(adjust_range_for_edit(&(2..6), &(2..6), 3), 2..5);
    }
}
