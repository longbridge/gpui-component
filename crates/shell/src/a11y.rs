//! The accessibility roles a script may name.
//!
//! `role(...)` takes a string, so something has to turn `"list_box_option"`
//! into [`gpui::Role::ListBoxOption`]. The list below is that translation, and
//! it is written once: the macro produces both the parser the runtime
//! dispatches through and the name list [`crate::typings`] declares, so a name
//! that type-checks in an editor is a name the runtime accepts.
//!
//! Spelling it out is what makes drift visible. `accesskit` owns these
//! variants; renaming one upstream breaks this file at compile time, which is
//! the only moment anybody could act on it. Deriving the list at runtime
//! instead would need `accesskit`'s `enumn` feature, which is enabled here only
//! because a Linux-only dependency happens to ask for it.
//!
//! One variant is deliberately absent. `Role::GenericContainer` is filtered out
//! of the accessibility tree — GPUI debug-asserts against it — so naming it
//! would produce an element that announces nothing while looking as though it
//! announced something.

/// Names the runtime rejects, and why.
///
/// A script that asks for this has a reason in mind, and "unknown role" is not
/// the answer to it.
pub const FILTERED_ROLE: &str = "generic_container";

macro_rules! roles {
    ($($variant:ident),+ $(,)?) => {
        /// Every role name a script may pass to `role(...)`, in the order
        /// `accesskit` declares them.
        pub fn role_names() -> Vec<String> {
            vec![$(snake_case(stringify!($variant))),+]
        }

        /// The role a script named, or `None` if no variant spells it.
        ///
        /// Only the snake_case spelling is accepted. The names here are the
        /// script API, and one role reachable under two spellings is one that
        /// reads differently in two files for no reason.
        pub fn role_from_name(name: &str) -> Option<gpui::Role> {
            if name.chars().any(|character| character.is_ascii_uppercase()) {
                return None;
            }
            match pascal_case(name).as_str() {
                $(stringify!($variant) => Some(gpui::Role::$variant),)+
                _ => None,
            }
        }
    };
}

// Wrapped by hand: one variant per line would be 182 lines of single words,
// and the point of the list is that it can be read against `accesskit`'s own.
#[rustfmt::skip]
roles!(
    Unknown, TextRun, Cell, Label, Image, Link, Row, ListItem, ListMarker, TreeItem, ListBoxOption,
    MenuItem, MenuListOption, Paragraph, CheckBox, RadioButton, TextInput, Button, DefaultButton,
    Pane, RowHeader, ColumnHeader, RowGroup, List, Table, LayoutTableCell, LayoutTableRow,
    LayoutTable, Switch, Menu, MultilineTextInput, SearchInput, DateInput, DateTimeInput, WeekInput,
    MonthInput, TimeInput, EmailInput, NumberInput, PasswordInput, PhoneNumberInput, UrlInput, Abbr,
    Alert, AlertDialog, Application, Article, Audio, Banner, Blockquote, Canvas, Caption, Caret,
    Code, ColorWell, ComboBox, EditableComboBox, Complementary, Comment, ContentDeletion,
    ContentInsertion, ContentInfo, Definition, DescriptionList, Details, Dialog, DisclosureTriangle,
    Document, EmbeddedObject, Emphasis, Feed, FigureCaption, Figure, Footer, Form, Grid, GridCell,
    Group, Header, Heading, Iframe, IframePresentational, ImeCandidate, Keyboard, Legend, LineBreak,
    ListBox, Log, Main, Mark, Marquee, Math, MenuBar, MenuItemCheckBox, MenuItemRadio,
    MenuListPopup, Meter, Navigation, Note, PluginObject, ProgressIndicator, RadioGroup, Region,
    RootWebArea, Ruby, RubyAnnotation, ScrollBar, ScrollView, Search, Section, SectionFooter,
    SectionHeader, Slider, SpinButton, Splitter, Status, Strong, Suggestion, SvgRoot, Tab, TabList,
    TabPanel, Term, Time, Timer, TitleBar, Toolbar, Tooltip, Tree, TreeGrid, Video, WebView, Window,
    PdfActionableHighlight, PdfRoot, GraphicsDocument, GraphicsObject, GraphicsSymbol, DocAbstract,
    DocAcknowledgements, DocAfterword, DocAppendix, DocBackLink, DocBiblioEntry, DocBibliography,
    DocBiblioRef, DocChapter, DocColophon, DocConclusion, DocCover, DocCredit, DocCredits,
    DocDedication, DocEndnote, DocEndnotes, DocEpigraph, DocEpilogue, DocErrata, DocExample,
    DocFootnote, DocForeword, DocGlossary, DocGlossRef, DocIndex, DocIntroduction, DocNoteRef,
    DocNotice, DocPageBreak, DocPageFooter, DocPageHeader, DocPageList, DocPart, DocPreface,
    DocPrologue, DocPullquote, DocQna, DocSubtitle, DocTip, DocToc, ListGrid, Terminal,
);

/// `ListBoxOption` → `list_box_option`.
fn snake_case(variant: &str) -> String {
    let mut out = String::with_capacity(variant.len() + 4);
    for (index, character) in variant.char_indices() {
        if character.is_ascii_uppercase() {
            if index > 0 {
                out.push('_');
            }
            out.push(character.to_ascii_lowercase());
        } else {
            out.push(character);
        }
    }
    out
}

/// `list_box_option` → `ListBoxOption`. The inverse of [`snake_case`], which is
/// what lets one written list of variants serve both directions.
fn pascal_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut capitalize = true;
    for character in name.chars() {
        if character == '_' {
            capitalize = true;
            continue;
        }
        if capitalize {
            out.extend(character.to_uppercase());
            capitalize = false;
        } else {
            out.push(character);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_declared_name_parses_back_to_a_role() {
        let names = role_names();
        assert!(names.len() > 150, "the role table looks truncated");
        for name in &names {
            assert!(
                role_from_name(name).is_some(),
                "`{name}` is declared but does not parse"
            );
        }
    }

    #[test]
    fn names_are_the_snake_case_of_the_variant_they_mirror() {
        assert_eq!(
            role_from_name("list_box_option"),
            Some(gpui::Role::ListBoxOption)
        );
        assert_eq!(role_from_name("combo_box"), Some(gpui::Role::ComboBox));
        assert_eq!(role_from_name("check_box"), Some(gpui::Role::CheckBox));
        assert!(role_names().contains(&"list_box_option".to_owned()));
    }

    /// The one variant the table leaves out, so that a script naming it is told
    /// why rather than told it does not exist.
    #[test]
    fn the_filtered_role_is_not_a_name_the_runtime_accepts() {
        assert_eq!(role_from_name(FILTERED_ROLE), None);
        assert!(!role_names().contains(&FILTERED_ROLE.to_owned()));
    }

    #[test]
    fn an_unknown_name_is_not_coerced_into_a_role() {
        assert_eq!(role_from_name("listbox"), None);
        assert_eq!(role_from_name(""), None);
        assert_eq!(role_from_name("Button"), None);
    }
}
