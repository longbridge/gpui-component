const API_URL = "https://api.github.com/repos/longbridge/gpui-component";
const IS_BUILD = process.env.NODE_ENV === "production";

export default {
  async load() {
    const res = await fetch(API_URL);
    if (!res.ok) {
      if (IS_BUILD) {
        throw new Error(
          `GitHub API request failed: ${res.status} ${res.statusText}`
        );
      }
      console.warn(`[repo.data] GitHub API rate limited (${res.status}), using fallback`);
      return { stargazers_count: 0 };
    }

    const data = await res.json();
    if (typeof data.stargazers_count !== "number") {
      if (IS_BUILD) {
        throw new Error(
          `GitHub API returned unexpected data: ${JSON.stringify(data).slice(0, 200)}`
        );
      }
      console.warn("[repo.data] GitHub API returned unexpected data, using fallback");
      return { stargazers_count: 0 };
    }

    return data;
  },
};
