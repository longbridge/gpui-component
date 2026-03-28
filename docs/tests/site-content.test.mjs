import test from "node:test";
import assert from "node:assert/strict";
import { access, readFile } from "node:fs/promises";
import { constants } from "node:fs";

const root = new URL("../", import.meta.url);

async function readText(path) {
  return readFile(new URL(path, root), "utf8");
}

test("首页替换为 OnetCli 产品文案并指向 GitHub Releases", async () => {
  const home = await readText("index.vue");
  const homeEntry = await readText("index.md");

  assert.match(home, /OnetCli/);
  assert.match(home, /数据库/);
  assert.match(home, /SSH/);
  assert.match(home, /GitHub Releases/);
  assert.doesNotMatch(homeEntry, /GPUI Component/);
});

test("官网最小页面集合已经创建", async () => {
  for (const file of ["features.md", "download.md", "changelog.md", "guide.md"]) {
    await access(new URL(file, root), constants.F_OK);
  }

  const features = await readText("features.md");
  const download = await readText("download.md");
  const changelog = await readText("changelog.md");
  const guide = await readText("guide.md");

  assert.match(features, /# 功能/);
  assert.match(download, /# 下载/);
  assert.match(changelog, /# 更新日志/);
  assert.match(guide, /# 文档/);
  assert.match(guide, /GitHub Releases/);
});
