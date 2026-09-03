#!/usr/bin/env bun
/**
 * Publish a snapshot of Zed's GPUI crates to crates.io as `gpui-pre-*`.
 *
 * The `gpui`, `gpui_platform` and `gpui_macros` names on crates.io belong to
 * Zed, and Zed only publishes them occasionally. This script lets GPUI Kit
 * publish its own pre-release builds straight from any Zed commit:
 *
 * 1. Fetch the requested Zed revision into `target/gpui-pre/zed`
 *    (or use an existing checkout passed with `--zed`).
 * 2. Walk the workspace `path` dependencies of the three root crates and
 *    collect every internal crate they need.
 * 3. Rename each crate (`gpui` -> `gpui-pre`, `gpui_platform` ->
 *    `gpui-pre-platform`, `collections` -> `gpui-pre-collections`, ...),
 *    give all of them the same version, and keep the original crate name as
 *    the `[lib]` name so `use gpui::*` keeps working.
 * 4. Drop optional dependencies that come from git without a crates.io
 *    version (crates.io rejects those), together with the features that
 *    enable them. Non-optional ones abort the run.
 * 5. Write a standalone workspace to `target/gpui-pre/workspace`, verify it
 *    with `cargo publish --workspace --dry-run`, then publish it.
 *
 * crates.io only accepts a handful of brand-new crates per ten minutes. The
 * publish step re-checks crates.io before every attempt, skips versions that
 * already exist, waits out the rate limit, and can be re-run at any time.
 *
 * Usage:
 *     script/bump-gpui.ts [VERSION] [--rev REV] [--zed PATH]
 *                         [--dry-run] [--stage-only] [--no-verify] [--no-wait]
 *
 * The version comes from the `VERSION` constant below; bump it before each
 * publish. A positional VERSION overrides it for one run.
 */

import {
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, normalize, relative, resolve } from "node:path";
import { parseArgs } from "node:util";

/**
 * The version every `gpui-pre-*` crate is published at.
 *
 * crates.io never accepts the same version twice, so bump this before each
 * publish; the Zed commit it was built from is recorded in each crate's
 * description and `[package.metadata.gpui-pre]`.
 */
const VERSION = "0.3.0";

const ZED_GIT_URL = "https://github.com/zed-industries/zed";
const ZED_DEFAULT_REV = "main";
const PUBLISH_PREFIX = "gpui-pre";
const ROOT_CRATES = ["gpui", "gpui_platform", "gpui_macros"];
const CRATES_IO_API = "https://crates.io/api/v1/crates";
const USER_AGENT = "gpui-kit bump-gpui (https://github.com/longbridge/gpui-component)";
const RATE_LIMIT_FALLBACK_MS = (10 * 60 + 30) * 1000;

const DEP_TABLES = ["dependencies", "build-dependencies"] as const;
const DEV_DEP_TABLE = "dev-dependencies";

const REPO_ROOT = resolve(import.meta.dir, "..");
const WORK_DIR = join(REPO_ROOT, "target", "gpui-pre");

type Toml = Record<string, any>;

// ---------------------------------------------------------------------------
// Logging (mirrors script/bump-version.sh)
// ---------------------------------------------------------------------------

const USE_COLOR = Boolean(process.stdout.isTTY) && process.env.NO_COLOR === undefined;

const paint = (code: string, text: string) => (USE_COLOR ? `\x1b[${code}m${text}\x1b[0m` : text);
const bold = (text: string) => paint("1", text);
const dim = (text: string) => paint("2", text);

function logHeader(message: string) {
  const line = "═".repeat(56);
  console.log();
  console.log(paint("1;34", `╔${line}╗`));
  console.log(`${paint("1;34", "║")}  ${paint("1;36", message)}`);
  console.log(paint("1;34", `╚${line}╝`));
  console.log();
}

const logStep = (step: string, message: string) => console.log(`${paint("1;35", `[${step}]`)} ${message}`);
const logSuccess = (message: string) => console.log(`${paint("1;32", "✓")} ${message}`);
const logInfo = (message: string) => console.log(`${paint("36", "ℹ")} ${message}`);
const logWarn = (message: string) => console.log(`${paint("1;33", "!")} ${message}`);
const logError = (message: string) => console.error(`${paint("1;31", "✗")} ${message}`);

/** A user-facing failure; the message is printed without a stack trace. */
class BumpError extends Error {}

// ---------------------------------------------------------------------------
// Shell helpers
// ---------------------------------------------------------------------------

interface RunOptions {
  cwd?: string;
  /** Capture output instead of streaming it to the terminal. */
  capture?: boolean;
}

/** Run a command, echoing it first. Throws BumpError on failure. */
async function run(cmd: string[], options: RunOptions = {}): Promise<string> {
  const shown = options.cwd ? `(cd ${options.cwd} && ${cmd.join(" ")})` : cmd.join(" ");
  console.log(dim(`$ ${shown}`));
  const { code, output } = await spawn(cmd, options.cwd, !options.capture);
  if (code !== 0) {
    const detail = output.trim() ? `\n${output.trim()}` : "";
    throw new BumpError(`command failed (${code}): ${cmd[0]}${detail}`);
  }
  return output;
}

