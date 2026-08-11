use std::{
    collections::{BTreeSet, HashSet},
    env, fs,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "gpui-components.json";
const BASE_DEPENDENCY: &str = "gpui-component-base";
const EMBEDDED_BUTTON_ITEM: &str = include_str!("../registry/ui/button.json");
const EMBEDDED_CHECKBOX_ITEM: &str = include_str!("../registry/ui/checkbox.json");
const EMBEDDED_SWITCH_ITEM: &str = include_str!("../registry/ui/switch.json");
const EMBEDDED_RADIO_ITEM: &str = include_str!("../registry/ui/radio.json");
const EMBEDDED_BUTTON_SOURCE: &str = include_str!("../registry/ui/button.rs");
const EMBEDDED_CHECKBOX_SOURCE: &str = include_str!("../registry/ui/checkbox.rs");
const EMBEDDED_SWITCH_SOURCE: &str = include_str!("../registry/ui/switch.rs");
const EMBEDDED_RADIO_SOURCE: &str = include_str!("../registry/ui/radio.rs");

#[derive(Debug, Deserialize, Serialize)]
struct ProjectConfig {
    #[serde(rename = "$schema")]
    schema: String,
    style: String,
    ui: String,
    theme: String,
    icons: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            schema: "https://gpui.rs/components.schema.json".into(),
            style: "default".into(),
            ui: "src/ui".into(),
            theme: "src/theme.rs".into(),
            icons: "lucide".into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryItem {
    name: String,
    #[serde(rename = "type")]
    item_type: String,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    registry_dependencies: Vec<String>,
    files: Vec<RegistryFile>,
}

#[derive(Debug, Deserialize)]
struct RegistryFile {
    path: String,
    #[serde(rename = "type")]
    file_type: String,
    target: String,
}

pub fn run(args: impl IntoIterator<Item = String>, cwd: &Path) -> Result<String, String> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("init") if args.next().is_none() => init(cwd),
        Some("add") => {
            let names = args.collect::<Vec<_>>();
            if names.is_empty() {
                return Err("usage: gpui-component add <component>...".into());
            }
            add(cwd, &names)
        }
        Some("help") | Some("--help") | Some("-h") | None => Ok(help().into()),
        Some(command) => Err(format!("unknown command `{command}`\n\n{}", help())),
    }
}

fn help() -> &'static str {
    "gpui-component\n\nUSAGE:\n  gpui-component init\n  gpui-component add <component>..."
}

fn init(root: &Path) -> Result<String, String> {
    require_cargo_project(root)?;
    let config_path = root.join(CONFIG_FILE);
    let config = if config_path.exists() {
        read_config(root)?
    } else {
        let config = ProjectConfig::default();
        write_json(&config_path, &config)?;
        config
    };

    let ui_dir = root.join(&config.ui);
    fs::create_dir_all(&ui_dir).map_err(|error| format!("create {}: {error}", ui_dir.display()))?;
    write_if_missing(
        &ui_dir.join("mod.rs"),
        "//! Application-owned UI components installed by gpui-component.\n",
    )?;
    let theme_path = root.join(&config.theme);
    if let Some(parent) = theme_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    write_if_missing(
        &theme_path,
        "//! Semantic theme tokens for application-owned components.\n\npub use gpui_component_base::{\n    ColorTokens, RadiusTokens, SemanticThemeTokens, ShadowTokens, SpacingTokens,\n    TextStyleToken, TypographyTokens,\n};\n",
    )?;
    ensure_dependency(&root.join("Cargo.toml"), BASE_DEPENDENCY)?;

    Ok("Initialized GPUI Components\nCreated gpui-components.json, src/ui/mod.rs, and src/theme.rs".into())
}

