<script setup lang="ts">
import { computed, ref } from "vue";

const props = defineProps<{
    currentLocale: 'en' | 'zh-CN';
    pathname: string;
    langMenuLabel?: string;
    screenMenu?: boolean;
}>();

const isZh = computed(() => props.currentLocale === 'zh-CN');

const localeItems = computed(() => [
    {
        text: 'English',
        link: isZh.value
            ? props.pathname.replace(/\/gpui-component\/zh-CN\//, '/gpui-component/')
            : props.pathname,
        active: props.currentLocale === 'en',
    },
    {
        text: '简体中文',
        link: isZh.value
            ? props.pathname
            : props.pathname.replace(/\/gpui-component\//, '/gpui-component/zh-CN/'),
        active: props.currentLocale === 'zh-CN',
    },
]);

const label = computed(() => props.langMenuLabel || (isZh.value ? '语言' : 'Languages'));
const open = ref(false);
</script>

<template>
    <div v-if="!screenMenu" class="LanguageSwitcher">
        <button
            type="button"
            class="lang-btn"
            :aria-expanded="open"
            @click="open = !open"
            @blur="open = false"
        >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/><path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
            </svg>
            {{ label }}
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M6 9l6 6 6-6"/>
            </svg>
        </button>
        <div v-if="open" class="lang-menu">
            <a
                v-for="locale in localeItems"
                :key="locale.text"
                :href="locale.link"
                class="lang-item"
                :class="{ active: locale.active }"
                @mousedown.prevent
            >
                {{ locale.text }}
            </a>
        </div>
    </div>

    <div v-else class="LanguageSwitcherScreen">
        <p class="screen-title">{{ label }}</p>
        <ul class="screen-list">
            <li v-for="locale in localeItems" :key="locale.text">
                <a
                    :href="locale.link"
                    class="screen-link"
                    :class="{ active: locale.active }"
                >
                    {{ locale.text }}
                </a>
            </li>
        </ul>
    </div>
</template>

<style scoped>
.LanguageSwitcher {
    position: relative;
    display: inline-block;
}

.lang-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    height: 2rem;
    padding: 0 0.6rem;
    border-radius: var(--radius-control);
    color: var(--foreground);
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 140ms ease;
}

.lang-btn:hover {
    background: var(--secondary);
}

.lang-menu {
    position: absolute;
    top: calc(100% + 0.4rem);
    right: 0;
    min-width: 140px;
    padding: 0.25rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--popover);
    box-shadow: var(--shadow-panel);
    z-index: 50;
}

.lang-item {
    display: block;
    padding: 0.35rem 0.7rem;
    border-radius: var(--radius-control);
    color: var(--foreground);
    font-size: 0.875rem;
    text-decoration: none;
    transition: background 140ms ease;
}

.lang-item:hover { background: var(--secondary); }
.lang-item.active { font-weight: 600; }

.LanguageSwitcherScreen { padding: 0.75rem 0; }
.screen-title {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--foreground);
    margin-bottom: 0.5rem;
}

.screen-list { list-style: none; padding: 0; margin: 0; }
.screen-link {
    display: block;
    padding: 0.25rem 0;
    color: var(--foreground);
    font-size: 0.8125rem;
    text-decoration: none;
}
.screen-link.active { color: var(--brand); font-weight: 600; }
</style>
