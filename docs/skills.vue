<template>
    <div class="skills-page">
        <h1>GPUI Component Skills</h1>
        <p class="description">
            Skills available for working with GPUI Component. These skills
            provide guidance and best practices for building GPUI applications.
        </p>
        <div class="skills-list">
            <div v-for="skill in skills" :key="skill.id" class="skill-card">
                <a
                    :href="`https://github.com/longbridge/gpui-component/tree/main/.claude/skills/${skill.id}/SKILL.md`"
                    target="_blank"
                    class="skill-link"
                >
                    <h3 class="skill-name">
                        {{ skill.name }}
                    </h3>
                    <div class="skill-description">{{ skill.description }}</div>
                </a>
            </div>
        </div>
    </div>
</template>

<script setup>
import { data } from "./data/skills.data";
import { ref } from "vue";

const skills = data;
const expandedSkills = ref(new Set());

function toggleSkill(skillId) {
    if (expandedSkills.value.has(skillId)) {
        expandedSkills.value.delete(skillId);
    } else {
        expandedSkills.value.add(skillId);
    }
}

function isExpanded(skillId) {
    return expandedSkills.value.has(skillId);
}

function getPreview(content) {
    // Get first paragraph or first 200 characters
    const lines = content.split("\n").filter((line) => line.trim());
    if (lines.length > 0) {
        const firstLine = lines[0].trim();
        if (firstLine.length > 200) {
            return firstLine.substring(0, 200) + "...";
        }
        return firstLine;
    }
    return content.substring(0, 200) + (content.length > 200 ? "..." : "");
}

function formatMarkdown(content) {
    // Simple markdown to HTML conversion for basic formatting
    let html = content;

    // Code blocks (must come before inline code)
    html = html.replace(/```(\w+)?\n([\s\S]*?)```/gim, (match, lang, code) => {
        const language = lang ? ` class="language-${lang}"` : "";
        return `<pre><code${language}>${escapeHtml(code.trim())}</code></pre>`;
    });

    // Inline code
    html = html.replace(/`([^`\n]+)`/gim, "<code>$1</code>");

    // Headers
    html = html.replace(/^#### (.*$)/gim, "<h4>$1</h4>");
    html = html.replace(/^### (.*$)/gim, "<h3>$1</h3>");
    html = html.replace(/^## (.*$)/gim, "<h2>$1</h2>");
    html = html.replace(/^# (.*$)/gim, "<h1>$1</h1>");

    // Bold and italic
    html = html.replace(/\*\*(.*?)\*\*/gim, "<strong>$1</strong>");
    html = html.replace(/\*(.*?)\*/gim, "<em>$1</em>");

    // Lists
    html = html.replace(/^\- (.*$)/gim, "<li>$1</li>");
    html = html.replace(/^(\d+)\. (.*$)/gim, "<li>$2</li>");

    // Wrap consecutive list items in ul/ol
    html = html.replace(/(<li>.*<\/li>\n?)+/gim, (match) => {
        return "<ul>" + match + "</ul>";
    });

    // Paragraphs (split by double newlines)
    const paragraphs = html.split(/\n\n+/);
    html = paragraphs
        .map((p) => {
            p = p.trim();
            if (!p || p.startsWith("<")) return p; // Already formatted
            return "<p>" + p + "</p>";
        })
        .join("\n\n");

    return html;
}

function escapeHtml(text) {
    // Simple HTML escaping for SSR compatibility
    return text
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
}
</script>

<style lang="scss" scoped>
@reference "./.vitepress/theme/style.css";

.skills-page {
    h1 {
        @apply text-3xl xl:text-4xl font-bold mb-4 text-(--primary);
    }

    .description {
        @apply text-lg text-(--muted-foreground) mb-8;
    }
}

.skills-list {
    @apply flex flex-col gap-6;
}

.skill-card {
    @apply border border-(--border) rounded-lg py-3 gap-3 px-4 hover:bg-(--secondary) transition-colors;

    a {
        @apply no-underline flex flex-col gap-3;
    }

    .skill-name {
        @apply text-xl font-semibold m-0;

        .skill-link {
            @apply text-(--primary) hover:text-(--primary)/80 no-underline;
        }
    }

    .skill-description {
        @apply text-sm text-(--muted-foreground);
    }

    .skill-preview {
        @apply text-sm text-(--muted-foreground) mb-4 italic;
    }

    .skill-content {
        @apply text-sm text-(--foreground) mb-4;

        :deep(h2) {
            @apply text-lg font-semibold mt-4 mb-2 text-(--primary);
        }

        :deep(h3) {
            @apply text-base font-semibold mt-3 mb-2 text-(--primary);
        }

        :deep(p) {
            @apply mb-2;
        }

        :deep(code) {
            @apply bg-(--secondary) px-1.5 py-0.5 rounded text-sm;
        }

        :deep(pre) {
            @apply bg-(--secondary) p-4 rounded-lg overflow-x-auto my-4;

            code {
                @apply bg-transparent p-0;
            }
        }

        :deep(ul),
        :deep(ol) {
            @apply ml-6 mb-2;
        }

        :deep(li) {
            @apply mb-1;
        }
    }

    .skill-actions {
        @apply flex items-center gap-4 mt-4;
    }

    .view-detail-link {
        @apply text-sm text-(--primary) hover:text-(--primary)/80 no-underline font-medium;
    }

    .toggle-button {
        @apply text-sm text-(--muted-foreground) hover:text-(--foreground) underline cursor-pointer bg-transparent border-none p-0;
    }
}
</style>
