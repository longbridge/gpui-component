#!/usr/bin/env node
// Static coverage audit for the JavaScript scaffold. It reads the canonical
// component-shell inventory and the explicit catalog imports without loading
// gpui, so it remains runnable while the adapter is still being built.
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const fixtureDirectory = dirname(fileURLToPath(import.meta.url));
const repository = resolve(fixtureDirectory, "../../..");
const read = (path) => readFileSync(resolve(repository, path), "utf8");
const fail = (message) => {
  throw new Error(`JavaScript Story coverage: ${message}`);
};

const inventory = JSON.parse(
  read("crates/component-shell/component-inventory.json"),
);
const catalogSource = read("examples/js_story/catalog.js");
const coverageSource = read("examples/js_story/stories/coverage.js");
const familyFiles = [
  ...catalogSource.matchAll(/from "\.\/stories\/([^"\n]+)"/g),
].map((match) => `examples/js_story/stories/${match[1]}`);

if (familyFiles.length === 0)
  fail("catalog.js does not explicitly import a family module");

const records = familyFiles.flatMap((file) => {
  const source = read(file);
  return [...source.matchAll(/pendingStory\(\{([\s\S]*?)\}\)/g)].map(
    (match) => {
      const field = (name) =>
        match[1].match(new RegExp(`${name}: "([^"]+)"`))?.[1];
      return {
        id: field("id"),
        rustStory: field("rustStory"),
        api: field("api"),
        availability: field("availability"),
        file,
      };
    },
  );
});

const inventoryStories = inventory.items.filter(
  (item) => item.source === "story",
);
const inventorySurfaces = new Map();
for (const item of inventory.items) {
  if (!item.registration) continue;
  const surface =
    item.registration.status === "registered"
      ? (item.registration.api ?? item.registration.descriptor)
      : item.registration.target;
  const current = inventorySurfaces.get(surface);
  const next = {
    status: item.registration.status,
    category: item.registration.category,
  };
  if (
    current &&
    (current.status !== next.status || current.category !== next.category)
  ) {
    fail(`inventory disagrees about ${surface} status`);
  }
  inventorySurfaces.set(surface, next);
}
const renderableRegistrations = [...inventorySurfaces.keys()].sort();
const inventoryNameFor = (rustStory) => {
  const name = rustStory.replace(/Story$/, "");
  if (name === "ThemeColors") return "theme";
  return name.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
};

if (new Set(records.map((record) => record.id)).size !== records.length) {
  fail("catalog route ids are not unique");
}
if (records.length !== inventoryStories.length) {
  fail(
    `catalog has ${records.length} routes; inventory has ${inventoryStories.length} Story entries`,
  );
}

for (const item of inventoryStories) {
  const record = records.find(
    (candidate) => inventoryNameFor(candidate.rustStory) === item.name,
  );
  if (!record) fail(`inventory Story ${item.name} has no catalog route`);
  if (item.classification === "infrastructure") {
    if (
      record.id !== "introduction" &&
      record.id !== "shell" &&
      record.availability !== "infrastructure"
    ) {
      fail(`${record.id} must declare infrastructure availability`);
    }
  } else {
    const registration =
      item.registration.status === "registered"
        ? (item.registration.api ?? item.registration.descriptor)
        : item.registration.target;
    if (record.api === registration) continue;
    fail(
      `${record.id} expects ${record.api}; inventory tracks ${registration} as ${item.registration.status}`,
    );
  }
}

const order = catalogSource
  .match(/const RUST_STORY_ORDER = \[([\s\S]*?)\];/)?.[1]
  .match(/"([^"]+)"/g)
  ?.map((name) => name.slice(1, -1));
if (!order || order.length !== records.length) {
  fail("catalog order does not enumerate every route");
}
if (
  new Set(order).size !== order.length ||
  order.some((name) => !records.some((record) => record.rustStory === name))
) {
  fail("catalog order and family route records disagree");
}

const coverageBody = coverageSource.match(
  /export const coveredBy = \[([\s\S]*?)\n\];/,
)?.[1];
if (!coverageBody) fail("coverage.js has no explicit coveredBy metadata");
const coverage = [
  ...coverageBody.matchAll(
    /\{ route: "([^"]+)", registrations: \[([^\]]*)\] \}/g,
  ),
].map((match) => ({
  route: match[1],
  registrations: [...match[2].matchAll(/"([^"]+)"/g)].map(
    (registration) => registration[1],
  ),
}));
if (coverage.length !== records.length) {
  fail(
    `coveredBy has ${coverage.length} route entries; catalog has ${records.length}`,
  );
}

