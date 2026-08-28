//! The one action type a script can name.
//!
//! GPUI actions are Rust types: `actions!(editor, [Save])` generates a `Save`
//! struct, and everything downstream — `on_action::<Save>`, a key binding, the
//! dispatch itself — is keyed by that type. A script cannot produce a Rust
//! type, so the whole family collapses into [`ShellAction`], one type carrying
//! the script's own id.
//!
//! That works because GPUI matches a dispatch by `TypeId` and hands the
//! listener the action instance: every script action shares one `TypeId`, so
//! one listener registration hears all of them, and the id inside decides
//! which handler actually runs. The filtering moves from GPUI's dispatch table
//! into one comparison at the listener, and nothing else about the model
//! changes — the keymap, the context predicates, and the bubble up the focus
//! path are GPUI's own.
//!
//! # Why this is not just `on_key_down`
//!
//! A keystroke handler hears a chord where the pointer of attention already
//! is. An action says "this means Save" once, and lets the keymap decide which
//! chord means it, in which context, on which platform — and lets a menu item
//! or a button dispatch the same thing without pretending to be a keyboard.
//! The two are different levels, and a script that only had the first would
//! have to reimplement the second badly.

use std::any::Any;

use gpui::{Action, SharedString};

/// One action named by a script.
///
/// The `id` is the script's own string — `"save"`, `"workspace::toggle-left"`
/// — and it is the whole identity: two `ShellAction`s are equal when their ids
/// are, and a listener runs when the id it was registered for matches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellAction {
    id: SharedString,
}

impl ShellAction {
    /// Names one script action.
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self { id: id.into() }
    }

    /// The script's own name for it.
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl Action for ShellAction {
    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }

    /// Compares the ids, not just the types.
    ///
    /// Every script action is a `ShellAction`, so comparing types would make
    /// all of them equal to each other — which is exactly the question GPUI
    /// asks when it looks up which keystroke currently triggers a given
    /// action, and the answer would be the first binding in the keymap for
    /// every action alike.
    fn partial_eq(&self, action: &dyn Action) -> bool {
        action
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| other.id == self.id)
    }

    fn name(&self) -> &'static str {
        Self::name_for_type()
    }

    /// One name for the whole family, and it has to be: the trait returns
    /// `&'static str`, and a script id is discovered at run time.
    ///
    /// Nothing downstream depends on the name distinguishing two script
    /// actions — dispatch is by `TypeId` and the id is compared at the
    /// listener — so what this name is for is the keymap's JSON form and
    /// anything that displays an action to a person.
    fn name_for_type() -> &'static str {
        "gpui_shell::ShellAction"
    }

    fn build(value: serde_json::Value) -> anyhow::Result<Box<dyn Action>> {
        let id = value
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "a `gpui_shell::ShellAction` needs the script's own name for it: \
                     `{{ \"id\": \"save\" }}`"
                )
            })?;
        Ok(Box::new(Self::new(id.to_owned())))
    }
}

/// Reads the script id out of an action a listener was handed.
///
/// A listener registered through GPUI's typed builder already receives a
/// `&ShellAction`; this exists for the boxed path, where what arrives is a
/// `&dyn Action` that may well be one of the host's own.
pub fn script_id(action: &dyn Any) -> Option<&str> {
    action.downcast_ref::<ShellAction>().map(ShellAction::id)
}
