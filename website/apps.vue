<template>
    <div class="apps-page">
        <div class="apps-hero">
            <span class="apps-kicker">{{ copy.kicker }}</span>
            <h1>{{ copy.title }}</h1>
            <p class="apps-lead">{{ copy.lead }}</p>
            <ul class="apps-signals">
                <li><Boxes /> {{ copy.signalCount }}</li>
                <li><Monitor /> macOS / Windows / Linux</li>
                <li><Github /> {{ copy.signalLicense }}</li>
            </ul>
        </div>

        <div
            class="apps-filter"
            role="group"
            :aria-label="copy.filterLabel"
        >
            <button
                v-for="category in categories"
                :key="category.id"
                type="button"
                class="apps-filter__chip"
                :aria-pressed="String(category.id === active)"
                @click="active = category.id"
            >
                {{ category.label }}
                <span class="apps-filter__count">{{ category.count }}</span>
            </button>
        </div>

        <div class="apps-grid">
            <article v-for="app in visibleApps" :key="app.id" class="app-card">
                <!-- The screenshots are the apps' own published images, so they
                     already carry whatever window chrome each app draws. They
                     must not be wrapped in `.mac-window`: a second set of
                     traffic lights around a real titlebar reads as a mock. -->
                <a
                    class="app-card__shot"
                    :href="app.site ?? app.source"
                    target="_blank"
                    rel="noopener noreferrer"
                    :aria-label="app.name"
                >
                    <img
                        :src="app.image"
                        :alt="app.name"
                        loading="lazy"
                        decoding="async"
                    />
                </a>

                <div class="app-card__body">
                    <h3 class="app-card__name">{{ app.name }}</h3>
                    <p class="app-card__blurb">{{ app.blurb[locale] }}</p>

                    <ul class="app-card__meta">
                        <li>{{ app.platforms.join(" / ") }}</li>
                        <li>
                            {{ app.source ? copy.openSource : copy.commercial }}
                        </li>
                        <li v-if="app.building">{{ copy.building }}</li>
                    </ul>

                    <div class="app-card__links">
                        <a
                            v-if="app.site"
                            :href="app.site"
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            {{ copy.visit }} <ArrowUpRight />
                        </a>
                        <a
                            v-if="app.source"
                            :href="app.source"
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            {{ copy.sourceLink }} <ArrowUpRight />
                        </a>
                    </div>
                </div>
            </article>
        </div>

        <div class="apps-cta">
            <h2>{{ copy.ctaTitle }}</h2>
            <p>{{ copy.ctaLead }}</p>
            <a
                class="apps-cta__action"
                href="https://github.com/longbridge/gpui-component/discussions/989"
                target="_blank"
                rel="noopener noreferrer"
            >
                {{ copy.ctaAction }} <ArrowRight />
            </a>
        </div>
    </div>
</template>

<script setup>
import { computed, ref } from "vue";
import { useData } from "vitepress";
import {
    ArrowRight,
    ArrowUpRight,
    Boxes,
    Github,
    Monitor,
} from "lucide-vue-next";

const { localeIndex } = useData();
const isZh = computed(() => localeIndex.value === "zh-CN");
const locale = computed(() => (isZh.value ? "zh" : "en"));

