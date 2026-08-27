<template>
    <header class="home-nav shadow-sm">
        <div class="home-nav__inner">
            <a :href="homeHref" class="home-brand">
                <img :src="isDark ? darkLogoHref : logoHref" alt="" />
                <span>GPUI Component</span>
            </a>
            <nav aria-label="Primary navigation">
                <template v-for="item in navItems" :key="item.text">
                    <a
                        v-if="item.link"
                        :href="item.href"
                        :target="item.external ? '_blank' : undefined"
                        >{{ item.text }}</a
                    >
                    <span v-else class="home-nav__group">
                        <button type="button">
                            {{ item.text }} <ChevronDown />
                        </button>
                        <span class="home-nav__menu">
                            <a
                                v-for="child in item.items"
                                :key="child.text"
                                :href="child.href"
                                :target="child.external ? '_blank' : undefined"
                                >{{ child.text }}</a
                            >
                        </span>
                    </span>
                </template>
            </nav>
            <div class="home-nav__actions">
                <a :href="searchHref" class="home-nav__search">
                    <Search />
                    <span>{{ copy.searchShort }}</span>
                </a>
                <a
                    href="https://github.com/longbridge/gpui-component"
                    target="_blank"
                    class="home-nav__github"
                    :title="`${stars} stars on GitHub`"
                >
                    <Github /> <span>{{ starLabel }}</span>
                </a>
                <span
                    class="home-nav__rule home-nav__rule--tight"
                    aria-hidden="true"
                ></span>
                <a
                    :href="languageHref"
                    class="home-nav__icon home-nav__icon--text"
                >
                    {{ isZh ? "EN" : "中" }}
                </a>
                <button
                    class="home-nav__icon"
                    :aria-label="copy.themeNav"
                    @click="isDark = !isDark"
                >
                    <Sun v-if="isDark" />
                    <Moon v-else />
                </button>
                <button
                    class="home-nav__icon home-nav__burger"
                    :aria-label="copy.menuNav"
                    :aria-expanded="menuOpen"
                    @click="menuOpen = !menuOpen"
                >
                    <X v-if="menuOpen" />
                    <Menu v-else />
                </button>
            </div>
        </div>

        <!-- The section links are hidden on phones, so they need a way back. -->
        <div v-if="menuOpen" class="home-nav__drawer">
            <template v-for="item in navItems" :key="item.text">
                <a
                    v-if="item.link"
                    :href="item.href"
                    :target="item.external ? '_blank' : undefined"
                    @click="menuOpen = false"
                    >{{ item.text }}</a
                >
                <template v-else>
                    <span class="home-nav__drawer-label">{{ item.text }}</span>
                    <a
                        v-for="child in item.items"
                        :key="child.text"
                        :href="child.href"
                        :target="child.external ? '_blank' : undefined"
                        class="is-child"
                        @click="menuOpen = false"
                        >{{ child.text }}</a
                    >
                </template>
            </template>
        </div>
    </header>

    <main class="home">
        <section class="hero">
            <div class="hero__grid" aria-hidden="true"></div>
            <div class="hero__inner">
                <div class="hero__copy">
                    <a href="https://gpui.rs" target="_blank" class="eyebrow">
                        <span class="eyebrow__pulse" aria-hidden="true"></span>
                        {{ copy.eyebrow }}
                        <ArrowRight />
                    </a>
                    <h1>{{ copy.title }}</h1>
                    <p class="hero__lead">{{ copy.lead }}</p>
                    <div class="hero__actions">
                        <a :href="gettingStartedHref" class="btn btn--primary">
                            {{ copy.startComponent }} <ArrowRight />
                        </a>
                        <a :href="componentsHref" class="btn">{{
                            copy.componentsAction
                        }}</a>
                    </div>
                    <ul class="hero__signals">
                        <li>
                            <Star />
                            <strong>{{ starLabel }}</strong>
                            {{ copy.signalStars }}
                        </li>
                        <li><Scale /> {{ copy.signalLicense }}</li>
                        <li><Monitor /> {{ copy.signalPlatforms }}</li>
                    </ul>
                    <div class="hero__install">
                        <span class="hero__install-label">Cargo.toml</span>
                        <code>{{ installCommand }}</code>
                        <button
                            type="button"
                            :aria-label="copy.copyLabel"
                            :data-copied="copied || null"
                            @click="copyInstall"
                        >
                            <Check v-if="copied" />
                            <Copy v-else />
                        </button>
                    </div>
                </div>

                <!-- Real code from the Quick Start guide: it paints instantly,
                     states plainly that this is a Rust library, and cannot be
                     mistaken for a product the project does not ship. -->
                <div class="hero__code mac-window">
                    <div class="mac-window__bar">
                        <span class="mac-window__lights" aria-hidden="true"
                            ><i /><i /><i
                        /></span>
                        <span class="mac-window__title">main.rs</span>
                    </div>
                    <pre><code><span class="c-kw">use</span> gpui_component::{<span class="c-mod">button</span>::*, *};

