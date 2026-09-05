<template>
    <div class="apps-page">
        <div class="apps-hero">
            <span class="apps-kicker">{{ copy.kicker }}</span>
            <h1>{{ copy.title }}</h1>
            <p class="apps-lead">{{ copy.lead }}</p>
            <p class="apps-policy">{{ copy.selectionPolicy }}</p>
            <p class="apps-policy">{{ copy.rankingPolicy }}</p>
            <ul class="apps-signals">
                <li><Boxes :size="15" /> {{ copy.signalCount }}</li>
                <li><Monitor :size="15" /> macOS / Windows / Linux</li>
                <li><Github :size="15" /> {{ copy.signalLicense }}</li>
            </ul>
        </div>

        <div class="apps-toolbar">
            <label class="apps-search">
                <span>{{ copy.searchLabel }}</span>
                <input v-model="query" type="search" :placeholder="copy.searchPlaceholder" />
            </label>
            <label class="apps-sort">
                <span>{{ copy.sortLabel }}</span>
                <select v-model="sort">
                    <option value="newest">{{ copy.newest }}</option>
                    <option value="stars">{{ copy.mostStars }}</option>
                </select>
            </label>
        </div>

        <div class="apps-filter" role="group" :aria-label="copy.filterLabel">
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

        <p class="apps-results" aria-live="polite">{{ copy.results(groups.featured.length + groups.community.length) }}</p>
        <section v-for="section in sections" :key="section.id" class="apps-section" :aria-labelledby="`apps-${section.id}`">
            <h2 :id="`apps-${section.id}`">{{ section.title }} <span>{{ section.apps.length }}</span></h2>
            <p class="apps-section__lead">{{ section.description }}</p>
            <div class="apps-grid">
            <article v-for="app in section.apps" :key="app.id" class="app-card">
                <a
                    class="app-card__shot"
                    :href="app.hasReadme ? detailUrl(app.id) : (app.website ?? app.source)"
                    :target="app.hasReadme ? undefined : '_blank'"
                    rel="noopener noreferrer"
                    :aria-label="app.name"
                >
                    <img :src="app.image" :alt="app.name" loading="lazy" decoding="async" />
                </a>
                <div class="app-card__body">
                    <h3 class="app-card__name">{{ app.name }}</h3>
                    <p class="app-card__author">{{ app.author }}</p>
                    <p class="app-card__blurb">{{ app.description }}</p>
                    <ul class="app-card__meta">
                        <li>{{ app.platforms.join(" / ") }}</li>
                        <li>{{ app.source ? copy.openSource : copy.commercial }}</li>
                        <li v-if="app.stars !== null" :title="app.starsUpdatedAt ? `${copy.starsUpdated} ${app.starsUpdatedAt.slice(0, 10)}` : undefined">★ {{ app.stars.toLocaleString() }} GitHub Stars</li>
                        <li v-if="app.publishedAt"><time :datetime="app.publishedAt">{{ copy.published }} {{ app.publishedAt.slice(0, 10) }}</time></li>
                        <li v-if="app.building">{{ copy.building }}</li>
                    </ul>
                    <div class="app-card__links">
                        <a v-if="app.hasReadme" :href="detailUrl(app.id)">{{ copy.details }} <ArrowRight :size="13" /></a>
                        <a v-if="app.website" :href="app.website" target="_blank" rel="noopener noreferrer">
                            {{ copy.visit }} <ArrowUpRight :size="13" />
                        </a>
                        <a v-if="app.source" :href="app.source" target="_blank" rel="noopener noreferrer">
                            {{ copy.sourceLink }} <ArrowUpRight :size="13" />
                        </a>
                    </div>
                </div>
            </article>
            </div>
        </section>
        <div v-if="!groups.featured.length && !groups.community.length" class="apps-empty">
            <p>{{ copy.empty }}</p>
            <button type="button" @click="query = ''; active = 'all'">{{ copy.clearFilters }}</button>
        </div>

        <div class="apps-cta">
            <h2>{{ copy.ctaTitle }}</h2>
            <p>{{ copy.ctaLead }}</p>
            <a
                class="apps-cta__action"
                href="https://github.com/longbridge/gpui-kit-showcases#submit-an-app"
                target="_blank"
                rel="noopener noreferrer"
            >
                {{ copy.ctaAction }} <ArrowRight :size="15" />
            </a>
        </div>
    </div>
