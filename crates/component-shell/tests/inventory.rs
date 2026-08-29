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
            "component" | "platform" => match entry.registration.as_ref() {
                Some(Registration::Registered {
                    descriptor,
                    exports,
                }) => {
                    assert!(
                        !descriptor.is_empty(),
                        "registered descriptor cannot be empty"
                    );
                    assert!(!exports.is_empty(), "registered exports cannot be empty");
                }
                Some(Registration::Deferred {
                    target,
                    category,
                    reason,
                }) => {
                    assert!(!target.is_empty(), "deferred target cannot be empty");
                    assert!(!category.is_empty(), "deferred category cannot be empty");
                    assert!(!reason.is_empty(), "deferred reason cannot be empty");
                }
                None => panic!(
                    "{}:{} needs an explicit registered or deferred status",
                    entry.source, entry.name
                ),
            },
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

#[test]
fn registered_inventory_matches_the_frozen_component_catalog() {
    let inventory = Inventory::load();
    let frozen = gpui_component_shell::components().expect("frozen component catalog");
    let actual = frozen
        .descriptors()
        .map(|descriptor| {
            (
                descriptor.name.to_owned(),
                descriptor
                    .constructors
                    .iter()
                    .map(|constructor| constructor.export.to_owned())
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut inventoried_descriptors = BTreeSet::new();
    let mut inventoried_exports = BTreeSet::new();
    for entry in inventory.entries {
        let Some(Registration::Registered {
            descriptor,
            exports,
        }) = entry.registration
        else {
            continue;
        };
        let actual_exports = actual.get(&descriptor).unwrap_or_else(|| {
            panic!(
                "{}:{} claims missing descriptor `{descriptor}`",
                entry.source, entry.name
            )
        });
        let claimed_exports = exports.into_iter().collect::<BTreeSet<_>>();
        assert_eq!(
            &claimed_exports, actual_exports,
            "{}:{} has stale exports for `{descriptor}`",
            entry.source, entry.name
        );
        inventoried_descriptors.insert(descriptor);
        inventoried_exports.extend(claimed_exports);
    }

    let actual_descriptors = actual.keys().cloned().collect::<BTreeSet<_>>();
    let actual_exports = actual.values().flatten().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        inventoried_descriptors, actual_descriptors,
        "registered descriptors must all be inventoried"
    );
    assert_eq!(
        inventoried_exports, actual_exports,
        "registered constructor exports must all be inventoried"
    );
}

struct Inventory {
    entries: Vec<Entry>,
    sources: BTreeSet<(String, String)>,
}

struct Entry {
    source: String,
    name: String,
    classification: String,
    registration: Option<Registration>,
    explanation: Option<String>,
}

enum Registration {
    Registered {
        descriptor: String,
        exports: Vec<String>,
    },
    Deferred {
        target: String,
        category: String,
        reason: String,
    },
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
                registration: item.get("registration").map(|registration| {
                    let status = registration["status"]
                        .as_str()
                        .expect("registration status");
                    match status {
                        "registered" => Registration::Registered {
                            descriptor: registration["descriptor"]
                                .as_str()
                                .expect("registered descriptor")
                                .to_owned(),
                            exports: registration["exports"]
                                .as_array()
                                .expect("registered exports")
                                .iter()
                                .map(|export| {
                                    export.as_str().expect("registered export").to_owned()
                                })
                                .collect(),
                        },
                        "deferred" => Registration::Deferred {
                            target: registration["target"]
                                .as_str()
                                .expect("deferred target")
                                .to_owned(),
                            category: registration["category"]
                                .as_str()
                                .expect("deferred category")
                                .to_owned(),
                            reason: registration["reason"]
                                .as_str()
                                .expect("deferred reason")
                                .to_owned(),
                        },
                        other => panic!("unknown registration status {other}"),
                    }
                }),
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