<span class="c-kw">impl</span> <span class="c-type">Render</span> <span class="c-kw">for</span> <span class="c-type">HelloWorld</span> {
    <span class="c-kw">fn</span> <span class="c-fn">render</span>(&<span class="c-kw">mut</span> self, _: &<span class="c-kw">mut</span> <span class="c-type">Window</span>, cx: &<span class="c-kw">mut</span> <span class="c-type">Context</span>&lt;Self&gt;) -&gt; <span class="c-kw">impl</span> <span class="c-type">IntoElement</span> {
        <span class="c-fn">div</span>()
            .<span class="c-fn">v_flex</span>()
            .<span class="c-fn">gap_2</span>()
            .<span class="c-fn">items_center</span>()
            .<span class="c-fn">child</span>(<span class="c-str">"Hello, World!"</span>)
            .<span class="c-fn">child</span>(
                <span class="c-type">Button</span>::<span class="c-fn">new</span>(<span class="c-str">"ok"</span>)
                    .<span class="c-fn">primary</span>()
                    .<span class="c-fn">label</span>(<span class="c-str">"Let's Go!"</span>)
                    .<span class="c-fn">on_click</span>(cx.<span class="c-fn">listener</span>(Self::go)),
            )
    }
}</code></pre>
                </div>
            </div>
        </section>

        <section class="band">
            <div class="band__inner">
                <header class="section-head">
                    <span class="section-kicker">{{ copy.capsKicker }}</span>
                    <h2>{{ copy.capsTitle }}</h2>
                    <p>{{ copy.capsDescription }}</p>
                </header>

                <div class="caps__grid">
                    <article
                        v-for="cap in copy.caps"
                        :key="cap.title"
                        class="cap"
                    >
                        <div class="cap__head">
                            <component :is="capIcons[cap.icon]" />
                            <h3>{{ cap.title }}</h3>
                        </div>
                        <p>{{ cap.description }}</p>
                        <ul class="cap__api">
                            <li v-for="api in cap.apis" :key="api">
                                {{ api }}
                            </li>
                        </ul>
                        <div
                            class="cap__preview"
                            :class="`cap__preview--${cap.icon}`"
                            aria-hidden="true"
                        >
                            <template v-if="cap.icon === 'perf'">
                                <span class="cap__budget" />
                                <u
                                    v-for="(frame, i) in frames"
                                    :key="i"
                                    :style="{ height: `${frame}%` }"
                                />
                            </template>
                            <template v-else-if="cap.icon === 'table'">
                                <i v-for="n in 5" :key="n"><b /><b /><b /></i>
                            </template>
                            <template v-else-if="cap.icon === 'list'">
                                <i v-for="w in [86, 68, 92, 74, 60]" :key="w">
                                    <b :style="{ width: `${w}%` }" />
                                </i>
                                <span class="cap__track">
                                    <span class="cap__thumb" />
                                </span>
                            </template>
                            <template v-else-if="cap.icon === 'editor'">
                                <i
                                    v-for="(line, row) in editorLines"
                                    :key="row"
                                    :style="{
                                        paddingLeft: `${line.indent * 0.6}rem`,
                                    }"
                                >
                                    <s class="cap__gutter" />
                                    <b
                                        v-for="(w, i) in line.tokens"
                                        :key="i"
                                        :style="{ width: `${w}rem` }"
                                    />
                                </i>
                            </template>
                            <template v-else-if="cap.icon === 'dock'">
                                <b /><b /><b />
                            </template>
                            <template v-else>
                                <em v-for="t in 6" :key="t" />
                            </template>
                        </div>
                    </article>
                </div>
            </div>
        </section>

        <section class="band">
            <div class="band__inner">
                <header class="section-head">
                    <span class="section-kicker">{{ copy.chooseKicker }}</span>
                    <h2>{{ copy.chooseTitle }}</h2>
                    <p>{{ copy.chooseDescription }}</p>
                </header>

                <div class="paths__grid">
                    <article class="path path--primary">
                        <div class="path__meta">
                            <Blocks />
                            <span>gpui-component</span>
                        </div>
                        <h3>{{ copy.shipTitle }}</h3>
                        <p>{{ copy.shipDescription }}</p>
                        <pre><code><span class="c-type">Button</span>::<span class="c-fn">new</span>(<span class="c-str">"save"</span>)
    .<span class="c-fn">primary</span>()
    .<span class="c-fn">label</span>(<span class="c-str">"Save changes"</span>)
    .<span class="c-fn">on_click</span>(cx.<span class="c-fn">listener</span>(Self::save))</code></pre>
                        <ul>
                            <li v-for="item in copy.shipPoints" :key="item">
                                <Check />{{ item }}
                            </li>
                        </ul>
                        <a :href="gettingStartedHref" class="path__link">
                            {{ copy.startComponent }} <ArrowRight />
                        </a>
                    </article>

                    <article class="path">
                        <div class="path__meta">
                            <Layers3 />
                            <span>gpui-base</span>
                        </div>
                        <h3>{{ copy.ownTitle }}</h3>
                        <p>{{ copy.ownDescription }}</p>
                        <pre><code><span class="c-type">Button</span>::<span class="c-fn">new</span>(<span class="c-str">"save"</span>)
    .<span class="c-fn">on_click</span>(cx.<span class="c-fn">listener</span>(Self::save))
    .<span class="c-fn">rounded_md</span>()
    .<span class="c-fn">child</span>(<span class="c-str">"Save changes"</span>)</code></pre>
                        <ul>
                            <li v-for="item in copy.ownPoints" :key="item">
                                <Check />{{ item }}
                            </li>
                        </ul>
                        <a :href="baseHref" class="path__link"
                            >{{ copy.startBase }} <ArrowRight
                        /></a>
                    </article>

                    <article class="path">
                        <div class="path__meta">
                            <Braces />
                            <span>gpui-shell</span>
                        </div>
                        <h3>{{ copy.scriptTitle }}</h3>
                        <p>{{ copy.scriptDescription }}</p>
                        <pre><code><span class="c-kw">import</span> { Button, text } <span class="c-kw">from</span> <span class="c-str">"gpui"</span>;

<span class="c-type">Button</span>.<span class="c-fn">new</span>(<span class="c-str">"save"</span>)
    .<span class="c-fn">on_click</span>((_e, cx) =&gt; <span class="c-kw">this</span>.<span class="c-fn">save</span>(cx))
    .<span class="c-fn">child</span>(<span class="c-fn">text</span>(<span class="c-str">"Save changes"</span>))</code></pre>
                        <ul>
                            <li v-for="item in copy.scriptPoints" :key="item">
                                <Check />{{ item }}
                            </li>
                        </ul>
                        <a :href="shellHref" class="path__link"
                            >{{ copy.startShell }} <ArrowRight
                        /></a>
                    </article>
                </div>
            </div>
        </section>

        <section class="band band--principle">
            <div class="principle__grid" aria-hidden="true"></div>
            <div class="band__inner principle">
                <div class="principle__quote">
                    <span class="section-kicker">{{
                        copy.principleKicker
                    }}</span>
                    <blockquote>
                        <span>{{ copy.principleLead }}</span>
                        <span class="principle__accent">{{
                            copy.principleTail
                        }}</span>
                    </blockquote>
                </div>
                <div class="principle__aside">
                    <p>{{ copy.principleDetail }}</p>
                    <a :href="baseHref" class="btn btn--primary"
                        >{{ copy.baseAction }} <ArrowRight
                    /></a>
                </div>
            </div>
        </section>
    </main>

    <footer class="home-footer">
        <div class="home-footer__brand">
            <strong>GPUI Component</strong>
            <p>
                {{ copy.footerPrefix }}
                <a href="https://longbridge.com" target="_blank">Longbridge</a>.
            </p>
        </div>
        <nav :aria-label="copy.footerNav">
            <a href="https://gpui.rs" target="_blank">GPUI</a>
            <a :href="contributorsHref">{{ copy.contributors }}</a>
            <a :href="skillsHref" target="_blank">Skills</a>
            <a :href="llmsHref" target="_blank">llms-full.txt</a>
            <a
                href="https://github.com/longbridge/gpui-component/issues"
                target="_blank"
                >{{ copy.reportBug }}</a
            >
            <a
                href="https://github.com/longbridge/gpui-component/discussions"
                target="_blank"
                >{{ copy.discussion }}</a
            >
        </nav>
        <p class="home-footer__credits">
            {{ copy.iconCredits }}
            <a href="https://lucide.dev" target="_blank">Lucide</a>
            {{ copy.and }}
            <a href="https://isocons.app" target="_blank">Isocons</a>.
        </p>
    </footer>
