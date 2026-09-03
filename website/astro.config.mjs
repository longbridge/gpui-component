import { defineConfig } from 'astro/config';
import vue from '@astrojs/vue';
import tailwindcss from '@tailwindcss/vite';
import { ViteToml } from 'vite-plugin-toml';
import remarkMath from 'remark-math';
import rehypeMathjax from 'rehype-mathjax';
import pagefind from 'astro-pagefind';
import { wasmExamplesDevServer } from './src/lib/wasm-middleware.js';

export default defineConfig({
  site: 'https://longbridge.github.io',
  base: '/gpui-component',
  output: 'static',
  trailingSlash: 'never',

  integrations: [
    vue({ devtools: false }),
    pagefind(),
  ],

  i18n: {
    defaultLocale: 'en',
    locales: ['en', 'zh-CN'],
    routing: 'manual',
  },

  markdown: {
    remarkPlugins: [remarkMath],
    rehypePlugins: [rehypeMathjax],
    shikiConfig: {
      themes: {
        light: 'github-light',
        dark: 'github-dark',
      },
      defaultColor: 'light',
      langs: ['rust'],
      langAlias: { rs: 'rust' },
    },
    defaultHighlightLang: 'rust',
  },

  vite: {
    plugins: [tailwindcss(), ViteToml(), wasmExamplesDevServer()],
  },
});