// Ordering is the page's only ranking signal, so it carries the editorial
// judgement: how complete and shipped an app is first, GitHub traction second.
// Star counts themselves are deliberately not printed — a hard-coded number
// goes stale, and fetching 27 repositories at build time would exhaust the
// unauthenticated GitHub rate limit that `repo.data.js` already draws on.
//
// Screenshots point at the URLs the authors published on GitHub, exactly as
// submitted in discussion #989.
const apps = [
    {
        id: "longbridge-pro",
        name: "Longbridge Pro",
        category: "work",
        platforms: ["macOS", "Windows", "Linux"],
        site: "https://longbridge.com/desktop",
        source: null,
        image: "https://github.com/user-attachments/assets/4100dcc7-1316-4105-8ab2-ee6f84d95206",
        blurb: {
            en: "The trading desktop GPUI Component was built for. Real-time quotes, charts and dense market data, shipped on all three platforms.",
            zh: "GPUI Component 最初就是为它而生的交易桌面端。实时行情、图表与高密度市场数据，同时发布于三大平台。",
        },
    },
    {
        id: "openlogi",
        name: "OpenLogi",
        category: "system",
        platforms: ["macOS", "Windows", "Linux"],
        site: "https://openlogi.org",
        source: "https://github.com/AprilNEA/OpenLogi",
        image: "https://github.com/user-attachments/assets/d7e42a74-a3c5-49bb-9719-cc450fcedbce",
        blurb: {
            en: "A local-first alternative to Logitech Options+. Remap buttons, DPI and SmartShift over HID++ — with no account and no telemetry.",
            zh: "Logitech Options+ 的本地优先替代品。通过 HID++ 重映射按键、DPI 与 SmartShift，无需账号，也没有遥测。",
        },
    },
    {
        id: "zedis",
        name: "Zedis",
        category: "dev",
        platforms: ["macOS", "Windows", "Linux"],
        site: "https://zedis.net/",
        source: "https://github.com/vicanso/zedis",
        image: "https://raw.githubusercontent.com/vicanso/zedis/main/docs/images/key-browser.png",
        blurb: {
            en: "A native Redis GUI that opens a million-key database without a spinner: virtual-scrolled SCAN, typed value viewers, a memory analyzer and live metrics.",
            zh: "原生 Redis GUI，打开百万级键的数据库也不用等转圈：虚拟滚动的 SCAN、按类型定制的值查看器、内存分析与实时指标。",
        },
    },
    {
        id: "tty7",
        name: "tty7",
        category: "terminal",
        platforms: ["macOS", "Linux"],
        site: null,
        source: "https://github.com/l0ng-ai/tty7",
        image: "https://github.com/user-attachments/assets/bae50352-bb22-46b8-8c45-c9c5dff1cd89",
        blurb: {
            en: "A terminal workbench in pure Rust: persistent sessions, SSH, remote work and coding agents, over Alacritty's VT core.",
            zh: "纯 Rust 编写的终端工作台：持久会话、SSH、远程办公与编码 Agent，VT 内核来自 Alacritty。",
        },
    },
    {
        id: "omarchist",
        name: "Omarchist",
        category: "system",
        platforms: ["Omarchy Linux"],
        site: null,
        source: "https://github.com/tahayvr/omarchist",
        image: "https://raw.githubusercontent.com/tahayvr/omarchist/main/screenshots/omarchist-themes.png",
        blurb: {
            en: "The configuration and theme designer for Omarchy Linux, with visual theme editing, live previews and a built-in theme collection.",
            zh: "Omarchy Linux 的配置与主题设计工具，支持可视化主题编辑、实时预览，并内置一套主题集合。",
        },
    },
    {
        id: "aloud-ink",
        name: "Aloud Ink",
        category: "work",
        platforms: ["macOS"],
        site: "https://aloud.ink/",
        source: null,
        image: "https://github.com/user-attachments/assets/5c2b5a5f-d4cc-4e8d-a72f-a298ac86bf23",
        blurb: {
            en: "A native macOS dictation app: hold a global shortcut to speak, release to get clean, filler-free text at your cursor in any app.",
            zh: "macOS 原生听写应用：按住全局快捷键说话，松开即在任意应用的光标处得到去掉语气词的干净文本。",
        },
    },
    {
        id: "longbridge-lite",
        name: "Longbridge Lite",
        category: "work",
        platforms: ["macOS", "Windows", "Linux"],
        site: null,
        source: "https://github.com/longbridge/longbridge-lite",
        image: "https://github.com/user-attachments/assets/90f3a5b6-34c9-4a23-a9b4-864825cda4ef",
        blurb: {
            en: "A market-reading Longbridge client made for Omarchy, following its system theme and keyboard conventions — and the reference for running a JavaScript application natively through GPUI Shell.",
            zh: "为 Omarchy 定制的 Longbridge 行情客户端，跟随其系统主题与键盘约定；同时也是用 GPUI Shell 让 JavaScript 应用以原生方式运行的参考实现。",
        },
    },
    {
        id: "openprocmon",
        name: "OpenProcMon",
        category: "dev",
        platforms: ["Windows"],
        site: null,
        source: "https://github.com/progmboy/openprocmon",
        image: "https://raw.githubusercontent.com/progmboy/openprocmon/master/docs/snapshots/main.png",
        blurb: {
            en: "An open-source Windows Process Monitor: a kernel miniFilter driver, Procmon-compatible PML capture and replay, and an MCP interface.",
            zh: "开源的 Windows Process Monitor：内核 miniFilter 驱动、兼容 Procmon 的 PML 抓取与回放，并提供 MCP 接口。",
        },
    },
    {
        id: "dbflux",
        name: "DBFlux",
        category: "dev",
        platforms: ["macOS", "Windows", "Linux"],
        site: null,
        source: "https://github.com/0xErwin1/dbflux",
        image: "https://raw.githubusercontent.com/0xErwin1/dbflux/main/resources/dbflux.png",
        blurb: {
            en: "A keyboard-first database client for relational and non-relational stores, with charts, dashboards, Lua scripting and MCP integration.",
            zh: "键盘优先的数据库客户端，同时支持关系型与非关系型数据库，内置图表、仪表盘、Lua 脚本与 MCP 集成。",
        },
    },
    {
        id: "scope",
        name: "Scope",
        category: "work",
        platforms: ["macOS", "Windows", "Linux"],
        site: null,
        source: "https://github.com/scopeclient/scope",
        image: "https://github.com/user-attachments/assets/0688be40-5f9b-4171-bc18-fef4e5fa2384",
        blurb: {
            en: "A native Discord client built for power users.",
            zh: "面向重度用户的原生 Discord 客户端。",
        },
    },
    {
        id: "reviu",
        name: "Reviu",
        category: "dev",
        platforms: ["macOS", "Windows", "Linux"],
        site: "https://reviu.dev",
        source: "https://github.com/reviu-dev/reviu",
        image: "https://raw.githubusercontent.com/reviu-dev/reviu/main/website/src/assets/app_screenshots/git_dark.png",
        blurb: {
            en: "A keyboard-first Git client for reviewing AI-generated changes before you push, with GitHub pull requests, inline threads and AI briefs.",
            zh: "键盘优先的 Git 客户端，让你在推送前审阅 AI 生成的改动，支持 GitHub Pull Request、行内讨论与 AI 摘要。",
        },
    },
    {
        id: "cadence",
        name: "Cadence",
        category: "work",
        platforms: ["macOS"],
        site: null,
        source: "https://github.com/infomiho/cadence",
        image: "https://github.com/user-attachments/assets/043a556c-0f65-4b83-aea2-fe0a44014bd6",
        blurb: {
            en: "A minimal native Spotify player for macOS.",
            zh: "macOS 上的极简原生 Spotify 播放器。",
        },
    },
    {
        id: "based",
        name: "Based",
        category: "dev",
        platforms: ["macOS", "Windows", "Linux"],
        site: "https://based.pavi2410.com",
        source: "https://github.com/pavi2410/based",
        image: "https://github.com/user-attachments/assets/e4b98277-2983-43da-8f52-6bc3cf411071",
        blurb: {
            en: "A local-first, Git-friendly database client. Connections and saved queries live in a committed .based/ directory, with no backend service.",
            zh: "本地优先、对 Git 友好的数据库客户端。连接配置与保存的查询都放在纳入版本库的 .based/ 目录里，无需后端服务。",
        },
    },
    {
        id: "baudrun",
        name: "Baudrun",
        category: "terminal",
        platforms: ["macOS", "Windows", "Linux"],
        site: "https://packetthrower.github.io/Baudrun/",
        source: "https://github.com/packetThrower/Baudrun",
        image: "https://raw.githubusercontent.com/packetThrower/Baudrun/main/docs-next/public/screenshots/macos-dark-baudrun.png",
        blurb: {
            en: "A serial terminal for switch consoles and router CLIs, with saved device profiles, auto-reconnect, safe paste and XMODEM/YMODEM transfers.",
            zh: "面向交换机 Console 与路由器 CLI 的串口终端，支持设备配置、自动重连、安全粘贴与 XMODEM/YMODEM 传输。",
        },
    },
    {
        id: "oxidal",
        name: "Oxidal",
        category: "terminal",
        platforms: ["macOS", "Windows", "Linux"],
        site: null,
        source: "https://github.com/sh4den/Oxidal",
        image: "https://github.com/user-attachments/assets/c4483678-4f32-4e09-86b2-ef6a7b4a83ec",
        blurb: {
            en: "A native SSH session manager for people who avoid Electron: organized connections, an integrated terminal and local-only configuration.",
            zh: "为不想用 Electron 的人准备的原生 SSH 会话管理器：分组连接、内置终端，配置全部保存在本地。",
        },
    },
    {
        id: "nyx",
        name: "Nyx",
        category: "terminal",
        platforms: ["Windows", "Linux"],
        site: null,
        source: "https://github.com/BX-Team/Nyx",
        image: "https://raw.githubusercontent.com/BX-Team/Nyx/master/.github/branding/preview.png",
        blurb: {
            en: "A lightweight desktop GUI for the Mihomo proxy core, with profiles, proxy groups, rules, TUN mode and a connection inspector.",
            zh: "Mihomo 代理内核的轻量桌面 GUI，涵盖配置、代理组、规则、TUN 模式与连接检查器。",
        },
    },
    {
        id: "hadron",
        name: "Hadron",
        category: "dev",
        platforms: ["macOS", "Windows", "Linux"],
        site: null,
        source: "https://github.com/s0lda/hadron",
        image: "https://raw.githubusercontent.com/s0lda/hadron/main/assets/demo_2.png",
        blurb: {
            en: "A multi-agent execution environment: isolated Git worktrees, an automatic merge gate, interactive terminals and per-agent telemetry.",
            zh: "多 Agent 执行环境：隔离的 Git worktree、自动合并闸门、交互式终端与逐 Agent 的运行指标。",
        },
    },
    {
        id: "broquest",
        name: "Broquest",
        category: "dev",
        platforms: ["macOS", "Windows", "Linux"],
        site: null,
        source: "https://github.com/zanmato/broquest",
        image: "https://raw.githubusercontent.com/zanmato/broquest/main/docs/images/screenshot.webp",
        blurb: {
            en: "A local-first API client with JavaScript scripting and secrets management — an alternative to Postman, Insomnia and Bruno.",
            zh: "本地优先的 API 客户端，支持 JavaScript 脚本与密钥管理，可替代 Postman、Insomnia 与 Bruno。",
        },
    },
    {
        id: "nohrs",
        name: "Nohrs",
        category: "system",
        platforms: ["macOS"],
        site: null,
        source: "https://github.com/noh-rs/nohrs",
        image: "https://raw.githubusercontent.com/noh-rs/nohrs/develop/assets/doc/screen-shot.jpeg",
        building: true,
        blurb: {
            en: "A Raycast-style launcher and a keyboard-driven file explorer in one Finder alternative, extensible with sandboxed WASM plugins and its own search index.",
            zh: "把 Raycast 式启动器与键盘驱动的文件浏览器合成一个 Finder 替代品，可用沙箱化的 WASM 插件扩展，并自带搜索索引。",
        },
    },
    {
        id: "coop",
        name: "Coop",
        category: "work",
        platforms: ["macOS", "Windows", "Linux"],
        site: "https://coopchat.xyz",
        source: "https://git.reya.su/reya/coop",
        image: "https://github.com/user-attachments/assets/2bbdb5f0-944e-4ac2-9c30-a91912307d49",
        blurb: {
            en: "A Nostr direct-message app, built on a fork of the component library.",
            zh: "基于组件库分支构建的 Nostr 私信应用。",
        },
    },
    {
        id: "piku",
        name: "Piku",
        category: "system",
        platforms: ["macOS", "Windows", "Linux"],
        site: null,
        source: "https://github.com/pikachuu184/Piku",
        image: "https://raw.githubusercontent.com/pikachuu184/Piku/main/assets/branding/Screenshot%20%2813%29.png",
        blurb: {
            en: "A fast, monochrome, keyboard-driven file manager with tabbed and split workspaces and rich previews for images, PDFs, code and archives.",
            zh: "快速的单色键盘驱动文件管理器，支持标签与分屏工作区，并为图片、PDF、代码与压缩包提供丰富预览。",
        },
    },
    {
        id: "orrery",
        name: "Orrery",
        category: "dev",
        platforms: ["Linux"],
        site: "https://hankanman.github.io/Orrery/",
        source: "https://github.com/Hankanman/Orrery",
        image: "https://github.com/user-attachments/assets/ca69a657-8d13-416d-aa8f-ea15e12f4b90",
        building: true,
        blurb: {
            en: "A command center for every git repository in your dev directories: live status in one dense grid, host enrichment, on-device AI summaries, and one-click launch into an IDE or terminal agent.",
            zh: "面向 Git 仓库的指挥中心，把开发目录里的每个仓库汇总成一张高密度网格：实时状态、托管平台信息、本地 AI 摘要，一键在 IDE 或终端 Agent 中打开。",
        },
    },
    {
        id: "protide",
        name: "Protide",
        category: "dev",
        platforms: ["macOS", "Linux"],
        site: null,
        source: "https://github.com/dreygur/protide",
        image: "https://raw.githubusercontent.com/dreygur/protide/main/screenshot.png",
        blurb: {
            en: "An API testing tool covering HTTP, GraphQL, WebSocket, gRPC, tRPC and Socket.IO, with mock servers and local-first P2P collaboration.",
            zh: "API 测试工具，覆盖 HTTP、GraphQL、WebSocket、gRPC、tRPC 与 Socket.IO，并支持 Mock 服务与本地优先的 P2P 协作。",
        },
    },
    {
        id: "shouting-robin",
        name: "Shouting Robin",
        category: "dev",
        platforms: ["macOS", "Windows", "Linux"],
        site: null,
        source: "https://github.com/zanmato/shouting-robin",
        image: "https://raw.githubusercontent.com/zanmato/shouting-robin/main/docs/images/screenshot-dark.webp",
        blurb: {
            en: "An SEO crawler for e-commerce sites, crawling over plain HTTP or a real Chrome through spider-rs.",
            zh: "面向电商站点的 SEO 爬虫，可通过 spider-rs 使用纯 HTTP 或真实 Chrome 抓取。",
        },
    },
    {
        id: "ferrispass",
        name: "FerrisPass",
        category: "system",
        platforms: ["macOS"],
        site: null,
        source: "https://github.com/elias-tilegant/ferrispass",
        image: "https://raw.githubusercontent.com/elias-tilegant/ferrispass/master/docs/img/sharepoint/01-welcome.jpeg",
        blurb: {
            en: "A KeePass-compatible password manager that reads and writes KDBX 4 vaults, with TOTP, Auto-Type, auto-lock and a headless CLI.",
            zh: "兼容 KeePass 的密码管理器，可读写 KDBX 4 保险库，支持 TOTP、Auto-Type、自动锁定与无界面 CLI。",
        },
    },
    {
        id: "zenclash",
        name: "ZenClash",
        category: "terminal",
        platforms: ["macOS", "Windows", "Linux"],
        site: null,
        source: "https://github.com/HaiwenZhang/ZenClash",
        image: "https://github.com/user-attachments/assets/f9f0deb3-7bab-4288-87fb-aa5581bd59d1",
        blurb: {
            en: "A desktop client for the Mihomo proxy core.",
            zh: "Mihomo 代理内核的桌面客户端。",
        },
    },
    {
        id: "kazeterm",
        name: "KazeTerm",
        category: "terminal",
        platforms: ["macOS", "Windows", "Linux"],
        site: null,
        source: "https://github.com/bikesheddev/kazeterm",
        image: "https://raw.githubusercontent.com/bikesheddev/kazeterm/master/assets/screenshots/Screenshot_2026-03-14.webp",
        building: true,
        blurb: {
            en: "A lightweight terminal inspired by Windows Terminal and built around Alacritty, with tabs, theme overlays and swappable terminal backends.",
            zh: "受 Windows Terminal 启发的轻量终端，基于 Alacritty 构建，支持标签、主题覆盖与可替换的终端后端。",
        },
    },
];