</template>

<script setup>
import { computed, onBeforeUnmount, ref } from "vue";
import { useData, withBase } from "vitepress";
import {
    ArrowRight,
    ChevronDown,
    Blocks,
    Braces,
    Check,
    Copy,
    Gauge,
    Github,
    Menu,
    Layers3,
    LayoutDashboard,
    List,
    Monitor,
    Moon,
    Palette,
    Scale,
    Search,
    SquareCode,
    Star,
    Sun,
    Table2,
    X,
} from "lucide-vue-next";
import { data as repo } from "./data/repo.data";

const { isDark, localeIndex, theme } = useData();

// The navbar renders the very same `nav` array that config.mts feeds to the
// docs navbar, so the two can never drift apart.
const isExternal = (link) => /^https?:\/\//.test(link);
const resolve = (link) => (isExternal(link) ? link : withBase(link));
const navItems = computed(() =>
    (theme.value.nav ?? [])
        .filter((item) => item.text)
        .map((item) => ({
            text: item.text,
            link: item.link,
            href: item.link ? resolve(item.link) : undefined,
            external: item.link ? isExternal(item.link) : false,
            items: (item.items ?? []).map((child) => ({
                text: child.text,
                href: resolve(child.link),
                external: isExternal(child.link),
            })),
        })),
);
const isZh = computed(() => localeIndex.value === "zh-CN");
const localePrefix = computed(() => (isZh.value ? "/zh-CN" : ""));

const menuOpen = ref(false);
const stars = repo.stargazers_count ?? 0;
const starLabel = stars >= 1000 ? `${(stars / 1000).toFixed(1)}k` : `${stars}`;

const gettingStartedHref = computed(() =>
    withBase(`${localePrefix.value}/docs/getting-started`),
);
const componentsHref = computed(() =>
    withBase(`${localePrefix.value}/docs/components`),
);
const baseHref = computed(() => withBase("/base/"));
const shellHref = computed(() => withBase(`${localePrefix.value}/shell/`));
const docsHref = computed(() => withBase(`${localePrefix.value}/docs/`));
const searchHref = computed(() =>
    withBase(`${localePrefix.value}/docs/components`),
);
const homeHref = computed(() => withBase(`${localePrefix.value}/`));
const logoHref = computed(() => withBase("/logo.svg"));
const darkLogoHref = computed(() => withBase("/logo-dark.svg"));
const languageHref = computed(() => withBase(isZh.value ? "/" : "/zh-CN/"));
const contributorsHref = computed(() =>
    withBase(`${localePrefix.value}/contributors`),
);
const skillsHref = computed(() => withBase(`${localePrefix.value}/skills`));
const llmsHref = computed(() => withBase("/llms-full.txt"));

// Indent depth and token widths (rem) per line — an indent pattern that reads
// as code without pretending to be a specific snippet.
const editorLines = [
    { indent: 0, tokens: [0.45, 1.5, 0.75] },
    { indent: 0, tokens: [0.9, 1.15, 1.8] },
    { indent: 1, tokens: [1.2, 0.8, 1.35] },
    { indent: 2, tokens: [0.7, 1.55] },
    { indent: 2, tokens: [1.05, 0.65, 0.9] },
    { indent: 1, tokens: [0.55, 1.25] },
    { indent: 0, tokens: [0.4] },
];

// A dense frame-time trace: every bar is one frame, and the point is that they
// all clear the budget line.
const frames = [
    72, 78, 74, 81, 76, 84, 79, 73, 88, 77, 82, 75, 86, 80, 71, 83, 76, 90, 74,
    79, 85, 72, 81, 77, 87, 75, 80, 73, 84, 78,
];

const capIcons = {
    perf: Gauge,
    table: Table2,
    list: List,
    editor: SquareCode,
    dock: LayoutDashboard,
    theme: Palette,
};

// The crate is not published yet, so the real dependency is a git one — the
// hero must not suggest a `cargo add` that would fail. The line on screen is
// the headline dependency; the clipboard gets the full block from the Getting
// Started guide, because gpui-component alone does not build.
const installCommand =
    'gpui-component = { git = "https://github.com/longbridge/gpui-component" }';
const installSnippet = [
    "[dependencies]",
    'gpui = { git = "https://github.com/zed-industries/zed" }',
    'gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit"] }',
    'gpui-component = { git = "https://github.com/longbridge/gpui-component" }',
    'gpui-component-assets = { git = "https://github.com/longbridge/gpui-component" }',
].join("\n");

const copied = ref(false);
let copyTimer;
const copyInstall = async () => {
    try {
        await navigator.clipboard.writeText(installSnippet);
        copied.value = true;
        clearTimeout(copyTimer);
        copyTimer = setTimeout(() => (copied.value = false), 1600);
    } catch {
        // Clipboard access can be denied; the command stays selectable as text.
    }
};

onBeforeUnmount(() => {
    clearTimeout(copyTimer);
});

