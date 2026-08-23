# Interface Language and Copywriting

Words are part of interface architecture. Prefer the shortest wording that remains accurate in its context.

## Context Economy

- Do not repeat information that the surrounding surface already establishes (`Users`, not `User Management`; `Shortcuts`, not `Shortcut Configuration Management`).
- Use **nouns for destinations and objects** (`Users`, `Appearance`, `Orders`);
- Use **verbs for commands** (`Save`, `Duplicate`, `Export`);
- Use **adjectives/short phrases for states** (`Offline`, `Up to date`, `Pending review`).
- Avoid fluff words: `Management`, `Module`, `Page`, `Function`, `Operation`, `System`.

---

## Natural Localization

- Start from shared intent, hierarchy, and terminology, then compose each locale as natural interface language.
- Do not preserve the source language's word order or grammatical filler.
- Preserve established framework terms and API identifiers as code.

---

## Buttons and Confirmation Dialogs

Button labels describe the result, not the gesture (`Save`, `Delete`, not `Click to save`).

| Context | Weak | Prefer |
| --- | --- | --- |
| Delete dialog | `Yes`, `Sure`, `Confirm deletion` | `Delete` |
| Unsaved changes | `Confirm`, `Yes` | `Discard changes` |
| Pure acknowledgement | `Confirm operation` | `OK` or `Done` |
| Complex consent without verb | `Yes` | `Confirm` |

### Confirmation Dialog Structure

- **Title**: The decision or condition (e.g. `Delete “Roadmap”?`);
- **Body**: Only new scope, consequence, or recovery information;
- **Actions**: `Cancel` and the specific result verb (e.g. `Delete`);
- **Tone**: Omit ritual phrases (“Are you sure you want to…”, “Please note that…”, “successfully”).

---

## Capitalization, Punctuation, and Ellipses

- **Sentence case default** in English: `Reset layout`, not `Reset Layout`.
- **Terminal punctuation**: Labels, buttons, menu items, tabs, and headings do not take a final period. Full sentences (errors, warnings) do.
- **Ellipsis character (`…`)**:
  - Use the single ellipsis character (`…`), never three dots (`...`).
  - Append to any command that opens a dialog, sheet, or window, or requires more input before completing (`Settings…`, `Export…`).
  - Do not append to immediately executed commands.
