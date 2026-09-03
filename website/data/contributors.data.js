const IGNORE_LOGINS = ["dependabot[bot]", "copilot"];
const API_URL =
  "https://api.github.com/repos/longbridge/gpui-component/contributors";
const MAX_CONTRIBUTORS = 24;

function requestHeaders() {
  const headers = { Accept: "application/vnd.github+json" };
  const token = process.env.GITHUB_TOKEN;
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }
  return headers;
}

export default {
  async load() {
    try {
      const res = await fetch(API_URL, { headers: requestHeaders() });
      const items = await res.json();
      if (!res.ok || !Array.isArray(items)) {
        console.warn(
          `[contributors] GitHub API returned ${res.status}: ${items?.message ?? "unexpected response"}`,
        );
        return [];
      }
      return items
        .filter((item) => !IGNORE_LOGINS.includes(item.login.toLowerCase()))
        .slice(0, MAX_CONTRIBUTORS);
    } catch (error) {
      console.warn(`[contributors] Failed to fetch contributors: ${error}`);
      return [];
    }
  },
};
