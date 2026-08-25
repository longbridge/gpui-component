//! Who a plugin is, what it may do, and where its data lives.
//!
//! A host that runs one application from a directory needs none of this: the
//! command line names the directory, the grant is decided by the act of typing
//! the command, and storage is keyed by the path. A host that runs *several*
//! applications cannot do any of that, because the three questions become
//! per plugin — identity, permission, storage — and all three have to be
//! answerable **before** the plugin's code runs.
//!
//! That is the whole reason a manifest exists, and it is why the manifest has
//! exactly five fields (design doc §18.1): `id`, `name`, `version`, `entry`,
//! `capabilities`. Commands, panels, keybindings, settings and themes are
//! registered from script instead of being declared here a second time —
//! *capabilities are permission, contributions are behavior*. A permission has
//! to be shown to a user and approved before any code runs, so it belongs in
//! data; a contribution is code, so it belongs in code. Declaring contributions
//! in both places would create a class of bug (manifest and script disagreeing)
//! while producing no information the script did not already carry.
//!
//! Two consequences run through this module:
//!
//! - **Discovery executes nothing.** [`PluginManager::discover`] reads
//!   manifests and stops. A host with thirty installed plugins lists thirty
//!   names, versions and permission sets without starting thirty programs.
//!   Only [`PluginManager::load`] evaluates script.
//! - **The API version is not in the manifest.** A plugin states its
//!   requirement in script with `gpui.require_api("1.0")` (§18.1, §23.3), so
//!   the manifest has nothing to say about it and this module publishes the
//!   number the runtime implements instead: [`API_VERSION`].

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{Result, bail};
use gpui::{App, AppContext as _, Entity, Window};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    capability::{Capabilities, ExecuteGrant},
    engine::ShellRuntime,
    policy::Policy,
    scope::{self, ScopePhase},
    view::ScriptView,
};

/// The script API version this runtime implements.
///
/// A plugin does not declare it in the manifest — it states it in script,
/// `gpui.require_api("1.0")` (§18.1, §23.3) — so the only thing the host has to
/// publish is its own number, and it has to be published by the module that
/// defines what a plugin is. It cannot live in the manifest (five fields, no
/// sixth), and it cannot live below the `engine/` seam: §23.1 requires one
/// version to mean the same behaviors under either engine, and two copies below
/// the seam would be two numbers that can drift.
///
/// The number itself is owned by [`crate::plugin_api`], which also implements
/// the comparison a `require_api` binding performs
/// ([`crate::plugin_api::check`], with the message a mismatch has to print).
/// This is an alias, not a second definition, so the plugin model can be read
/// without leaving this module while there is still exactly one number.
pub const API_VERSION: &str = crate::plugin_api::VERSION;

/// The file a plugin directory is recognized by.
///
/// §18.1 shows the manifest's content but never names the file. `plugin.json`
/// is chosen here because the directory is the plugin: nothing else in it needs
/// the name, and `manifest.json` and `package.json` are both already spoken for
/// by other ecosystems whose schemas would be mistaken for this one.
pub const MANIFEST_FILE: &str = "plugin.json";

/// The entry file the engine can currently load. See [`PluginManager::load`].
const ENGINE_ENTRY: &str = "main.js";

/// The JSON Schema for a manifest, for editor validation.
///
/// §18.1 keeps the schema worth generating but small enough to read: five
/// fields, one nested permission object. `crates/ui/src/theme/schema.rs` is the
/// precedent for generating rather than hand-writing it — the schema and the
/// parser then cannot disagree, because both come from the same type.
pub fn manifest_schema() -> serde_json::Value {
    schemars::schema_for!(ManifestFile).to_value()
}

/// A plugin's identity and the permissions it asks for.
///
/// Cloneable and inert: holding one runs nothing, which is what lets a host
/// list, sort and display installed plugins — or show a permission sheet — with
/// no plugin code loaded.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginManifest {
    id: String,
    name: String,
    version: String,
    entry: String,
    capabilities: CapabilitiesFile,
}

impl PluginManifest {
    /// Parses manifest source.
    ///
    /// Every failure names the field and says what was expected, because this
    /// is the first thing an author of a plugin meets and it is usually the
    /// only diagnostic they get: nothing has run yet, so there is no stack
    /// trace to fall back on.
    pub fn parse(source: &str) -> Result<Self, ManifestError> {
        Self::parse_inner(source).map_err(ManifestError::from)
    }

    /// Reads `<directory>/plugin.json`.
    ///
    /// The path is carried into the error, because a host reads many manifests
    /// in one pass and "missing field `id`" is not actionable without it.
    pub fn read(directory: &Path) -> Result<Self, ManifestError> {
        let path = directory.join(MANIFEST_FILE);
        let source = std::fs::read_to_string(&path).map_err(|error| ManifestError {
            path: Some(path.clone()),
            problem: ManifestProblem::Unreadable(error.to_string()),
        })?;

        Self::parse_inner(&source).map_err(|problem| ManifestError {
            path: Some(path),
            problem,
        })
    }

    fn parse_inner(source: &str) -> Result<Self, ManifestProblem> {
        let value: serde_json::Value = serde_json::from_str(source)
            .map_err(|error| ManifestProblem::NotJson(error.to_string()))?;

        let serde_json::Value::Object(fields) = value else {
            return Err(ManifestProblem::NotAnObject(json_type_of(&value)));
        };

        // Unknown fields are rejected before missing ones, so that a typo
        // reports itself rather than reporting the field it was meant to be.
        // This is the case the design is most exposed to: `"capabilites"` is
        // optional-looking, and accepting it would hand the plugin an empty
        // grant while its author believes it was granted everything listed.
        for field in fields.keys() {
            if !FIELDS.contains(&field.as_str()) {
                return Err(ManifestProblem::UnknownField {
                    field: field.clone(),
                    suggestion: nearest_field(field),
                });
            }
        }

        let id = string_field(&fields, "id")?;
        validate_id(&id)?;
        let name = string_field(&fields, "name")?;
        let version = string_field(&fields, "version")?;
        validate_version(&version)?;
        let entry = string_field(&fields, "entry")?;
        validate_entry(&entry)?;

        // `capabilities` is the one field that may be omitted, and the only one
        // whose default cannot be an accident: absent means the empty grant
        // (§5.7), which is also what an explicit `{}` means. Requiring the key
        // would add a line that says "nothing" to every plugin that wants
        // nothing.
        let capabilities = match fields.get("capabilities") {
            None | Some(serde_json::Value::Null) => CapabilitiesFile::default(),
            Some(value) => serde_json::from_value(value.clone())
                .map_err(|error| ManifestProblem::Capabilities(error.to_string()))?,
        };
        capabilities.validate_placeholders()?;

        Ok(Self {
            id,
            name,
            version,
            entry,
            capabilities,
        })
    }