</template>

<script setup lang="ts">
import { selectShowcases } from "../lib/showcase-browser.ts";
import { computed, ref } from "vue";
import { ArrowRight, ArrowUpRight, Boxes, Github, Monitor } from "lucide-vue-next";

interface ShowcaseApp {
    id: string; name: string; author: string; hasReadme: boolean; category: string; platforms: string[];
    website: string | null; source: string | null; image: string;
    description: string; building: boolean;
    featured: boolean; publishedAt: string | null; stars: number | null; starsUpdatedAt: string | null;
}

const props = defineProps<{ lang: 'en' | 'zh-CN'; apps: ShowcaseApp[] }>();
const apps = props.apps;
const detailUrl = (id: string) => `${props.lang === "zh-CN" ? "/zh-CN" : ""}/apps/${id}`;

const isZh = computed(() => props.lang === 'zh-CN');
const locale = computed(() => (isZh.value ? "zh" : "en"));

const CATEGORY_LABELS: Record<string, { en: string; zh: string }> = {
    all: { en: "All", zh: "全部" },
    dev: { en: "Developer Tools", zh: "开发工具" },
    terminal: { en: "Terminal & Network", zh: "终端与网络" },
    system: { en: "System & Desktop", zh: "系统与桌面" },
    work: { en: "Productivity & Media", zh: "效率与媒体" },
};

const active = ref("all");
const query = ref("");
const sort = ref("newest");

const categories = computed(() =>
    Object.entries(CATEGORY_LABELS).map(([id, label]) => ({
        id,
        label: label[locale.value as 'en' | 'zh'],
        count: id === "all" ? apps.length : apps.filter((a) => a.category === id).length,
    })),
);

const groups = computed(() => selectShowcases(apps, { query: query.value, category: active.value, sort: sort.value }));
const sections = computed(() => [
    { id: "featured", title: copy.value.featured, description: copy.value.featuredLead, apps: groups.value.featured },
    { id: "community", title: copy.value.community, description: copy.value.communityLead, apps: groups.value.community },
].filter(section => section.apps.length));

const copy = computed(() =>
    isZh.value
        ? { details: "查看详情", starsUpdated: "Stars 更新于", featured: "Featured · 精选应用", featuredLead: "由维护者挑选，展示完整、优质的应用案例。", community: "更多应用", communityLead: "探索社区应用，找到适合你的工具。GitHub Stars 每周及案例 PR 合并后更新。", searchLabel: "搜索应用", searchPlaceholder: "搜索名称、简介、平台或作者…", sortLabel: "更多应用排序", newest: "最新发布", mostStars: "GitHub Stars 最多", published: "发布于", results: (count: number) => `找到 ${count} 个应用`, empty: "没有找到匹配的应用。", clearFilters: "清除筛选", kicker: "应用案例", title: "用 GPUI Kit 做出来的真实应用。", lead: "下面每一个都基于 GPUI Kit 构建，是人们真正下载并每天使用的桌面软件——从生产环境的交易终端，到数据库客户端、终端与系统工具。", selectionPolicy: "向 Showcase 仓库提交 PR，审核合并后，应用都会列在 App Stories 中，但不保证进入 Featured。", rankingPolicy: "Featured 由维护者结合项目历史、实现情况、完整度与品质挑选。我们会根据各应用后续更新和整体情况微调名单，尽量展示更完整、有代表性的应用。其他应用可按发布时间或 GitHub Stars 排序，也可搜索。", signalCount: `${apps.length} 个应用`, signalLicense: "开源与商业产品", filterLabel: "按类别筛选", openSource: "开源", commercial: "商业产品", building: "开发中", visit: "官网", sourceLink: "源码", ctaTitle: "你也用 GPUI Kit 做了应用？", ctaLead: "请在 Showcase 仓库提交 PR，包含应用清单和清晰、完整、整洁的窗口截图。审核合并后自动列在本页，Featured 由维护者另行挑选。", ctaAction: "提交你的应用" }
        : { details: "View details", starsUpdated: "Stars updated", featured: "Featured", featuredLead: "Complete, carefully crafted apps selected by the maintainers.", community: "More apps", communityLead: "Explore apps from the community. GitHub Stars refresh weekly and after Showcase PRs merge.", searchLabel: "Search apps", searchPlaceholder: "Search names, descriptions, platforms or authors…", sortLabel: "Sort more apps", newest: "Newest published", mostStars: "Most GitHub Stars", published: "Published", results: (count: number) => `${count} apps found`, empty: "No apps match your search.", clearFilters: "Clear filters", kicker: "App Stories", title: "Real apps, shipped with GPUI Kit.", lead: "Every app below is built on GPUI Kit — desktop software people download and use every day, from a production trading terminal to database clients, terminals and system utilities.", selectionPolicy: "Every app accepted through a merged PR in the Showcase repository is listed in App Stories. A listing does not guarantee a place in Featured.", rankingPolicy: "Maintainers select Featured apps based on project history, implementation, completeness and quality, and revisit the selection as apps evolve to highlight complete, representative examples. Browse other apps by publication date or GitHub Stars, or search the collection.", signalCount: `${apps.length} apps`, signalLicense: "Open source and commercial", filterLabel: "Filter by category", openSource: "Open source", commercial: "Commercial", building: "In development", visit: "Website", sourceLink: "Source", ctaTitle: "Built something with GPUI Kit?", ctaLead: "Open a PR in the Showcase repository with your app manifest and clear, complete, tidy window screenshots. Every merged app PR is published here automatically; maintainers select Featured apps separately.", ctaAction: "Submit your app" },
);
</script>

