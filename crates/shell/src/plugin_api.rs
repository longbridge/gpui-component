//! The version of the script API.
//!
//! An application states what it needs — `gpui.require_api("1.0")` — and the
//! runtime either agrees or refuses at the first line, before anything is
//! built. That is the only moment where a mismatch is cheap: once a view has
//! rendered, a missing method is an exception in the middle of an interface.
//!
//! The version tracks the *script* surface, not the crate. Adding a binding is
//! a minor; changing or removing one is a major.

/// The script API this runtime provides.
pub const VERSION: &str = "1.0";

/// Checks a requested version against [`VERSION`].
///
/// The grammar is deliberately small: `"1"` or `"1.0"`. A caret or a range
/// would invite the impression that this runtime resolves versions, which it
/// does not — there is exactly one implementation present.
pub fn check(wanted: &str) -> Result<(), String> {
    let (wanted_major, wanted_minor) = parse(wanted)
        .ok_or_else(|| format!("`{wanted}` is not an API version; expected `1` or `1.0`"))?;
    let (major, minor) = parse(VERSION).expect("the runtime's own version parses");

    if wanted_major != major {
        return Err(format!(
            "this application requires script API {wanted}, and the runtime provides {VERSION}; \
             a different major version is not compatible"
        ));
    }

    if wanted_minor > minor {
        return Err(format!(
            "this application requires script API {wanted}, and the runtime provides {VERSION}; \
             upgrade gpui-shell"
        ));
    }

    Ok(())
}

fn parse(version: &str) -> Option<(u32, u32)> {
    let mut parts = version.trim().split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = match parts.next() {
        Some(minor) => minor.parse().ok()?,
        None => 0,
    };
    parts.next().is_none().then_some((major, minor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_version_is_accepted_in_both_spellings() {
        assert!(check("1.0").is_ok());
        assert!(check("1").is_ok());
    }

    #[test]
    fn a_newer_minor_is_refused_with_the_action_to_take() {
        let error = check("1.9").unwrap_err();
        assert!(error.contains("upgrade gpui-shell"), "{error}");
    }

    #[test]
    fn a_different_major_is_refused_as_incompatible() {
        assert!(check("2.0").unwrap_err().contains("not compatible"));
    }

    #[test]
    fn nonsense_names_the_expected_shape() {
        assert!(check("^1.0").unwrap_err().contains("expected"));
        assert!(check("1.0.0").unwrap_err().contains("expected"));
    }
}
