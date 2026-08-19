use std::rc::Rc;

use gpui::{Action, AnyElement, App, IntoElement, SharedString, Window};

use crate::{Disableable, Icon};

/// A single command in a [`crate::command::Command`] palette.
///
/// The value is the item's identity: it is reported to palette callbacks and
/// is used as the label when none is set.
pub struct CommandItem {
    value: SharedString,
    label: Option<SharedString>,
    keywords: Vec<SharedString>,
    /// Boxed: an [`Icon`] carries a whole `StyleRefinement`, which would make
    /// every item — and so the palette's item vector — kilobytes wide.
    pub(crate) icon: Option<Box<Icon>>,
    pub(crate) action: Option<Box<dyn Action>>,
    pub(crate) checked: bool,
    disabled: bool,
    pub(crate) content: Option<Rc<CommandItemContent>>,
}

impl CommandItem {
    /// Create a new item with the given value.
    ///
    /// The value doubles as the label until [`Self::label`] sets one.
    pub fn new(value: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: None,
            keywords: Vec::new(),
            icon: None,
            action: None,
            checked: false,
            disabled: false,
            content: None,
        }
    }

    /// Set the label to display, when it differs from the value.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the leading icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(Box::new(icon.into()));
        self
    }

    /// Set the Action dispatched when this item is clicked or confirmed.
    ///
    /// The Action's active keybinding is also shown by the default row.
    pub fn action(mut self, action: Box<dyn Action>) -> Self {
        self.action = Some(action);
        self
    }

    /// Mark this item as the chosen one, drawing a check at the right end of
    /// the row.
    ///
    /// A resolved Action binding takes that slot, so an item with one shows no
    /// check.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Add extra terms the search matches against, besides the value and label.
    pub fn keywords<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.keywords
            .extend(keywords.into_iter().map(|keyword| keyword.into()));
        self
    }

    /// Replace the row content (icon and label) with a lazily built child.
    ///
    /// The builder may run more than once for measurement and rendering, so it
    /// must be side-effect-free. Custom children own their complete visual
    /// presentation, including any keybinding hint.
    pub fn child<F, E>(mut self, builder: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.content = Some(Rc::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        self
    }

    /// The value that identifies this item.
    pub fn value(&self) -> &SharedString {
        &self.value
    }

    /// The label shown in the row, falling back to the value.
    pub fn title(&self) -> &SharedString {
        self.label.as_ref().unwrap_or(&self.value)
    }

    /// Whether this item is marked as the chosen one.
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    /// Whether this item is non-interactive.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Whether this item matches the search query, ignoring case.
    ///
    /// An empty query matches everything.
    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }

        let query = query.to_lowercase();

        self.title().to_lowercase().contains(&query)
            || self.value.to_lowercase().contains(&query)
            || self
                .keywords
                .iter()
                .any(|keyword| keyword.to_lowercase().contains(&query))
    }
}

pub(crate) type CommandItemContent = dyn Fn(&mut Window, &mut App) -> AnyElement;

impl Disableable for CommandItem {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A titled section of [`CommandItem`]s.
///
/// The heading is hidden while every item in the group is filtered out.
pub struct CommandGroup {
    heading: Option<SharedString>,
    pub(crate) items: Vec<CommandItem>,
}

impl CommandGroup {
    /// Create a new group with the given heading.
    pub fn new(heading: impl Into<SharedString>) -> Self {
        Self {
            heading: Some(heading.into()),
            items: Vec::new(),
        }
    }

    /// Create a new group without a heading.
    pub fn unlabeled() -> Self {
        Self {
            heading: None,
            items: Vec::new(),
        }
    }

    /// Add an item to the group.
    pub fn item(mut self, item: CommandItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple items to the group.
    pub fn items(mut self, items: impl IntoIterator<Item = CommandItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// The heading of the group, when it has one.
    pub fn heading(&self) -> Option<&SharedString> {
        self.heading.as_ref()
    }
}

/// A top-level entry in a [`crate::command::Command`].
pub enum CommandEntry {
    /// A single ungrouped item.
    Item(CommandItem),
    /// A titled group of items.
    Group(CommandGroup),
    /// A divider between groups.
    ///
    /// A separator that ends up leading, trailing, or next to another
    /// separator once the query has filtered the list is not rendered.
    Separator,
}

impl From<CommandItem> for CommandEntry {
    fn from(item: CommandItem) -> Self {
        Self::Item(item)
    }
}

impl From<CommandGroup> for CommandEntry {
    fn from(group: CommandGroup) -> Self {
        Self::Group(group)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_falls_back_to_value() {
        assert_eq!(CommandItem::new("Calendar").title(), "Calendar");
        assert_eq!(
            CommandItem::new("calendar").label("Calendar").title(),
            "Calendar"
        );
    }

    #[test]
    fn matches_value_label_and_keywords() {
        let item = CommandItem::new("profile")
            .label("Profile")
            .keywords(["account", "user"]);

        assert!(item.matches(""));
        assert!(item.matches("PRO"));
        assert!(item.matches("profile"));
        assert!(item.matches("Account"));
        assert!(!item.matches("billing"));
    }
}