const CATEGORY_LABELS = {
    all: { en: "All", zh: "全部" },
    dev: { en: "Developer Tools", zh: "开发工具" },
    terminal: { en: "Terminal & Network", zh: "终端与网络" },
    system: { en: "System & Desktop", zh: "系统与桌面" },
    work: { en: "Productivity & Media", zh: "效率与媒体" },
};

const active = ref("all");

const categories = computed(() =>
    Object.entries(CATEGORY_LABELS).map(([id, label]) => ({
        id,
        label: label[locale.value],
        count:
            id === "all"
                ? apps.length
                : apps.filter((app) => app.category === id).length,
    })),
);

const visibleApps = computed(() =>
    active.value === "all"
        ? apps
        : apps.filter((app) => app.category === active.value),
);

const copy = computed(() =>
    isZh.value
        ? {
              kicker: "应用案例",
              title: "用 GPUI Component 做出来的真实应用。",
              lead: "下面每一个都基于 GPUI Component 构建，是人们真正下载并每天使用的桌面软件——从生产环境的交易终端，到数据库客户端、终端与系统工具。",
              signalCount: `${apps.length} 个应用`,
              signalLicense: "开源与商业产品",
              filterLabel: "按类别筛选",
              openSource: "开源",
              commercial: "商业产品",
              building: "开发中",
              visit: "官网",
              sourceLink: "源码",
              ctaTitle: "你也用 GPUI Component 做了应用？",
              ctaLead: "把它发到 Showcase 讨论区，就有机会出现在这个页面上。",
              ctaAction: "提交你的应用",
          }
        : {
              kicker: "App Stories",
              title: "Real apps, shipped with GPUI Component.",
              lead: "Every app below is built on GPUI Component — desktop software people download and use every day, from a production trading terminal to database clients, terminals and system utilities.",
              signalCount: `${apps.length} apps`,
              signalLicense: "Open source and commercial",
              filterLabel: "Filter by category",
              openSource: "Open source",
              commercial: "Commercial",
              building: "In development",
              visit: "Website",
              sourceLink: "Source",
              ctaTitle: "Built something with GPUI Component?",
              ctaLead: "Post it in the showcase discussion and it can appear on this page.",
              ctaAction: "Submit your app",
          },
);
</script>

