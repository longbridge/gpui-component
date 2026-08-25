// Persistence.
//
// `gpui.store` is capability-gated: a host that did not grant storage makes
// every call throw. That is not an error condition for this app — it just means
// the list is in-memory for this run — so the failure is absorbed here rather
// than checked at every call site.

import { store, log } from "gpui";
/** @import { Json } from "gpui" */

const KEY = "todolist.items";

/**
 * The two casts below are the only ones in this application, and they are where
 * they belong: storage is untyped JSON in both directions, so the shape has to
 * be asserted somewhere. Doing it here means every other file works in `Todo`
 * and nothing downstream has to wonder.
 *
 * @returns {Todo[]}
 */
export function load() {
  try {
    const saved = store.get(KEY);
    return Array.isArray(saved) ? /** @type {Todo[]} */ (/** @type {unknown} */ (saved)) : [];
  } catch (/** @type {any} */ error) {
    log.warn(`todolist: storage unavailable, starting empty (${error.message})`);
    return [];
  }
}

/// Returns whether the write reached disk, so the interface can say so.
/** @param {Todo[]} items */
export function save(items) {
  try {
    store.set(KEY, /** @type {Json} */ (/** @type {unknown} */ (items)));
    return true;
  } catch (/** @type {any} */ error) {
    log.warn(`todolist: could not save (${error.message})`);
    return false;
  }
}
