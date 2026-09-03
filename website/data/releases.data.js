import { fileURLToPath } from "node:url";
import { createMarkdownRenderer } from "vitepress";
import { darkTheme, lightTheme } from "../.vitepress/language";

const REPO = "longbridge/gpui-kit";
const API_URL = `https://api.github.com/repos/${REPO}/releases?per_page=100`;
const SRC_DIR = fileURLToPath(new URL("..", import.meta.url));

function requestHeaders() {
  const headers = { Accept: "application/vnd.github+json" };
  const token = process.env.GITHUB_TOKEN;
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }
  return headers;
}

// GitHub turns `#1652` and `@login` into links when it shows a release; the
// API hands back the markdown as typed. Only bare references are linked, so a
// `#` inside a URL fragment or an `@` inside an address stays as it is.
function linkReferences(markdown) {
  return markdown
    .replace(
      /(^|[\s(])#(\d+)(?=[\s.,;:)]|$)/gm,
      `$1[#$2](https://github.com/${REPO}/issues/$2)`,
    )
    .replace(
      /(^|[\s(])@([A-Za-z\d](?:[A-Za-z\d-]{0,37}[A-Za-z\d])?)(?=[\s.,;:)]|$)/gm,
      "$1[@$2](https://github.com/$2)",
    );
}

// The site renderer adds an id and a permalink to every heading. Release
// notes repeat their headings across versions, so those ids would collide;
// the version headings the page adds itself are the anchors readers need.
function stripHeadingAnchors(html) {
  return html
    .replace(/<a class="header-anchor"[^>]*>[\s\S]*?<\/a>/g, "")
    .replace(/<h([1-6]) id="[^"]*"/g, "<h$1")
    .replace(/<h([1-6]) id='[^']*'/g, "<h$1");
}

async function fetchReleases() {
  try {
    const res = await fetch(API_URL, { headers: requestHeaders() });
    const items = await res.json();
    if (!res.ok || !Array.isArray(items)) {
      console.warn(
        `[releases] GitHub API returned ${res.status}: ${items?.message ?? "unexpected response"}`,
      );
      return [];
    }
    return items;
  } catch (error) {
    console.warn(`[releases] Failed to fetch releases: ${error}`);
    return [];
  }
}

export default {
  async load() {
    const items = await fetchReleases();
    if (items.length === 0) {
      return [];
    }

    // The same renderer the docs use, so code in the notes gets the docs'
    // highlighting and adaptive theme.
    const md = await createMarkdownRenderer(
      SRC_DIR,
      {
        languages: ["rust"],
        languageAlias: { rs: "rust" },
        defaultHighlightLang: "rust",
        theme: { light: lightTheme, dark: darkTheme },
      },
      "/",
    );

    return items
      .filter((item) => !item.draft)
      .sort((a, b) => Date.parse(b.published_at) - Date.parse(a.published_at))
      .map((item) => {
        const body = linkReferences((item.body ?? "").replace(/\r\n/g, "\n"));
        return {
          tag: item.tag_name,
          name: item.name || item.tag_name,
          // The date alone; the page shows it as written, so the server and
          // the browser cannot disagree on its format.
          date: item.published_at.slice(0, 10),
          url: item.html_url,
          prerelease: item.prerelease,
          html: stripHeadingAnchors(md.render(body)),
        };
      });
  },
};
