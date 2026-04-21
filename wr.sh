sed -i '' \
  -e 's/\.label(cx\.theme()\.notification\.placement\.to_string())/.label(format!("{:?}", cx.theme().notification.placement))/' \
  -e 's/PopupMenuItem::new(placement\.to_string())/PopupMenuItem::new(format!("{:?}", placement))/' \
  crates/story/src/stories/notification_story.rs
