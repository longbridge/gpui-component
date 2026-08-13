import { watchEffect } from "vue";
import { inBrowser, useData, withBase } from "vitepress";

/**
 * Keep the tab icon on the same theme the page is showing.
 *
 * The site's appearance toggle is independent of the operating system, so an
 * icon link selected with `prefers-color-scheme` follows the OS and leaves the
 * dark mark on a light page (and the reverse) whenever the two disagree. One
 * link, repointed here, always matches what the reader is looking at.
 */
export function useThemeFavicon() {
  const { isDark } = useData();

  if (!inBrowser) return;

  watchEffect(() => {
    const link = document.querySelector<HTMLLinkElement>('link[rel="icon"]');
    if (!link) return;

    link.href = withBase(isDark.value ? "/logo-dark.svg" : "/logo.svg");
  });
}
