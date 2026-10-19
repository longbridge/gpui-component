#!/usr/bin/env bun
/** Keep the NSH module inventory honest as Component's public surface grows. */
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, relative, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const read = (path: string) => readFileSync(resolve(root, path), "utf8");
function files(directory: string, extension: string): string[] {
  return readdirSync(resolve(root, directory), { withFileTypes: true })
    .flatMap((entry) => {
      const path = `${directory}/${entry.name}`;
      return entry.isDirectory() ? files(path, extension) : path.endsWith(extension) ? [path] : [];
    }).sort();
}

/** Value exports of the generated NSH module, excluding all type exports. */
function generatedValues(source: string): string[] {
  return Array.from(source.matchAll(/\bexport\s*\{([^}]*)\}\s*from\s*["']nsh\/ui\/gpui["']/g))
    .flatMap((match) => match[1].split(",").map((item) => item.trim()))
    .filter((item) => item && !/^type\s/.test(item))
    .map((item) => item.split(/\s+as\s+/)[0].trim())
    .filter((item) => /^[A-Za-z_$][\w$]*$/.test(item));
}

/** Source evidence only: declarations do not establish runtime coverage. */
function registrations(paths: string[]): Map<string, string[]> {
  const result = new Map<string, string[]>();
  for (const path of paths) {
    const source = read(path);
    const declared = Array.from(source.matchAll(/["']gpui-component\.([A-Za-z0-9_]+)["']/g), (match) => match[1]);
    if (path.endsWith(".ts")) declared.push(...generatedValues(source));
    // Resolve literal name lists in small registration macros without asking
    // their callers to repeat the same names in a second metadata file.
    for (const [, macro, body] of source.matchAll(/macro_rules!\s+(\w+)\s*\{([^\n]*)/g)) {
      if (!body.includes('concat!("gpui-component.",stringify!(') || !body.includes("registry.register")) continue;
      const invocation = new RegExp(`\\b${macro}!\\(\\s*([A-Za-z_][\\w\\s,]*)\\)`, "g");
      for (const [, names] of source.matchAll(invocation)) {
        declared.push(...names.split(",").map((name) => name.trim()).filter(Boolean));
      }
    }
    // A common registry loop uses format!("gpui-component.{name}"). Resolve
    // only its literal array, never infer registrations from arbitrary strings.
    for (const [, variable, names, body] of source.matchAll(/for\s+(\w+)\s+in\s*\[([^\]]*)\]\s*\{([\s\S]*?components\.register\([\s\S]*?\);)/g)) {
      if (body.includes(`gpui-component.{${variable}}`)) {
        declared.push(...Array.from(names.matchAll(/"([A-Za-z0-9_]+)"/g), (match) => match[1]));
      }
    }
    for (const binding of declared) {
      const sources = result.get(binding) ?? [];
      if (!sources.includes(path)) sources.push(path);
      result.set(binding, sources);
    }
  }
  return result;
}
const source = read("crates/component/src/lib.rs");
const modules = new Set(Array.from(source.matchAll(/^pub mod ([a-z_]+);/gm), (match) => match[1]));
for (const [, module] of source.matchAll(/^pub use ([a-z_]+)::/gm)) {
  if (!["crate", "gpui_base", "gpui_component_macros"].includes(module)) modules.add(module);
}

interface Entry {
  module: string;
  status: "infrastructure" | "pending" | "partial" | "complete";
  bindings: string[];
  reason?: string;
  coverage?: {
    publicApiReview: string;
    verification: string[];
  };
}
const catalog = JSON.parse(read("crates/nsh/catalog.json")) as {
  modules: Entry[];
  platformBindings?: { binding: string; source: string; status: "partial" | "complete"; reason: string }[];
};
const errors: string[] = [];
const seen = new Set<string>();
const bindings = new Set<string>();
for (const entry of catalog.modules) {
  if (seen.has(entry.module)) errors.push(`duplicate module ${entry.module}`);
  seen.add(entry.module);
  if (!modules.has(entry.module)) errors.push(`removed or unknown module ${entry.module}`);
  if (!["infrastructure", "pending", "partial", "complete"].includes(entry.status)) errors.push(`invalid status for ${entry.module}`);
  if (entry.status === "infrastructure" && !entry.reason) errors.push(`missing infrastructure reason for ${entry.module}`);
  if (["partial", "complete"].includes(entry.status) && entry.bindings.length === 0) errors.push(`no bindings listed for ${entry.module}`);
  if (["pending", "infrastructure"].includes(entry.status) && entry.bindings.length > 0) errors.push(`unexpected bindings for ${entry.module}`);
  if (entry.status === "complete" && (!entry.coverage?.publicApiReview || !Array.isArray(entry.coverage.verification) || !entry.coverage.verification.length)) {
    errors.push(`complete module ${entry.module} needs a public API review and verification evidence`);
  }
  if (process.argv.includes("--require-complete") && ["pending", "partial"].includes(entry.status)) {
    errors.push(`unfinished module ${entry.module}: ${entry.status}`);
  }
  for (const binding of entry.bindings) {
    if (bindings.has(binding)) errors.push(`duplicate binding ${binding}`);
    bindings.add(binding);
  }
}
for (const module of modules) {
  if (!seen.has(module)) errors.push(`missing module ${module}`);
}
for (const entry of catalog.platformBindings ?? []) {
  if (!entry.source || !entry.reason) errors.push(`missing platform binding explanation for ${entry.binding}`);
  if (bindings.has(entry.binding)) errors.push(`duplicate binding ${entry.binding}`);
  bindings.add(entry.binding);
}

const rustFiles = files("crates/nsh/src", ".rs");
const tsFiles = files("crates/nsh/js", ".ts");
const rust = registrations(rustFiles);
const ts = registrations(tsFiles);
// A forwarding entry has no component list to maintain. Its exports come from
// the Rust registry; CLI type generation and execution are separate checks.
const generatedForwarders = tsFiles.filter((path) => /\bexport\s*\*\s*from\s*["']nsh\/ui\/gpui["']/.test(read(path)));
if (generatedForwarders.length > 1) errors.push("multiple generated module compatibility entries");
for (const source of generatedForwarders) {
  for (const binding of rust.keys()) {
    if (!ts.has(binding)) ts.set(binding, [source]);
  }
}
for (const [label, actual] of [["Rust", rust], ["TSX", ts]] as const) {
  for (const binding of bindings) if (!actual.has(binding)) errors.push(`${label} is missing ${binding}`);
  for (const binding of actual.keys()) if (!bindings.has(binding)) errors.push(`inventory is missing ${label} binding ${binding}`);
  for (const [binding, sources] of actual) {
    if (sources.length > 1) errors.push(`duplicate ${label} declaration for ${binding}: ${sources.join(", ")}`);
  }
}

const reportIndex = process.argv.indexOf("--report");
if (reportIndex !== -1) {
  const argument = process.argv[reportIndex + 1];
  if (!argument || argument.startsWith("--")) throw new Error("--report requires an output path");
  const path = resolve(root, argument);
  const publicSources = files("crates/component/src", ".rs");
  const publicDeclarations = publicSources.flatMap((source) => {
    const content = read(source);
    return Array.from(content.matchAll(/^\s*pub\s+(?:(?:async|const|unsafe)\s+)?(?:struct|enum|trait|type|fn|const|static)\s+([A-Za-z_][A-Za-z0-9_]*)[^\n]*/gm), (match) => {
      const start = match.index + match[0].indexOf("pub");
      const declaration = content.slice(start).split(/[;{]/, 1)[0].trim();
      return {
        source,
        line: content.slice(0, start).split("\n").length,
        name: match[1],
        declaration,
        review: "unreviewed",
      };
    });
  });
  const renderableDeclarations = publicSources.flatMap((source) => {
    const content = read(source);
    const publicStructs = new Set(Array.from(content.matchAll(/\bpub struct (\w+)/g), (match) => match[1]));
    return Array.from(content.matchAll(/impl(?:<[^\n]*>)?\s+(?:RenderOnce|Render)\s+for\s+(\w+)/g), (match) => match[1])
      .filter((name) => publicStructs.has(name))
      .map((name) => ({
        name, source,
        matchingBinding: ts.has(name) ? name : null,
        review: "unreviewed",
        note: "A component part or state may map through a parent binding or named slot. A matching name does not prove complete support; absence needs an explicit mapping review.",
      }));
  });
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify({
    schemaVersion: 1,
    scope: "Source declarations and registration names. Declared props/events/commands exclude inherited NSH native props. Each declaration requires explicit review; this is not a claim of NSH API parity or runtime verification. Trait methods, generic implementations, macro-generated items and inherited extension traits need additional review.",
    bindings: [...new Set([...bindings, ...rust.keys(), ...ts.keys()])].sort().map((binding) => ({
      binding,
      module: catalog.modules.find((entry) => entry.bindings.includes(binding))?.module,
      rustSources: rust.get(binding) ?? [],
      tsSources: ts.get(binding) ?? [],
      declarations: (ts.get(binding) ?? []).map((source) => {
        const content = read(source);
        const start = content.indexOf(`gpui-component.${binding}`);
        if (start === -1 && (generatedValues(content).includes(binding) || generatedForwarders.includes(source))) {
          return {
            source,
            entry: "generated-module-value-export",
            module: "nsh/ui/gpui",
            props: null, events: null, commands: null,
            note: "Generated from Rust native specs. This source export does not prove the generated declarations or runtime factory have been validated.",
          };
        }
        const options = content.slice(start, content.indexOf("});", start) + 3);
        const arrayNames = (array: string, seen = new Set<string>()): string[] => {
          const result = Array.from(array.matchAll(/["']([^"']+)["']/g), (match) => match[1]);
          for (const [, spread] of array.matchAll(/\.\.\.(\w+)/g)) {
            if (seen.has(spread)) continue;
            seen.add(spread);
            const definition = content.match(new RegExp(`\\bconst\\s+${spread}\\s*=\\s*\\[([^\\]]*)\\]`));
            if (definition) result.push(...arrayNames(definition[1], seen));
          }
          return [...new Set(result)].sort();
        };
        const names = (field: string) => {
          const inline = options.match(new RegExp(`\\b${field}\\s*:\\s*\\[([^\\]]*)\\]`));
          if (inline) return arrayNames(inline[1]);
          const reference = options.match(new RegExp(`\\b${field}\\s*:\\s*(\\w+)`))?.[1];
          const definition = reference && content.match(new RegExp(`\\bconst\\s+${reference}\\s*=\\s*\\[([^\\]]*)\\]`));
          return definition ? arrayNames(definition[1]) : [];
        };
        return { source, props: names("props"), events: names("events"), commands: names("commands") };
      }),
      commands: "requires adapter-specific review",
      slots: "requires bridge and adapter-specific review",
      retainedState: "requires adapter-specific review",
      featuresAndPlatforms: "requires adapter-specific review",
      review: "unreviewed",
    })),
    publicDeclarations,
    renderableDeclarations,
    errors,
  }, null, 2)}\n`);
  console.log(`Wrote source evidence to ${relative(root, path)}`);
}
if (errors.length) {
  console.error(errors.join("\n"));
  process.exit(1);
}
const count = (status: Entry["status"]) => catalog.modules.filter((entry) => entry.status === status).length;
console.log(`NSH inventory: ${count("complete")} complete, ${count("partial")} partial, ${count("pending")} pending modules; ${count("infrastructure")} infrastructure modules; ${bindings.size} bindings`);
console.log("This checks inventory consistency, not runtime behavior or full public API coverage.");