/** Run a command, streaming its output while also capturing it. */
async function runStreaming(cmd: string[], cwd: string): Promise<{ code: number; output: string }> {
  console.log(dim(`$ (cd ${cwd} && ${cmd.join(" ")})`));
  return spawn(cmd, cwd, true);
}

async function spawn(cmd: string[], cwd: string | undefined, echo: boolean) {
  const process_ = Bun.spawn(cmd, { cwd, stdin: "inherit", stdout: "pipe", stderr: "pipe" });
  const chunks: string[] = [];
  const decoder = new TextDecoder();
  const pump = async (stream: ReadableStream<Uint8Array>) => {
    for await (const chunk of stream) {
      const text = decoder.decode(chunk, { stream: true });
      chunks.push(text);
      if (echo) process.stdout.write(text);
    }
  };
  await Promise.all([pump(process_.stdout), pump(process_.stderr)]);
  const code = await process_.exited;
  return { code, output: chunks.join("") };
}

// ---------------------------------------------------------------------------
// Zed checkout
// ---------------------------------------------------------------------------

async function prepareZed(rev: string, existing: string | undefined): Promise<string> {
  if (existing !== undefined) {
    const zed = resolve(existing.replace(/^~(?=$|\/)/, process.env.HOME ?? "~"));
    if (!existsSync(join(zed, "Cargo.toml"))) {
      throw new BumpError(`${zed} does not look like a Zed checkout (no Cargo.toml)`);
    }
    logInfo(`Using existing Zed checkout at ${bold(zed)}`);
    return zed;
  }

  const zed = join(WORK_DIR, "zed");
  if (!existsSync(join(zed, ".git"))) {
    mkdirSync(zed, { recursive: true });
    await run(["git", "init", "-q"], { cwd: zed });
    await run(["git", "remote", "add", "origin", ZED_GIT_URL], { cwd: zed });
  }
  logInfo(`Fetching ${bold(rev)} from ${ZED_GIT_URL}`);
  await run(["git", "fetch", "--depth", "1", "--no-tags", "origin", rev], { cwd: zed });
  await run(["git", "checkout", "-q", "--detach", "--force", "FETCH_HEAD"], { cwd: zed });
  return zed;
}

async function zedRevision(zed: string): Promise<string> {
  try {
    return (await run(["git", "rev-parse", "HEAD"], { cwd: zed, capture: true })).trim();
  } catch {
    throw new BumpError(`${zed} is not a git checkout; cannot determine the Zed revision`);
  }
}

// ---------------------------------------------------------------------------
// Workspace model
// ---------------------------------------------------------------------------

/** One Zed workspace member selected for publishing. */
interface Crate {
  relDir: string;
  manifest: Toml;
  name: string;
  version: string;
  publishedName: string;
  prunedDeps: string[];
  prunedFeatures: string[];
}

interface Workspace {
  root: string;
  manifest: Toml;
  /** relDir -> manifest */
  members: Map<string, Toml>;
  /** relDir -> what pruning removed, so a recomputed closure keeps the record */
  pruned: Map<string, { deps: string[]; features: string[] }>;
}

function workspaceDependencies(ws: Workspace): Toml {
  return ws.manifest.workspace.dependencies ?? {};
}

function readToml(path: string): Toml {
  return Bun.TOML.parse(readFileSync(path, "utf8")) as Toml;
}

function loadWorkspace(zed: string): Workspace {
  const manifest = readToml(join(zed, "Cargo.toml"));
  const members = new Map<string, Toml>();
  for (const pattern of manifest.workspace.members as string[]) {
    const matches = [...new Bun.Glob(pattern).scanSync({ cwd: zed, onlyFiles: false })].sort();
    for (const match of matches) {
      const cargoToml = join(zed, match, "Cargo.toml");
      if (existsSync(cargoToml)) {
        members.set(normalize(match), readToml(cargoToml));
      }
    }
  }
  return { root: zed, manifest, members, pruned: new Map() };
}

/** `gpui` -> `gpui-pre`, `gpui_macros` -> `gpui-pre-macros`, `collections` -> `gpui-pre-collections`. */
function publishedName(zedName: string): string {
  if (zedName === "gpui") return PUBLISH_PREFIX;
  const suffix = zedName.startsWith("gpui_") ? zedName.slice("gpui_".length) : zedName;
  return `${PUBLISH_PREFIX}-${suffix.replaceAll("_", "-")}`;
}

/** `util` and `gpui_util` would both become `gpui-pre-util`; refuse rather than publish the wrong one. */
function ensureUniqueNames(crates: Crate[]) {
  const byPublished = new Map<string, string[]>();
  for (const crate of crates) {
    byPublished.set(crate.publishedName, [...(byPublished.get(crate.publishedName) ?? []), crate.name]);
  }
  const clashes = [...byPublished].filter(([, names]) => names.length > 1);
  if (clashes.length > 0) {
    const detail = clashes.map(([published, names]) => `${published} <- ${names.join(", ")}`).join("\n  ");
    throw new BumpError(`these Zed crates would publish under the same name:\n  ${detail}`);
  }
}

