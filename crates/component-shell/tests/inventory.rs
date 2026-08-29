use std::collections::BTreeSet;

use serde_json::Value;

const INVENTORY: &str = include_str!("../component-inventory.json");
const UI_LIB: &str = include_str!("../../ui/src/lib.rs");
const STORIES_MOD: &str = include_str!("../../story/src/stories/mod.rs");

#[test]
fn every_public_component_and_story_is_accounted_for() {
    let inventory = Inventory::load();

    let expected = public_ui_modules(UI_LIB)
        .into_iter()
        .map(|name| ("ui".to_owned(), name.to_owned()))
        .chain(
            public_story_modules(STORIES_MOD)
                .into_iter()
                .map(|name| ("story".to_owned(), name.to_owned())),
        )
        .collect::<BTreeSet<_>>();

    assert_eq!(
        inventory.entries.len(),
        inventory.sources.len(),
        "duplicate inventory item"
    );
    assert_eq!(
        inventory.sources, expected,
        "inventory drifted from public exports"
    );
}

#[test]
fn inventory_entries_have_a_registration_or_a_reason() {
    for entry in Inventory::load().entries {
        match entry.classification.as_str() {
            "component" | "platform" => assert!(
                entry.registration.is_some_and(|value| !value.is_empty()),
                "{}:{} needs a descriptor name",
                entry.source,
                entry.name
            ),
            "infrastructure" => assert!(
                entry.explanation.is_some_and(|value| !value.is_empty()),
                "{}:{} needs an infrastructure explanation",
                entry.source,
                entry.name
            ),
            other => panic!(
                "{}:{} has unknown classification {other}",
                entry.source, entry.name
            ),
        }
    }
}

struct Inventory {
    entries: Vec<Entry>,
    sources: BTreeSet<(String, String)>,
}

struct Entry {
    source: String,
    name: String,
    classification: String,
    registration: Option<String>,
    explanation: Option<String>,
}

impl Inventory {
    fn load() -> Self {
        let document: Value = serde_json::from_str(INVENTORY).expect("valid component inventory");
        let entries = document["items"]
            .as_array()
            .expect("inventory items array")
            .iter()
            .map(|item| Entry {
                source: item["source"]
                    .as_str()
                    .expect("inventory item source")
                    .to_owned(),
                name: item["name"]
                    .as_str()
                    .expect("inventory item name")
                    .to_owned(),
                classification: item["classification"]
                    .as_str()
                    .expect("inventory item classification")
                    .to_owned(),
                registration: item["registration"].as_str().map(ToOwned::to_owned),
                explanation: item["explanation"].as_str().map(ToOwned::to_owned),
            })
            .collect::<Vec<_>>();
        let sources = entries
            .iter()
            .map(|entry| (entry.source.clone(), entry.name.clone()))
            .collect();

        Self { entries, sources }
    }
}

fn public_ui_modules(source: &str) -> BTreeSet<&str> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub mod "))
        .filter_map(module_name)
        .collect()
}

fn public_story_modules(source: &str) -> BTreeSet<&str> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub use "))
        .filter_map(|line| line.split_once("::"))
        .map(|(module, _)| module.trim_end_matches("_story"))
        .collect()
}

fn module_name(line: &str) -> Option<&str> {
    line.split([' ', '{', ';'])
        .next()
        .filter(|name| !name.is_empty())
}
