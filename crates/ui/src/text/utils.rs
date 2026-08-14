use std::path::PathBuf;

use gpui::{ImageSource, SharedUri};

const NUMBERED_PREFIXES_1: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const NUMBERED_PREFIXES_2: &str = "abcdefghijklmnopqrstuvwxyz";

const BULLETS: [&str; 5] = ["•", "◦", "▪", "‣", "⁃"];

/// Returns the prefix for a list item.
pub(super) fn list_item_prefix(ix: usize, ordered: bool, depth: usize) -> String {
    if ordered {
        if depth == 0 {
            return format!("{}. ", ix + 1);
        }

        if depth == 1 {
            return format!(
                "{}. ",
                NUMBERED_PREFIXES_1
                    .chars()
                    .nth(ix % NUMBERED_PREFIXES_1.len())
                    .unwrap()
            );
        } else {
            return format!(
                "{}. ",
                NUMBERED_PREFIXES_2
                    .chars()
                    .nth(ix % NUMBERED_PREFIXES_2.len())
                    .unwrap()
            );
        }
    } else {
        let depth = depth.min(BULLETS.len() - 1);
        let bullet = BULLETS[depth];
        return format!("{} ", bullet);
    }
}

/// Converts an image URL from a document into an [`ImageSource`].
///
/// URLs with a scheme (e.g. `https:`, `data:`) load as remote resources, while
/// `file://` URLs and plain paths load from the filesystem. Relative paths
/// resolve against the process working directory. A single-letter scheme is
/// treated as a Windows drive letter.
pub(super) fn image_source(url: &SharedUri) -> ImageSource {
    let url_str = url.as_ref();
    if let Some(path) = url_str.strip_prefix("file://") {
        return PathBuf::from(path).into();
    }

    let has_scheme = url_str.split_once(':').is_some_and(|(scheme, _)| {
        scheme.len() > 1
            && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    });

    if has_scheme {
        url.clone().into()
    } else {
        PathBuf::from(url_str).into()
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::Arc};

    use gpui::{ImageSource, Resource};

    use crate::text::utils::{image_source, list_item_prefix};

    #[test]
    fn test_image_source() {
        fn source(url: &str) -> Resource {
            match image_source(&url.to_string().into()) {
                ImageSource::Resource(resource) => resource,
                _ => panic!("expected a resource for {url:?}"),
            }
        }
        fn assert_uri(url: &str) {
            match source(url) {
                Resource::Uri(uri) => assert_eq!(uri.as_ref(), url),
                other => panic!("expected Uri for {url:?}, got {other:?}"),
            }
        }
        fn assert_path(url: &str, expected: &str) {
            match source(url) {
                Resource::Path(path) => assert_eq!(path, Arc::from(Path::new(expected))),
                other => panic!("expected Path for {url:?}, got {other:?}"),
            }
        }

        assert_uri("https://example.com/logo.png");
        assert_uri("http://example.com/logo.png");
        assert_uri("data:image/png;base64,iVBORw0KGgo=");

        assert_path("website/public/logo.svg", "website/public/logo.svg");
        assert_path("./images/a.png", "./images/a.png");
        assert_path("../images/a.png", "../images/a.png");
        assert_path("/absolute/path/logo.svg", "/absolute/path/logo.svg");
        assert_path("file:///absolute/path/logo.svg", "/absolute/path/logo.svg");
        // A single-letter scheme is a Windows drive letter, not a URL scheme.
        assert_path(r"C:\images\logo.png", r"C:\images\logo.png");
        // A colon inside a path segment does not make it a URL.
        assert_path("docs/a:b.png", "docs/a:b.png");
    }

    #[test]
    fn test_list_item_prefix() {
        assert_eq!(list_item_prefix(0, true, 0), "1. ");
        assert_eq!(list_item_prefix(1, true, 0), "2. ");
        assert_eq!(list_item_prefix(2, true, 0), "3. ");
        assert_eq!(list_item_prefix(10, true, 0), "11. ");
        assert_eq!(list_item_prefix(0, true, 1), "A. ");
        assert_eq!(list_item_prefix(1, true, 1), "B. ");
        assert_eq!(list_item_prefix(2, true, 1), "C. ");
        assert_eq!(list_item_prefix(0, true, 2), "a. ");
        assert_eq!(list_item_prefix(1, true, 2), "b. ");
        assert_eq!(list_item_prefix(6, true, 2), "g. ");
        assert_eq!(list_item_prefix(0, true, 1), "A. ");
        assert_eq!(list_item_prefix(0, true, 2), "a. ");
        assert_eq!(list_item_prefix(0, false, 0), "• ");
        assert_eq!(list_item_prefix(0, false, 1), "◦ ");
        assert_eq!(list_item_prefix(0, false, 2), "▪ ");
        assert_eq!(list_item_prefix(0, false, 3), "‣ ");
        assert_eq!(list_item_prefix(0, false, 4), "⁃ ");
    }
}