const copy = computed(() =>
    isZh.value
        ? {
              searchNav: "搜索文档",
              searchShort: "搜索",
              themeNav: "切换主题",
              menuNav: "打开菜单",
              copyLabel: "复制安装命令",
              eyebrow: "基于 GPUI，经过 Longbridge 生产验证",
              title: "构建出色的高性能桌面应用。",
              lead: "一个综合性的 Rust 桌面开发框架，集完整 UI 系统、数据表格、Dock 布局、图表与代码编辑器于一体，并可用 JavaScript 扩展；从第一天起用于构建 Longbridge Pro。",
              componentsAction: "浏览组件",
              baseAction: "探索 gpui-base",
              signalStars: "GitHub stars",
              signalLicense: "Apache-2.0 许可",
              signalPlatforms: "macOS / Windows / Linux",
              capsKicker: "核心能力",
              capsTitle: "为信息密集型软件而生。",
              capsDescription:
                  "复杂桌面应用真正需要的系统能力，都已整合在框架之中。",
              caps: [
                  {
                      icon: "perf",
                      title: "120 FPS 渲染",
                      description:
                          "每一帧都由 GPU 绘制，高密度界面依然稳定流畅，不掉帧。",
                      apis: ["RenderOnce", "GPU"],
                  },
                  {
                      icon: "table",
                      title: "复杂数据表格",
                      description:
                          "虚拟滚动、列固定、列宽调整、排序与单元格选择，可承载数十万行。",
                      apis: ["Table", "DataTable"],
                  },
                  {
                      icon: "list",
                      title: "高性能虚拟列表",
                      description: "只渲染可见区域，超长列表滚动依然保持流畅。",
                      apis: ["VirtualList", "List"],
                  },
                  {
                      icon: "editor",
                      title: "完整代码编辑器",
                      description:
                          "Rope 存储，20 万行仍保持稳定性能；内置 Tree-sitter 高亮与 LSP 诊断、补全、悬浮提示。",
                      apis: ["Rope", "Tree-Sitter", "LSP", "Highlighter"],
                  },
                  {
                      icon: "dock",
                      title: "Dock 自由布局",
                      description:
                          "面板停靠、拖拽重排、缩放与 Tiles 自由布局，并可序列化保存。",
                      apis: ["DockArea", "DockLayout", "TabGroup"],
                  },
                  {
                      icon: "theme",
                      title: "多主题支持",
                      description:
                          "基于语义化 token 的明暗与多主题切换，而非无尽的样式字段。",
                      apis: ["Theme", "ThemeColor", "ActiveTheme"],
                  },
              ],
              chooseKicker: "三个层次，一个生态",
              chooseTitle: "决定由谁掌控视觉系统。",
              chooseDescription:
                  "使用 gpui-component 保持统一风格，基于 gpui-base 构建自己的设计系统，或用 gpui-shell 让应用可以被 JavaScript 扩展。",
              shipTitle: "保持风格统一",
              shipDescription:
                  "gpui-component 提供完整、成熟且开箱即用的视觉与交互系统。",
              shipPoints: [
                  "60+ 个成品组件",
                  "内置明暗主题",
                  "开箱即用的交互细节",
              ],
              startComponent: "开始使用",
              ownTitle: "拥有设计系统",
              ownDescription:
                  "复用焦点、选择、浮层与虚拟化行为，视觉完全由你决定。",
              ownPoints: [
                  "零样式原语",
                  "完整可访问性行为",
                  "视觉表达 100% 自主",
              ],
              startBase: "阅读 gpui-base 文档",
              scriptTitle: "用 JavaScript 扩展应用",
              scriptDescription:
                  "宿主仍然是 Rust，扩展是 JavaScript：脚本能碰到什么由宿主逐项授予，界面则在同一个进程里画出来。",
              scriptPoints: [
                  "扩展产品不必 fork，也不必发新版本",
                  "默认不授予任何系统能力",
                  "保存文件即 hot-reload，无需重启",
              ],
              startShell: "了解 gpui-shell",
              principleKicker: "设计原则",
              principleLead: "行为属于基础层。",
              principleTail: "视觉属于应用。",
              principleDetail:
                  "gpui-base 处理困难的交互机制：焦点、浮层定位、虚拟化与无障碍；你的产品决定它们最终呈现的样子。",
              footerPrefix: "基于 Apache-2.0 许可证开源，由",
              footerNav: "页脚导航",
              contributors: "贡献者",
              reportBug: "报告问题",
              discussion: "讨论",
              iconCredits: "图标资源来自",
              and: "与",
          }
        : {
              searchNav: "Search documentation",
              searchShort: "Search",
              themeNav: "Toggle color theme",
              menuNav: "Open menu",
              copyLabel: "Copy install command",
              eyebrow: "Built on GPUI. Proven at Longbridge.",
              title: "Build fantastic, high-performance desktop apps.",
              lead: "A comprehensive Rust desktop framework with a complete UI system, data tables, docking, charts, and a code editor — extensible in JavaScript, and used to build Longbridge Pro from day one.",
              componentsAction: "Browse components",
              baseAction: "Explore gpui-base",
              signalStars: "stars on GitHub",
              signalLicense: "Apache-2.0",
              signalPlatforms: "macOS, Windows, Linux",
              capsKicker: "Capabilities",
              capsTitle: "Built for information-dense software.",
              capsDescription:
                  "The systems that real desktop applications need are integrated into one framework.",
              caps: [
                  {
                      icon: "perf",
                      title: "120 FPS rendering",
                      description:
                          "Every frame is drawn by the GPU, so dense interfaces stay smooth instead of dropping frames.",
                      apis: ["RenderOnce", "GPU"],
                  },
                  {
                      icon: "table",
                      title: "Complex data tables",
                      description:
                          "Virtual scrolling, fixed and resizable columns, sorting and cell selection across hundreds of thousands of rows.",
                      apis: ["Table", "DataTable"],
                  },
                  {
                      icon: "list",
                      title: "Virtualized lists",
                      description:
                          "Only the visible range is rendered, so very long lists keep scrolling smoothly.",
                      apis: ["VirtualList", "List"],
                  },
                  {
                      icon: "editor",
                      title: "A real code editor",
                      description:
                          "Rope-backed text that stays stable at 200K lines, with Tree-sitter highlighting and LSP diagnostics, completion and hover.",
                      apis: ["Rope", "Tree-Sitter", "LSP", "Highlighter"],
                  },
                  {
                      icon: "dock",
                      title: "Freeform dock layout",
                      description:
                          "Dockable panels with drag-to-rearrange, zooming and freeform tiles — all serializable.",
                      apis: ["DockArea", "DockLayout", "TabGroup"],
                  },
                  {
                      icon: "theme",
                      title: "Multi-theme support",
                      description:
                          "Light, dark and custom themes driven by semantic tokens instead of endless style fields.",
                      apis: ["Theme", "ThemeColor", "ActiveTheme"],
                  },
              ],
              chooseKicker: "Three layers. One ecosystem.",
              chooseTitle: "Choose who owns the visual system.",
              chooseDescription:
                  "Use gpui-component for a coherent product, build and own your design system on gpui-base, or open the application to JavaScript extensions with gpui-shell.",
              shipTitle: "Keep the product coherent",
              shipDescription:
                  "gpui-component provides a complete, polished visual and interaction system ready to ship.",
              shipPoints: [
                  "60+ finished components",
                  "Light and dark themes included",
                  "Interaction details already handled",
              ],
              startComponent: "Get started",
              ownTitle: "Own the design system",
              ownDescription:
                  "Reuse focus, selection, overlay and virtualization behavior while owning every pixel.",
              ownPoints: [
                  "Zero-style primitives",
                  "Full accessibility behavior",
                  "100% visual ownership",
              ],
              startBase: "Read the gpui-base docs",
              scriptTitle: "Extend it in JavaScript",
              scriptDescription:
                  "The host stays Rust and grants what a script may reach, one capability at a time; the script draws real interface in the same process.",
              scriptPoints: [
                  "Extend the product without a fork or a release",
                  "No capability granted by default",
                  "Hot-reload on save, no restart",
              ],
              startShell: "Explore gpui-shell",
              principleKicker: "Principle",
              principleLead: "Behavior belongs to the foundation.",
              principleTail: "Presentation belongs to the application.",
              principleDetail:
                  "gpui-base handles the difficult interaction mechanics — focus, overlay positioning, virtualization and accessibility. Your product decides how they should look and feel.",
              footerPrefix:
                  "Open source under the Apache-2.0 License, developed by",
              footerNav: "Footer navigation",
              contributors: "Contributors",
              reportBug: "Report Bug",
              discussion: "Discussion",
              iconCredits: "Icon resources by",
              and: "and",
          },
);
</script>

