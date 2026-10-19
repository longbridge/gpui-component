# NSH example

A GPUI Kit window embedding an NSH app (`app/App.tsx`): a gallery of every GPUI Base control `gpui-nsh` registers, in
five pages (Buttons, Choices, Fields, Pickers, Content). Layout and text are NSH's; the controls are GPUI Base's, and the
app's class strings are their whole look, in GPUI Base's color tokens. The line at the bottom shows the last event a
control sent. The toolbar's button switches GPUI Kit between light and dark, and the app follows.

```sh
make example-nsh         # builds app/ with the nsh CLI, then runs the example
cargo run -p example-nsh # runs it with the app already built
```

`nsh` is the Native Shell CLI ([installation](https://nsh.dev/docs/installation)).

Both run from the repository root. Its dependencies are its own (`Cargo.toml`), not the workspace's.
