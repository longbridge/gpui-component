<script setup lang="ts">
import { computed, nextTick, ref, shallowRef, watch } from "vue";

interface Page { text: string; link: string; group: string; }

const props = defineProps<{
    pages: Page[];
    currentPath: string;
    lang: 'en' | 'zh-CN';
}>();

const query = ref("");
const activeIndex = ref(0);
const list = shallowRef<HTMLElement | null>(null);

const labels = computed(() =>
    props.lang === 'zh-CN'
        ? { placeholder: "过滤", empty: "没有匹配的页面", clear: "清除" }
        : { placeholder: "Filter", empty: "No matching pages", clear: "Clear" },
);

function score(text: string, q: string): number | null {
    const haystack = text.toLowerCase();
    if (haystack.startsWith(q)) return 0;
    const at = haystack.indexOf(q);
    if (at >= 0) return 1 + at / 1000;
    let matched = 0;
    for (const char of haystack) {
        if (char === q[matched]) matched++;
    }
    return matched === q.length ? 2 : null;
}

const results = computed(() => {
    const needle = query.value.trim().toLowerCase();
    if (!needle) return [];
    return props.pages
        .map((item) => ({ item, rank: score(item.text, needle) }))
        .filter((h): h is { item: Page; rank: number } => h.rank !== null)
        .sort((a, b) => a.rank - b.rank || a.item.text.length - b.item.text.length)
        .map((h) => h.item);
});

const filtering = computed(() => query.value.trim().length > 0);
const showGroup = computed(() => new Set(results.value.map((i) => i.group)).size > 1);

watch(results, () => { activeIndex.value = 0; });

function clear() { query.value = ""; }

async function move(delta: number) {
    const total = results.value.length;
    if (!total) return;
    activeIndex.value = (activeIndex.value + delta + total) % total;
    await nextTick();
    list.value?.querySelector(".is-active")?.scrollIntoView({ block: "nearest" });
}

function open() {
    const hit = results.value[activeIndex.value];
    if (hit) window.location.href = hit.link;
}
</script>

<template>
    <div class="SidebarFilter" :data-filtering="filtering">
        <div class="field">
            <svg class="icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
            </svg>
            <input
                v-model="query"
                type="search"
                class="input"
                :placeholder="labels.placeholder"
                :aria-label="labels.placeholder"
                autocomplete="off"
                spellcheck="false"
                @keydown.down.prevent="move(1)"
                @keydown.up.prevent="move(-1)"
                @keydown.enter.prevent="open"
                @keydown.esc.prevent="clear"
            />
            <button v-if="filtering" class="clear" type="button" :aria-label="labels.clear" @click="clear">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M18 6 6 18M6 6l12 12"/>
                </svg>
            </button>
        </div>

        <div v-if="filtering" ref="list" class="results">
            <a
                v-for="(item, index) in results"
                :key="item.link"
                :href="item.link"
                class="result"
                :class="{ 'is-active': index === activeIndex }"
                @mouseenter="activeIndex = index"
            >
                <span class="text">{{ item.text }}</span>
                <span v-if="showGroup && item.group" class="group">{{ item.group }}</span>
            </a>
            <p v-if="!results.length" class="empty">{{ labels.empty }}</p>
        </div>
    </div>
</template>

<style scoped>
.SidebarFilter {
    padding-top: 1.25rem;
    padding-bottom: 0.25rem;
}

.field {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    height: 2rem;
    padding: 0 0.5rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--background);
    transition: border-color 140ms ease;
}

.field:focus-within {
    border-color: color-mix(in srgb, var(--foreground) 22%, var(--border));
}

.icon {
    flex-shrink: 0;
    color: var(--muted-foreground);
}

.input {
    width: 100%;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--foreground);
    font-size: 0.8125rem;
}

.input:focus { outline: 0; }
.input::placeholder { color: var(--muted-foreground); }
.input::-webkit-search-cancel-button { display: none; }

.clear {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    color: var(--muted-foreground);
    border-radius: var(--radius-control);
    cursor: pointer;
}
.clear:hover { color: var(--foreground); }

.results { padding-top: 0.6rem; }

.result {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.25rem 0.4rem;
    border-radius: var(--radius);
    color: var(--muted-foreground);
    font-size: 0.8125rem;
    line-height: 1.5;
    text-decoration: none;
}

.result.is-active {
    background: var(--secondary);
    color: var(--foreground);
}

.group {
    flex-shrink: 0;
    color: var(--muted-foreground);
    font-size: 0.68rem;
}

.empty {
    padding: 0.25rem 0.4rem;
    color: var(--muted-foreground);
    font-size: 0.8125rem;
}
</style>

<style>
/* Hide sidebar tree when filter is active */
.docs-sidebar:has(.SidebarFilter[data-filtering="true"]) .sidebar-group > ul {
    display: none;
}
</style>
