import { readdirSync, readFileSync, statSync } from 'node:fs';
import { extname, join, relative } from 'node:path';

const SITE_TITLE = 'GPUI Component';
const SITE_DESCRIPTION =
  'A comprehensive Rust framework for building fantastic, high-performance desktop apps with GPUI.';
const BASE_URL = import.meta.env.BASE_URL.replace(/\/$/, '');

interface PageEntry {
  title: string;
  url: string;
  body: string;
}

function parseFrontmatterTitle(content: string): string | undefined {
  const match = content.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!match) return undefined;
  return match[1].match(/^title:\s*(.+)$/m)?.[1]?.trim().replace(/^["']|["']$/g, '');
}

function bodyWithoutFrontmatter(content: string): string {
  return content.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, '').trim();
}

function scanDir(dir: string, baseDir: string, urlPrefix: string): PageEntry[] {
  const results: PageEntry[] = [];
  let entries: string[];
  try {
    entries = readdirSync(dir);
  } catch {
    return results;
  }

  for (const name of entries) {
    const fullPath = join(dir, name);
    let stat: ReturnType<typeof statSync>;
    try { stat = statSync(fullPath); } catch { continue; }

    if (stat.isDirectory()) {
      const sub = scanDir(fullPath, baseDir, `${urlPrefix}/${name}`);
      results.push(...sub);
    } else if (extname(name) === '.md') {
      let content = '';
      try { content = readFileSync(fullPath, 'utf-8'); } catch { continue; }

      const title =
        parseFrontmatterTitle(content) ||
        content.match(/^#\s+(.+)$/m)?.[1]?.trim() ||
        name.replace(/\.md$/, '');

      const relPath = relative(baseDir, fullPath)
        .replace(/\.md$/, '')
        .replace(/index$/, '');
      const url = `${BASE_URL}/${urlPrefix}/${relPath}`.replace(/\/+/g, '/');
      const body = bodyWithoutFrontmatter(content);

      try {
        results.push({ title, url, body });
      } catch (err) {
        console.warn(`[llms] skipping ${fullPath}:`, err);
      }
    }
  }
  return results;
}

export function buildLlmsContent(websiteRoot: string): string {
  const sections = [
    { dir: join(websiteRoot, 'docs'), prefix: 'docs' },
    { dir: join(websiteRoot, 'shell'), prefix: 'shell' },
    { dir: join(websiteRoot, 'base'), prefix: 'base' },
    { dir: join(websiteRoot, 'zh-CN/docs'), prefix: 'zh-CN/docs' },
    { dir: join(websiteRoot, 'zh-CN/shell'), prefix: 'zh-CN/shell' },
    { dir: join(websiteRoot, 'zh-CN/base'), prefix: 'zh-CN/base' },
  ];

  const header = `# ${SITE_TITLE}\n\n> ${SITE_DESCRIPTION}\n\n---\n`;

  const pages: string[] = [];
  for (const { dir, prefix } of sections) {
    const entries = scanDir(dir, dir, prefix);
    for (const entry of entries) {
      pages.push(`# ${entry.title}\n\nSource: ${entry.url}\n\n${entry.body}`);
    }
  }

  return header + pages.join('\n\n---\n\n');
}
