import { defineConfig } from 'astro/config';
import vue from '@astrojs/vue';
import tailwindcss from '@tailwindcss/vite';
import { ViteToml } from 'vite-plugin-toml';
import remarkMath from 'remark-math';
import rehypeMathjax from 'rehype-mathjax';
import pagefind from 'astro-pagefind';
import { wasmExamplesDevServer } from './src/lib/wasm-middleware.js';
import { shikiConfig, defaultHighlightLang } from './src/lib/markdown.js';

const BASE = '/';

export default defineConfig({
  site: 'https://gpui-kit.com',
  base: BASE,
  output: 'static',
  trailingSlash: 'never',

  integrations: [
    vue({ devtools: false }),
    pagefind(),
  ],

  markdown: {
    remarkPlugins: [remarkMath],
    rehypePlugins: [rehypeMathjax],
    shikiConfig,
    defaultHighlightLang,
  },

  vite: {
    plugins: [tailwindcss(), ViteToml(), wasmExamplesDevServer(BASE)],
  },
});