interface DepEntry {
  tablePath: string[];
  name: string;
  spec: any;
}

/** Every dependency entry of a manifest, with the table it lives in. */
function depTables(manifest: Toml, dev = false): DepEntry[] {
  const names = dev ? [DEV_DEP_TABLE] : [...DEP_TABLES];
  const entries: DepEntry[] = [];
  for (const table of names) {
    for (const [name, spec] of Object.entries(manifest[table] ?? {})) {
      entries.push({ tablePath: [table], name, spec });
    }
  }
  for (const [cfg, cfgTables] of Object.entries(manifest.target ?? {})) {
    for (const table of names) {
      for (const [name, spec] of Object.entries((cfgTables as Toml)[table] ?? {})) {
        entries.push({ tablePath: ["target", cfg, table], name, spec });
      }
    }
  }
  return entries;
}

/** Merge a crate's dependency entry with its workspace definition. */
function effectiveSpec(ws: Workspace, name: string, spec: any): Toml {
  if (typeof spec === "string") return { version: spec };
  if (spec.workspace) {
    const base = workspaceDependencies(ws)[name];
    if (base === undefined) {
      throw new BumpError(`dependency \`${name}\` inherits from the workspace but is not defined there`);
    }
    const merged: Toml = typeof base === "string" ? { version: base } : { ...base };
    merged.optional = Boolean(spec.optional);
    merged.workspace = true;
    return merged;
  }
  return { ...spec };
}

/** The workspace-relative directory of a path dependency, if it is one. */
function pathDepTarget(crateDir: string, spec: Toml): string | undefined {
  if (spec.path === undefined) return undefined;
  return normalize(spec.workspace ? spec.path : join(crateDir, spec.path));
}

/** Every workspace crate the root crates need, in dependency order. */
function collectClosure(ws: Workspace): Crate[] {
  const byName = new Map<string, string>();
  for (const [rel, manifest] of ws.members) byName.set(manifest.package.name, rel);
  for (const root of ROOT_CRATES) {
    if (!byName.has(root)) throw new BumpError(`crate \`${root}\` was not found in the Zed workspace`);
  }

  const order: string[] = [];
  const visiting = new Set<string>();
  const done = new Set<string>();

  const visit = (relDir: string) => {
    if (done.has(relDir)) return;
    if (visiting.has(relDir)) throw new BumpError(`dependency cycle through ${relDir}`);
    visiting.add(relDir);
    const manifest = ws.members.get(relDir);
    if (manifest === undefined) throw new BumpError(`path dependency \`${relDir}\` is not a workspace member`);
    for (const { name, spec } of depTables(manifest)) {
      const target = pathDepTarget(relDir, effectiveSpec(ws, name, spec));
      if (target !== undefined) visit(target);
    }
    visiting.delete(relDir);
    done.add(relDir);
    order.push(relDir);
  };

  for (const root of ROOT_CRATES) visit(byName.get(root)!);

  return order.map((relDir) => {
    const manifest = ws.members.get(relDir)!;
    const name = manifest.package.name as string;
    return {
      relDir,
      manifest,
      name,
      version: String(manifest.package.version ?? "0.0.0"),
      publishedName: publishedName(name),
      prunedDeps: [],
      prunedFeatures: [],
    };
  });
}

/** crates.io needs a version for every dependency, git or not. */
function isPublishableSource(spec: Toml): boolean {
  if (spec.path !== undefined) return true;
  if (spec.git !== undefined) return spec.version !== undefined;
  return true;
}

// ---------------------------------------------------------------------------
// Pruning unpublishable optional dependencies
// ---------------------------------------------------------------------------

/** Split a feature entry into [dependency-or-feature, sub-feature]. */
function featureTargets(entry: string): [string, string | undefined] {
  if (entry.startsWith("dep:")) return [entry.slice(4), undefined];
  const slash = entry.indexOf("/");
  if (slash !== -1) return [entry.slice(0, slash).replace(/\?$/, ""), entry.slice(slash + 1)];
  return [entry, undefined];
}

function removeDependency(manifest: Toml, tablePath: string[], name: string) {
  let table = manifest;
  for (const key of tablePath) table = table[key];
  delete table[name];
}

/**
 * Remove dependencies crates.io would reject, and whatever only existed for them.
 *
 * Manifests are edited in place inside `ws.members`, so a later
 * `collectClosure` sees the pruned dependency graph.
 */
