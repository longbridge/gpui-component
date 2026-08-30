//! Descriptor helpers shared by every component family.
//!
//! These exist because the same three or four shapes — a boolean setter, a
//! string setter, a component that rejects style — recur in nearly every
//! family. Writing them once keeps one error message and one documentation
//! contract for all of them, instead of a dozen copies that drift apart.

// An integration test `#[path]`-includes one family beside this module, so a
// helper the chosen family happens not to use is dead only in that build.
#![allow(dead_code)]

use std::any::Any;

use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentPayload, MethodDescriptor,
    anyhow, gpui,
};

/// The payload of a component whose constructor takes no arguments.
#[derive(Clone, Copy)]
pub(super) struct Empty;

/// A `name(value: boolean)` method recording `make(value)`.
///
/// `component` prefixes the error a script sees, so a rejected call names the
/// component it was made on rather than a bare method name.
pub(super) fn bool_method<T: Any + Send + Sync>(
    component: &'static str,
    name: &'static str,
    documentation: &'static str,
    make: fn(bool) -> T,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Boolean)],
        move |arguments| match arguments {
            [ComponentArgument::Boolean(value)] => Ok(ComponentPayload::new(make(*value))),
            _ => Err(format!("{component}.{name} expects one boolean")),
        },
    )
    .with_documentation(documentation)
}

/// A `name(value: string)` method recording `make(value)`.
pub(super) fn string_method<T: Any + Send + Sync>(
    component: &'static str,
    name: &'static str,
    documentation: &'static str,
    make: fn(String) -> T,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::String)],
        move |arguments| match arguments {
            [ComponentArgument::String(value)] => Ok(ComponentPayload::new(make(value.clone()))),
            _ => Err(format!("{component}.{name} expects one string")),
        },
    )
    .with_documentation(documentation)
}

/// Refuses style on a component that has nowhere to put it.
///
/// A data-carrying component such as `MenuItem` or `TableCell` renders no box
/// of its own. Silently dropping a script's `.bg(…)` would look like a shell
/// bug; saying so names the component that cannot honour it.
pub(super) fn reject_style(style: gpui::StyleRefinement, name: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        style == gpui::StyleRefinement::default(),
        "{name} carries data rather than a box, so it does not implement Styled"
    );
    Ok(())
}

/// Requires a typed child to be one of the components `parent` accepts.
///
/// `actual` is `None` for an ordinary element, which is the common script
/// mistake, so it is named in the message rather than reported as "no child".
pub(super) fn require_child(
    parent: &str,
    actual: Option<&'static str>,
    allowed: &[&str],
) -> anyhow::Result<()> {
    anyhow::ensure!(
        actual.is_some_and(|name| allowed.contains(&name)),
        "{parent} accepts only registered {} children; received {}",
        allowed.join(" or "),
        actual.unwrap_or("an ordinary element")
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_parent_names_what_it_accepts_and_what_it_received() {
        assert!(require_child("Form", Some("Field"), &["Field"]).is_ok());
        assert!(
            require_child("TableRow", Some("TableCell"), &["TableHead", "TableCell"]).is_ok(),
            "a parent may accept more than one child component"
        );

        let ordinary = require_child("Form", None, &["Field"]).unwrap_err();
        assert_eq!(
            ordinary.to_string(),
            "Form accepts only registered Field children; received an ordinary element"
        );

        let wrong = require_child("Table", Some("TableCell"), &["TableBody"]).unwrap_err();
        assert_eq!(
            wrong.to_string(),
            "Table accepts only registered TableBody children; received TableCell"
        );
    }
}