    /// The namespace this plugin owns.
    ///
    /// It is the panel-name prefix (`script:<id>/<panel>`, §15.4), the storage
    /// key, the log field and the identity a capability approval is recorded
    /// against — which is why [`validate_id`] is as strict as it is.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The human-readable name, for menus and permission sheets.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The plugin's own version — not the API version (§23.1), which the
    /// plugin states in script.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The module evaluated at load, relative to the plugin directory.
    pub fn entry(&self) -> &str {
        &self.entry
    }

    /// The grant this manifest asks for, resolved against the two directories
    /// only the host knows.
    ///
    /// The manifest writes `${pluginDir}` and `${dataDir}` (§18.1) rather than
    /// real paths, for the same reason a plugin cannot name its own storage
    /// location: a path chosen by the plugin is a path the plugin can point
    /// anywhere. So the *shape* of the grant comes from the manifest and
    /// nowhere else, while the two directories it is anchored to come from the
    /// host and nowhere else. A relative path is anchored to the plugin
    /// directory; an absolute path is taken as written, and is the case a host
    /// policy or an approval prompt (§19.2) exists to gate.
    pub fn capabilities(&self, plugin_dir: &Path, data_dir: &Path) -> Capabilities {
        self.capabilities.grant(plugin_dir, data_dir)
    }
}

const FIELDS: [&str; 5] = ["id", "name", "version", "entry", "capabilities"];

fn string_field(
    fields: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<String, ManifestProblem> {
    match fields.get(field) {
        None | Some(serde_json::Value::Null) => Err(ManifestProblem::MissingField(field)),
        Some(serde_json::Value::String(value)) if value.trim().is_empty() => {
            Err(ManifestProblem::EmptyField(field))
        }
        Some(serde_json::Value::String(value)) => Ok(value.clone()),
        Some(other) => Err(ManifestProblem::WrongType {
            field,
            found: json_type_of(other),
        }),
    }
}

/// An `id` is used verbatim as a directory name, a panel-name prefix and a log
/// field, so the characters it may contain are decided by the strictest of
/// those three uses rather than by taste.
///
/// The two rules that are security and not style: no path separators and no
/// `..`, because `data_dir/<id>` must stay inside the data directory; and no
/// uppercase, because two ids differing only in case would be one directory on
/// a case-insensitive filesystem and two everywhere else.
fn validate_id(id: &str) -> Result<(), ManifestProblem> {
    if let Some(character) = id
        .chars()
        .find(|character| !matches!(character, 'a'..='z' | '0'..='9' | '.' | '-' | '_'))
    {
        return Err(ManifestProblem::InvalidId {
            id: id.to_owned(),
            reason: format!("`{character}` is not allowed"),
        });
    }

    let bounded_by_separator =
        |value: &str| value.starts_with(['.', '-', '_']) || value.ends_with(['.', '-', '_']);
    if bounded_by_separator(id) || id.contains("..") {
        return Err(ManifestProblem::InvalidId {
            id: id.to_owned(),
            reason: "it must begin and end with a letter or a digit".to_owned(),
        });
    }

    Ok(())
}

/// A plugin version is compared across an upgrade (§19.4: an update that adds
/// capabilities asks again), and comparison needs an agreed shape. Semver's is
/// the one §23 already uses.
fn validate_version(version: &str) -> Result<(), ManifestProblem> {
    let core = version
        .split_once(['-', '+'])
        .map_or(version, |(core, _)| core);
    let components: Vec<&str> = core.split('.').collect();
    let well_formed = components.len() == 3
        && components.iter().all(|component| {
            !component.is_empty() && component.bytes().all(|b| b.is_ascii_digit())
        });

    if well_formed {
        Ok(())
    } else {
        Err(ManifestProblem::InvalidVersion(version.to_owned()))
    }
}

/// The entry is resolved inside the plugin directory, so it must be a path that
/// cannot leave it. This is the same rule the module resolver applies to every
/// `import` (§19.1); applying it here means a manifest cannot ask for a file
/// the resolver would refuse anyway.
fn validate_entry(entry: &str) -> Result<(), ManifestProblem> {
    let path = Path::new(entry);
    let escapes = path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        || entry.starts_with('\\')
        || entry.contains(':');

    if escapes {
        Err(ManifestProblem::InvalidEntry(entry.to_owned()))
    } else {
        Ok(())
    }
}

fn json_type_of(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "an array",
        serde_json::Value::Object(_) => "an object",
    }
}