function pruneUnpublishable(ws: Workspace, crates: Crate[]) {
  const errors: string[] = [];
  for (const crate of crates) {
    const features: Record<string, string[]> = (crate.manifest.features ??= {});
    // Optional dependencies named with `dep:` get no implicit feature, so
    // once every feature that enabled them is gone nobody can turn them on.
    const explicitOnly = new Set(
      Object.values(features)
        .flat()
        .filter((entry) => entry.startsWith("dep:"))
        .map((entry) => featureTargets(entry)[0]),
    );

    const removed: string[] = [];
    for (const { tablePath, name, spec } of depTables(crate.manifest)) {
      const merged = effectiveSpec(ws, name, spec);
      if (isPublishableSource(merged)) continue;
      const source = merged.git ?? "?";
      if (!merged.optional) {
        errors.push(`${crate.name}: \`${name}\` comes from ${source} without a crates.io version`);
        continue;
      }
      removeDependency(crate.manifest, tablePath, name);
      removed.push(name);
      logWarn(`${crate.name}: dropping optional dependency \`${name}\` (${source} has no crates.io version)`);
    }

    const removedSet = new Set(removed);
    const droppedFeatures: string[] = [];
    for (const [feature, entries] of Object.entries(features)) {
      const needsRemoved = entries.filter((e) => e.startsWith("dep:") && removedSet.has(featureTargets(e)[0]));
      if (needsRemoved.length > 0) {
        // The feature exists to turn this dependency on; without it the
        // feature would enable code that cannot compile.
        delete features[feature];
        droppedFeatures.push(feature);
        logWarn(`${crate.name}: dropping feature \`${feature}\` (needs \`${featureTargets(needsRemoved[0])[0]}\`)`);
      } else {
        features[feature] = entries.filter((e) => !removedSet.has(featureTargets(e)[0]));
      }
    }

    const referenced = new Set(Object.values(features).flat().map((e) => featureTargets(e)[0]));
    for (const { tablePath, name, spec } of depTables(crate.manifest)) {
      const merged = effectiveSpec(ws, name, spec);
      if (merged.optional && explicitOnly.has(name) && !referenced.has(name)) {
        removeDependency(crate.manifest, tablePath, name);
        removed.push(name);
        logWarn(`${crate.name}: dropping optional dependency \`${name}\` (no feature enables it any more)`);
      }
    }

    let pruned = ws.pruned.get(crate.relDir);
    if (pruned === undefined) ws.pruned.set(crate.relDir, (pruned = { deps: [], features: [] }));
    pruned.deps.push(...removed);
    pruned.features.push(...droppedFeatures);
  }

  // Features that referenced a dropped feature, in this or another crate.
  const droppedByName = new Map(crates.map((c) => [c.name, new Set(ws.pruned.get(c.relDir)?.features ?? [])]));
  for (const crate of crates) {
    const depNames = new Map<string, string>();
    for (const { name, spec } of depTables(crate.manifest)) {
      const target = pathDepTarget(crate.relDir, effectiveSpec(ws, name, spec));
      if (target !== undefined) depNames.set(name, ws.members.get(target)!.package.name);
    }
    const features: Record<string, string[]> = crate.manifest.features ?? {};
    for (const [feature, entries] of Object.entries(features)) {
      features[feature] = entries.filter((entry) => {
        const [dep, sub] = featureTargets(entry);
        if (sub === undefined) return !droppedByName.get(crate.name)?.has(dep);
        return !droppedByName.get(depNames.get(dep) ?? "")?.has(sub);
      });
    }
  }

  if (errors.length > 0) {
    throw new BumpError(`these dependencies cannot be published to crates.io:\n  ${errors.join("\n  ")}`);
  }
}

/** Compute the publish set, pruning until the dependency graph is stable. */
function selectCrates(ws: Workspace): Crate[] {
  let crates = collectClosure(ws);
  for (;;) {
    pruneUnpublishable(ws, crates);
    const again = collectClosure(ws);
    if (again.map((c) => c.relDir).join("\n") === crates.map((c) => c.relDir).join("\n")) break;
    crates = again;
  }
  for (const crate of crates) {
    const pruned = ws.pruned.get(crate.relDir);
    crate.prunedDeps = pruned?.deps ?? [];
    crate.prunedFeatures = pruned?.features ?? [];
  }
  ensureUniqueNames(crates);
  return crates;
}

// ---------------------------------------------------------------------------
// Manifest generation
// ---------------------------------------------------------------------------

const BARE_KEY = /^[A-Za-z0-9_-]+$/;

function tomlKey(key: string): string {
  if (BARE_KEY.test(key)) return key;
  if (!key.includes("'")) return `'${key}'`;
  return JSON.stringify(key);
}

const isPlainObject = (value: unknown): value is Toml =>
  typeof value === "object" && value !== null && !Array.isArray(value) && !(value instanceof Date);

function tomlScalar(value: unknown): string {
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "number") return String(value);
  if (typeof value === "string") return JSON.stringify(value);
  if (Array.isArray(value)) {
    const items = value.map(tomlScalar);
    const joined = items.join(", ");
    if (joined.length > 80) return `[\n${items.map((item) => `    ${item},\n`).join("")}]`;
    return `[${joined}]`;
  }
  if (isPlainObject(value)) {
    const inner = Object.entries(value)
      .map(([k, v]) => `${tomlKey(k)} = ${tomlScalar(v)}`)
      .join(", ");
    return inner ? `{ ${inner} }` : "{}";
  }
  throw new BumpError(`cannot serialize ${typeof value} to TOML`);
}

