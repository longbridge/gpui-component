//! What a piece of running script is allowed to do, and who answers for it.
//!
//! A policy is the capability grant, the settings file and the native modules,
//! held together because they answer the same question: *this code, right now,
//! under whose authority?*
//!
//! # Why it is not thread state
//!
//! It used to be. `set_capabilities` wrote into a thread-local and every `fs`
//! call read it back, which is sound for exactly one running application and
//! wrong the moment there are two. Two runtimes on one thread shared the last
//! installer's permissions; and a plugin host — two plugins inside *one*
//! runtime — could only install one grant at a time and swap it around each
//! call.
//!
//! That swap cannot be made correct, and the reason is worth stating because it
//! rules out the obvious repairs. It is a guard in *time*:
//!
//! ```text
//! activate(A) ─▶ A runs ─▶ await ────────────────▶ A resumes
//!                                  activate(B)         ↑
//!                                                 under B's grant
//! ```
//!
//! `await` crosses time. So does a second panel rendering between two frames of
//! the first. Anything that answers "whose grant?" by asking what was installed
//! most recently is answering a question about the past.
//!
//! Moving the state onto the runtime does not help either: two plugins share one
//! runtime, so they would share the field.
//!
//! # Where it lives instead
//!
//! On the call. The internal call scope already pushes a frame for every entry into
//! script and pops it on the way out — it exists to answer "am I inside a legal
//! host call?" — so the policy is a field of that frame. Code executing under a
//! frame reads its policy; a continuation resuming after an `await` brings its
//! own frame back with it, and nothing can be swapped underneath it.
//!
//! A [`crate::view::ScriptView`] carries the policy its script runs under, and
//! every entry point that has a view takes the policy from it. The entry points
//! that have no view yet — loading a module, constructing a view — take one
//! explicitly, because "under whose authority is this code being loaded" is a
//! question the caller has to answer rather than inherit.

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use crate::{capability::Capabilities, native::NativeModules, store::Store};

/// The authority one application or plugin runs under.
///
/// Built once by the host and shared by handle: a view holds one, a call frame
/// borrows it, and two plugins hold two.
pub struct Policy {
    capabilities: Capabilities,
    /// A live handle, not a value — and deliberately unlike `capabilities`.
    ///
    /// A grant is frozen with the code it was given to: that is the whole point
    /// of a policy, and a view that could have its permissions changed after it
    /// was built would be back where this started. The module registry is the
    /// opposite question. It is not the script's authority but the *host's own
    /// surface*, and every module closure holds GPUI entity handles — so
    /// [`crate::native::clear_modules`] has to actually revoke, or a host that
    /// tears itself down leaves handles registered and GPUI reports the leak.
    ///
    /// Shared by every copy of a policy, so an edit made through
    /// [`update_default`] reaches the views already holding the old one. Not
    /// shared *between* policies: a plugin's registry is its own, and a host
    /// clearing its modules does not reach into one.
    modules: Rc<RefCell<Rc<NativeModules>>>,
    /// `None` when the host named no settings file, which is a denial with its
    /// own message rather than an empty store.
    ///
    /// Shared like `modules` and for a sharper reason: a store *is* its file, so
    /// two caches for one path is never a thing anyone wants. Two would answer
    /// `get` differently and run two write queues over the same temporary file.
    store: Rc<RefCell<Option<Store>>>,
}

impl Default for Policy {
    fn default() -> Self {
        Self::new()
    }
}

impl Policy {
    /// A policy that permits nothing. Every grant is added deliberately.
    pub fn new() -> Self {
        Self {
            capabilities: Capabilities::default(),
            modules: Rc::new(RefCell::new(Rc::new(NativeModules::default()))),
            store: Rc::new(RefCell::new(None)),
        }
    }

    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Names the settings file and reads it.
    ///
    /// The read happens here rather than on first use, because here is
    /// start-up — before a window exists — and first use is wherever the script
    /// happens to touch the store, which may be `render`.
    pub fn with_store_path(self, path: PathBuf) -> Self {
        let mut store = Store::new(path);
        store.warm = Some(store.load());
        *self.store.borrow_mut() = Some(store);
        self
    }

