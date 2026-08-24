// The object the application is about: one release, and the work it still
// needs. Kept separate from the screen so the screen stays about presentation.

export const RELEASE = "v0.5.2";

export const SECTIONS = [
  {
    name: "Build",
    items: [
      { id: "tag", caption: "Tag the release commit", done: true },
      { id: "crates", caption: "Publish crates to the registry", done: false, blocking: true },
      { id: "binaries", caption: "Attach platform binaries", done: false, blocking: true },
    ],
  },
  {
    name: "Verify",
    items: [
      { id: "suite", caption: "Full test suite on three platforms", done: true },
      { id: "story", caption: "Story gallery opens on a clean profile", done: true },
      { id: "upgrade", caption: "Upgrade path from the previous minor", done: false },
    ],
  },
  {
    name: "Communicate",
    items: [
      { id: "changelog", caption: "Changelog entry with breaking changes", done: false, blocking: true },
      { id: "docs", caption: "Documentation site rebuilt", done: false },
      { id: "announce", caption: "Announcement drafted", done: false },
    ],
  },
];

export const FILTERS = [
  { id: "all", caption: "All" },
  { id: "open", caption: "Open" },
  { id: "done", caption: "Done" },
];

export const matches = (item, filter) =>
  filter === "all" || (filter === "done") === item.done;