<style lang="scss">
@reference "./.vitepress/theme/style.css";

/* ---------------------------------------------------------------- shell */

.home {
    --page: 1280px;
    --gutter: 1.5rem;
    --section-gap: clamp(3.5rem, 6vw, 5.5rem);
    color: var(--foreground);
}

/* Every band shares one container and one vertical rhythm, with a full-bleed
   hairline between them — without this the sections run together. */
.home > section {
    position: relative;
    border-top: 1px solid var(--border);
}

.home > section:first-child {
    border-top: 0;
}

.band__inner {
    position: relative;
    width: min(100% - 3rem, var(--page));
    margin-inline: auto;
    padding-block: var(--section-gap);
}

.band--principle {
    overflow: hidden;
    background: var(--sidebar);
}

.home-nav {
    position: sticky;
    z-index: 50;
    top: 0;
    height: 3.5rem;
    background: color-mix(in srgb, var(--background) 80%, transparent);
}

/* A toolbar, not a marketing header: the brand and the sections sit together
   on the left, controls collect on the right. */
.home-nav__inner {
    display: flex;
    align-items: center;
    gap: 1.9rem;
    width: min(100% - 3rem, var(--page, 1280px));
    height: 100%;
    margin-inline: auto;
}

.home-nav__rule {
    display: block;
    flex-shrink: 0;
    width: 1px;
    height: 1.1rem;
    background: var(--border);
}

.home-nav__rule--tight {
    margin-inline: 0.15rem;
}

.home-brand {
    display: inline-flex;
    flex-shrink: 0;
    align-items: center;
    gap: 0.55rem;
    color: var(--foreground) !important;
    font-size: 0.875rem;
    font-weight: 620;
    letter-spacing: -0.022em;
    text-decoration: none !important;

    img {
        width: 1.35rem;
        height: 1.4rem;
    }
}

.home-nav nav {
    display: flex;
    align-items: center;
    gap: 0.1rem;
}

.home-nav nav a {
    display: inline-flex;
    align-items: center;
    height: 2rem;
    padding: 0 0.6rem;
    border-radius: var(--radius-control);
    color: var(--foreground);
    font-size: 0.8125rem;
    font-weight: 500;
    text-decoration: none;
    transition:
        color 140ms ease,
        background 140ms ease;

    &:hover {
        background: var(--secondary);
    }
}

.home-nav__group {
    position: relative;

    > button {
        display: inline-flex;
        align-items: center;
        gap: 0.25rem;
        height: 2rem;
        padding: 0 0.6rem;
        border-radius: var(--radius-control);
        color: var(--foreground);
        font-size: 0.8125rem;
        font-weight: 500;
        cursor: pointer;
        transition: background 140ms ease;

        svg {
            width: 0.8rem;
            height: 0.8rem;
            opacity: 0.55;
        }
    }

    &:hover > button {
        background: var(--secondary);
    }
}

.home-nav__menu {
    position: absolute;
    top: calc(100% + 0.4rem);
    left: 0;
    display: grid;
    min-width: 11rem;
    padding: 0.25rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--popover);
    box-shadow: var(--shadow-panel);
    opacity: 0;
    visibility: hidden;
    transition:
        opacity 140ms ease,
        visibility 140ms ease;

    a {
        height: auto;
        padding: 0.4rem 0.55rem;
        font-weight: 450;
        white-space: nowrap;
    }
}

.home-nav__group:hover .home-nav__menu {
    opacity: 1;
    visibility: visible;
}

.home-nav__actions {
    display: flex;
    align-items: center;
    margin-left: auto;
    gap: 0.3rem;
}

/* The search control reads as a real input, which is what it becomes on the
   docs pages. */
.home-nav__search {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    height: 2rem;
    min-width: 11rem;
    padding: 0 0.35rem 0 0.6rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--background);
    color: var(--muted-foreground) !important;
    font-size: 0.8125rem;
    text-decoration: none !important;
    transition: border-color 140ms ease;

    &:hover {
        border-color: color-mix(in srgb, var(--foreground) 22%, var(--border));
    }
    svg {
        width: 0.9rem;
        height: 0.9rem;
    }
    span {
        margin-right: auto;
    }

    kbd {
        padding: 0.1rem 0.3rem;
        border: 1px solid var(--border);
        border-radius: 0.25rem;
        background: var(--secondary);
        color: var(--muted-foreground);
        font: 0.68rem/1.4 var(--vp-font-family-mono);
    }
}

.home-nav__github,
.home-nav__icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 2rem;
    border-radius: var(--radius-control);
    color: var(--foreground) !important;
    text-decoration: none !important;
    transition: background 140ms ease;

    &:hover {
        background: var(--secondary);
    }
    svg {
        width: 0.95rem;
        height: 0.95rem;
    }
}

.home-nav__github {
    gap: 0.4rem;
    padding: 0 0.55rem;
    font-size: 0.8125rem;
    font-weight: 560;
    font-variant-numeric: tabular-nums;
}

.home-nav__icon {
    width: 2rem;
    cursor: pointer;
}

/* Only exists on phones, where the section links are hidden. */
.home-nav__burger {
    display: none;
}

.home-nav__drawer {
    display: grid;
    gap: 0.15rem;
    padding: 0.5rem 0 0.85rem;
    border-top: 1px solid var(--border);
    background: var(--background);

    a {
        padding: 0.6rem 0;
        color: var(--foreground);
        font-size: 0.95rem;
        font-weight: 520;
        text-decoration: none;

        &.is-child {
            padding-left: 0.85rem;
            color: var(--muted-foreground);
            font-size: 0.88rem;
            font-weight: 450;
        }
    }
}

