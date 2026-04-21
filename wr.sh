#!/bin/bash
set -e

echo "=== Final Anchor import fixes ==="

# 1. Add 'use gpui::Anchor;' to all files that currently lack it but use Anchor.
#    We'll add it after the existing 'use gpui::...' lines, or at the top if none exist.
for file in crates/ui/src/hover_card.rs \
            crates/ui/src/root.rs \
            crates/ui/src/menu/popup_menu.rs \
            crates/ui/src/notification.rs \
            crates/ui/src/tooltip.rs \
            crates/ui/src/input/element.rs; do
    if ! grep -q "use gpui::Anchor;" "$file"; then
        # Insert after first 'use gpui::' line, or at line 1 if none
        sed -i '' '/use gpui::/ {
            h
            s/.*/use gpui::Anchor;/
            p
            x
            s/.*//
        }' "$file"
        # If no 'use gpui::' lines, put at top
        if ! grep -q "use gpui::Anchor;" "$file"; then
            sed -i '' '1i\
use gpui::Anchor;
' "$file"
        fi
    fi
done

# 2. Fix the border_corners call in button_group.rs: it expects Corners<bool> but we cannot use Corners::all because bool doesn't satisfy Half.
#    Instead, use Corners { top_left: false, top_right: false, bottom_left: false, bottom_right: false }
sed -i '' 's/\.border_corners(Corners::all(false))/\.border_corners(Corners { top_left: false, top_right: false, bottom_left: false, bottom_right: false })/g' \
    crates/ui/src/button/button_group.rs

# 3. In input/element.rs, the 'corners' variable is of type Vec<Anchor<Point<Pixels>>> (which is wrong; Anchor is an enum, not a struct).
#    Actually the code is trying to store corner radii points, so it should be Vec<Corners<Point<Pixels>>>.
#    Replace Anchor<Point<Pixels>> with Corners<Point<Pixels>>.
sed -i '' 's/Anchor<Point<Pixels>>/Corners<Point<Pixels>>/g' crates/ui/src/input/element.rs

# 4. Ensure 'use gpui::Corners;' is present in input/element.rs
if ! grep -q "use gpui::Corners" crates/ui/src/input/element.rs; then
    sed -i '' '1s/^/use gpui::Corners;\n/' crates/ui/src/input/element.rs
fi

# 5. Remove any lingering 'use crate::Anchor;' statements (they cause errors)
find crates -name "*.rs" -exec sed -i '' '/use crate::Anchor;/d' {} \;

echo "=== All fixes applied. Now run 'cargo build' ==="