/// The closest known field name, so a typo is answered with the word the author
/// meant rather than with a list they have to scan.
fn nearest_field(unknown: &str) -> Option<&'static str> {
    FIELDS
        .iter()
        .map(|field| (edit_distance(field, unknown), *field))
        .filter(|(distance, _)| *distance <= 3)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, field)| field)
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];

    for (row, left_character) in left.chars().enumerate() {
        current[0] = row + 1;
        for (column, right_character) in right.iter().enumerate() {
            let substitution = usize::from(left_character != *right_character);
            current[column + 1] = (previous[column] + substitution)
                .min(previous[column + 1] + 1)
                .min(current[column] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.len()]
}

// -----------------------------------------------------------------------------
// The file form of the manifest
// -----------------------------------------------------------------------------

/// The serde and schemars view of a manifest.
///
/// It exists only so `schemars` can describe the file; parsing goes through
/// [`PluginManifest::parse`], which reports one field at a time. Keeping the
/// two in one module is what stops the schema from describing a file the
/// parser would reject.
// The fields are never read through this type — parsing goes field by field so
// each failure can carry its own explanation — but they are what `schemars`
// walks, and they are read by the test that keeps this type and the parser
// agreeing.
#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ManifestFile {
    /// Reverse-DNS identity, e.g. `com.example.inbox`. Also the namespace for
    /// panels, storage and capability records.
    id: String,
    /// Human-readable name, shown in menus and in the permission prompt.
    name: String,
    /// The plugin's own semantic version, e.g. `1.2.0`.
    version: String,
    /// The module evaluated at load, relative to the plugin directory.
    entry: String,
    /// What the plugin is allowed to do. Absent means nothing.
    #[serde(default)]
    capabilities: CapabilitiesFile,
}

/// The `capabilities` block, before it is anchored to real directories.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CapabilitiesFile {
    /// Filesystem and subprocess access.
    #[serde(default)]
    fs: Option<FsGrantFile>,
    /// Outbound network access, by host.
    #[serde(default)]
    network: Option<NetworkGrantFile>,
    /// Whether `gpui.store` is available.
    #[serde(default)]
    store: bool,
    /// Clipboard access.
    #[serde(default)]
    clipboard: Option<ClipboardGrantFile>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FsGrantFile {
    /// Directories that may be read. `${pluginDir}` and `${dataDir}` expand to
    /// the plugin's own directory and its storage directory.
    #[serde(default)]
    read: Vec<String>,
    /// Directories that may be written.
    #[serde(default)]
    write: Vec<String>,
    /// Commands `gpui.process.run` may start.
    #[serde(default)]
    execute: Option<ExecuteFile>,
}

/// Either an allowlist of command names, or the string `"*"`.
///
/// Unrestricted execution has to be spellable — a host that cannot express it
/// pushes its users to grant a wildcard read root instead, which is worse — but
/// it is spelled differently from an allowlist so that a permission sheet can
/// show it at the severity it deserves (§19.2).
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(untagged)]
enum ExecuteFile {
    Allowed(Vec<String>),
    Unrestricted(String),
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NetworkGrantFile {
    /// Hosts that may be reached, e.g. `api.example.com`.
    #[serde(default)]
    hosts: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ClipboardGrantFile {
    #[serde(default)]
    read: bool,
    #[serde(default)]
    write: bool,
}

const PLUGIN_DIR_PLACEHOLDER: &str = "${pluginDir}";
const DATA_DIR_PLACEHOLDER: &str = "${dataDir}";

impl CapabilitiesFile {
    fn grant(&self, plugin_dir: &Path, data_dir: &Path) -> Capabilities {
        let fs = self.fs.clone().unwrap_or_default();
        let clipboard = self.clipboard.clone().unwrap_or_default();
        let execute = match fs.execute.clone() {
            None => ExecuteGrant::Denied,
            Some(ExecuteFile::Unrestricted(_)) => ExecuteGrant::Unrestricted,
            Some(ExecuteFile::Allowed(commands)) => ExecuteGrant::Allowed(commands),
        };

        Capabilities::new()
            .read_roots(expand_all(&fs.read, plugin_dir, data_dir))
            .write_roots(expand_all(&fs.write, plugin_dir, data_dir))
            .with_execute(execute)
            .with_network_hosts(
                self.network
                    .clone()
                    .unwrap_or_default()
                    .hosts
                    .into_iter()
                    .map(|host| host.to_lowercase()),
            )
            .store(self.store)
            .clipboard_read(clipboard.read)
            .clipboard_write(clipboard.write)
    }

    /// A placeholder the host does not expand would otherwise reach
    /// [`Capabilities`] as the literal directory name `${dataDir}`, and grant
    /// access to a directory that does not exist. Catching it at parse time
    /// makes it a manifest error, which is where an author can see it.
    fn validate_placeholders(&self) -> Result<(), ManifestProblem> {
        let Some(fs) = &self.fs else {
            return Ok(());
        };

        for (field, paths) in [("read", &fs.read), ("write", &fs.write)] {
            for path in paths {
                if let Some(placeholder) = unknown_placeholder(path) {
                    return Err(ManifestProblem::UnknownPlaceholder {
                        field: format!("capabilities.fs.{field}"),
                        placeholder,
                    });
                }
            }
        }

        Ok(())
    }
}

fn unknown_placeholder(value: &str) -> Option<String> {
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        let tail = &rest[start..];
        let end = tail.find('}')? + 1;
        let placeholder = &tail[..end];
        if placeholder != PLUGIN_DIR_PLACEHOLDER && placeholder != DATA_DIR_PLACEHOLDER {
            return Some(placeholder.to_owned());
        }
        rest = &tail[end..];
    }
    None
}

fn expand_all(paths: &[String], plugin_dir: &Path, data_dir: &Path) -> Vec<PathBuf> {
    paths
        .iter()
        .map(|path| expand(path, plugin_dir, data_dir))
        .collect()
}

fn expand(raw: &str, plugin_dir: &Path, data_dir: &Path) -> PathBuf {
    let expanded = raw
        .replace(
            PLUGIN_DIR_PLACEHOLDER,
            plugin_dir.to_string_lossy().as_ref(),
        )
        .replace(DATA_DIR_PLACEHOLDER, data_dir.to_string_lossy().as_ref());

    let path = PathBuf::from(expanded);
    if path.is_absolute() {
        path
    } else {
        plugin_dir.join(path)
    }
}

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

/// Why a manifest could not be used, and — when it was read from disk — which
/// file it was.
///
/// The path is a separate field rather than a variant so that every problem
/// gains it without the enum doubling in size, and so a caller can match on the
/// problem without unwrapping a location first.
#[derive(Debug, PartialEq)]
pub struct ManifestError {
    path: Option<PathBuf>,
    problem: ManifestProblem,
}

impl ManifestError {
    /// The manifest file, when the manifest came from one.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn problem(&self) -> &ManifestProblem {
        &self.problem
    }
}

