//! Native content state; NSH derives the value codecs and JS/TS declarations.
#[path = "content/charts.rs"]
mod charts;
#[path = "content/diff.rs"]
mod diff;
#[path = "content/plot.rs"]
mod plot;
#[path = "content/speech.rs"]
mod speech;
#[path = "content/text.rs"]
mod text;
#[path = "content/text_slots.rs"]
mod text_slots;

pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    text::register_typed(registry);
    diff::register_typed(registry);
    charts::register_typed(registry);
    plot::register_typed(registry);
    speech::register_typed(registry);
}