function isDepTable(path: string[]): boolean {
  const last = path.at(-1);
  if (last !== undefined && [...DEP_TABLES, DEV_DEP_TABLE].includes(last as any)) return true;
  return path.length === 2 && path[0] === "workspace" && path[1] === "dependencies";
}

function inlineDict(path: string[], value: Toml): boolean {
  if (path.length === 0) return false;
  if (isDepTable(path)) return true;
  if ("workspace" in value) return true;
  // `[lints.clippy] style = { level = "allow", priority = -1 }` and
  // `[profile.dev.package] foo = { opt-level = 3 }` stay one entry per line.
  const last = path.at(-1)!;
  return (path.includes("lints") || path.includes("profile")) && last !== "lints" && last !== "profile";
}

function emitTable(lines: string[], path: string[], table: Toml) {
  const values: [string, unknown][] = [];
  const subtables: [string, Toml][] = [];
  const arraysOfTables: [string, Toml[]][] = [];
  for (const [key, value] of Object.entries(table)) {
    if (isPlainObject(value) && !inlineDict(path, value)) {
      subtables.push([key, value]);
    } else if (Array.isArray(value) && value.length > 0 && value.every(isPlainObject)) {
      arraysOfTables.push([key, value]);
    } else {
      values.push([key, value]);
    }
  }

  if (path.length > 0 && (values.length > 0 || (subtables.length === 0 && arraysOfTables.length === 0))) {
    if (lines.length > 0) lines.push("");
    lines.push(`[${path.map(tomlKey).join(".")}]`);
  }
  for (const [key, value] of values) lines.push(`${tomlKey(key)} = ${tomlScalar(value)}`);
  for (const [key, value] of subtables) emitTable(lines, [...path, key], value);
  for (const [key, items] of arraysOfTables) {
    const header = [...path, key].map(tomlKey).join(".");
    for (const item of items) {
      lines.push("", `[[${header}]]`);
      for (const [k, v] of Object.entries(item)) lines.push(`${tomlKey(k)} = ${tomlScalar(v)}`);
    }
  }
}

function tomlDump(document: Toml): string {
  const lines: string[] = [];
  emitTable(lines, [], document);
  return `${lines.join("\n")}\n`;
}

const withoutSource = (spec: Toml) =>
  Object.fromEntries(Object.entries(spec).filter(([k]) => !["path", "package", "version"].includes(k)));

function crateManifest(crate: Crate, cratesByDir: Map<string, Crate>, version: string, zedSha: string): Toml {
  const source = crate.manifest;
  const pkg: Toml = { ...source.package };
  if (pkg.license === undefined && pkg["license-file"] === undefined) {
    throw new BumpError(`${crate.name}: no \`license\` in Cargo.toml; crates.io requires one`);
  }

  const snapshot = `(${PUBLISH_PREFIX} snapshot of zed@${zedSha.slice(0, 7)})`;
  const description: string = pkg.description || `Zed's \`${crate.name}\` crate`;
  pkg.name = crate.publishedName;
  pkg.version = version;
  pkg.publish = true;
  pkg.description = `${description.replace(/\.+$/, "")} ${snapshot}`;
  pkg.repository ??= ZED_GIT_URL;
  pkg.metadata = {
    ...(pkg.metadata ?? {}),
    [PUBLISH_PREFIX]: { "zed-crate": crate.name, "zed-version": crate.version, "zed-rev": zedSha },
  };

  const lib: Toml = { ...(source.lib ?? {}) };
  lib.name ??= crate.name;

  const out: Toml = { package: pkg, lib };
  if (source.features !== undefined) out.features = source.features;

  const rewriteTable = (table: Toml): Toml => {
    const rewritten: Toml = {};
    for (const [name, spec] of Object.entries(table)) {
      let entry = spec;
      if (isPlainObject(spec) && !spec.workspace && spec.path !== undefined) {
        const target = pathDepTarget(crate.relDir, spec);
        const dep = cratesByDir.get(target ?? "");
        if (dep === undefined) {
          throw new BumpError(`${crate.name}: path dependency \`${name}\` (${target}) is not being published`);
        }
        entry = {
          path: relative(crate.relDir, dep.relDir),
          package: dep.publishedName,
          version: `=${version}`,
          ...withoutSource(spec),
        };
      }
      rewritten[name] = entry;
    }
    return rewritten;
  };

  for (const table of DEP_TABLES) {
    if (source[table] !== undefined) out[table] = rewriteTable(source[table]);
  }
  if (source.target !== undefined) {
    const targets: Toml = {};
    for (const [cfg, cfgTables] of Object.entries(source.target as Toml)) {
      const kept: Toml = {};
      for (const [table, value] of Object.entries(cfgTables as Toml)) {
        if (table !== DEV_DEP_TABLE) kept[table] = rewriteTable(value as Toml);
      }
      if (Object.keys(kept).length > 0) targets[cfg] = kept;
    }
    if (Object.keys(targets).length > 0) out.target = targets;
  }

  for (const [key, value] of Object.entries(source)) {
    if (!(key in out) && ![DEV_DEP_TABLE, "target", "workspace"].includes(key)) out[key] = value;
  }
  return out;
}

