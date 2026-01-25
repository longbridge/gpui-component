const NUMBERED_PREFIXES_1: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const NUMBERED_PREFIXES_2: &str = "abcdefghijklmnopqrstuvwxyz";

const BULLETS: [&str; 5] = ["▪", "•", "◦", "‣", "⁃"];

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

#[cfg(test)]
mod tests {
  use crate::text::utils::list_item_prefix;

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
    assert_eq!(list_item_prefix(0, false, 0), "▪ ");
    assert_eq!(list_item_prefix(0, false, 1), "• ");
    assert_eq!(list_item_prefix(0, false, 2), "◦ ");
    assert_eq!(list_item_prefix(0, false, 3), "‣ ");
    assert_eq!(list_item_prefix(0, false, 4), "⁃ ");
  }
}
