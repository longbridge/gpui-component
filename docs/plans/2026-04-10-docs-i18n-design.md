# Docs I18n Design

## Goal
Add Simplified Chinese documentation to the VitePress site while keeping only `docs/*` content isolated. The home page and shared utility pages remain reusable.

## Decisions
- Use VitePress `locales` instead of a custom header dropdown.
- Keep English docs under `docs/docs/**`.
- Add Chinese docs under `docs/zh/docs/**`.
- Add lightweight wrapper pages for `/zh/`, `/zh/contributors`, and `/zh/skills` so locale navigation stays consistent without duplicating Vue components.
- Keep English and Chinese doc paths mirrored so the built-in language switch can map `/docs/...` to `/zh/docs/...`.

## Scope
- Update VitePress config for `root` and `zh` locales.
- Generate separate sidebars for English and Chinese docs trees.
- Make shared page links locale-aware.
- Scaffold Chinese docs pages so the language switch does not route to missing pages.

## Non-Goals
- Full manual translation of every existing English document.
- Separate Vue implementations for home, contributors, or skills pages.
- Any deployment pipeline changes outside the existing docs build.

## Validation
- `bun run build` from `docs/` must pass.
- Built routes must include both `/docs/...` and `/zh/docs/...`.
- Shared page CTA links must point at the current locale’s docs tree.