fn add(root: &Path, names: &[String]) -> Result<String, String> {
    require_cargo_project(root)?;
    let config = read_config(root)?;
    let registry = registry_root();
    let mut visiting = HashSet::new();
    let mut installed = BTreeSet::new();
    let mut ordered = Vec::new();
    for name in names {
        resolve(
            name,
            registry.as_deref(),
            &mut visiting,
            &mut installed,
            &mut ordered,
        )?;
    }

    let mut installed_targets = Vec::new();
    let mut created = Vec::new();
    for (item, source_root) in ordered {
        for file in item.files {
            validate_registry_file(&file)?;
            let relative_target = configured_target(&file.target, &config.ui);
            let target = root.join(&relative_target);
            let source = read_registry_source(source_root.as_deref(), &item.name, &file.path)?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("create {}: {error}", parent.display()))?;
            }
            if !target.exists() {
                fs::write(&target, source)
                    .map_err(|error| format!("write {}: {error}", target.display()))?;
                created.push(relative_target.clone());
            }
            installed_targets.push(relative_target);
        }
        for dependency in item.dependencies {
            ensure_dependency(&root.join("Cargo.toml"), &dependency)?;
        }
    }

    update_ui_module(root, &config.ui, &installed_targets)?;
    let summary = if created.is_empty() {
        "No files created; existing application-owned sources were preserved".into()
    } else {
        format!("Created:\n  {}", created.join("\n  "))
    };
    Ok(format!("Added {}\n{summary}", names.join(", ")))
}

fn configured_target(registry_target: &str, configured_ui: &str) -> String {
    Path::new(registry_target)
        .strip_prefix("src/ui")
        .ok()
        .map(|suffix| Path::new(configured_ui).join(suffix))
        .unwrap_or_else(|| PathBuf::from(registry_target))
        .to_string_lossy()
        .into_owned()
}

fn resolve(
    name: &str,
    registry: Option<&Path>,
    visiting: &mut HashSet<String>,
    installed: &mut BTreeSet<String>,
    ordered: &mut Vec<(RegistryItem, Option<PathBuf>)>,
) -> Result<(), String> {
    if installed.contains(name) {
        return Ok(());
    }
    if !visiting.insert(name.to_string()) {
        return Err(format!("registry dependency cycle at `{name}`"));
    }
    let (item, source_root) = read_registry_item(registry, name)?;
    if item.name != name {
        return Err(format!(
            "registry item `{name}` declares name `{}`",
            item.name
        ));
    }
    if !matches!(
        item.item_type.as_str(),
        "registry:ui" | "registry:block" | "registry:theme" | "registry:lib"
    ) {
        return Err(format!(
            "unsupported registry item type `{}`",
            item.item_type
        ));
    }
    for dependency in &item.registry_dependencies {
        resolve(dependency, registry, visiting, installed, ordered)?;
    }
    visiting.remove(name);
    installed.insert(name.to_string());
    ordered.push((item, source_root));
    Ok(())
}

fn read_registry_item(
    registry: Option<&Path>,
    name: &str,
) -> Result<(RegistryItem, Option<PathBuf>), String> {
    if let Some(registry) = registry {
        for category in ["ui", "blocks", "themes", "lib"] {
            let path = registry.join(category).join(format!("{name}.json"));
            if path.is_file() {
                let text = fs::read_to_string(&path)
                    .map_err(|error| format!("read {}: {error}", path.display()))?;
                let item = serde_json::from_str(&text)
                    .map_err(|error| format!("parse {}: {error}", path.display()))?;
                return Ok((item, Some(registry.to_path_buf())));
            }
        }
    }
    if matches!(name, "button" | "checkbox" | "switch" | "radio") {
        let item_json = match name {
            "button" => EMBEDDED_BUTTON_ITEM,
            "checkbox" => EMBEDDED_CHECKBOX_ITEM,
            "switch" => EMBEDDED_SWITCH_ITEM,
            "radio" => EMBEDDED_RADIO_ITEM,
            _ => unreachable!(),
        };
        let item = serde_json::from_str(item_json)
            .map_err(|error| format!("parse built-in {name} registry item: {error}"))?;
        return Ok((item, None));
    }
    Err(format!("component `{name}` was not found in the registry"))
}

