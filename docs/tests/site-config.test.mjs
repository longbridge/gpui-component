import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const configPath = new URL("../.vitepress/config.mts", import.meta.url);

async function readConfig() {
  return readFile(configPath, "utf8");
}

test("站点配置切换为 OnetCli 品牌和 GitHub Pages 路径", async () => {
  const config = await readConfig();

  assert.match(config, /title:\s*"OnetCli"/);
  assert.match(config, /base:\s*"\/onetcli\/"/);
  assert.match(config, /description:[\s\S]*数据库/);
});

test("导航包含官网最小页面集合", async () => {
  const config = await readConfig();

  for (const text of ["首页", "功能", "下载", "更新日志", "文档"]) {
    assert.match(config, new RegExp(`text:\\s*"${text}"`));
  }
});
