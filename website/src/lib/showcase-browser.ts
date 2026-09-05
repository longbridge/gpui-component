/** Shared by the hydrated UI and tests; never changes editorial featured order. */
export function selectShowcases(apps, { query = '', category = 'all', sort = 'newest' } = {}) {
  const terms = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  const matches = apps.filter(app => {
    const text = [app.name, app.author ?? "", app.id, app.description, ...app.platforms, app.source ?? ''].join(' ').toLocaleLowerCase();
    return (category === 'all' || app.category === category) && terms.every(term => text.includes(term));
  });
  const featured = matches.filter(app => app.featured);
  const community = matches.filter(app => !app.featured);
  community.sort((a, b) => {
    if (sort === 'stars' && a.stars !== b.stars) return (b.stars ?? -1) - (a.stars ?? -1);
    return (Date.parse(b.publishedAt) || 0) - (Date.parse(a.publishedAt) || 0) || a.name.localeCompare(b.name);
  });
  return { featured, community };
}