<style lang="scss" scoped>
@reference "./.vitepress/theme/style.css";

.apps-page {
    color: var(--foreground);
}

/* -------------------------------------------------------------- hero */

.apps-hero {
    max-width: 46rem;
    margin-bottom: clamp(2.5rem, 5vw, 3.5rem);
}

.apps-kicker {
    display: block;
    margin-bottom: 0.9rem;
    color: var(--muted-foreground);
    font-family: var(--vp-font-family-mono, ui-monospace, monospace);
    font-size: 0.68rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
}

html[lang^="zh"] .apps-kicker {
    letter-spacing: 0.06em;
}

.apps-hero h1 {
    margin: 0;
    border: 0;
    padding: 0;
    font-size: clamp(2rem, 3.6vw, 3rem);
    font-weight: 660;
    letter-spacing: -0.045em;
    line-height: 1.1;
}

html[lang^="zh"] .apps-hero h1 {
    letter-spacing: normal;
}

.apps-lead {
    margin: 1.1rem 0 0;
    color: var(--muted-foreground);
    font-size: 1.05rem;
    line-height: 1.7;
}

.apps-signals {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1.5rem;
    margin: 1.6rem 0 0;
    padding: 0;
    list-style: none;
    color: var(--muted-foreground);
    font-size: 0.85rem;
    font-variant-numeric: tabular-nums;
}