impl From<ManifestProblem> for ManifestError {
    fn from(problem: ManifestProblem) -> Self {
        Self {
            path: None,
            problem,
        }
    }
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{}: {}", path.display(), self.problem),
            None => self.problem.fmt(f),
        }
    }
}

impl std::error::Error for ManifestError {}

/// What was wrong with a manifest.
///
/// Wording follows `CapabilityError`: say what is wrong, then say what to
/// write instead. A plugin author reading one of these has no other diagnostic
/// available — nothing has run yet.
#[derive(Debug, PartialEq)]
pub enum ManifestProblem {
    Unreadable(String),
    NotJson(String),
    NotAnObject(&'static str),
    MissingField(&'static str),
    EmptyField(&'static str),
    WrongType {
        field: &'static str,
        found: &'static str,
    },
    UnknownField {
        field: String,
        suggestion: Option<&'static str>,
    },
    InvalidId {
        id: String,
        reason: String,
    },
    DuplicateId {
        id: String,
        first: PathBuf,
    },
    InvalidVersion(String),
    InvalidEntry(String),
    UnknownPlaceholder {
        field: String,
        placeholder: String,
    },
    Capabilities(String),
}

/// What each field is for, in one line, appended to the error that reports it
/// missing. A field name alone tells an author which key to add but not what to
/// put in it.
fn field_expectation(field: &str) -> &'static str {
    match field {
        "id" => {
            "a reverse-DNS identifier such as \"com.example.inbox\"; it is also the plugin's namespace for panels, storage and permissions"
        }
        "name" => {
            "a human-readable name such as \"Inbox\", shown in menus and in the permission prompt"
        }
        "version" => "the plugin's own semantic version such as \"1.2.0\"",
        "entry" => {
            "the module to evaluate at load, such as \"main.js\", relative to the plugin directory"
        }
        _ => "a value",
    }
}

impl std::fmt::Display for ManifestProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestProblem::Unreadable(error) => {
                write!(f, "cannot read the manifest: {error}")
            }
            ManifestProblem::NotJson(error) => {
                write!(f, "the manifest is not valid JSON: {error}")
            }
            ManifestProblem::NotAnObject(found) => write!(
                f,
                "the manifest must be a JSON object with the fields {}, found {found}",
                FIELDS.join(", ")
            ),
            ManifestProblem::MissingField(field) => write!(
                f,
                "missing field `{field}`: expected {}",
                field_expectation(field)
            ),
            ManifestProblem::EmptyField(field) => write!(
                f,
                "field `{field}` is empty: expected {}",
                field_expectation(field)
            ),
            ManifestProblem::WrongType { field, found } => write!(
                f,
                "field `{field}` must be a string, found {found}: expected {}",
                field_expectation(field)
            ),
            ManifestProblem::UnknownField { field, suggestion } => {
                write!(f, "unknown field `{field}`")?;
                if let Some(suggestion) = suggestion {
                    write!(f, "; did you mean `{suggestion}`?")?;
                }
                write!(
                    f,
                    " A manifest has exactly the fields {}. Commands, panels, keybindings, settings and themes are registered in script, not declared here.",
                    FIELDS.join(", ")
                )
            }
            ManifestProblem::InvalidId { id, reason } => write!(
                f,
                "invalid `id` \"{id}\": {reason}; an id may contain lowercase letters, digits, `.`, `-` and `_`, because it is used verbatim as a panel name prefix (script:<id>/<panel>) and as a directory name under the user's data directory"
            ),
            ManifestProblem::DuplicateId { id, first } => write!(
                f,
                "`{id}` is already provided by {}; a plugin id is a namespace, and two plugins sharing one would share their storage, their panel names and their permissions",
                first.display()
            ),
            ManifestProblem::InvalidVersion(version) => write!(
                f,
                "invalid `version` \"{version}\": expected a semantic version such as \"1.2.0\""
            ),
            ManifestProblem::InvalidEntry(entry) => write!(
                f,
                "invalid `entry` \"{entry}\": expected a path inside the plugin directory, such as \"main.js\"; an absolute path or one containing `..` is refused for the same reason an `import` of one is"
            ),
            ManifestProblem::UnknownPlaceholder { field, placeholder } => write!(
                f,
                "unknown placeholder `{placeholder}` in {field}: the manifest may use {PLUGIN_DIR_PLACEHOLDER} and {DATA_DIR_PLACEHOLDER}"
            ),
            ManifestProblem::Capabilities(error) => write!(
                f,
                "invalid `capabilities`: {error}. The block accepts fs (read, write, execute), network (hosts), store and clipboard (read, write)"
            ),
        }
    }
}

// -----------------------------------------------------------------------------
// Loaded plugins
// -----------------------------------------------------------------------------

/// One installed plugin: its manifest, where it lives, and — once loaded — what
/// it turned into.
pub struct Plugin {
    manifest: PluginManifest,
    root: PathBuf,
    data_dir: PathBuf,
    store_path: PathBuf,
    /// The authority this plugin's code runs under, built once at load from its
    /// manifest and then never swapped. Callbacks it registers capture it, so a
    /// timer firing inside plugin A cannot run with plugin B's grant.
    policy: Rc<Policy>,
    view: Option<Entity<ScriptView>>,
}

impl Plugin {
    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    pub fn id(&self) -> &str {
        self.manifest.id()
    }

    /// The directory the manifest was read from.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where this plugin's data lives, keyed by `id` so it survives an upgrade
    /// that moves the plugin directory.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// The file behind `gpui.store`.
    pub fn store_path(&self) -> &Path {
        &self.store_path
    }

    /// The grant in force while this plugin runs. It comes from the manifest
    /// and nowhere else.
    pub fn capabilities(&self) -> &Capabilities {
        self.policy.capabilities()
    }

