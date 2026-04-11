<script setup lang="ts">
import { computed } from "vue";
import { useData } from "vitepress";
import VPFlyout from "vitepress/dist/client/theme-default/components/VPFlyout.vue";
import VPLink from "vitepress/dist/client/theme-default/components/VPLink.vue";

defineProps<{
  screenMenu?: boolean;
}>();

const { site, localeIndex, page, theme, hash } = useData();

const currentLink = computed(
  () =>
    site.value.locales[localeIndex.value]?.link ||
    (localeIndex.value === "root" ? "/" : `/${localeIndex.value}/`),
);

const localeItems = computed(() => {
  const orderedKeys = ["root", "zh-CN"] as const;

  return orderedKeys
    .map((key) => {
      const value = site.value.locales[key];
      if (!value) return null;

      const baseLink = value.link || (key === "root" ? "/" : `/${key}/`);
      const link =
        normalizeLink(
          baseLink,
          theme.value.i18nRouting !== false,
          page.value.relativePath.slice(currentLink.value.length - 1),
          !site.value.cleanUrls,
        ) + hash.value;

      return {
        text: value.label,
        link,
        active: localeIndex.value === key,
      };
    })
    .filter(Boolean) as Array<{ text: string; link: string; active: boolean }>;
});

function ensureStartingSlash(path: string) {
  return /^\//.test(path) ? path : `/${path}`;
}

function normalizeLink(link: string, addPath: boolean, path: string, addExt: boolean) {
  return addPath
    ? link.replace(/\/$/, "") +
        ensureStartingSlash(
          path.replace(/(^|\/)index\.md$/, "$1").replace(/\.md$/, addExt ? ".html" : ""),
        )
    : link;
}
</script>

<template>
  <VPFlyout
    v-if="!screenMenu && localeItems.length"
    class="LanguageSwitcher"
    icon="vpi-languages"
    :label="theme.langMenuLabel || 'Change language'"
  >
    <div class="items">
      <VPLink
        v-for="locale in localeItems"
        :key="locale.text"
        class="link"
        :class="{ active: locale.active }"
        :href="locale.link"
      >
        {{ locale.text }}
      </VPLink>
    </div>
  </VPFlyout>

  <div v-else-if="localeItems.length" class="LanguageSwitcherScreen">
    <p class="title">
      <span class="vpi-languages icon" />
      {{ theme.langMenuLabel || "Languages" }}
    </p>
    <ul class="list">
      <li v-for="locale in localeItems" :key="locale.text" class="item">
        <VPLink class="screen-link" :class="{ active: locale.active }" :href="locale.link">
          {{ locale.text }}
        </VPLink>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.items {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 140px;
}

.link {
  display: block;
  border-radius: 6px;
  padding: 6px 12px;
  font-size: 14px;
  color: var(--vp-c-text-1);
  transition:
    background-color 0.25s,
    color 0.25s;
}

.link:hover {
  background: var(--vp-c-default-soft);
  color: var(--vp-c-brand-1);
}

.link.active {
  background: var(--vp-c-default-soft);
  color: var(--vp-c-text-1);
  font-weight: 600;
}

.LanguageSwitcherScreen {
  padding: 12px 0;
}

.title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 500;
  color: var(--vp-c-text-1);
}

.icon {
  font-size: 16px;
}

.list {
  padding: 8px 0 0 24px;
}

.item + .item {
  margin-top: 4px;
}

.screen-link {
  font-size: 13px;
  color: var(--vp-c-text-1);
}

.screen-link.active {
  color: var(--vp-c-brand-1);
  font-weight: 600;
}
</style>
