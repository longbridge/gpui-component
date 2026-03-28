import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const docsRoot = new URL("../", import.meta.url);

async function readText(path) {
  return readFile(new URL(path, docsRoot), "utf8");
}

test("站点配置排除旧的 gpui 文档和内部规格目录", async () => {
  const config = await readText(".vitepress/config.mts");

  assert.match(config, /srcExclude:/);
  assert.match(config, /contributors\.md/);
  assert.match(config, /skills\.md/);
  assert.match(config, /docs\/\*\*/);
  assert.match(config, /superpowers\/\*\*/);
});

test("GitHub 导航组件不再依赖构建时网络请求", async () => {
  const component = await readText(".vitepress/theme/components/GitHubStar.vue");

  assert.doesNotMatch(component, /repo\.data/);
  assert.doesNotMatch(component, /stargazers_count/);
  assert.match(component, /GitHub/);
});
