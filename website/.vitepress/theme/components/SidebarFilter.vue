<script setup lang="ts">
import { computed, nextTick, ref, shallowRef, watch } from "vue";
import { useData, useRouter, withBase } from "vitepress";
import { useLayout } from "vitepress/theme";
import VPLink from "vitepress/dist/client/theme-default/components/VPLink.vue";

type Page = { text: string; link: string; group: string };
type SidebarItem = { text?: string; link?: string; items?: SidebarItem[] };

const { lang, page } = useData();
const router = useRouter();
const { sidebarGroups } = useLayout();

const query = ref("");
const activeIndex = ref(0);
const list = shallowRef<HTMLElement | null>(null);

const labels = computed(() =>
  lang.value.startsWith("zh")
    ? { placeholder: "过滤", empty: "没有匹配的页面", clear: "清除" }
    : { placeholder: "Filter", empty: "No matching pages", clear: "Clear" },
);

/** Every linked page of the sidebar the current route resolves to. */
const pages = computed(() => {
  const collected: Page[] = [];

  const walk = (items: SidebarItem[] | undefined, group: string) => {
    for (const item of items ?? []) {
      if (item.link) {
        collected.push({ text: item.text ?? "", link: item.link, group });
      }
      walk(item.items, item.text ?? group);
    }
  };

  walk(sidebarGroups.value as SidebarItem[], "");
  return collected;
});

/**
 * Prefix, then substring, then a loose in-order character match so `dtpick`
 * still finds "DatePicker". The number only orders the list; null drops the
 * page from it.
 */
function score(text: string, query: string): number | null {
  const haystack = text.toLowerCase();
  if (haystack.startsWith(query)) return 0;

  const at = haystack.indexOf(query);
  if (at >= 0) return 1 + at / 1000;

  let matched = 0;
  for (const char of haystack) {
    if (char === query[matched]) matched += 1;
  }
  return matched === query.length ? 2 : null;
}

const results = computed(() => {
  const needle = query.value.trim().toLowerCase();
  if (!needle) return [];

  return pages.value
    .map((item) => ({ item, rank: score(item.text, needle) }))
    .filter((hit): hit is { item: Page; rank: number } => hit.rank !== null)
    .sort(
      (left, right) =>
        left.rank - right.rank || left.item.text.length - right.item.text.length,
    )
    .map((hit) => hit.item);
});

const filtering = computed(() => query.value.trim().length > 0);

// Repeating the same group on every row is noise when the whole result set
// comes from one place — it only earns its space when the rows come from
// different parts of the tree.
const showGroup = computed(() => new Set(results.value.map((item) => item.group)).size > 1);

watch(results, () => {
  activeIndex.value = 0;
});

// A jump lands on the page the reader asked for, so the tree — with that page
// marked active — is what they need next.
watch(() => page.value.relativePath, clear);

function clear() {
  query.value = "";
}

async function move(delta: number) {
  const total = results.value.length;
  if (!total) return;

  activeIndex.value = (activeIndex.value + delta + total) % total;
  await nextTick();
  list.value?.querySelector(".is-active")?.scrollIntoView({ block: "nearest" });
}

function open() {
  const hit = results.value[activeIndex.value];
  if (hit) router.go(withBase(hit.link));
}
</script>

<template>
  <div class="SidebarFilter" :data-filtering="filtering">
    <div class="field">
      <span class="vpi-search icon" />
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
        <span class="vpi-delete icon" />
      </button>
    </div>

    <div v-if="filtering" ref="list" class="results">
      <VPLink
        v-for="(item, index) in results"
        :key="item.link"
        class="result"
        :class="{ 'is-active': index === activeIndex }"
        :href="item.link"
        @mouseenter="activeIndex = index"
      >
        <span class="text">{{ item.text }}</span>
        <span v-if="showGroup && item.group" class="group">{{ item.group }}</span>
      </VPLink>

      <p v-if="!results.length" class="empty">{{ labels.empty }}</p>
    </div>
  </div>
</template>

<style scoped>
.SidebarFilter {
  /* The sidebar's own padding-top equals the nav height, so without this the
     field sits flush against the nav's lower edge. */
  padding-top: 32px;
  padding-bottom: 4px;
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
  width: 0.85rem;
  height: 0.85rem;
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

.input:focus {
  outline: 0;
}

.input::placeholder {
  color: var(--muted-foreground);
}

/* The type=search widget draws its own clear button, which would sit beside
   ours in WebKit. */
.input::-webkit-search-cancel-button {
  display: none;
}

.clear {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-control);
  cursor: pointer;
}

.clear:hover .icon {
  color: var(--foreground);
}

.results {
  padding-top: 10px;
}

.result {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.25rem 0.4rem;
  border-radius: var(--radius);
  color: var(--vp-c-text-2);
  font-size: 0.8125rem;
  line-height: 1.5;
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
/* While a query is active the flat result list stands in for the tree, so the
   groups VitePress renders after this slot step aside. */
#VPSidebarNav:has(.SidebarFilter[data-filtering="true"]) > .group {
  display: none;
}
</style>