.home-nav__drawer-label {
    padding: 0.85rem 0 0.25rem;
    color: var(--muted-foreground);
    font: 620 0.66rem/1 var(--vp-font-family-mono);
    letter-spacing: 0.11em;
    text-transform: uppercase;
}
.home-nav__icon--text {
    font-size: 0.78rem;
    font-weight: 560;
}

/* ------------------------------------------------------------- primitives */

.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.45rem;
    min-height: 2.6rem;
    padding: 0 1.05rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--background);
    color: var(--foreground) !important;
    font-size: 0.875rem;
    font-weight: 600;
    text-decoration: none !important;
    box-shadow: var(--shadow-raise);
    transition:
        background 150ms ease,
        border-color 150ms ease,
        transform 150ms ease,
        box-shadow 150ms ease;

    svg {
        width: 1rem;
        height: 1rem;
    }
    &:hover {
        background: var(--secondary);
    }
    &:active {
        transform: translateY(1px);
    }
}

.btn--primary {
    border-color: var(--brand);
    background: var(--brand);
    color: var(--brand-contrast) !important;
    box-shadow: var(--shadow-raise);

    &:hover {
        border-color: var(--brand-hover);
        background: var(--brand-hover);
    }
}

.section-head {
    max-width: 44rem;
    margin-bottom: 2.75rem;

    h2 {
        margin: 0.85rem 0 0;
        font-size: clamp(2rem, 3.6vw, 3rem);
        font-weight: 660;
        letter-spacing: -0.045em;
        line-height: 1.04;
    }

    p {
        max-width: 38rem;
        margin: 1rem 0 0;
        color: var(--muted-foreground);
        font-size: 1rem;
        line-height: 1.7;
    }
}

.section-kicker {
    display: inline-flex;
    align-items: center;
    color: var(--muted-foreground);
    font: 600 0.68rem/1 var(--vp-font-family-mono);
    letter-spacing: 0.14em;
    text-transform: uppercase;
}

/* Wide tracking is a Latin small-caps device; on CJK it just pulls the
   characters apart. */
html[lang^="zh"] .section-kicker {
    letter-spacing: 0.04em;
}

/* ------------------------------------------------------------------ hero */

.hero {
    overflow: hidden;
}

/* A hairline blueprint grid, faded out toward the edges. No colour wash — the
   accent stays reserved for signals, so the surface reads as instrumentation
   rather than decoration. */
.hero__grid {
    position: absolute;
    inset: 0;
    background-image:
        linear-gradient(to right, var(--grid-line) 1px, transparent 1px),
        linear-gradient(to bottom, var(--grid-line) 1px, transparent 1px);
    background-size: 64px 64px;
    mask-image: radial-gradient(115% 85% at 30% 0%, black 15%, transparent 74%);
    pointer-events: none;
}

.hero__inner {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 0.93fr) minmax(0, 1.07fr);
    align-items: center;
    gap: clamp(2rem, 4vw, 3.5rem);
    width: min(100% - 3rem, var(--page));
    margin-inline: auto;
    padding-block: clamp(2.75rem, 5vw, 4.25rem);
}

.eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.38rem 0.7rem 0.38rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--background);
    color: var(--foreground) !important;
    font-size: 0.72rem;
    font-weight: 560;
    text-decoration: none !important;
    transition: border-color 150ms ease;

    svg {
        width: 0.82rem;
        height: 0.82rem;
        color: var(--muted-foreground);
    }
    &:hover {
        border-color: var(--brand-line);
    }
}

.eyebrow__pulse {
    position: relative;
    width: 0.4rem;
    height: 0.4rem;
    border-radius: 50%;
    background: var(--success);

    &::after {
        position: absolute;
        inset: -0.15rem;
        border: 1px solid var(--success);
        border-radius: 50%;
        opacity: 0.5;
        content: "";
    }
}

.hero h1 {
    max-width: 18ch;
    margin: 1.25rem 0 0;
    font-size: clamp(2.2rem, 4.3vw, 3.6rem);
    font-weight: 660;
    letter-spacing: -0.042em;
    line-height: 0.98;
}

.hero__lead {
    max-width: min(32rem, 100%);
    margin: 1.25rem 0;
    color: var(--muted-foreground);
    font-size: 1.03rem;
    line-height: 1.7;
}

.hero__actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.65rem;
    margin-top: 1.5rem;
}

.hero__install {
    display: inline-flex;
    align-items: center;
    gap: 0.6rem;
    max-width: 100%;
    margin-top: 1.25rem;
    padding: 0.4rem 0.4rem 0.4rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--sidebar);

    code {
        min-width: 0;
        overflow-x: auto;
        color: var(--foreground);
        font: 0.73rem/1.6 var(--vp-font-family-mono);
        mask-image: linear-gradient(
            to right,
            black calc(100% - 1.25rem),
            transparent
        );
        scrollbar-width: none;
        white-space: nowrap;
    }

    button {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 1.7rem;
        height: 1.7rem;
        border-radius: 0.3rem;
        color: var(--muted-foreground);
        cursor: pointer;
        transition:
            background 140ms ease,
            color 140ms ease;

        svg {
            width: 0.85rem;
            height: 0.85rem;
        }
        &:hover {
            background: var(--secondary);
            color: var(--foreground);
        }
        &[data-copied] {
            color: var(--foreground);
        }
    }
}

.hero__code {
    min-width: 0;

    pre {
        margin: 0;
        padding: 1.05rem 1.25rem 1.25rem;
        overflow-x: auto;
        background: var(--code-bg);
        font: 0.72rem/1.7 var(--vp-font-family-mono);
        scrollbar-width: thin;
        tab-size: 4;
    }

    code {
        color: var(--code-fg);
    }
}

.hero__install-label {
    flex-shrink: 0;
    padding-right: 0.6rem;
    border-right: 1px solid var(--border);
    color: var(--muted-foreground);
    font: 0.66rem/1.6 var(--vp-font-family-mono);
    letter-spacing: 0.04em;
}

/* Concrete, verifiable facts — the fastest way for a first-time visitor to
   judge whether this project is real. */
.hero__signals {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1.5rem;
    margin: 1.5rem 0 0;
    padding: 0;
    list-style: none;
    color: var(--muted-foreground);
    font-size: 0.8rem;

    li {
        display: inline-flex;
        align-items: center;
        gap: 0.4rem;
    }
    strong {
        color: var(--foreground);
        font-weight: 620;
        font-variant-numeric: tabular-nums;
    }
    svg {
        width: 0.9rem;
        height: 0.9rem;
        opacity: 0.75;
    }
}

/* ------------------------------------------------------------------ caps */

.caps__grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1px;
    border: 1px solid var(--border);
    border-radius: var(--radius-surface);
    background: var(--border);
    overflow: hidden;
}

