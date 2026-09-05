import assert from 'node:assert/strict';
import test from 'node:test';
import { selectShowcases } from '../src/lib/showcase-browser.ts';
const apps = [
  { id: 'picked', name: 'Picked', featured: true, category: 'dev', description: 'A database', author: 'Database Team', platforms: ['Linux'], stars: 1, publishedAt: '2026-01-01' },
  { id: 'older', name: 'Older', featured: false, category: 'dev', description: 'A terminal', platforms: ['Linux'], stars: 100, publishedAt: '2026-01-01' },
  { id: 'newer', name: 'Newer', featured: false, category: 'work', description: 'Notes', platforms: ['macOS'], stars: null, publishedAt: '2026-09-01' },
];
test('featured remains separate and community defaults to newest', () => {
  const result = selectShowcases(apps);
  assert.deepEqual(result.featured.map(a => a.id), ['picked']);
  assert.deepEqual(result.community.map(a => a.id), ['newer', 'older']);
});
test('star sorting puts unknown counts last without reordering featured', () => {
  const result = selectShowcases(apps, { sort: 'stars' });
  assert.deepEqual(result.community.map(a => a.id), ['older', 'newer']);
  assert.equal(result.featured[0].id, 'picked');
});
test('search matches descriptions and authors and combines with categories', () => {
  assert.equal(selectShowcases(apps, { query: 'Database Team' }).featured.length, 1);
  assert.equal(selectShowcases(apps, { query: '  TERMINAL ', category: 'dev' }).community[0].id, 'older');
  assert.equal(selectShowcases(apps, { query: 'notes', category: 'dev' }).community.length, 0);
});