    /// The authority this plugin runs under.
    ///
    /// A host that wants to run something on the plugin's behalf outside its
    /// view — evaluating a command handler, say — opens a scope with this so
    /// that code sees the plugin's grant rather than whatever was in force.
    pub fn policy(&self) -> &Rc<Policy> {
        &self.policy
    }

    /// The view the entry module default-exported.
    ///
    /// Optional because §18.1's entry module only *registers* — it need not
    /// export a view at all. The current engine requires one (see
    /// [`PluginManager::load`]), so today this is `Some` for every loaded
    /// plugin; the option is what stops that from becoming an assumption
    /// callers bake in.
    pub fn view(&self) -> Option<&Entity<ScriptView>> {
        self.view.as_ref()
    }
}

/// Discovers, loads and unloads plugins.
///
/// Every loaded plugin holds its own [`Policy`] — its grant, its store, its
/// native modules — and every call into its code runs under that policy because
/// the policy travels on the call frame rather than in a process-wide slot. Two
/// plugins loaded at once hold two different grants at the same time, and
/// neither can see the other's files.
///
/// This used to be one slot with a guard around each call, and the guard could
/// not be made correct: a plugin that `await`s hands control back before its
/// guard drops, so the grant in force during the continuation was whichever
/// plugin happened to be running when the promise resolved. Time is what the
/// swap could not account for. Authority now belongs to the code rather than to
/// the moment.
pub struct PluginManager {
    directories: Vec<PathBuf>,
    data_home: PathBuf,
    catalog: Vec<CatalogEntry>,
    discovered: bool,
    loaded: BTreeMap<String, Plugin>,
}

struct CatalogEntry {
    manifest: PluginManifest,
    root: PathBuf,
}

impl PluginManager {
    /// `directories` are searched in order; an earlier directory wins a
    /// duplicate `id`, which is what lets a user's own copy shadow a bundled
    /// one.
    pub fn new(directories: Vec<PathBuf>) -> Self {
        Self {
            directories,
            data_home: default_data_home(),
            catalog: Vec::new(),
            discovered: false,
            loaded: BTreeMap::new(),
        }
    }

    /// Overrides where plugin data lives. A host that keeps a portable profile
    /// needs this, and so does a test that must not touch the real one.
    pub fn with_data_home(mut self, path: PathBuf) -> Self {
        self.data_home = path;
        self
    }

    pub fn directories(&self) -> &[PathBuf] {
        &self.directories
    }

    /// Reads every manifest found under the configured directories.
    ///
    /// **Nothing is executed.** This is the whole point of the manifest: a host
    /// can show thirty plugins, their versions and the permissions each asks
    /// for, having started none of them.
    ///
    /// Both outcomes are returned in one list rather than the whole pass
    /// failing on the first bad manifest — one broken plugin must not hide the
    /// twenty-nine working ones, and the broken one still has to be reportable.
    /// Results are ordered by path so a listing does not reshuffle itself
    /// between runs.
    pub fn discover(&mut self) -> Vec<Result<PluginManifest, ManifestError>> {
        self.catalog.clear();
        self.discovered = true;

        let mut results = Vec::new();
        let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();

        for directory in self.directories.clone() {
            for root in plugin_roots(&directory) {
                let manifest = match PluginManifest::read(&root) {
                    Ok(manifest) => manifest,
                    Err(error) => {
                        results.push(Err(error));
                        continue;
                    }
                };

                // First directory wins, and the loser is reported rather
                // than dropped: a shadowed plugin that simply never appears is
                // the hardest kind of install problem to diagnose.
                if let Some(first) = seen.get(manifest.id()) {
                    results.push(Err(ManifestError {
                        path: Some(root.join(MANIFEST_FILE)),
                        problem: ManifestProblem::DuplicateId {
                            id: manifest.id().to_owned(),
                            first: first.clone(),
                        },
                    }));
                    continue;
                }

                seen.insert(manifest.id().to_owned(), root.clone());
                self.catalog.push(CatalogEntry {
                    manifest: manifest.clone(),
                    root,
                });
                results.push(Ok(manifest));
            }
        }

        results
    }

    /// The manifests found by the last [`discover`](Self::discover).
    pub fn available(&self) -> impl Iterator<Item = &PluginManifest> {
        self.catalog.iter().map(|entry| &entry.manifest)
    }