.cap {
    display: flex;
    flex-direction: column;
    padding: 1.75rem;
    background: var(--background);
    transition: background 160ms ease;

    &:hover {
        background: var(--sidebar);
    }
}

.cap__head {
    display: flex;
    align-items: center;
    gap: 0.6rem;

    svg {
        width: 1.05rem;
        height: 1.05rem;
        color: var(--brand);
    }

    h3 {
        margin: 0;
        font-size: 1rem;
        font-weight: 620;
        letter-spacing: -0.015em;
    }
}

.cap p {
    margin: 0.7rem 0 auto;
    color: var(--muted-foreground);
    font-size: 0.875rem;
    line-height: 1.65;
}

.cap__api {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin: 1.25rem 0 0;
    padding: 0;
    list-style: none;

    li {
        padding: 0.18rem 0.42rem;
        border: 1px solid var(--border);
        border-radius: 0.3rem;
        background: var(--secondary);
        color: var(--muted-foreground);
        font: 0.7rem/1.5 var(--vp-font-family-mono);
    }
}

/* Each capability ends in a small preview pane, framed like a control surface
   so the six cards share one rhythm. These are diagrams, not product mocks. */
.cap__preview {
    position: relative;
    display: flex;
    align-items: stretch;
    gap: 0.32rem;
    height: 6rem;
    margin-top: 1.1rem;
    padding: 0.75rem;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--sidebar);
}

/* Every variant lays out inside the same padding box with the same gap, so all
   six panes line up with each other. */
.cap__preview--table,
.cap__preview--list,
.cap__preview--editor {
    flex-direction: column;
    justify-content: center;
    gap: 0.42rem;

    i {
        display: flex;
        align-items: center;
        gap: 0.32rem;
    }

    b {
        height: 0.3rem;
        border-radius: 999px;
        background: var(--input);
    }
}

.cap__preview--table {
    b:first-child {
        flex: 1.6;
    }
    b:nth-child(2) {
        flex: 1;
    }
    b:last-child {
        flex: 0.55;
    }

    /* A header row, then a selected row. */
    i:first-child b {
        background: color-mix(in srgb, var(--foreground) 40%, transparent);
    }
    i:nth-child(3) b:first-child {
        background: var(--data-2);
    }
}

/* A thumb on its own looked like a rendering glitch, so it sits in a track and
   the rows leave room for it. */
.cap__preview--list {
    padding-right: 1.4rem;

    i:nth-child(2) b {
        background: var(--data-2);
    }
}

.cap__track {
    position: absolute;
    top: 0.75rem;
    right: 0.7rem;
    bottom: 0.75rem;
    width: 0.28rem;
    border-radius: 999px;
    background: color-mix(in srgb, var(--foreground) 8%, transparent);
}

.cap__thumb {
    display: block;
    width: 100%;
    height: 42%;
    border-radius: 999px;
    background: color-mix(in srgb, var(--foreground) 30%, transparent);
}

.cap__preview--editor {
    gap: 0.3rem;

    i:nth-child(1) b:nth-child(2) {
        background: var(--data-2);
    }
    i:nth-child(3) b:last-child {
        background: color-mix(in srgb, var(--success) 60%, transparent);
    }
    i:nth-child(5) b:first-child {
        background: color-mix(in srgb, var(--data-2) 55%, transparent);
    }
}

/* Line-number gutter. */
.cap__gutter {
    flex-shrink: 0;
    width: 0.5rem;
    height: 0.28rem;
    border-radius: 999px;
    background: color-mix(in srgb, var(--foreground) 12%, transparent);
    text-decoration: none;
}

/* A dense frame-time trace. Thirty bars, all above the budget line — the point
   is the absence of spikes, which a single flat line failed to convey. */
.cap__preview--perf {
    align-items: flex-end;
    gap: 0.1rem;

    u {
        flex: 1;
        min-width: 0;
        border-radius: 0.08rem 0.08rem 0 0;
        background: linear-gradient(
            to top,
            color-mix(in srgb, var(--data-2) 35%, transparent),
            var(--data-2)
        );
    }
}

.cap__budget {
    position: absolute;
    top: 40%;
    right: 0.75rem;
    left: 0.75rem;
    border-top: 1px dashed
        color-mix(in srgb, var(--foreground) 28%, transparent);
}

.cap__preview--dock {
    b {
        border: 1px solid var(--border);
        border-radius: 0.3rem;
        background: var(--background);
    }

    b:first-child {
        flex: 0.55;
    }
    b:nth-child(2) {
        flex: 1.3;
        border-color: color-mix(in srgb, var(--data-2) 45%, var(--border));
        background: color-mix(in srgb, var(--data-2) 10%, transparent);
    }
    b:last-child {
        flex: 0.8;
    }
}

.cap__preview--theme {
    em {
        flex: 1;
        border: 1px solid var(--border);
        border-radius: 0.3rem;
    }

    em:nth-child(1) {
        background: #0a0a0a;
    }
    em:nth-child(2) {
        background: #404040;
    }
    em:nth-child(3) {
        background: #a3a3a3;
    }
    em:nth-child(4) {
        background: #f5f5f5;
    }
    em:nth-child(5) {
        background: var(--background);
    }
    em:nth-child(6) {
        background: var(--data-2);
    }
}

/* ----------------------------------------------------------------- paths */

.paths__grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1.25rem;
}

.path {
    display: flex;
    flex-direction: column;
    padding: clamp(1.75rem, 3vw, 2.5rem);
    border: 1px solid var(--border);
    border-radius: var(--radius-surface);
    background: var(--background);
    transition:
        border-color 180ms ease,
        box-shadow 180ms ease;

    &:hover {
        border-color: color-mix(in srgb, var(--foreground) 20%, var(--border));
        box-shadow: var(--shadow-panel);
    }

    h3 {
        margin: 1rem 0 0;
        font-size: 1.35rem;
        font-weight: 640;
        letter-spacing: -0.03em;
    }
    p {
        min-height: 3.05rem;
        margin: 0.65rem 0 0;
        color: var(--muted-foreground);
        font-size: 0.92rem;
        line-height: 1.65;
    }

    pre {
        margin: 1.5rem 0 0;
        padding: 1rem 1.1rem;
        overflow-x: auto;
        border: 1px solid var(--border);
        border-radius: var(--radius-card);
        background: var(--code-bg);
        color: var(--code-fg);
        font: 0.78rem/1.75 var(--vp-font-family-mono);
    }

    ul {
        margin: 1.25rem 0 0;
        padding: 0;
        list-style: none;
    }

    li {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.3rem 0;
        color: var(--muted-foreground);
        font-size: 0.84rem;

        svg {
            width: 0.85rem;
            height: 0.85rem;
            flex-shrink: 0;
            color: var(--brand);
        }
    }
}

