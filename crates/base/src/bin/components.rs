#[path = "../../examples/showcase/mod.rs"]
mod showcase;

fn main() {
    let component = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "overview".to_string());

    showcase::run_native(&component);
}
