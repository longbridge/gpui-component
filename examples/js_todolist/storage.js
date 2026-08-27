// Persistence.
//
// `localStorage` is capability-gated: a host that did not grant storage makes
// every call throw. That is not an error condition for this app — it just means
// the list is in-memory for this run — so the failure is absorbed here rather
// than checked at every call site.
//
// Values are strings, exactly as on the web, so the list goes through JSON in
// both directions. That conversion and the cast it needs live here, which is
// the point of having one storage module: every other file works in `Todo`.

const KEY = "todolist.items";

/**
 * @returns {Todo[]}
 */
export function load() {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved === null) return [];
    const items = JSON.parse(saved);
    return Array.isArray(items) ? /** @type {Todo[]} */ (/** @type {unknown} */ (items)) : [];
  } catch (/** @type {any} */ error) {
    console.warn(`todolist: storage unavailable, starting empty (${error.message})`);
    return [];
  }
}

/// Returns whether the write reached disk, so the interface can say so.
/** @param {Todo[]} items */
export function save(items) {
  try {
    localStorage.setItem(KEY, JSON.stringify(items));
    return true;
  } catch (/** @type {any} */ error) {
    console.warn(`todolist: could not save (${error.message})`);
    return false;
  }
}
