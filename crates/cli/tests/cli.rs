use std::{fs, path::Path, process::Command};

fn project(root: &Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
}

fn cli(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_gpui-component"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}

fn cli_with_registry(root: &Path, registry: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_gpui-component"))
        .current_dir(root)
        .env("GPUI_COMPONENT_REGISTRY", registry)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn init_scaffolds_an_application_and_is_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    project(temp.path());

    assert!(cli(temp.path(), &["init"]).status.success());
    let theme = fs::read_to_string(temp.path().join("src/theme.rs")).unwrap();
    assert!(theme.contains("SemanticThemeTokens"));
    assert!(theme.contains("gpui_component_base"));
    fs::write(temp.path().join("src/theme.rs"), "// application edit\n").unwrap();
    assert!(cli(temp.path(), &["init"]).status.success());

    assert!(temp.path().join("gpui-components.json").is_file());
    assert!(temp.path().join("src/ui/mod.rs").is_file());
    assert_eq!(
        fs::read_to_string(temp.path().join("src/theme.rs")).unwrap(),
        "// application edit\n"
    );
    let manifest = fs::read_to_string(temp.path().join("Cargo.toml")).unwrap();
    assert_eq!(manifest.matches("gpui-component-base").count(), 1);
}

#[test]
fn add_button_installs_editable_source_and_updates_module() {
    let temp = tempfile::tempdir().unwrap();
    project(temp.path());
    assert!(cli(temp.path(), &["init"]).status.success());
    assert!(cli(temp.path(), &["add", "button"]).status.success());

    let button = temp.path().join("src/ui/button.rs");
    let source = fs::read_to_string(&button).unwrap();
    assert!(source.contains("pub struct Button"));
    assert!(source.contains("gpui_component_base as base"));
    let module = fs::read_to_string(temp.path().join("src/ui/mod.rs")).unwrap();
    assert!(module.contains("mod button;"));
    assert!(module.contains("pub use button::*;"));

    fs::write(&button, "// owned by the application\n").unwrap();
    assert!(cli(temp.path(), &["add", "button"]).status.success());
    assert_eq!(
        fs::read_to_string(button).unwrap(),
        "// owned by the application\n"
    );
}

#[test]
fn add_requires_initialization() {
    let temp = tempfile::tempdir().unwrap();
    project(temp.path());
    let output = cli(temp.path(), &["add", "button"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("run `gpui-component init` first"));
}

#[test]
fn add_resolves_registry_dependencies_before_the_requested_item() {
    let temp = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    project(temp.path());
    assert!(cli(temp.path(), &["init"]).status.success());

    fs::create_dir_all(registry.path().join("ui")).unwrap();
    fs::create_dir_all(registry.path().join("lib")).unwrap();
    fs::write(
        registry.path().join("ui/button.json"),
        r#"{
          "name":"button","type":"registry:ui","dependencies":["gpui-component-base"],
          "registryDependencies":["utils"],
          "files":[{"path":"ui/button.rs","type":"registry:ui","target":"src/ui/button.rs"}]
        }"#,
    )
    .unwrap();
    fs::write(
        registry.path().join("lib/utils.json"),
        r#"{
          "name":"utils","type":"registry:lib","dependencies":[],"registryDependencies":[],
          "files":[{"path":"lib/utils.rs","type":"registry:lib","target":"src/ui/utils.rs"}]
        }"#,
    )
    .unwrap();
    fs::write(registry.path().join("ui/button.rs"), "pub struct Button;\n").unwrap();
    fs::write(
        registry.path().join("lib/utils.rs"),
        "pub fn utility() {}\n",
    )
    .unwrap();

    let output = cli_with_registry(temp.path(), registry.path(), &["add", "button"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(temp.path().join("src/ui/utils.rs").is_file());
    assert!(temp.path().join("src/ui/button.rs").is_file());
    let module = fs::read_to_string(temp.path().join("src/ui/mod.rs")).unwrap();
    assert!(module.find("mod utils;").unwrap() < module.find("mod button;").unwrap());
}

#[test]
fn add_rejects_configured_paths_outside_the_project() {
    let temp = tempfile::tempdir().unwrap();
    project(temp.path());
    assert!(cli(temp.path(), &["init"]).status.success());
    let config = fs::read_to_string(temp.path().join("gpui-components.json"))
        .unwrap()
        .replace("src/ui", "../outside");
    fs::write(temp.path().join("gpui-components.json"), config).unwrap();

    let output = cli(temp.path(), &["add", "button"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("inside the Cargo project"));
    assert!(
        !temp
            .path()
            .parent()
            .unwrap()
            .join("outside/button.rs")
            .exists()
    );
}

#[test]
fn init_rejects_an_invalid_existing_config_before_scaffolding() {
    let temp = tempfile::tempdir().unwrap();
    project(temp.path());
    fs::write(temp.path().join("gpui-components.json"), "{ not-json }\n").unwrap();

    let output = cli(temp.path(), &["init"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("parse"));
    assert!(!temp.path().join("src/ui").exists());
}

#[test]
fn installed_binary_fallback_adds_checkbox_switch_and_radio_without_overwriting() {
    let temp = tempfile::tempdir().unwrap();
    let missing_registry = temp.path().join("missing-registry");
    project(temp.path());
    assert!(cli(temp.path(), &["init"]).status.success());

    let output = cli_with_registry(
        temp.path(),
        &missing_registry,
        &["add", "checkbox", "switch", "radio"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    for name in ["checkbox", "switch", "radio"] {
        let path = temp.path().join(format!("src/ui/{name}.rs"));
        let source = fs::read_to_string(&path).unwrap();
        assert!(source.contains("gpui_component_base"));
        assert!(source.contains(&format!("pub struct {}", uppercase_first(name))));
        assert!(source.contains(if name == "switch" {
            "pub fn on_toggle"
        } else {
            "pub fn on_change"
        }));
    }

    let checkbox = temp.path().join("src/ui/checkbox.rs");
    fs::write(&checkbox, "// application-owned edit\n").unwrap();
    assert!(
        cli_with_registry(temp.path(), &missing_registry, &["add", "checkbox"])
            .status
            .success()
    );
    assert_eq!(
        fs::read_to_string(checkbox).unwrap(),
        "// application-owned edit\n"
    );
}

#[test]
fn source_checkout_registry_adds_checkbox_switch_and_radio_presentations() {
    let temp = tempfile::tempdir().unwrap();
    project(temp.path());
    assert!(cli(temp.path(), &["init"]).status.success());
    let output = cli(temp.path(), &["add", "checkbox", "switch", "radio"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let checkbox = fs::read_to_string(temp.path().join("src/ui/checkbox.rs")).unwrap();
    let switch = fs::read_to_string(temp.path().join("src/ui/switch.rs")).unwrap();
    let radio = fs::read_to_string(temp.path().join("src/ui/radio.rs")).unwrap();
    assert!(checkbox.contains("base::Checkbox::new"));
    assert!(checkbox.contains("pub fn on_change"));
    assert!(switch.contains("base::Switch::new"));
    assert!(switch.contains("pub fn on_toggle"));
    assert!(radio.contains("base::Radio::new"));
    assert!(radio.contains("pub fn on_change"));
}

fn uppercase_first(value: &str) -> String {
    let mut chars = value.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

#[test]
fn embedded_registry_resources_match_the_canonical_registry() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let canonical_root = crate_root.join("../../registry/ui");
    let embedded_root = crate_root.join("registry/ui");

    for name in ["button", "checkbox", "switch", "radio"] {
        for extension in ["json", "rs"] {
            let file = format!("{name}.{extension}");
            let canonical = fs::read(canonical_root.join(&file)).unwrap();
            let embedded = fs::read(embedded_root.join(&file)).unwrap();
            assert_eq!(
                embedded, canonical,
                "embedded registry resource {file} drifted"
            );
            if extension == "json" {
                let value: serde_json::Value = serde_json::from_slice(&embedded).unwrap();
                assert_eq!(value["name"], name);
                assert_eq!(value["type"], "registry:ui");
            }
        }
    }
}
