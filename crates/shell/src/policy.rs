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
//! On the call. [`crate::scope`] already pushes a frame for every entry into
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
    modules: Rc<NativeModules>,
    /// `None` when the host named no settings file, which is a denial with its
    /// own message rather than an empty store.
    store: RefCell<Option<Store>>,
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
            modules: Rc::new(NativeModules::default()),
            store: RefCell::new(None),
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
    pub fn with_native_modules(mut self, modules: NativeModules) -> Self {
        self.modules = Rc::new(modules);
        self
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    pub fn modules(&self) -> Rc<NativeModules> {
        self.modules.clone()
    }

    /// Runs `body` against the settings file, if the host named one.
    pub fn with_store<R>(&self, body: impl FnOnce(&mut Store) -> R) -> Option<R> {
        self.store.borrow_mut().as_mut().map(body)
    }

    pub fn has_store(&self) -> bool {
        self.store.borrow().is_some()
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
    /// A copy that shares the modules and the store but not the handle.
    ///
    /// Used when the default is edited while a view already holds it: the view
    /// keeps what it was given, and the edit lands on a new default. A store is
    /// deliberately *not* duplicated — two policies pointing at one file would
    /// be two caches disagreeing about it — so the copy takes the path and reads
    /// it again.
    fn duplicate(&self) -> Self {
        let store = self.store.borrow().as_ref().map(|store| store.path.clone());

        let copy = Self {
            capabilities: self.capabilities.clone(),
            modules: self.modules.clone(),
            store: RefCell::new(None),
        };
        match store {
            Some(path) => copy.with_store_path(path),
            None => copy,
        }
    }
}