<style>
.apps-page { color: var(--foreground); }

.apps-hero { max-width: 46rem; margin-bottom: clamp(2.5rem, 5vw, 3.5rem); }

.apps-kicker {
    display: block; margin-bottom: 0.9rem;
    color: var(--muted-foreground);
    font: 600 0.68rem/1 var(--font-mono);
    letter-spacing: 0.14em; text-transform: uppercase;
}

html[lang^="zh"] .apps-kicker { letter-spacing: 0.06em; }

.apps-hero h1 { margin: 0; border: 0; padding: 0; font-size: clamp(2rem, 3.6vw, 3rem); font-weight: 660; letter-spacing: -0.045em; line-height: 1.1; }
html[lang^="zh"] .apps-hero h1 { letter-spacing: normal; }
.apps-lead { margin: 1.1rem 0 0; color: var(--muted-foreground); font-size: 1.05rem; line-height: 1.7; }

.apps-policy { margin: 0.8rem 0 0; color: var(--muted-foreground); font-size: 0.9rem; line-height: 1.7; }

.apps-signals { display: flex; flex-wrap: wrap; gap: 0.5rem 1.5rem; margin: 1.6rem 0 0; padding: 0; list-style: none; color: var(--muted-foreground); font-size: 0.85rem; font-variant-numeric: tabular-nums; }
.apps-signals li { display: inline-flex; align-items: center; gap: 0.45rem; margin: 0; }

.apps-toolbar { display: flex; flex-wrap: wrap; align-items: end; gap: 1rem; margin-bottom: 1.25rem; }
.apps-search { flex: 1 1 18rem; }
.apps-sort { flex: 0 1 15rem; }
.apps-toolbar label > span { display: block; margin-bottom: 0.4rem; font-size: 0.85rem; font-weight: 550; }
.apps-toolbar input, .apps-toolbar select { width: 100%; border: 1px solid var(--border); border-radius: var(--radius-control); background: var(--card); color: var(--foreground); padding: 0.65rem 0.8rem; font: inherit; }
.apps-toolbar input:focus-visible, .apps-toolbar select:focus-visible { outline: 2px solid var(--brand); outline-offset: 2px; }
.apps-results, .apps-section__lead { color: var(--muted-foreground); font-size: 0.9rem; }
.apps-section { margin-top: 2rem; }
.apps-section + .apps-section { margin-top: 3.5rem; border-top: 1px solid var(--border); padding-top: 2rem; }
.apps-section h2 { margin: 0; padding: 0; border: 0; font-size: 1.5rem; }
.apps-section h2 span { margin-left: 0.4rem; color: var(--muted-foreground); font-size: 0.9rem; font-weight: 400; }
.apps-section__lead { margin: 0.5rem 0 1.5rem; }
.apps-empty { padding: 3rem 1rem; text-align: center; }
.apps-empty button { color: var(--brand); text-decoration: underline; cursor: pointer; }

