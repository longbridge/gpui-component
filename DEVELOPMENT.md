## Contributing Guide

Contributions are welcome, if you find some bugs or have some ideas, please open an issue or submit a pull request.

Please ensure that you are using clean code, following the coding style and code organization in existing code, and make sure all the tests pass.

Please submit one PR that does one thing, this is important, and helps us to review your code more easily and push to merge fast.

## Development and Testing

There are a lot of UI test cases in the `crates/story` folder, if you change the existing features you can run the tests to make sure they are working.

### Run story

Use `cargo run` to run the complete story examples to display them all in a gallery of GPUI components.

```bash
cargo run
```

### Run single example

There is also available some split examples, run `cargo run --example` to see the available examples.

```bash
cargo run --example table
```
