
wr:
    watchexec -w ./wr.sh --clear -r "sh ./wr.sh"
test:
    cargo nextest run

# -----------------------------------------------------------------------
# GPUI Component story gallery & documentation
# -----------------------------------------------------------------------
# Launch the interactive story gallery (native desktop app)
story:
    cargo run -p gpui-component-story

# Start the VitePress documentation website locally
docs:
    cd docs && bun install && bun run dev
