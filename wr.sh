# Remove the extra arguments from the actions! macro
sed -i '' 's/actions!(number_input, \[Increment, Decrement\], Window, Pixels);/actions!(number_input, [Increment, Decrement]);/' crates/ui/src/input/number_input.rs

# Remove stray 'Window, Pixels' from the import block
sed -i '' '/^use gpui::/,/)/ {
    s/, Window//g
    s/, Pixels//g
}' crates/ui/src/input/number_input.rs