    /// The native modules this application may reach.
    ///
    /// Per policy rather than per process, because "which host functions may
    /// this plugin call" is exactly as much a grant as "which directories may it
    /// read". A global registry cannot express a host that gives one plugin a
    /// module and not another.
    pub fn with_native_modules(self, modules: NativeModules) -> Self {
        *self.modules.borrow_mut() = Rc::new(modules);
        self
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    pub(crate) fn modules(&self) -> Rc<NativeModules> {
        // Cloned out rather than borrowed across the call: dispatching into a
        // module may register modules again, and the borrow would still be open.
        self.modules.borrow().clone()
    }

    /// Runs `body` against the settings file, if the host named one.
    pub(crate) fn with_store<R>(&self, body: impl FnOnce(&mut Store) -> R) -> Option<R> {
        self.store.borrow_mut().as_mut().map(body)
    }
}

thread_local! {
    /// The policy a view gets when nobody gave it one.
    ///
    /// A host that runs a single application configures this and never thinks
    /// about policies again; a plugin host builds one per plugin and hands each
    /// to its view. The default exists so the simple case stays simple, not so
    /// that the policy can be ambient — nothing *reads* this at call time, it is
    /// only what a view is born holding.
    static DEFAULT: RefCell<Rc<Policy>> = RefCell::new(Rc::new(Policy::new()));
}

/// Replaces the policy new views are created with.
pub fn set_default(policy: Policy) {
    DEFAULT.with(|current| *current.borrow_mut() = Rc::new(policy));
}

/// Edits the default in place, for the host entry points that grant one thing.
pub fn update_default(edit: impl FnOnce(Policy) -> Policy) {
    DEFAULT.with(|current| {
        let existing = current.borrow().clone();
        let policy = Rc::try_unwrap(existing).unwrap_or_else(|shared| shared.duplicate());
        *current.borrow_mut() = Rc::new(edit(policy));
    });
}

/// The policy a view is created with.
pub fn default() -> Rc<Policy> {
    DEFAULT.with(|current| current.borrow().clone())
}

impl Policy {
    /// A copy that keeps the grant and shares everything else.
    ///
    /// Used when the default is edited while a view already holds it: the view
    /// keeps what it was given, and the edit lands on a new default handle.
    ///
    /// The split is the rule this whole module exists for. **The capability
    /// grant is the one thing a view freezes** — authority belongs to the code,
    /// and a view whose permissions could be changed after it was built is
    /// exactly the hole the policy replaced. Everything else here is the host's
    /// live configuration of one application: revoking a module has to reach the
    /// views that can call it, and a store has to stay one cache over one file.
    pub(crate) fn duplicate(&self) -> Self {
        Self {
            capabilities: self.capabilities.clone(),
            modules: self.modules.clone(),
            store: self.store.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capabilities;

    fn reading(root: &str) -> Policy {
        Policy::new().with_capabilities(Capabilities::new().read_roots([PathBuf::from(root)]))
    }

    /// Two plugins loaded at once hold two grants. This is the whole point.
    #[test]
    fn two_policies_do_not_share_a_grant() {
        let plugin = Rc::new(reading("/tmp/plugin"));
        let application = Rc::new(Policy::new());

        assert!(plugin.capabilities().has_read_access());
        assert!(!application.capabilities().has_read_access());
    }

    /// Editing the default while a view holds it must not change what that view
    /// was granted — otherwise a host that grants a second application something
    /// silently widens the first.
    #[test]
    fn editing_the_default_does_not_widen_a_grant_already_handed_out() {
        set_default(Policy::new());
        let held = default();
        assert!(!held.capabilities().has_read_access());

        update_default(|policy| {
            policy.with_capabilities(Capabilities::new().read_roots([PathBuf::from("/tmp")]))
        });

        assert!(
            !held.capabilities().has_read_access(),
            "the grant a view is holding is frozen"
        );
        assert!(default().capabilities().has_read_access());
    }

    /// The deliberate exception. A module closure holds GPUI entity handles, so
    /// a host tearing itself down has to be able to revoke — leaving them
    /// registered is a leak GPUI reports at shutdown.
    #[test]
    fn revoking_a_module_reaches_a_policy_already_handed_out() {
        set_default(Policy::new().with_native_modules(crate::native::NativeModules::new()));
        let held = default();

        update_default(|policy| policy.with_native_modules(crate::native::NativeModules::new()));

        assert!(
            Rc::ptr_eq(&held.modules(), &default().modules()),
            "the registry is one live handle, not a copy per holder"
        );
    }

    /// One file, one cache, one write queue. Two would answer `get` differently
    /// and race each other through the same temporary file.
    #[test]
    fn a_copy_of_a_policy_shares_its_store() {
        let path = std::env::temp_dir().join("gpui-shell-policy-test.json");
        set_default(Policy::new().with_store_path(path));
        let held = default();

        update_default(|policy| policy.with_capabilities(Capabilities::new()));

        held.with_store(|store| store.touch());
        assert!(
            default()
                .with_store(|store| store.is_dirty())
                .expect("the copy has the same store"),
            "the two handles are one store"
        );
    }
}
