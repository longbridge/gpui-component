//! Styled Component controls backed by NSH's typed native registry.
#[path = "display/typed.rs"]
mod typed;
pub(crate) use typed::{ComponentSize, register as register_typed, size};
