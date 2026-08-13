import { defineConfig } from "vitepress";
import type { UserConfig } from "vitepress";
import { generateSidebar } from "vitepress-sidebar";
import llmstxt from "vitepress-plugin-llms";
import tailwindcss from "@tailwindcss/vite";
import { lightTheme, darkTheme } from "./language";
import { ViteToml } from "vite-plugin-toml";
import { createReadStream, existsSync, statSync } from "node:fs";
import { extname, join, resolve } from "node:path";

function wasmExamplesDevServer() {
  const roots = new Map([
    ["/gpui-component/examples/base", resolve("../crates/base/examples/wasm/www/dist")],
    ["/gpui-component/gallery", resolve("../crates/story-web/www/dist")],
  ]);
  const contentTypes: Record<string, string> = {
    ".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8",
    ".wasm": "application/wasm", ".css": "text/css; charset=utf-8", ".svg": "image/svg+xml",
  };
  return {
    name: "wasm-examples-dev-server",
    configureServer(server: any) {
      server.middlewares.use((req: any, res: any, next: () => void) => {
        const pathname = new URL(req.url ?? "/", "http://localhost").pathname;
        const entry = [...roots].find(([prefix]) => pathname === prefix || pathname.startsWith(`${prefix}/`));
        if (!entry) return next();
        const [prefix, root] = entry;
        const relative = pathname.slice(prefix.length).replace(/^\/+/, "");
        let file = join(root, relative || "index.html");
        if (!existsSync(file) || !statSync(file).isFile()) file = join(root, "index.html");
        if (!existsSync(file)) {
          res.statusCode = 503;
          res.end("WASM example is not built. Run its Makefile build target first.");
          return;
        }
        res.setHeader("Content-Type", contentTypes[extname(file)] ?? "application/octet-stream");
        createReadStream(file).pipe(res);
      });
    },
  };
}

/**
 * https://github.com/jooy2/vitepress-sidebar
 */
function createSidebar(scanStartPath: string, rootGroupText: string) {
  const routePrefix = `/${scanStartPath.replace(/^\/+|\/+$/g, "")}/`;
  const sidebar = generateSidebar([
    {
      scanStartPath: scanStartPath.replace(/^\/+|\/+$/g, ""),
      resolvePath: routePrefix,
      basePath: routePrefix,
      rootGroupText,
      collapsed: false,
      useTitleFromFrontmatter: true,
      useTitleFromFileHeading: true,
      sortMenusByFrontmatterOrder: true,
      includeRootIndexFile: false,
    },
  ]) as any;

  const rootItems = sidebar[routePrefix]?.items?.[0];
  if (!rootItems) return sidebar;

  rootItems.text = rootGroupText;

  const catalog = rootItems.items?.find(
    (item: any) =>
      Array.isArray(item.items) &&
      ["components", "primitives"].includes(item.text.toLowerCase()),
  );
  if (catalog) {
    catalog.text =
      catalog.text.toLowerCase() === "primitives" ? "Primitives" : "Components";
    catalog.items.sort((left: any, right: any) =>
      left.text.localeCompare(right.text, "en", { sensitivity: "base" }),
    );

    // Keep guide pages ahead of the API catalogue while preserving their
    // frontmatter-defined order.
    rootItems.items = [
      ...rootItems.items.filter((item: any) => item !== catalog),
      catalog,
    ];
  }

  return sidebar;
}

const enSidebar = createSidebar("/docs/", "Introduction");
const baseSidebar = createSidebar("/base/", "GPUI Base");
const zhSidebar = createSidebar("/zh-CN/docs/", "文档");

function createFooter(prefix = "", locale: "en" | "zh" = "en") {
  const contributorsText = locale === "zh" ? "贡献者" : "Contributors";
  const skillsText = "Skills";
  const reportBugText = locale === "zh" ? "报告问题" : "Report Bug";
  const discussionText = locale === "zh" ? "讨论" : "Discussion";
  const message =
    locale === "zh"
      ? `GPUI Component 是一个基于 Apache-2.0 许可证的开源项目，
        由 <a href='https://longbridge.com' target='_blank'>Longbridge</a> 开发。`
      : `GPUI Component is an open source project under the Apache-2.0 License,
        developed by <a href='https://longbridge.com' target='_blank'>Longbridge</a>.`;

  return {
    message,
    copyright: `
      <a href="https://gpui.rs">GPUI</a>
      |
      <a href="/gpui-component${prefix}/contributors">${contributorsText}</a>
      |
      <a href="/gpui-component${prefix}/skills" target="_blank">${skillsText}</a>
      |
      <a href="/gpui-component/llms-full.txt" target="_blank">llms-full.txt</a>
      |
      <a href="https://github.com/longbridge/gpui-component/issues" target="_blank">${reportBugText}</a>
      |
      <a href="https://github.com/longbridge/gpui-component/discussions" target="_blank">${discussionText}</a>
      <br />
      Icon resources are used <a href="https://lucide.dev" target="_blank">Lucide</a>,
      <a href="https://isocons.app" target="_blank">Isocons</a>.
    `,
  };
}