.path__meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--muted-foreground);
    font: 0.72rem/1 var(--vp-font-family-mono);

    svg {
        width: 1rem;
        height: 1rem;
        color: var(--brand);
    }
}

.path__link {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    margin-top: auto;
    padding-top: 1.5rem;
    color: var(--brand) !important;
    font-size: 0.84rem;
    font-weight: 620;
    text-decoration: none !important;

    svg {
        width: 0.95rem;
        height: 0.95rem;
        transition: transform 160ms ease;
    }
    &:hover svg {
        transform: translateX(3px);
    }
}

.c-type {
    color: var(--code-type);
}
.c-fn {
    color: var(--code-fn);
}
.c-str {
    color: var(--code-string);
}
.c-cmt {
    color: var(--code-comment);
}
.c-kw {
    color: var(--code-keyword);
}
.c-mod {
    color: var(--code-fg);
}

/* ------------------------------------------------------------- principle */

.principle {
    display: grid;
    grid-template-columns: minmax(0, 1.05fr) minmax(0, 0.95fr);
    align-items: end;
    gap: clamp(2rem, 5vw, 4rem);

    blockquote {
        margin: 1.1rem 0 0;
        border: 0;
        padding: 0;
        font-size: clamp(1.5rem, 2.8vw, 2.25rem);
        font-weight: 640;
        letter-spacing: -0.04em;
        line-height: 1.14;

        span {
            display: block;
        }
    }

    p {
        margin: 0;
        color: var(--muted-foreground);
        line-height: 1.7;
    }

    .btn {
        margin-top: 1.5rem;
    }
}

.principle__aside {
    padding-bottom: 0.3rem;
}

.principle__accent {
    color: var(--brand);
}

.principle__grid {
    position: absolute;
    inset: 0;
    background-image:
        linear-gradient(to right, var(--grid-line) 1px, transparent 1px),
        linear-gradient(to bottom, var(--grid-line) 1px, transparent 1px);
    background-size: 48px 48px;
    mask-image: radial-gradient(90% 90% at 100% 50%, black, transparent 68%);
    pointer-events: none;
}

.principle > *:not(.principle__grid) {
    position: relative;
}

/* ---------------------------------------------------------------- footer */

.home-footer {
    display: grid;
    grid-template-columns: minmax(16rem, 1fr) minmax(22rem, auto);
    gap: 2rem 4rem;
    width: min(100% - 3rem, var(--page, 1280px));
    margin: clamp(4.5rem, 8vw, 7rem) auto 0;
    padding: 2.5rem 0;
    border-top: 1px solid var(--border);
    color: var(--muted-foreground);
    font-size: 0.75rem;

    strong {
        color: var(--foreground);
        font-size: 0.82rem;
    }
    &__brand p {
        max-width: 32rem;
        margin: 0.55rem 0 0;
        line-height: 1.6;
    }

    nav {
        display: flex;
        flex-wrap: wrap;
        justify-content: flex-end;
        align-content: start;
        gap: 0.5rem 1.25rem;
    }
    a {
        color: var(--muted-foreground);
        text-decoration: none;
    }
    a:hover {
        color: var(--brand);
    }
    &__credits {
        grid-column: 1 / -1;
        margin: 0;
        padding-top: 1.25rem;
        border-top: 1px solid var(--border);
    }
}

/* ------------------------------------------------------------ responsive */

@media (max-width: 1080px) {
    /* Below this the code window would be too narrow to read, and the hero is
       already complete without it. */
    .hero__inner {
        grid-template-columns: minmax(0, 1fr);
    }

    .hero__code {
        display: none;
    }

    .caps__grid {
        grid-template-columns: repeat(2, 1fr);
    }
}

@media (max-width: 1180px) {
    .paths__grid {
        grid-template-columns: repeat(2, 1fr);
    }
}

@media (max-width: 860px) {
    .paths__grid {
        grid-template-columns: 1fr;
    }

    .principle {
        grid-template-columns: 1fr;
        align-items: start;
    }
}

@media (max-width: 640px) {
    .home-nav__inner {
        grid-template-columns: 1fr auto;
        width: calc(100% - 2rem);
    }
    .home-nav nav {
        display: none;
    }

    .home-nav__burger {
        display: inline-flex;
    }

    /* The drawer is part of the sticky header, so it pushes nothing and closes
       over the page. */
    .home-nav {
        height: auto;
    }

    .home-nav__inner {
        height: 3.5rem;
    }

    .home-nav__drawer {
        width: calc(100% - 2rem);
        margin-inline: auto;
    }
    .home-nav__search span,
    .home-nav__github span {
        display: none;
    }
    .home-nav__search,
    .home-nav__github {
        width: 2rem;
        min-width: auto;
        padding: 0;
    }

    .home-nav__search {
        border-color: transparent;
        background: transparent;
    }

    .home-nav__search:hover {
        border-color: transparent;
        background: var(--secondary);
    }

    .hero__inner,
    .band__inner,
    .home-footer {
        width: calc(100% - 2rem);
    }

    .caps__grid {
        grid-template-columns: 1fr;
    }
    .home-footer {
        grid-template-columns: 1fr;
    }
    .home-footer nav {
        justify-content: flex-start;
    }
}

@media (prefers-reduced-motion: no-preference) {
    .hero__inner > * {
        animation: rise 620ms cubic-bezier(0.16, 1, 0.3, 1) both;
    }
    .hero__inner > :nth-child(2) {
        animation-delay: 70ms;
    }
    .hero__inner > :nth-child(3) {
        animation-delay: 140ms;
    }
    .hero__inner > :nth-child(4) {
        animation-delay: 210ms;
    }
    .hero__inner > :nth-child(5) {
        animation-delay: 280ms;
    }
    .eyebrow__pulse::after {
        animation: ping 2.4s ease-out infinite;
    }
}

@keyframes rise {
    from {
        opacity: 0;
        transform: translateY(0.85rem);
    }
    to {
        opacity: 1;
        transform: none;
    }
}

@keyframes ping {
    0% {
        opacity: 0.55;
        transform: scale(0.8);
    }
    70%,
    100% {
        opacity: 0;
        transform: scale(1.7);
    }
}

@keyframes breathe {
    0%,
    100% {
        box-shadow: 0 0 0 3px
            color-mix(in srgb, var(--success) 20%, transparent);
    }
    50% {
        box-shadow: 0 0 0 5px color-mix(in srgb, var(--success) 8%, transparent);
    }
}
</style>