.apps-filter { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-bottom: 1.75rem; border-top: 1px solid var(--border); padding-top: 1.75rem; }

.apps-filter__chip { display: inline-flex; align-items: center; gap: 0.45rem; border: 1px solid var(--border); border-radius: 999px; padding: 0.35rem 0.85rem; color: var(--foreground); font-size: 0.85rem; line-height: 1.4; cursor: pointer; transition: background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease; }
.apps-filter__chip:hover { background: var(--secondary); }
.apps-filter__chip[aria-pressed="true"] { border-color: var(--brand); background: var(--brand); color: var(--brand-contrast); }
.apps-filter__count { color: var(--muted-foreground); font-size: 0.75rem; font-variant-numeric: tabular-nums; }
.apps-filter__chip[aria-pressed="true"] .apps-filter__count { color: var(--brand-contrast); opacity: 0.65; }

.apps-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(20rem, 100%), 1fr)); gap: 1.5rem; }

.app-card { display: flex; flex-direction: column; overflow: hidden; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--card); transition: border-color 0.18s ease, box-shadow 0.18s ease; }
.app-card:hover { border-color: var(--brand-line); box-shadow: var(--shadow-raise); }
.app-card__shot { display: block; border-bottom: 1px solid var(--border); background: var(--secondary); }
.app-card__shot img { display: block; width: 100%; aspect-ratio: 16 / 10; object-fit: contain; object-position: center; }
.app-card__body { display: flex; flex: 1; flex-direction: column; padding: 1.15rem 1.25rem 1.25rem; }
.app-card__name { margin: 0; border: 0; padding: 0; font-size: 1.05rem; font-weight: 620; letter-spacing: -0.015em; line-height: 1.3; }
html[lang^="zh"] .app-card__name { letter-spacing: normal; }
.app-card__author { margin: 0.35rem 0 0; color: var(--muted-foreground); font-size: 0.8rem; }
.app-card__blurb { margin: 0.55rem 0 auto; padding-bottom: 1rem; color: var(--muted-foreground); font-size: 0.875rem; line-height: 1.65; }
.app-card__meta { display: flex; flex-wrap: wrap; gap: 0.4rem; margin: 0 0 0.9rem; padding: 0; list-style: none; }
.app-card__meta li { margin: 0; border: 1px solid var(--border); border-radius: var(--radius-control); padding: 0.15rem 0.45rem; color: var(--muted-foreground); font-size: 0.72rem; line-height: 1.5; white-space: nowrap; }
.app-card__links { display: flex; flex-wrap: wrap; gap: 1rem; border-top: 1px solid var(--border); padding-top: 0.9rem; }
.app-card__links a { display: inline-flex; align-items: center; gap: 0.2rem; color: var(--foreground); font-size: 0.85rem; font-weight: 500; text-decoration: none; transition: opacity 0.15s ease; }
.app-card__links a:hover { opacity: 0.66; }

.apps-cta { margin-top: clamp(3rem, 6vw, 4.5rem); border-top: 1px solid var(--border); padding-top: clamp(2rem, 4vw, 3rem); }
.apps-cta h2 { margin: 0; border: 0; padding: 0; font-size: 1.4rem; font-weight: 640; letter-spacing: -0.02em; line-height: 1.3; }
html[lang^="zh"] .apps-cta h2 { letter-spacing: normal; }
.apps-cta p { margin: 0.6rem 0 1.3rem; color: var(--muted-foreground); font-size: 0.95rem; line-height: 1.7; }
.apps-cta__action { display: inline-flex; align-items: center; gap: 0.4rem; border-radius: var(--radius-control); background: var(--brand); padding: 0.55rem 1.1rem; color: var(--brand-contrast); font-size: 0.9rem; font-weight: 500; text-decoration: none; transition: background-color 0.15s ease; }
.apps-cta__action:hover { background: var(--brand-hover); }

@media (max-width: 640px) { .apps-grid { gap: 1.15rem; } }
</style>