const catalogIds = new Set(records.map((record) => record.id));
if (
  new Set(coverage.map((entry) => entry.route)).size !== coverage.length ||
  coverage.some((entry) => !catalogIds.has(entry.route))
) {
  fail("coveredBy routes are not a one-to-one match for catalog routes");
}

for (const record of records) {
  const entry = coverage.find((candidate) => candidate.route === record.id);
  const inventoryItem = inventoryStories.find(
    (item) => item.name === inventoryNameFor(record.rustStory),
  );
  if (
    inventoryItem?.classification !== "infrastructure" &&
    !entry.registrations.includes(record.api)
  ) {
    fail(`${record.id} must explicitly cover its ${record.api} registration`);
  }
}

const coveredRegistrations = [
  ...new Set(coverage.flatMap((entry) => entry.registrations)),
].sort();
const missing = renderableRegistrations.filter(
  (registration) => !coveredRegistrations.includes(registration),
);
const unknown = coveredRegistrations.filter(
  (registration) => !renderableRegistrations.includes(registration),
);
if (missing.length !== 0 || unknown.length !== 0) {
  fail(
    `coveredBy registrations differ from inventory (missing: ${missing.join(", ") || "none"}; unknown: ${unknown.join(", ") || "none"})`,
  );
}

const statusSource = read("examples/js_story/stories/status.js");
const storySource = read("examples/js_story/stories/story.js");
const registeredSource = read("examples/js_story/stories/registered.js");
const registeredBody = statusSource.match(
  /export const REGISTERED_SURFACES = \[([\s\S]*?)\];/,
)?.[1];
const deferredBody = statusSource.match(
  /export const DEFERRED_SURFACES = \{([\s\S]*?)\};/,
)?.[1];
if (registeredBody == null || deferredBody == null)
  fail("status projection is missing");
const registered = new Set(
  [...registeredBody.matchAll(/"([^"]+)"/g)].map((match) => match[1]),
);
const deferred = new Map(
  [...deferredBody.matchAll(/(\w+): "([^"]+)"/g)].map((match) => [
    match[1],
    match[2],
  ]),
);

for (const [surface, expected] of inventorySurfaces) {
  if (expected.status === "registered") {
    if (!registered.has(surface) || deferred.has(surface)) {
      fail(
        `${surface} is registered in inventory but not registered in the gallery status projection`,
      );
    }
  } else if (
    deferred.get(surface) !== expected.category ||
    registered.has(surface)
  ) {
    fail(`${surface} deferred category drifts from component-inventory.json`);
  }
}
for (const surface of registered) {
  if (inventorySurfaces.get(surface)?.status !== "registered") {
    fail(`${surface} is marked registered outside component-inventory.json`);
  }
}
for (const surface of deferred.keys()) {
  if (inventorySurfaces.get(surface)?.status !== "deferred") {
    fail(`${surface} is marked deferred outside component-inventory.json`);
  }
}
if (!registeredSource.includes('from "gpui-component"')) {
  fail("registered examples do not import the public gpui-component module");
}
for (const surface of registered) {
  if (!registeredSource.includes(`case "${surface}"`)) {
    fail(`${surface} is registered but has no public constructor example`);
  }
  if (!registeredSource.includes(`new ${surface}(`)) {
    fail(`${surface} registered constructor example does not use new`);
  }
}
if (
  !storySource.includes('availability: "registered"') ||
  !storySource.includes('availability: "deferred"') ||
  !statusSource.includes("reason: `No public ${surface} constructor") ||
  !storySource.includes("coveredSurfaces(story.id)") ||
  !storySource.includes("deferredSurfaces.map") ||
  !storySource.includes("Category: ${surface.category}") ||
  !storySource.includes("Reason: ${surface.reason}")
) {
  fail(
    "story status rendering does not expose every covered deferred surface with its category and reason",
  );
}

console.log(
  `JavaScript Story coverage: ${records.length} routes track all ${renderableRegistrations.length} tracked catalog surfaces from component-inventory.json`,
);
