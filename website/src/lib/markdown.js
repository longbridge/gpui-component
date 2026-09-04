// Shiki settings shared by the docs pipeline in `astro.config.mjs` and the
// release-notes renderer in `releases.ts`, so a code block in a release note is
// highlighted exactly like the same block in the docs.
export const shikiConfig = {
  themes: {
    light: 'github-light',
    dark: 'github-dark',
  },
  defaultColor: 'light',
  langs: ['rust'],
  langAlias: { rs: 'rust' },
};

export const defaultHighlightLang = 'rust';