.apps-signals li {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    margin: 0;
}

.apps-signals :deep(svg) {
    width: 0.95rem;
    height: 0.95rem;
    opacity: 0.7;
}

/* ------------------------------------------------------------ filter */

.apps-filter {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 1.75rem;
    border-top: 1px solid var(--border);
    padding-top: 1.75rem;
}

.apps-filter__chip {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.35rem 0.85rem;
    color: var(--foreground);
    font-size: 0.85rem;
    line-height: 1.4;
    cursor: pointer;
    transition:
        background-color 0.15s ease,
        border-color 0.15s ease,
        color 0.15s ease;
}

.apps-filter__chip:hover {
    background: var(--secondary);
}

/* The active filter is a primary control, which is exactly what the brand
   colour is reserved for. */
.apps-filter__chip[aria-pressed="true"] {
    border-color: var(--brand);
    background: var(--brand);
    color: var(--brand-contrast);
}

.apps-filter__count {
    color: var(--muted-foreground);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
}

.apps-filter__chip[aria-pressed="true"] .apps-filter__count {
    color: var(--brand-contrast);
    opacity: 0.65;
}

/* -------------------------------------------------------------- grid */

.apps-grid {
    display: grid;
    /* `minmax(0, 1fr)`, never a bare `1fr`: a long unbroken app name would
       otherwise set the column's minimum and push the page wider than the
       viewport. */
    grid-template-columns: repeat(auto-fill, minmax(min(20rem, 100%), 1fr));
    gap: 1.5rem;
}

