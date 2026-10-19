# NSH example

A GPUI Kit window embedding an NSH app (`app/App.tsx`). The app lays out with NSH's views and text and uses GPUI Base's
`Tabs`, `Input`, `Select`, `Switch`, `Checkbox`, `RadioGroup`, `Slider`, `Progress` and `Button`, whose events update
its state. GPUI Base draws nothing of its own, so the app's class strings are the controls' whole look, in GPUI Base's
color tokens. The toolbar's button switches GPUI Kit between light and dark, and the app follows.

```sh
make example-nsh         # builds app/ with the nsh CLI, then runs the example
cargo run -p example-nsh # runs it with the app already built
```

`nsh` is the Native Shell CLI ([installation](https://nsh.dev/docs/installation)).

Both run from the repository root. Its dependencies are its own (`Cargo.toml`), not the workspace's.
