# Public API Design & Naming Conventions

## Reusable Component API Rules

- Constructors should establish valid defaults (`new` or semantic constructors).
- Builders take and return `Self` and use domain language.
- Fluent builders omit `set_` because they consume and return `Self`; mutation through `&mut self` uses `set_`.
- Evolvable behavioral seams use private fields, builders for construction, and readers for inspection.
- Boolean readers use `is_`, `has_`, or `can_` where a same-named builder exists (e.g. builder `disabled(bool)` vs reader `is_disabled()`).
- Non-boolean setters use `with_` when readers need the plain field name (e.g. `with_size(...)` / `size()`).
- Explicit compound parts are preferable to inspecting arbitrary descendants.

## Vocabulary & Naming Patterns

| Concept | Naming pattern | Example |
| --- | --- | --- |
| Value-like rendered control | noun | `Button`, `Checkbox`, `Tab` |
| Retained behavioral model | `<Control>State` | `InputState`, `TableState` |
| Imperative shared reference | `<Control>Handle` | `DialogHandle`, scroll handle |
| Semantic notification | `<Control>Event` | `TableEvent`, `SelectEvent` |
| Keyboard command | verb or intent noun | `Confirm`, `Cancel`, `SelectNext` |
| Pluggable data/behavior owner | `<Role>Delegate` / `<Role>Provider` | `TableDelegate`, `CompletionProvider` |
| Application-supplied presentation | `render_<part>` or `<part>_renderer` | `render_item` |
| Construction | `new`, or a semantic constructor | `new`, `horizontal`, `vertical` |
| Fluent property | noun/adjective | `label`, `disabled`, `selected`, `placement` |
| General non-boolean replacement builder | `with_<field>` | `with_size`, `with_mode` |
| In-place mutation | `set_<field>` | `set_items`, `set_selected_index` |
| Boolean reader | `is_` / `has_` / `can_` | `is_open`, `has_selection` |
| Plain value reader | field noun | `placement`, `selected_value` |
| Callback registration | `on_<event or intent>` | `on_click`, `on_open_change` |
| Rendering a named region | `render_<region>` | `render_toolbar`, `render_content` |

## Precise Domain Words

- **selected** is persistent membership or active item; **focused** is keyboard target; **hovered** is pointer presence; **confirmed** is activation result. Never use them interchangeably.
- **open/close** describes overlay/disclosure; **show/hide** is transient presentation; **expand/collapse** describes hierarchy.
- **disabled** prevents interaction; **read-only** permits navigation/selection but prevents editing; **loading** prevents duplicate work while pending.
- **index** is current positional coordinate; **id** is stable identity; `IndexPath` represents hierarchical position.
- **value** is controlled domain data; **presentation** is read-only rendering snapshot; **state** is retained behavior.
- **placement** is side/anchor policy; **position** is resolved geometry.
- **size** is semantic control tier; **width/height/bounds** are geometry.

## Callback & Documentation Rules

- Use `on_click` only for genuine click contracts. Controlled semantic primitives prefer `on_change(next_value, ...)`.
- Name before/after hooks precisely: `on_will_change` (veto/prepare), `on_change` (value contract), `on_confirm` (commit), `on_dismiss` (close).
- Translation keys describe stable intent (`dialog.delete_project.title`), not English sentences or coordinates. Never assemble sentences from translated fragments.
