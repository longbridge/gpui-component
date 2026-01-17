---
name: github-pull-request-description
description: Write a description to description GitHub Pull Request.
---

## Description

We less than 150 words description for a PR changes, including new features, bug fixes, and improvements. And if there have APIs break changes (Only `crates/ui` changes) we should have a section called `## Breaking Changes` to list them clearly.

## Breaking changes description

When a pull request introduces breaking changes to a codebase, it's important to clearly communicate these changes to users and developers who rely on the code. A well-written breaking changes description helps ensure that everyone understands what has changed, why it has changed, and how to adapt to the new version.

We can get the changes from the PR diff and summarize them in a clear and concise manner. Aim to provide a clear APIs changes for users to follow.

### Format

We pefer the following format for breaking changes descriptions:

1. Use bullet list for each breaking change item.
2. Each item should have title and a code block showing the old and new usage by use `diff`.
3. Use `## Breaking Changes` as the section title.
4. Use english language.

**For example:**

````md
## Breaking Changes

- Added `id` parameter to `Sidebar::new`.

```diff
- Sidebar::new()
+ Sidebar::new("sidebar")
```

- Removed the `left` and `right` methods; use `side` instead.
  > Default is left.

```diff
- Sidebar::right()
+ Sidebar::new("sidebar").side(Side::Right)
```
````