function workspaceManifest(ws: Workspace, crates: Crate[], version: string): Toml {
  const zedWs = ws.manifest.workspace;
  const byDir = new Map(crates.map((c) => [c.relDir, c]));
  const dependencies: Toml = {};
  for (const [name, spec] of Object.entries(workspaceDependencies(ws))) {
    if (isPlainObject(spec) && spec.path !== undefined) {
      const crate = byDir.get(normalize(spec.path));
      if (crate === undefined) continue;
      dependencies[name] = {
        path: crate.relDir,
        package: crate.publishedName,
        version: `=${version}`,
        ...withoutSource(spec),
      };
    } else {
      dependencies[name] = spec;
    }
  }

  return {
    workspace: {
      resolver: zedWs.resolver ?? "2",
      members: crates.map((c) => c.relDir),
      package: { ...(zedWs.package ?? {}), publish: true },
      dependencies,
      lints: zedWs.lints ?? {},
    },
  };
}

// ---------------------------------------------------------------------------
// Staging
// ---------------------------------------------------------------------------

function copyCrate(source: string, destination: string) {
  cpSync(source, destination, {
    recursive: true,
    dereference: true,
    force: true,
    filter: (path) => {
      const name = path.split("/").at(-1);
      if (name === "target" || name === ".git") return false;
      try {
        statSync(path); // drops dangling symlinks, which `dereference` cannot copy
        return true;
      } catch {
        return false;
      }
    },
  });
}

function stageWorkspace(ws: Workspace, crates: Crate[], version: string, zedSha: string): string {
  const staging = join(WORK_DIR, "workspace");
  if (existsSync(staging)) {
    for (const entry of readdirSync(staging)) {
      if (entry === "target") continue; // keep the build cache between runs
      rmSync(join(staging, entry), { recursive: true, force: true });
    }
  }
  mkdirSync(staging, { recursive: true });

  for (const crate of crates) copyCrate(join(ws.root, crate.relDir), join(staging, crate.relDir));

  const cratesByDir = new Map(crates.map((c) => [c.relDir, c]));
  for (const crate of crates) {
    const manifest = crateManifest(crate, cratesByDir, version, zedSha);
    writeFileSync(join(staging, crate.relDir, "Cargo.toml"), tomlDump(manifest));
  }
  vendorGpuiSourcesForApple(staging, crates);

  writeFileSync(join(staging, "Cargo.toml"), tomlDump(workspaceManifest(ws, crates, version)));
  const lock = join(ws.root, "Cargo.lock");
  if (existsSync(lock)) cpSync(lock, join(staging, "Cargo.lock"));
  for (const entry of readdirSync(ws.root)) {
    if (entry.startsWith("LICENSE") && statSync(join(ws.root, entry)).isFile()) {
      cpSync(join(ws.root, entry), join(staging, entry));
    }
  }

  const summary = {
    version,
    zed: { url: ZED_GIT_URL, rev: zedSha },
    crates: crates.map((c) => ({
      name: c.publishedName,
      zed_name: c.name,
      zed_version: c.version,
      path: c.relDir,
      dropped_dependencies: c.prunedDeps,
      dropped_features: c.prunedFeatures,
    })),
  };
  writeFileSync(join(WORK_DIR, "gpui-pre.json"), `${JSON.stringify(summary, null, 2)}\n`);
  return staging;
}

const GPUI_APPLE_SIBLING = '.join("../gpui")';
const GPUI_APPLE_VENDORED = '.join("vendor/gpui")';

/**
 * Make `gpui_apple`'s build script work outside the Zed workspace.
 *
 * Its `build.rs` feeds a few `gpui` source files to cbindgen to generate the
 * Metal shader header, and it finds them at `../gpui`. A crate unpacked from
 * crates.io has no such sibling, so copy exactly the files it names into the
 * crate and point the build script at the copy.
 */