fn read_registry_source(registry: Option<&Path>, item: &str, path: &str) -> Result<String, String> {
    if let Some(registry) = registry {
        let source_path = registry.join(path);
        return fs::read_to_string(&source_path)
            .map_err(|error| format!("read registry source {}: {error}", source_path.display()));
    }
    let source = match (item, path) {
        ("button", "ui/button.rs") => Some(EMBEDDED_BUTTON_SOURCE),
        ("checkbox", "ui/checkbox.rs") => Some(EMBEDDED_CHECKBOX_SOURCE),
        ("switch", "ui/switch.rs") => Some(EMBEDDED_SWITCH_SOURCE),
        ("radio", "ui/radio.rs") => Some(EMBEDDED_RADIO_SOURCE),
        _ => None,
    };
    if let Some(source) = source {
        return Ok(source.into());
    }
    Err(format!("built-in registry source `{path}` is unavailable"))
}

fn registry_root() -> Option<PathBuf> {
    env::var_os("GPUI_COMPONENT_REGISTRY")
        .map(PathBuf::from)
        .or_else(|| {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../registry");
            path.is_dir().then_some(path)
        })
}

fn validate_registry_file(file: &RegistryFile) -> Result<(), String> {
    if !matches!(
        file.file_type.as_str(),
        "registry:ui" | "registry:block" | "registry:theme" | "registry:lib"
    ) {
        return Err(format!(
            "unsupported registry file type `{}`",
            file.file_type
        ));
    }
    for path in [&file.path, &file.target] {
        let path = Path::new(path);
        if path.is_absolute()
            || path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(format!(
                "registry path `{}` must stay inside its root",
                path.display()
            ));
        }
    }
    Ok(())
}

fn update_ui_module(root: &Path, configured_ui: &str, created: &[String]) -> Result<(), String> {
    let module_path = root.join(configured_ui).join("mod.rs");
    let mut contents = fs::read_to_string(&module_path).unwrap_or_default();
    for target in created {
        let target = Path::new(target);
        if target.parent() != root.join(configured_ui).strip_prefix(root).ok() {
            continue;
        }
        let Some(stem) = target.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let declaration = format!("mod {stem};\npub use {stem}::*;\n");
        if !contents.contains(&format!("mod {stem};")) {
            if !contents.ends_with('\n') {
                contents.push('\n');
            }
            contents.push_str(&declaration);
        }
    }
    fs::write(&module_path, contents)
        .map_err(|error| format!("write {}: {error}", module_path.display()))
}

fn read_config(root: &Path) -> Result<ProjectConfig, String> {
    let path = root.join(CONFIG_FILE);
    let text = fs::read_to_string(&path).map_err(|_| {
        format!(
            "{} is missing; run `gpui-component init` first",
            CONFIG_FILE
        )
    })?;
    let config: ProjectConfig = serde_json::from_str(&text)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    validate_project_path("ui", &config.ui)?;
    validate_project_path("theme", &config.theme)?;
    Ok(config)
}

fn validate_project_path(field: &str, value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(format!(
            "configuration field `{field}` must be a path inside the Cargo project"
        ));
    }
    Ok(())
}

fn require_cargo_project(root: &Path) -> Result<(), String> {
    if root.join("Cargo.toml").is_file() {
        Ok(())
    } else {
        Err(format!("{} is not a Cargo project", root.display()))
    }
}

fn ensure_dependency(manifest: &Path, dependency: &str) -> Result<(), String> {
    let mut text = fs::read_to_string(manifest)
        .map_err(|error| format!("read {}: {error}", manifest.display()))?;
    if text.lines().any(|line| {
        let Some((key, _)) = line.split_once('=') else {
            return false;
        };
        let key = key.trim();
        key == dependency || key.starts_with(&format!("{dependency}."))
    }) {
        return Ok(());
    }
    let entry = format!("{dependency} = \"{}\"\n", env!("CARGO_PKG_VERSION"));
    if let Some(header) = text.find("[dependencies]") {
        let insertion = header + "[dependencies]".len();
        text.insert_str(insertion, &format!("\n{entry}"));
    } else {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&format!("\n[dependencies]\n{entry}"));
    }
    fs::write(manifest, text).map_err(|error| format!("write {}: {error}", manifest.display()))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let mut text = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    text.push('\n');
    fs::write(path, text).map_err(|error| format!("write {}: {error}", path.display()))
}

fn write_if_missing(path: &Path, contents: &str) -> Result<(), String> {
    if !path.exists() {
        fs::write(path, contents).map_err(|error| format!("write {}: {error}", path.display()))?;
    }
    Ok(())
}