function createNav(prefix = "", locale: "en" | "zh" = "en") {
  const componentsText = locale === "zh" ? "组件" : "Components";
  const resourcesText = locale === "zh" ? "资源" : "Resources";
  const contributorsText = locale === "zh" ? "贡献者" : "Contributors";
  const releasesText = locale === "zh" ? "版本发布" : "Releases";
  const issuesText = "Issues";
  const discussionText = locale === "zh" ? "讨论" : "Discussion";

  return [
    { text: componentsText, link: `${prefix}/docs/components` },
    // The gpui-base docs are English-only, so both locales point at the same
    // pages; the section keeps its full "GPUI Base" name in the sidebar.
    { text: "Base", link: "/base/" },
    {
      text: resourcesText,
      items: [
        {
          text: "API Doc",
          link: "https://docs.rs/gpui-component",
        },
        {
          text: contributorsText,
          link: `${prefix}/contributors` || "/contributors",
        },
        {
          text: releasesText,
          link: "https://github.com/longbridge/gpui-component/releases",
        },
        {
          text: issuesText,
          link: "https://github.com/longbridge/gpui-component/issues",
        },
        {
          text: discussionText,
          link: "https://github.com/longbridge/gpui-component/discussions",
        },
      ],
    },
  ];
}

const sharedThemeConfig = {
  logo: {
    light: "/logo.svg",
    dark: "/logo-dark.svg",
  },
  socialLinks: null,
  search: {
    provider: "local",
  },
};

// Absolute URLs are required for social cards; relative paths are ignored by
// every crawler.
const SITE_URL = "https://longbridge.github.io/gpui-component";
const SITE_TITLE = "GPUI Component";
const SITE_DESCRIPTION =
  "UI components for building fantastic desktop applications in Rust, using GPUI.";

// https://vitepress.dev/reference/site-config
const config: UserConfig = {
  title: "GPUI Component",
  base: "/gpui-component/",
  description:
    "Rust GUI components for building fantastic cross-platform desktop application by using GPUI.",
  cleanUrls: true,
  head: [
    // One icon link, not a `prefers-color-scheme` pair: the site's own
    // appearance toggle is what the reader sees, and it can disagree with the
    // OS setting. `useThemeFavicon` repoints this link on every switch.
    ["link", { rel: "icon", href: "/gpui-component/logo.svg" }],
    // The card image is one static asset for every page — the same approach
    // Base UI takes. A per-page image would need a server to render it, which
    // GitHub Pages does not give us.
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:site_name", content: SITE_TITLE }],
    ["meta", { property: "og:image", content: `${SITE_URL}/og.png` }],
    ["meta", { property: "og:image:type", content: "image/png" }],
    // The card is drawn at 1200×630 and captured at 2× — Slack and the other
    // unfurlers re-encode the image at their own preview size, and a 1× source
    // came back soft on high-density screens.
    ["meta", { property: "og:image:width", content: "2400" }],
    ["meta", { property: "og:image:height", content: "1260" }],
    ["meta", { property: "og:image:alt", content: SITE_TITLE }],
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
    ["meta", { name: "twitter:image", content: `${SITE_URL}/og.png` }],
  ],

  // Each page contributes its own title, description and canonical URL; the
  // image stays shared.
  transformPageData(pageData) {
    const title = pageData.frontmatter.title || pageData.title || SITE_TITLE;
    const description =
      pageData.frontmatter.description ||
      pageData.description ||
      SITE_DESCRIPTION;
    const path = pageData.relativePath
      .replace(/index\.md$/, "")
      .replace(/\.md$/, "");
    const url = `${SITE_URL}/${path}`;
    const socialTitle =
      title === SITE_TITLE ? title : `${title} · ${SITE_TITLE}`;

    pageData.frontmatter.head ??= [];
    pageData.frontmatter.head.push(
      ["meta", { property: "og:title", content: socialTitle }],
      ["meta", { property: "og:description", content: description }],
      ["meta", { property: "og:url", content: url }],
      ["meta", { name: "twitter:title", content: socialTitle }],
      ["meta", { name: "twitter:description", content: description }],
      ["link", { rel: "canonical", href: url }],
    );
  },
  vite: {
    plugins: [wasmExamplesDevServer(), llmstxt(), tailwindcss(), ViteToml()],
  },
  themeConfig: sharedThemeConfig,
  locales: {
    root: {
      label: "English",
      lang: "en-US",
      themeConfig: {
        ...sharedThemeConfig,
        langMenuLabel: "Languages",
        nav: createNav("", "en"),
        sidebar: {
          ...enSidebar,
          ...baseSidebar,
        },
        footer: createFooter("", "en"),
        editLink: {
          pattern:
            "https://github.com/longbridge/gpui-component/edit/main/website/:path",
        },
      },
    },
    "zh-CN": {
      label: "简体中文",
      lang: "zh-CN",
      link: "/zh-CN/",
      themeConfig: {
        ...sharedThemeConfig,
        nav: createNav("/zh-CN", "zh"),
        sidebar: zhSidebar,
        footer: createFooter("/zh-CN", "zh"),
        langMenuLabel: "语言",
        returnToTopLabel: "返回顶部",
        sidebarMenuLabel: "菜单",
        darkModeSwitchLabel: "外观",
        lightModeSwitchTitle: "切换到浅色模式",
        darkModeSwitchTitle: "切换到深色模式",
        editLink: {
          pattern:
            "https://github.com/longbridge/gpui-component/edit/main/website/:path",
        },
      },
    },
  },
  markdown: {
    math: true,
    defaultHighlightLang: "rs",
    theme: {
      light: lightTheme,
      dark: darkTheme,
    },
  },
};

export default defineConfig(config);
