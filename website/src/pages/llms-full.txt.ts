import type { APIRoute } from 'astro';
import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';
import { buildLlmsContent } from '../lib/llms';

export const GET: APIRoute = () => {
  const thisFile = fileURLToPath(import.meta.url);
  // pages/ → src/ → website root — walk up two levels
  const websiteRoot = join(dirname(thisFile), '../..');
  const content = buildLlmsContent(websiteRoot);
  return new Response(content, {
    headers: {
      'Content-Type': 'text/plain; charset=utf-8',
      'Cache-Control': 'public, max-age=3600',
    },
  });
};