.app-card {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--card);
    transition:
        border-color 0.18s ease,
        box-shadow 0.18s ease;
}

.app-card:hover {
    border-color: var(--brand-line);
    box-shadow: var(--shadow-raise);
}

.app-card__shot {
    display: block;
    border-bottom: 1px solid var(--border);
    background: var(--secondary);
}

.app-card__shot img {
    display: block;
    /* One ratio for every card, so a row of screenshots aligns even though the
       sources were captured at different sizes. Anchored to the top because an
       app's toolbar and content say more than its status bar. */
    width: 100%;
    aspect-ratio: 16 / 10;
    object-fit: cover;
    object-position: top center;
}

.app-card__body {
    display: flex;
    flex: 1;
    flex-direction: column;
    padding: 1.15rem 1.25rem 1.25rem;
}

.app-card__name {
    margin: 0;
    border: 0;
    padding: 0;
    font-size: 1.05rem;
    font-weight: 620;
    letter-spacing: -0.015em;
    line-height: 1.3;
}

html[lang^="zh"] .app-card__name {
    letter-spacing: normal;
}

/* The slack in a short description is absorbed here, so the meta row and the
   links of every card in a row sit on the same baseline. */
.app-card__blurb {
    margin: 0.55rem 0 auto;
    padding-bottom: 1rem;
    color: var(--muted-foreground);
    font-size: 0.875rem;
    line-height: 1.65;
}