function vendorGpuiSourcesForApple(staging: string, crates: Crate[]) {
  const apple = crates.find((c) => c.name === "gpui_apple");
  const gpui = crates.find((c) => c.name === "gpui");
  if (apple === undefined || gpui === undefined) return;
  const buildRs = join(staging, apple.relDir, "build.rs");
  const text = readFileSync(buildRs, "utf8");
  if (!text.includes(GPUI_APPLE_SIBLING)) {
    throw new BumpError(
      "crates/gpui_apple/build.rs no longer locates gpui with `../gpui`; " +
        "update vendorGpuiSourcesForApple in script/bump-gpui.ts",
    );
  }
  const sources = [...text.matchAll(/gpui_dir\.join\("([^"]+)"\)/g)].map((m) => m[1]);
  if (sources.length === 0) {
    throw new BumpError("crates/gpui_apple/build.rs lists no `gpui_dir.join(...)` sources; update script/bump-gpui.ts");
  }
  const vendor = join(staging, apple.relDir, "vendor", "gpui");
  for (const rel of sources) {
    const source = join(staging, gpui.relDir, rel);
    if (!existsSync(source)) throw new BumpError(`gpui_apple/build.rs needs ${rel}, which gpui does not have`);
    const destination = join(vendor, rel);
    mkdirSync(dirname(destination), { recursive: true });
    cpSync(source, destination);
  }
  writeFileSync(buildRs, text.replaceAll(GPUI_APPLE_SIBLING, GPUI_APPLE_VENDORED));
  logInfo(`gpui_apple: vendored ${sources.length} gpui source files for its shader bindings`);
}

// ---------------------------------------------------------------------------
// crates.io
// ---------------------------------------------------------------------------

async function cratesIoGet(path: string): Promise<Toml | undefined> {
  let response: Response;
  try {
    response = await fetch(`${CRATES_IO_API}/${path}`, { headers: { "User-Agent": USER_AGENT } });
  } catch (error) {
    throw new BumpError(`cannot reach crates.io: ${(error as Error).message}`);
  }
  if (response.status === 404) return undefined;
  if (!response.ok) throw new BumpError(`crates.io returned ${response.status} for ${path}`);
  return (await response.json()) as Toml;
}

async function versionIsPublished(name: string, version: string): Promise<boolean> {
  const data = await cratesIoGet(`${name}/${version}`);
  return data?.version !== undefined && !data.version.yanked;
}

async function unpublished(crates: Crate[], version: string): Promise<Crate[]> {
  const pending: Crate[] = [];
  for (const crate of crates) {
    if (!(await versionIsPublished(crate.publishedName, version))) pending.push(crate);
    await Bun.sleep(200); // be polite to the crates.io API
  }
  return pending;
}

const RATE_LIMIT_MARKERS = ["429", "Too Many Requests", "too many new crates", "too many crates"];

/** When crates.io says to try again, return that instant. */
function rateLimitDeadline(output: string): Date | undefined {
  if (!RATE_LIMIT_MARKERS.some((marker) => output.includes(marker))) return undefined;
  const match = /try again after ([^.\n]+?)(?: or |\.|$)/m.exec(output);
  if (match) {
    const parsed = new Date(match[1].trim());
    if (!Number.isNaN(parsed.getTime())) return new Date(parsed.getTime() + 15_000);
  }
  return new Date(Date.now() + RATE_LIMIT_FALLBACK_MS);
}

async function waitUntil(deadline: Date) {
  for (;;) {
    const remaining = deadline.getTime() - Date.now();
    if (remaining <= 0) break;
    const total = Math.floor(remaining / 1000);
    const minutes = String(Math.floor(total / 60)).padStart(2, "0");
    const seconds = String(total % 60).padStart(2, "0");
    process.stdout.write(`\r${paint("36", "ℹ")} crates.io rate limit; retrying in ${minutes}:${seconds} `);
    await Bun.sleep(Math.min(30_000, remaining));
  }
  console.log();
}

async function publish(staging: string, crates: Crate[], version: string, wait: boolean) {
  for (;;) {
    const pending = await unpublished(crates, version);
    if (pending.length === 0) {
      logSuccess(`All ${crates.length} crates are on crates.io at ${bold(version)}`);
      return;
    }
    const already = crates.filter((c) => !pending.includes(c));
    if (already.length > 0) logInfo(`Skipping ${already.length} crates already published at ${version}`);
    logInfo(`Publishing ${pending.length} crates: ${pending.map((c) => c.publishedName).join(", ")}`);

    const cmd = ["cargo", "publish", "--workspace", "--no-verify", "--allow-dirty"];
    for (const crate of already) cmd.push("--exclude", crate.publishedName);
    const { code, output } = await runStreaming(cmd, staging);
    if (code === 0) {
      logSuccess(`Published ${pending.length} crates`);
      return;
    }

    const deadline = rateLimitDeadline(output);
    if (deadline === undefined) throw new BumpError("cargo publish failed; fix the error above and re-run to resume");
    if (!wait) {
      throw new BumpError(
        "crates.io rate limit reached (new crates are limited per 10 minutes); re-run this command later to resume",
      );
    }
    await waitUntil(deadline);
  }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

const SEMVER =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/;

const USAGE = `Usage: script/bump-gpui.ts [VERSION] [options]

Publish Zed's GPUI crates to crates.io as ${PUBLISH_PREFIX}-*.

Arguments:
  VERSION           override the VERSION constant in this script for one run
                    (current constant: ${VERSION})

Options:
  --rev REV         Zed branch, tag or commit (default: ${ZED_DEFAULT_REV})
  --zed PATH        use this Zed checkout instead of fetching one
  --dry-run         stage and verify, but do not publish
  --stage-only      stage the workspace and stop
  --no-verify       skip \`cargo publish --dry-run\` verification
  --no-wait         abort instead of waiting on the crates.io rate limit
  -h, --help        show this help
`;

interface Args {
  version?: string;
  rev: string;
  zed?: string;
  dryRun: boolean;
  stageOnly: boolean;
  noVerify: boolean;
  noWait: boolean;
}

function parseCommandLine(argv: string[]): Args {
  let parsed: ReturnType<typeof parseArgs>;
  try {
    parsed = parseArgs({
      args: argv,
      allowPositionals: true,
      options: {
        rev: { type: "string", default: ZED_DEFAULT_REV },
        zed: { type: "string" },
        "dry-run": { type: "boolean", default: false },
        "stage-only": { type: "boolean", default: false },
        "no-verify": { type: "boolean", default: false },
        "no-wait": { type: "boolean", default: false },
        help: { type: "boolean", short: "h", default: false },
      },
    });
  } catch (error) {
    throw new BumpError(`${(error as Error).message}\n\n${USAGE}`);
  }
  if (parsed.values.help) {
    console.log(USAGE);
    process.exit(0);
  }
  if (parsed.positionals.length > 1) throw new BumpError(`unexpected argument \`${parsed.positionals[1]}\`\n\n${USAGE}`);
  const version = parsed.positionals[0];
  if (version !== undefined && !SEMVER.test(version)) throw new BumpError(`\`${version}\` is not a valid semver version`);
  return {
    version,
    rev: parsed.values.rev as string,
    zed: parsed.values.zed as string | undefined,
    dryRun: parsed.values["dry-run"] as boolean,
    stageOnly: parsed.values["stage-only"] as boolean,
    noVerify: parsed.values["no-verify"] as boolean,
    noWait: parsed.values["no-wait"] as boolean,
  };
}

async function main(argv: string[]): Promise<number> {
  const args = parseCommandLine(argv);
  if (Bun.which("cargo") === null) throw new BumpError("cargo is not installed");

  const totalSteps = args.stageOnly ? 3 : args.dryRun ? 4 : 5;
  logHeader(`Publishing GPUI from Zed as ${PUBLISH_PREFIX}`);
  mkdirSync(WORK_DIR, { recursive: true });

  logStep(`1/${totalSteps}`, "Preparing the Zed checkout");
  const zed = await prepareZed(args.rev, args.zed);
  const zedSha = await zedRevision(zed);
  logSuccess(`Zed at ${bold(zedSha.slice(0, 12))}`);
  console.log();

  logStep(`2/${totalSteps}`, "Collecting the crates that gpui needs");
  const ws = loadWorkspace(zed);
  const crates = selectCrates(ws);
  const version = args.version ?? VERSION;
  const width = Math.max(...crates.map((c) => c.name.length));
  for (const crate of crates) console.log(`    ${crate.name.padEnd(width)}  ->  ${crate.publishedName}`);
  logSuccess(`${crates.length} crates will be published as version ${bold(version)}`);
  console.log();

  logStep(`3/${totalSteps}`, "Staging a standalone workspace");
  const staging = stageWorkspace(ws, crates, version, zedSha);
  logSuccess(`Workspace written to ${bold(staging)}`);
  logInfo(`Summary written to ${join(WORK_DIR, "gpui-pre.json")}`);
  console.log();
  if (args.stageOnly) return 0;

  if (args.noVerify) {
    logWarn("Skipping verification (--no-verify)");
  } else {
    logStep(`4/${totalSteps}`, "Verifying with `cargo publish --dry-run`");
    const { code } = await runStreaming(["cargo", "publish", "--workspace", "--dry-run", "--allow-dirty"], staging);
    if (code !== 0) throw new BumpError("verification failed; inspect the staged workspace and fix the issue");
    logSuccess("Every crate packages and builds");
  }
  console.log();
  if (args.dryRun) {
    logInfo("Dry run complete; nothing was uploaded");
    return 0;
  }

  logStep(`5/${totalSteps}`, "Publishing to crates.io");
  await publish(staging, crates, version, !args.noWait);
  console.log();

  console.log(paint("1;32", `╔${"═".repeat(56)}╗`));
  console.log(`${paint("1;32", "║")}  ${bold(`🚀 ${PUBLISH_PREFIX} ${version} is live (zed@${zedSha.slice(0, 7)})`)}`);
  console.log(paint("1;32", `╚${"═".repeat(56)}╝`));
  console.log();
  console.log("Depend on it with:");
  console.log();
  console.log("    [workspace.dependencies]");
  for (const crate of crates) {
    if (ROOT_CRATES.includes(crate.name)) {
      console.log(`    ${crate.name} = { package = "${crate.publishedName}", version = "=${version}" }`);
    }
  }
  console.log();
  return 0;
}

try {
  process.exit(await main(process.argv.slice(2)));
} catch (error) {
  if (error instanceof BumpError) {
    logError(error.message);
    process.exit(1);
  }
  throw error;
}
