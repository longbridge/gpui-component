const API_BASE = 'https://api.github.com/repos/longbridge/gpui-component';

export async function fetchStarCount(): Promise<number> {
  try {
    const res = await fetch(API_BASE);
    if (!res.ok) return 0;
    const data = await res.json();
    return typeof data.stargazers_count === 'number' ? data.stargazers_count : 0;
  } catch {
    return 0;
  }
}

export async function fetchContributors(): Promise<Array<{
  login: string;
  avatar_url: string;
  html_url: string;
  contributions: number;
}>> {
  const IGNORE = ['dependabot[bot]', 'copilot'];
  try {
    const res = await fetch(`${API_BASE}/contributors`);
    if (!res.ok) return [];
    const items = await res.json();
    return items
      .filter((item: any) => !IGNORE.includes(item.login?.toLowerCase()))
      .slice(0, 24);
  } catch {
    return [];
  }
}

export function formatStarCount(count: number): string {
  if (count >= 1000) return `${(count / 1000).toFixed(1)}k`;
  return count.toString();
}