.app-card__meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin: 0 0 0.9rem;
    padding: 0;
    list-style: none;
}

.app-card__meta li {
    margin: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    padding: 0.15rem 0.45rem;
    color: var(--muted-foreground);
    font-size: 0.72rem;
    line-height: 1.5;
    white-space: nowrap;
}

.app-card__links {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    border-top: 1px solid var(--border);
    padding-top: 0.9rem;
}

.app-card__links a {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    color: var(--foreground);
    font-size: 0.85rem;
    font-weight: 500;
    text-decoration: none;
    transition: opacity 0.15s ease;
}

.app-card__links a:hover {
    opacity: 0.66;
}

.app-card__links :deep(svg) {
    width: 0.85rem;
    height: 0.85rem;
    opacity: 0.55;
}

/* --------------------------------------------------------------- cta */

.apps-cta {
    margin-top: clamp(3rem, 6vw, 4.5rem);
    border-top: 1px solid var(--border);
    padding-top: clamp(2rem, 4vw, 3rem);
}

.apps-cta h2 {
    margin: 0;
    border: 0;
    padding: 0;
    font-size: 1.4rem;
    font-weight: 640;
    letter-spacing: -0.02em;
    line-height: 1.3;
}

html[lang^="zh"] .apps-cta h2 {
    letter-spacing: normal;
}

.apps-cta p {
    margin: 0.6rem 0 1.3rem;
    color: var(--muted-foreground);
    font-size: 0.95rem;
    line-height: 1.7;
}

.apps-cta__action {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    border-radius: var(--radius-control);
    background: var(--brand);
    padding: 0.55rem 1.1rem;
    color: var(--brand-contrast);
    font-size: 0.9rem;
    font-weight: 500;
    text-decoration: none;
    transition: background-color 0.15s ease;
}

.apps-cta__action:hover {
    background: var(--brand-hover);
}

.apps-cta__action :deep(svg) {
    width: 0.95rem;
    height: 0.95rem;
}

@media (max-width: 640px) {
    .apps-grid {
        gap: 1.15rem;
    }
}
</style>