    /// Evaluates a plugin's entry module and constructs its view.
    ///
    /// This is the only method that runs script. The plugin's policy is built
    /// before anything is evaluated and the whole load runs inside it, because
    /// the entry module may read its own files while registering.
    pub fn load(
        &mut self,
        runtime: &Rc<ShellRuntime>,
        id: &str,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<()> {
        if !self.discovered {
            self.discover();
        }

        if self.loaded.contains_key(id) {
            bail!("plugin `{id}` is already loaded; unload it first to load it again");
        }

        let Some(entry) = self.catalog.iter().find(|entry| entry.manifest.id() == id) else {
            bail!("no plugin `{id}`{}", self.known_ids_hint());
        };
        let manifest = entry.manifest.clone();
        let root = entry.root.clone();

        // `ShellRuntime::load_app` hardcodes `main.js` — it is also what sets
        // the module resolver's root, so reading another entry file here would
        // silently break every relative `import` inside the plugin. Refusing is
        // the honest failure until the engine takes the entry name.
        if manifest.entry() != ENGINE_ENTRY {
            bail!(
                "plugin `{id}` declares entry `{}`, but the runtime can only load `{ENGINE_ENTRY}` today",
                manifest.entry()
            );
        }

        let data_dir = self.data_dir(id);
        let store_path = data_dir.join("store.json");
        if let Err(error) = std::fs::create_dir_all(&data_dir) {
            // Not fatal: a plugin that never touches storage still runs, and a
            // plugin that does will fail at the call, where the message can say
            // which key it was reaching for.
            tracing::warn!(
                "storage is unavailable for `{id}`: cannot create {}: {error}",
                data_dir.display()
            );
        }

        let policy = Rc::new(
            Policy::default()
                .with_capabilities(manifest.capabilities(&root, &data_dir))
                .with_store_path(store_path.clone()),
        );

        // The frame is what carries the grant, so everything the entry module
        // does — including anything it defers — happens inside one.
        let loaded = {
            let (_scope, _) = scope::enter_with(window, cx, ScopePhase::Task, None, policy.clone());
            runtime
                .load_app(&root, manifest.entry())
                .and_then(|view_type| runtime.instantiate(&view_type, window, cx))
        };

        let object = loaded.map_err(|error| error.context(format!("loading plugin `{id}`")))?;

        // Built with the policy rather than inheriting it: the view outlives
        // this call, and every later render and callback reads it from here.
        let view = cx.new(|_| ScriptView::with_policy(runtime.clone(), object, policy.clone()));
        self.loaded.insert(
            id.to_owned(),
            Plugin {
                manifest,
                root,
                data_dir,
                store_path,
                policy,
                view: Some(view),
            },
        );

        Ok(())
    }

    /// Drops a plugin's view, and with it its policy. Returns whether there was
    /// one.
    ///
    /// Dropping the [`Entity<ScriptView>`] releases the script object with it,
    /// which is as much teardown as the current runtime can do: there is no
    /// `deactivate()` call yet because there is nothing registered to tear down
    /// (§18.2's registration API does not exist).
    pub fn unload(&mut self, id: &str) -> bool {
        self.loaded.remove(id).is_some()
    }

    pub fn loaded(&self) -> impl Iterator<Item = &Plugin> {
        self.loaded.values()
    }

    pub fn plugin(&self, id: &str) -> Option<&Plugin> {
        self.loaded.get(id)
    }

    /// Where a plugin's data lives.
    ///
    /// Keyed by `id`, not by path: an upgrade that replaces the plugin
    /// directory must not lose the user's data, and two checkouts of the same
    /// plugin are the same installation — which is the opposite of the rule for
    /// a directory run from the command line, where the path *is* the identity.
    pub fn data_dir(&self, id: &str) -> PathBuf {
        self.data_home.join("gpui-shell").join("plugins").join(id)
    }

    fn known_ids_hint(&self) -> String {
        let ids: Vec<&str> = self
            .catalog
            .iter()
            .map(|entry| entry.manifest.id())
            .collect();
        if ids.is_empty() {
            format!(
                "; no plugin was found in {}",
                self.directories
                    .iter()
                    .map(|directory| directory.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            format!("; found {}", ids.join(", "))
        }
    }
}

/// A configured directory is either one plugin or a directory of them.
///
/// Both are worth supporting and they are distinguishable without guessing: a
/// directory holding a manifest is a plugin, anything else is a container. That
/// is what makes `--plugin ~/dev/my-plugin` work alongside a user's installed
/// plugin folder, with no second flag to say which kind it is.
fn plugin_roots(directory: &Path) -> Vec<PathBuf> {
    if directory.join(MANIFEST_FILE).is_file() {
        return vec![directory.to_path_buf()];
    }

    let mut roots: Vec<PathBuf> = std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join(MANIFEST_FILE).is_file())
        .collect();

    // Directory order is filesystem-dependent; a plugin list that reorders
    // itself between runs is a bug report waiting to happen.
    roots.sort();
    roots
}

/// The platform's per-user data directory.
///
/// Duplicated from `src/bin/gpui-shell.rs` on purpose rather than shared: this
/// module owns no file but itself. It belongs in `runtime.rs` so the binary and
/// the plugin manager cannot disagree about where a user's data is — see the
/// report accompanying this module.
fn default_data_home() -> PathBuf {
    if let Some(explicit) = std::env::var_os("XDG_DATA_HOME").filter(|it| !it.is_empty()) {
        return PathBuf::from(explicit);
    }

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);

    if cfg!(target_os = "macos") {
        home.join("Library").join("Application Support")
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Roaming"))
    } else {
        home.join(".local").join("share")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    const VALID: &str = r#"{
        "id": "com.example.inbox",
        "name": "Inbox",
        "version": "1.2.0",
        "entry": "main.js",
        "capabilities": {
            "fs": {
                "read": ["${pluginDir}", "${dataDir}"],
                "write": ["${dataDir}"],
                "execute": ["git"]
            },
            "network": { "hosts": ["api.example.com"] },
            "store": true,
            "clipboard": { "write": true }
        }
    }"#;

    /// A directory that removes itself. `tempfile` is not a dependency of this
    /// crate and one test module is not a reason to add one.
    struct TempTree(PathBuf);

    impl TempTree {
        fn new(label: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "gpui-shell-plugin-{label}-{}-{unique}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("cannot create a temporary directory");
            Self(path)
        }

        fn plugin(&self, directory: &str, manifest: &str) -> PathBuf {
            let root = self.0.join(directory);
            std::fs::create_dir_all(&root).expect("cannot create a plugin directory");
            std::fs::write(root.join(MANIFEST_FILE), manifest).expect("cannot write a manifest");
            root
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_valid_manifest_reads_back_what_was_written() {
        let manifest = PluginManifest::parse(VALID).expect("the manifest should parse");

        assert_eq!(manifest.id(), "com.example.inbox");
        assert_eq!(manifest.name(), "Inbox");
        assert_eq!(manifest.version(), "1.2.0");
        assert_eq!(manifest.entry(), "main.js");

        let capabilities =
            manifest.capabilities(Path::new("/plugins/inbox"), Path::new("/data/inbox"));
        assert!(capabilities.has_store());
        assert!(capabilities.is_clipboard_writable());
        assert!(!capabilities.is_clipboard_readable());
    }

    #[test]
    fn capabilities_become_a_real_grant() {
        let manifest = PluginManifest::parse(VALID).expect("the manifest should parse");
        // Real directories, because a grant is an open handle now: the
        // placeholders still resolve to paths, but a path is only half of one.
        let base = std::env::temp_dir().join(format!("gpui-shell-grant-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let plugin_dir = base.join("plugins/inbox");
        let data_dir = base.join("data/inbox");
        std::fs::create_dir_all(&plugin_dir).expect("a plugin directory");
        std::fs::create_dir_all(&data_dir).expect("a data directory");
        let capabilities = manifest.capabilities(&plugin_dir, &data_dir);

        assert_eq!(
            capabilities.execute(),
            &ExecuteGrant::Allowed(vec!["git".to_owned()])
        );
        assert!(capabilities.may_run("git"));
        assert!(!capabilities.may_run("curl"));
        assert!(capabilities.may_reach("api.example.com"));
        assert!(!capabilities.may_reach("evil.example.com"));

        // The placeholders are the only way a manifest can name a directory it
        // does not know the path of.
        assert_eq!(
            capabilities
                .open(Path::new("main.js"), crate::capability::Access::Read)
                .expect("the plugin directory should be readable")
                .path(),
            Path::new("main.js")
        );
        assert_eq!(
            capabilities
                .open(
                    &data_dir.join("items.json"),
                    crate::capability::Access::Write
                )
                .expect("the data directory should be writable")
                .path(),
            Path::new("items.json")
        );
        assert!(
            capabilities
                .open(Path::new("/etc/passwd"), crate::capability::Access::Read)
                .is_err()
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_absent_capabilities_block_grants_nothing() {
        let manifest = PluginManifest::parse(
            r#"{"id": "a.b", "name": "B", "version": "0.1.0", "entry": "main.js"}"#,
        )
        .expect("capabilities may be omitted");

        let capabilities = manifest.capabilities(Path::new("/plugins/b"), Path::new("/data/b"));
        assert!(!capabilities.has_store());
        assert!(!capabilities.has_read_access());
        assert!(!capabilities.has_write_access());
        assert_eq!(capabilities.execute(), &ExecuteGrant::Denied);
    }

    #[test]
    fn each_missing_field_names_itself() {
        for field in ["id", "name", "version", "entry"] {
            let mut fields: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(VALID).expect("the fixture should be an object");
            fields.remove(field);
            let source = serde_json::to_string(&fields).expect("re-encoding cannot fail");

            let error = PluginManifest::parse(&source).expect_err("the field is required");
            assert_eq!(error.problem(), &ManifestProblem::MissingField(field));

            let message = error.to_string();
            assert!(
                message.contains(&format!("`{field}`")),
                "the message must name the field: {message}"
            );
            assert!(
                message.contains("expected"),
                "the message must say what was expected: {message}"
            );
        }
    }

    #[test]
    fn a_field_of_the_wrong_type_says_which_and_what() {
        let error = PluginManifest::parse(
            r#"{"id": "a.b", "name": 7, "version": "0.1.0", "entry": "main.js"}"#,
        )
        .expect_err("a numeric name is not a name");

        assert_eq!(
            error.problem(),
            &ManifestProblem::WrongType {
                field: "name",
                found: "a number"
            }
        );
        assert!(error.to_string().contains("must be a string"));
    }

    #[test]
    fn a_typo_in_capabilities_is_refused_rather_than_granting_nothing() {
        let source = VALID.replacen("\"capabilities\"", "\"capabilites\"", 1);
        let error = PluginManifest::parse(&source).expect_err("an unknown field is not accepted");

        assert_eq!(
            error.problem(),
            &ManifestProblem::UnknownField {
                field: "capabilites".to_owned(),
                suggestion: Some("capabilities"),
            }
        );
        let message = error.to_string();
        assert!(
            message.contains("did you mean `capabilities`?"),
            "{message}"
        );
    }

    #[test]
    fn a_typo_inside_the_capabilities_block_is_refused_too() {
        let source = VALID.replacen("\"network\"", "\"netwrok\"", 1);
        let error = PluginManifest::parse(&source).expect_err("an unknown grant is not accepted");

        let message = error.to_string();
        assert!(message.contains("netwrok"), "{message}");
        assert!(message.contains("invalid `capabilities`"), "{message}");
    }

    #[test]
    fn a_contribution_declared_in_the_manifest_is_refused() {
        // The rule the manifest exists to hold: contributions live in script.
        let source = VALID.replacen("\"name\"", "\"contributes\"", 1);
        let error = PluginManifest::parse(&source).expect_err("`contributes` is not a field");

        let message = error.to_string();
        assert!(message.contains("registered in script"), "{message}");
    }

    #[test]
    fn an_id_that_could_leave_its_data_directory_is_refused() {
        for id in [
            "../escape",
            "com.Example.Inbox",
            "a/b",
            "..",
            ".hidden",
            "inbox.",
        ] {
            let source = VALID.replacen("com.example.inbox", id, 1);
            assert!(
                PluginManifest::parse(&source).is_err(),
                "`{id}` must be refused: it becomes a directory name and a panel prefix"
            );
        }
    }

    #[test]
    fn an_invalid_id_explains_the_rule() {
        let source = VALID.replacen("com.example.inbox", "../escape", 1);
        let error = PluginManifest::parse(&source).expect_err("an id may not contain a separator");
        assert!(matches!(error.problem(), ManifestProblem::InvalidId { .. }));
        assert!(error.to_string().contains("script:<id>/<panel>"));
    }

    #[test]
    fn a_version_must_be_comparable() {
        let source = VALID.replacen("\"1.2.0\"", "\"latest\"", 1);
        let error = PluginManifest::parse(&source).expect_err("`latest` is not a version");
        assert_eq!(
            error.problem(),
            &ManifestProblem::InvalidVersion("latest".to_owned())
        );

        for version in ["1.2.0", "0.0.1", "1.2.0-beta.1", "2.0.0+build.5"] {
            let source = VALID.replacen("\"1.2.0\"", &format!("\"{version}\""), 1);
            assert!(
                PluginManifest::parse(&source).is_ok(),
                "`{version}` should parse"
            );
        }
    }

    #[test]
    fn an_entry_outside_the_plugin_directory_is_refused() {
        let source = VALID.replacen("\"main.js\"", "\"../../etc/main.js\"", 1);
        let error = PluginManifest::parse(&source).expect_err("an entry may not escape");
        assert!(matches!(error.problem(), ManifestProblem::InvalidEntry(_)));
    }

    #[test]
    fn an_unexpanded_placeholder_is_caught_before_it_becomes_a_directory() {
        let source = VALID.replacen("${dataDir}", "${homeDir}", 1);
        let error = PluginManifest::parse(&source).expect_err("`${homeDir}` does not exist");
        assert_eq!(
            error.problem(),
            &ManifestProblem::UnknownPlaceholder {
                field: "capabilities.fs.read".to_owned(),
                placeholder: "${homeDir}".to_owned(),
            }
        );
    }

    #[test]
    fn an_unrestricted_execute_grant_is_spelled_differently_from_an_allowlist() {
        let source = VALID.replacen("[\"git\"]", "\"*\"", 1);
        let manifest = PluginManifest::parse(&source).expect("`*` is a valid execute grant");
        let capabilities = manifest.capabilities(Path::new("/plugins/a"), Path::new("/data/a"));
        assert_eq!(capabilities.execute(), &ExecuteGrant::Unrestricted);
        assert!(capabilities.may_run("anything"));
    }

    #[test]
    fn a_manifest_read_from_disk_carries_its_path() {
        let tree = TempTree::new("read");
        let root = tree.plugin("inbox", VALID);

        let manifest = PluginManifest::read(&root).expect("the manifest should parse");
        assert_eq!(manifest.id(), "com.example.inbox");

        let broken = tree.plugin("broken", "{ \"id\": \"a.b\" }");
        let error = PluginManifest::read(&broken).expect_err("`name` is missing");
        assert_eq!(error.path(), Some(broken.join(MANIFEST_FILE).as_path()));
        assert!(error.to_string().contains(MANIFEST_FILE));
    }

    #[test]
    fn discovery_returns_the_broken_manifest_beside_the_good_one() {
        let tree = TempTree::new("discover");
        tree.plugin("a-good", VALID);
        tree.plugin("b-broken", "{ \"id\": \"com.example.broken\" }");
        // Not a plugin: no manifest, so it is not reported at all.
        std::fs::create_dir_all(tree.path().join("c-not-a-plugin"))
            .expect("cannot create a directory");

        let mut manager = PluginManager::new(vec![tree.path().to_path_buf()]);
        let results = manager.discover();

        assert_eq!(results.len(), 2, "{results:?}");
        assert_eq!(
            results[0].as_ref().expect("the first is valid").id(),
            "com.example.inbox"
        );
        let error = results[1].as_ref().expect_err("the second is broken");
        assert_eq!(error.problem(), &ManifestProblem::MissingField("name"));

        // Only the readable manifest reached the catalog, and nothing ran.
        let available: Vec<&str> = manager.available().map(PluginManifest::id).collect();
        assert_eq!(available, vec!["com.example.inbox"]);
        assert_eq!(manager.loaded().count(), 0);
    }

    #[test]
    fn a_directory_that_is_itself_a_plugin_is_discovered() {
        let tree = TempTree::new("single");
        let root = tree.plugin("inbox", VALID);

        let mut manager = PluginManager::new(vec![root]);
        let results = manager.discover();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[test]
    fn the_same_id_in_two_directories_is_reported_once_and_refused_once() {
        let first = TempTree::new("dup-first");
        let second = TempTree::new("dup-second");
        first.plugin("inbox", VALID);
        second.plugin("inbox", VALID);

        let mut manager = PluginManager::new(vec![
            first.path().to_path_buf(),
            second.path().to_path_buf(),
        ]);
        let results = manager.discover();

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(
            matches!(
                results[1]
                    .as_ref()
                    .expect_err("the copy is refused")
                    .problem(),
                ManifestProblem::DuplicateId { .. }
            ),
            "the second copy must not win silently"
        );
        assert_eq!(manager.available().count(), 1);
    }

    #[test]
    fn storage_is_keyed_by_id_not_by_path() {
        let manager =
            PluginManager::new(Vec::new()).with_data_home(PathBuf::from("/home/user/.local/share"));
        assert_eq!(
            manager.data_dir("com.example.inbox"),
            PathBuf::from("/home/user/.local/share/gpui-shell/plugins/com.example.inbox")
        );
    }

    #[test]
    fn the_schema_type_accepts_exactly_what_the_parser_accepts() {
        // Two readers of one file: `schemars` describes `ManifestFile`, while
        // `parse` walks the fields by hand to explain each failure. This is the
        // check that they still describe the same file.
        let described: ManifestFile =
            serde_json::from_str(VALID).expect("the schema type must accept a valid manifest");
        let parsed = PluginManifest::parse(VALID).expect("and so must the parser");

        assert_eq!(described.id, parsed.id);
        assert_eq!(described.name, parsed.name);
        assert_eq!(described.version, parsed.version);
        assert_eq!(described.entry, parsed.entry);
        assert_eq!(described.capabilities, parsed.capabilities);
    }

    #[test]
    fn the_schema_describes_every_field() {
        let schema = manifest_schema().to_string();
        for field in FIELDS {
            assert!(schema.contains(field), "the schema must mention `{field}`");
        }
        for grant in [
            "fs",
            "read",
            "write",
            "execute",
            "network",
            "hosts",
            "store",
            "clipboard",
        ] {
            assert!(
                schema.contains(grant),
                "the schema must mention `{grant}` inside capabilities"
            );
        }
        // A sixth field must be a schema violation, not a silently ignored key.
        assert!(
            schema.contains("additionalProperties"),
            "the schema must refuse unknown fields as the parser does"
        );
    }

    #[test]
    fn the_plugin_model_and_the_runtime_publish_one_api_version() {
        assert_eq!(API_VERSION, crate::plugin_api::VERSION);
        assert!(
            crate::plugin_api::check(API_VERSION).is_ok(),
            "the runtime must satisfy its own version"
        );
    }
}
