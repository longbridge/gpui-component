<template>
    <div class="releases-page">
        <div class="releases-hero">
            <span class="releases-kicker">{{ copy.kicker }}</span>
            <h1>{{ copy.title }}</h1>
            <p class="releases-lead">{{ copy.lead }}</p>
            <ul v-if="releases.length" class="releases-signals">
                <li><Tag /> {{ copy.latest }} {{ releases[0].name }}</li>
                <li><Layers /> {{ copy.count }}</li>
                <li>
                    <Github />
                    <a
                        :href="`https://github.com/${REPO}/releases`"
                        target="_blank"
                        rel="noopener noreferrer"
                        >{{ copy.github }}</a
                    >
                </li>
            </ul>
        </div>

        <p v-if="!releases.length" class="releases-empty">
            {{ copy.empty }}
            <a
                :href="`https://github.com/${REPO}/releases`"
                target="_blank"
                rel="noopener noreferrer"
                >{{ copy.emptyLink }}</a
            >
        </p>

        <article
            v-for="(release, index) in releases"
            :id="release.tag"
            :key="release.tag"
            class="release"
        >
            <header class="release__header">
                <h2 class="release__version">
                    <a :href="`#${release.tag}`">{{ release.name }}</a>
                </h2>
                <div class="release__meta">
                    <time :datetime="release.date">{{ release.date }}</time>
                    <span v-if="index === 0" class="release__badge">
                        {{ copy.latestBadge }}
                    </span>
                    <span v-if="release.prerelease" class="release__badge">
                        {{ copy.prerelease }}
                    </span>
                    <a
                        :href="release.url"
                        class="release__link"
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        GitHub <ArrowUpRight />
                    </a>
                </div>
            </header>
            <!-- Rendered at build time from the GitHub release body, by the
                 same markdown pipeline as the docs. -->
            <div class="release__body" v-html="release.html" />
        </article>
    </div>
</template>

<script setup>
import { computed } from "vue";
import { useData } from "vitepress";
import { ArrowUpRight, Github, Layers, Tag } from "lucide-vue-next";
import { data as releases } from "./data/releases.data";

const REPO = "longbridge/gpui-kit";

const { localeIndex } = useData();
const isZh = computed(() => localeIndex.value === "zh-CN");

const copy = computed(() =>
    isZh.value
        ? {
              kicker: "版本发布",
              title: "发布说明",
              lead: "GPUI Kit 每个已发布版本，以及它在 GitHub Release 中的说明，最新版本在最前。说明保留 GitHub 上的英文原文。",
              latest: "最新版本",
              count: `${releases.length} 个版本`,
              github: "在 GitHub 上查看全部版本",
              latestBadge: "最新",
              prerelease: "预发布",
              empty: "构建时未能读取发布说明。",
              emptyLink: "前往 GitHub 查看版本发布。",
          }
        : {
              kicker: "Releases",
              title: "Release notes",
              lead: "Every published version of GPUI Kit, with the notes from its GitHub release. The newest version comes first.",
              latest: "Latest",
              count: `${releases.length} releases`,
              github: "All releases on GitHub",
              latestBadge: "Latest",
              prerelease: "Pre-release",
              empty: "The release notes could not be loaded when this page was built.",
              emptyLink: "See the releases on GitHub.",
          },
);
</script>

<style scoped>
.releases-page {
    color: var(--foreground);
}

/* -------------------------------------------------------------- hero */

.releases-hero {
    max-width: 46rem;
    margin-bottom: clamp(2.5rem, 5vw, 3.5rem);
}

.releases-kicker {
    display: block;
    margin-bottom: 0.9rem;
    color: var(--muted-foreground);
    font-family: var(--vp-font-family-mono, ui-monospace, monospace);
    font-size: 0.68rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
}

html[lang^="zh"] .releases-kicker {
    letter-spacing: 0.06em;
}

.releases-hero h1 {
    margin: 0;
    border: 0;
    padding: 0;
    font-size: clamp(2rem, 3.6vw, 3rem);
    font-weight: 660;
    letter-spacing: -0.045em;
    line-height: 1.1;
}

html[lang^="zh"] .releases-hero h1 {
    letter-spacing: normal;
}

.releases-lead {
    margin: 1.1rem 0 0;
    color: var(--muted-foreground);
    font-size: 1.05rem;
    line-height: 1.7;
}

.releases-signals {
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

.releases-signals li {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    margin: 0;
}

.releases-signals :deep(svg) {
    width: 0.95rem;
    height: 0.95rem;
    opacity: 0.7;
}

.releases-empty {
    max-width: 46rem;
    color: var(--muted-foreground);
}

/* ----------------------------------------------------------- release */

/* Each version is a band under a hairline: the version and date on the
   left stay in view while its notes scroll, so a reader skimming down the
   page always knows which release they are in. */
.release {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 1rem 3rem;
    border-top: 1px solid var(--border);
    padding: 2rem 0 2.5rem;
}

@media (min-width: 960px) {
    .release {
        grid-template-columns: 13rem minmax(0, 1fr);
        padding: 2.5rem 0 3rem;
    }
}

.release__header {
    align-self: start;
}

@media (min-width: 960px) {
    .release__header {
        position: sticky;
        top: calc(var(--vp-nav-height) + 1.5rem);
    }
}

.release__version {
    margin: 0;
    border: 0;
    padding: 0;
    font-size: 1.35rem;
    font-weight: 640;
    letter-spacing: -0.022em;
    line-height: 1.2;
    font-variant-numeric: tabular-nums;
}

.release__version a {
    color: inherit;
    text-decoration: none;
}

.release__meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem 0.75rem;
    margin-top: 0.6rem;
    color: var(--muted-foreground);
    font-size: 0.85rem;
    font-variant-numeric: tabular-nums;
}

.release__badge {
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.05rem 0.55rem;
    font-size: 0.72rem;
    line-height: 1.5;
}

.release__link {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    color: inherit;
    text-decoration: none;
}

.release__link:hover {
    color: var(--foreground);
}

.release__link :deep(svg) {
    width: 0.85rem;
    height: 0.85rem;
}

/* The notes are GitHub markdown: mostly lists, with a heading per component
   in the larger releases. Docs-level heading rhythm would be too loud for a
   changelog, so headings inside a release are compact and unruled. */
.release__body {
    min-width: 0;
    max-width: 46rem;
}

.release__body :deep(h1),
.release__body :deep(h2),
.release__body :deep(h3),
.release__body :deep(h4) {
    margin: 1.5rem 0 0.5rem;
    border: 0;
    padding: 0;
    font-size: 1rem;
    font-weight: 640;
    letter-spacing: -0.012em;
    line-height: 1.4;
}

.release__body :deep(> :first-child) {
    margin-top: 0;
}

.release__body :deep(p),
.release__body :deep(ul),
.release__body :deep(ol) {
    max-width: none;
}

.release__body :deep(img) {
    max-width: 100%;
    height: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
}
</style>
