// Persistence.
//
// `gpui.store` is capability-gated: a host that did not grant storage makes
// every call throw. That is not an error condition for this app — it just means
// the list is in-memory for this run — so the failure is absorbed here rather
// than checked at every call site.

import { store, log } from "gpui";

const KEY = "todolist.items";

export function load() {
  try {
    const saved = store.get(KEY);
    return Array.isArray(saved) ? saved : [];
  } catch (error) {
    log.warn(`todolist: storage unavailable, starting empty (${error.message})`);
    return [];
  }
}

/// Returns whether the write reached disk, so the interface can say so.
export function save(items) {
  try {
    store.set(KEY, items);
    return true;
  } catch (error) {
    log.warn(`todolist: could not save (${error.message})`);
    return false;
  }
}
