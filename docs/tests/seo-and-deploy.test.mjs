import test from "node:test";
import assert from "node:assert/strict";
import { access, readFile } from "node:fs/promises";
import { constants } from "node:fs";

const docsRoot = new URL("../", import.meta.url);
const repoRoot = new URL("../../", import.meta.url);

async function readDocs(path) {
  return readFile(new URL(path, docsRoot), "utf8");
}

async function readRepo(path) {
  return readFile(new URL(path, repoRoot), "utf8");
}

test("首页包含 FAQ 和结构化数据", async () => {
  const home = await readDocs("index.vue");

  assert.match(home, /FAQ/);
  assert.match(home, /SoftwareApplication/);
  assert.match(home, /FAQPage/);
});

test("官网截图资源已放入 docs public 目录", async () => {
  for (const file of ["app.png", "database.png", "ssh.png", "chatdb.png"]) {
    await access(new URL(`public/screenshots/${file}`, docsRoot), constants.F_OK);
  }
});

test("GitHub Pages 工作流使用面向 dev 的 Pages 部署链路", async () => {
  const workflow = await readRepo(".github/workflows/release-docs.yml");

  assert.match(workflow, /push:/);
  assert.match(workflow, /branches:\s*\n\s*-\s*dev/);
  assert.doesNotMatch(workflow, /Release Crate/);
  assert.match(workflow, /actions\/deploy-pages@v4/);
});

test("本地可视化目录已加入 gitignore", async () => {
  const gitignore = await readRepo(".gitignore");

  assert.match(gitignore, /\.superpowers\//);
});
